//! Code Generator - Pure Generation Pass
//!
//! This module implements pure code generation that processes SchemaDiscovery and
//! TypeAnalysis results and produces final Rust code without any analysis.
//!
//! This is the FOURTH pass in our 4-pass compilation system:
//! Discovery → Analysis → Optimization → **Generation**

use crate::TctResult;
use crate::generator::encoding_decision::{
    CompilationConstraints, EncodingDecision, EncodingStrategy,
};
use crate::generator::generation_output::{CodeGeneration, RootCodeGeneration};
use crate::generator::pattern_validator_generator::PatternValidatorGenerator;
use crate::generator::root_generator::RootGenerator;
use crate::generator::schema_discovery::{MemberType, SchemaDiscovery, TypeDiscovery, TypeKind};
use crate::generator::type_analysis::{TraitDerivability, TypeAnalysis};
use proc_macro2::TokenStream;
use quote::quote;

mod runtime_support;
mod type_dispatch;
mod type_helpers;

use runtime_support::{
    generate_codec_helpers as render_codec_helpers,
    generate_collection_encoders as render_collection_encoders,
    generate_number_encoders as render_number_encoders,
    generate_partial_encoding_types as render_partial_encoding_types,
};

/// Pure code generator - NO analysis, ONLY code generation
pub struct CodeGenerator {
    /// Schema discovery input
    discovery: SchemaDiscovery,
    /// Type analysis input
    analysis: TypeAnalysis,
    /// Encoding decisions from optimization pass (optional for backwards compatibility)
    encoding_strategy: Option<EncodingStrategy>,
    /// Compilation constraints that affect emitted token ranges.
    compilation_constraints: CompilationConstraints,
    /// Code generation results being built
    generation: CodeGeneration,
    /// Whether streaming-friendly encoding is enabled
    streaming_encode: bool,
    /// Base for multi-token encoding (for Vec lengths, unbounded integers, etc.)
    encoding_base: u32,
    /// Maximum collection length accepted by strict generated decoders.
    max_collection_length: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct CodeGeneratorOptions {
    pub max_vocab: Option<u32>,
    pub max_base_vocab: Option<u32>,
    pub compilation_constraints: CompilationConstraints,
    pub streaming_encode: bool,
    pub max_collection_length: usize,
    /// Digit-aligned decimal text encoding for unbounded i64/u64/f64 (opt-in;
    /// token streams are incompatible with the default bit-split encoding).
    pub decimal_number_encoding: bool,
}

impl Default for CodeGeneratorOptions {
    fn default() -> Self {
        Self {
            max_vocab: None,
            max_base_vocab: None,
            compilation_constraints: CompilationConstraints::default(),
            streaming_encode: false,
            max_collection_length: 2048,
            decimal_number_encoding: false,
        }
    }
}

impl CodeGenerator {
    /// Create new code generator
    pub fn new(
        discovery: SchemaDiscovery,
        analysis: TypeAnalysis,
        encoding_strategy: Option<EncodingStrategy>,
        compilation_constraints: CompilationConstraints,
        streaming_encode: bool,
        encoding_base: u32,
        max_collection_length: usize,
    ) -> Self {
        Self {
            discovery,
            analysis,
            encoding_strategy,
            compilation_constraints,
            generation: CodeGeneration::new(),
            streaming_encode,
            encoding_base,
            max_collection_length,
        }
    }

    /// Calculate the maximum structural token ID from schema structure
    ///
    /// This provides a correct alternative to EncodingStrategy::update_stats(),
    /// which has known bugs for BoundedInteger and PatternOptimized encodings
    /// (see encoding_decision.rs:206, 218-221).
    ///
    /// Token ID assignment rules:
    /// - Enums: variants use 0-indexed IDs → n variants = tokens 0 to (n-1)
    /// - Structs: optional field presence uses tokens 0 (absent) and 1 (present)
    ///
    /// Returns the maximum token ID that can appear in the token stream.
    pub fn calculate_max_structural_token_id(&self) -> u32 {
        // Handle None case - no encoding strategy means no structural tokens
        let Some(ref encoding_strategy) = self.encoding_strategy else {
            return 0;
        };

        let mut max_token_id = 0u32;

        for (type_name, type_discovery) in self.discovery.discovered_types.iter() {
            let decision = encoding_strategy.get_decision(type_name);
            let decision_max = match decision {
                // NO TOKEN ALLOCATION - Skip these
                EncodingDecision::Standard => match &type_discovery.type_kind {
                    TypeKind::Enum => type_discovery.members.len().saturating_sub(1) as u32,
                    _ => continue,
                },
                EncodingDecision::BitPackedStruct { .. } => continue,
                EncodingDecision::SingleVariantEnum { .. } => continue,
                EncodingDecision::ConstantInteger { .. } => continue,
                EncodingDecision::FixedSizeArray { .. } => continue,
                EncodingDecision::FormatOptimized { .. } => continue,

                // TOKEN ALLOCATION - Calculate max_token_id
                EncodingDecision::EnumIndex { variants } => {
                    // Enum uses tokens 0 to (variants.len() - 1)
                    // Use saturating_sub to handle empty enums gracefully
                    variants.len().saturating_sub(1) as u32
                }
                EncodingDecision::EnumCodec { codec_spec, .. } => {
                    // Codec digits use tokens 0 to (base - 1)
                    codec_spec.base - 1
                }
                EncodingDecision::BoundedInteger { max, .. } => {
                    // Direct value encoding - uses max value itself as token
                    max as u32
                }
                EncodingDecision::BoundedIntegerOffset { range, .. } => {
                    // Offset encoding uses variable-base digits plus a width marker.
                    // The digit alphabet is covered by encoding_base; this is the
                    // actual emitted token range, not the semantic value range.
                    if range == 0 {
                        continue;
                    }
                    self.encoding_base.saturating_sub(1)
                }
                EncodingDecision::BoundedIntegerCodec { codec_spec, .. } => {
                    // Fixed-width codec digits use the codec base, not the
                    // represented semantic value range.
                    codec_spec.base.saturating_sub(1)
                }
                EncodingDecision::PatternOptimized { strategy, .. } => {
                    // Sequential pattern tokens 0 to (usage - 1)
                    let usage = strategy.estimated_vocabulary_usage();
                    if usage > 0 {
                        (usage - 1) as u32
                    } else {
                        continue;
                    }
                }
            };

            max_token_id = max_token_id.max(decision_max);
        }

        max_token_id
    }

    /// Generate code from discovery and analysis results
    ///
    /// If encoding_strategy is None, uses standard encoding for all types.
    /// If max_vocab is Some(N), generates vocabulary-constrained helpers for collection lengths and unconstrained numbers.
    /// If max_base_vocab is Some(N), uses that for multi-token encoding base; otherwise defaults to 256.
    pub(crate) fn generate(
        discovery: SchemaDiscovery,
        analysis: TypeAnalysis,
        encoding_strategy: Option<EncodingStrategy>,
        options: CodeGeneratorOptions,
        // Note: max_string_length removed - end-marker encoding eliminates length constraints
    ) -> TctResult<CodeGeneration> {
        let discovery = Self::apply_member_type_refinements(discovery, encoding_strategy.as_ref());

        crate::log_debug!(
            "⚡ Starting CodeGenerator with {} discovered types",
            discovery.discovered_types.len()
        );

        if let Some(ref strategy) = encoding_strategy {
            crate::log_debug!(
                "🎯 Using encoding strategy with {} decisions",
                strategy.decisions.len()
            );
        } else {
            crate::log_debug!("📝 No encoding strategy provided - using standard encoding");
        }

        if let Some(vocab_size) = options.max_vocab {
            crate::log_debug!("🔢 Vocabulary constraint enabled: max_vocab={}", vocab_size);
        }

        // Compute encoding_base early - needed for StructGenerator
        let encoding_base =
            crate::optimization_pass::config::OptimizationConfig::compute_encoding_base(
                options.max_base_vocab,
            );

        let mut generator = Self::new(
            discovery,
            analysis,
            encoding_strategy,
            options.compilation_constraints,
            options.streaming_encode,
            encoding_base,
            options.max_collection_length,
        );

        // Log generation order
        crate::log_debug!(
            "📋 Generation order: {:?}",
            generator.analysis.generation_order
        );

        // Generate types in dependency order (from analysis) - clone to avoid borrow conflicts
        let generation_order = generator.analysis.generation_order.clone();
        for type_name in &generation_order {
            crate::log_debug!("🔨 Generating type: '{}'", type_name);

            generator.generate_type(type_name)?;
        }

        // Generate pattern validators
        crate::log_debug!("🎯 Generating pattern validators");
        generator.generate_pattern_validators()?;

        // Generate root type if present
        crate::log_debug!("🌱 Generating root type");
        generator.generate_root()?;

        // Build final definitions structure
        crate::log_debug!("🏗️  Building final definitions structure");
        generator.generation.build_definitions();

        // Generate BASE_VOCAB_SIZE constant (total base vocabulary before BPE)
        // This is ALWAYS generated, regardless of whether --max-vocab is specified
        //
        // BASE_VOCAB_SIZE = max(string_vocab, structural_vocab)
        // - string_vocab: 257 for UTF-8 (256 bytes + END_STRING), 129 for ASCII (128 + END_STRING)
        // - structural_vocab: enum variants, field IDs, bounded integers, etc.
        //
        // This matches the base_vocab calculation in Grammar construction and VOCAB_SIZE formula.
        let structural_vocab_size = if generator.encoding_strategy.is_some() {
            // Calculate directly from schema structure (correct approach)
            // Note: We use calculate_max_structural_token_id() instead of
            // encoding_strategy.stats.max_structural_token_id_plus_one because
            // EncodingStrategy::update_stats() has known bugs for BoundedInteger
            // and PatternOptimized encodings (see encoding_decision.rs:206, 218-221).
            let max_token_id = generator.calculate_max_structural_token_id();
            max_token_id + 1
        } else {
            0 // No optimizations = no vocabulary allocated
        };

        let string_vocab_size = generator
            .compilation_constraints
            .string_encoding
            .base_vocab_size();

        // Use encoding_base from generator (computed earlier)
        let encoding_base = generator.encoding_base;

        // BASE_VOCAB_SIZE = max(string_vocab, structural_vocab, encoding_base)
        // This ensures BPE tokens start after ALL base tokens:
        // - String content bytes (0-255 + END_MARKER)
        // - Structural tokens (enum variants, field IDs, bounded integers)
        // - Multi-token encoding base (for unbounded values)
        let base_vocab_size = string_vocab_size
            .max(structural_vocab_size)
            .max(encoding_base);
        generator.generation.stats.base_vocab_size = base_vocab_size;

        crate::log_debug!(
            "🔢 Generating BASE_VOCAB_SIZE constant: {} (string={}, structural={}, encoding_base={})",
            base_vocab_size,
            string_vocab_size,
            structural_vocab_size,
            encoding_base
        );
        let base_vocab_constant = quote! {
            /// Total base vocabulary size before BPE tokens.
            ///
            /// Formula: BASE_VOCAB_SIZE = max(string_vocab, structural_vocab, encoding_base)
            /// - string_vocab: 257 for UTF-8 (256 bytes + END_STRING), 129 for ASCII
            /// - structural_vocab: enum variants, field IDs, bounded integers, etc.
            /// - encoding_base: multi-token encoding base (default 256, configurable)
            ///
            /// BPE tokens start at index BASE_VOCAB_SIZE.
            pub const BASE_VOCAB_SIZE: u32 = #base_vocab_size;
        };
        generator
            .generation
            .definitions
            .constants
            .push(base_vocab_constant);

        // Generate MAX_VOCAB constant and codec helpers if vocabulary constraint is enabled
        if let Some(vocab_size) = options.max_vocab {
            crate::log_debug!("🔢 Generating MAX_VOCAB constant: {}", vocab_size);
            let max_vocab_constant = quote! {
                /// Maximum vocabulary size constraint - all token values must be in [0, MAX_VOCAB-1]
                const MAX_VOCAB: u32 = #vocab_size;
            };
            generator
                .generation
                .definitions
                .constants
                .push(max_vocab_constant);

            crate::log_debug!("🛠️  Generating vocabulary-constrained codec helpers");
            let codec_helpers = Self::generate_codec_helpers(vocab_size);
            generator
                .generation
                .definitions
                .constants
                .push(codec_helpers);

            crate::log_debug!("🔁 Generating vocabulary-constrained collection encoders");
            let collection_encoders =
                Self::generate_collection_encoders(generator.max_collection_length);
            generator
                .generation
                .definitions
                .constants
                .push(collection_encoders);

            crate::log_debug!("🔢 Generating vocabulary-constrained number encoders");
            let number_encoders = Self::generate_number_encoders(options.decimal_number_encoding);
            generator
                .generation
                .definitions
                .constants
                .push(number_encoders);
        }

        // Generate partial encoding support data structures (always generated)
        crate::log_debug!("🔄 Generating partial encoding data structures");
        let partial_encoding_types = Self::generate_partial_encoding_types();
        generator
            .generation
            .definitions
            .constants
            .push(partial_encoding_types);

        crate::log_debug!("📦 Final definitions built:");
        crate::log_debug!(
            "   • Structs: {}",
            generator.generation.definitions.structs.len()
        );
        crate::log_debug!(
            "   • Enums: {}",
            generator.generation.definitions.enums.len()
        );

        crate::log_debug!("✅ CodeGenerator completed successfully!");
        crate::log_debug!("📊 Generation Statistics:");
        crate::log_debug!(
            "   • Types generated: {}",
            generator.generation.type_definitions.len()
        );
        crate::log_debug!(
            "   • Root generation: {}",
            if generator.generation.root_generation.is_some() {
                "YES"
            } else {
                "NO"
            }
        );
        crate::log_debug!(
            "   • Definitions created: {}",
            generator.generation.definitions.structs.len()
                + generator.generation.definitions.enums.len()
        );

        Ok(generator.generation)
    }

    /// Generate root type (if schema has root type)
    fn generate_root(&mut self) -> TctResult<()> {
        let root_generator = RootGenerator::new();

        // Generate root using RootGenerator with closures for dependencies
        let root_generation = root_generator.generate_root(
            &self.discovery,
            &self.analysis,
            &|type_name| self.generation.has_type(type_name),
            &|type_discovery, trait_derivability| {
                self.generate_root_composition_direct(type_discovery, trait_derivability)
            },
        )?;

        // Set root generation if composition was generated
        if let Some(root_gen) = root_generation {
            self.generation.set_root_generation(root_gen);
        }

        Ok(())
    }

    /// Generate root composition directly for root_generation (no type_definitions entry)
    /// This prevents duplicates by bypassing the type_definitions -> build_definitions path
    fn generate_root_composition_direct(
        &self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TctResult<RootCodeGeneration> {
        let root_generator = RootGenerator::new();

        root_generator.generate_root_composition_direct(
            &self.discovery,
            type_discovery,
            trait_derivability,
            &|member_type| self.generate_member_type(member_type),
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits(trait_derivability, has_pattern_validators)
            },
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits_without_deserialize(
                    trait_derivability,
                    has_pattern_validators,
                )
            },
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits_without_default(
                    trait_derivability,
                    has_pattern_validators,
                )
            },
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits_without_deserialize_and_default(
                    trait_derivability,
                    has_pattern_validators,
                )
            },
            self.streaming_encode,
        )
    }

    /// Generate pattern validator structs
    fn generate_pattern_validators(&mut self) -> TctResult<()> {
        let pattern_validator_generator =
            PatternValidatorGenerator::new().with_streaming_encode(self.streaming_encode);

        let validators = pattern_validator_generator
            .generate_all(&self.discovery.pattern_validators, &|rust_type| {
                self.parse_rust_type(rust_type)
            })?;

        for validator in validators {
            self.generation.add_type_generation(validator);
        }

        Ok(())
    }

    /// Generate root composition (oneOf/anyOf at schema root)
    fn generate_derive_traits(
        &self,
        trait_derivability: Option<&TraitDerivability>,
        has_pattern_validators: bool,
    ) -> TokenStream {
        type_helpers::generate_derive_traits(
            trait_derivability,
            has_pattern_validators,
            self.streaming_encode,
        )
    }

    /// Generate derive traits excluding Default (for complex enums that need manual Default impl)
    ///
    /// Used when an enum has a first variant with data, making #[default] attribute unusable.
    /// In this case, enum_generator will provide a manual impl Default.
    fn generate_derive_traits_without_default(
        &self,
        trait_derivability: Option<&TraitDerivability>,
        has_pattern_validators: bool,
    ) -> TokenStream {
        type_helpers::generate_derive_traits_without_default(
            trait_derivability,
            has_pattern_validators,
        )
    }

    /// Generate derive traits excluding Deserialize (for custom Deserialize impls)
    fn generate_derive_traits_without_deserialize(
        &self,
        trait_derivability: Option<&TraitDerivability>,
        has_pattern_validators: bool,
    ) -> TokenStream {
        type_helpers::generate_derive_traits_without_deserialize(
            trait_derivability,
            has_pattern_validators,
            self.streaming_encode,
        )
    }

    /// Generate derive traits excluding both Deserialize AND Default
    ///
    /// Used for anyOf/oneOf enums that need manual Default impl (first variant has data)
    /// AND need custom Deserialize impl (schema-aware deserialization).
    fn generate_derive_traits_without_deserialize_and_default(
        &self,
        trait_derivability: Option<&TraitDerivability>,
        has_pattern_validators: bool,
    ) -> TokenStream {
        type_helpers::generate_derive_traits_without_deserialize_and_default(
            trait_derivability,
            has_pattern_validators,
        )
    }

    /// Generate TokenStream for a member type
    fn generate_member_type(&self, member_type: &MemberType) -> TctResult<TokenStream> {
        type_helpers::generate_member_type(member_type, &self.analysis)
    }

    /// Parse Rust type string into TokenStream
    fn parse_rust_type(&self, rust_type: &str) -> TokenStream {
        type_helpers::parse_rust_type(rust_type)
    }

    /// Sanitize field name for Rust identifier
    fn sanitize_field_name(&self, name: &str) -> String {
        type_helpers::sanitize_field_name(name)
    }

    /// Generate vocabulary-constrained codec helper functions
    ///
    /// These helpers ensure all token values respect the MAX_VOCAB constraint by using
    /// multi-token codec encoding when values exceed the vocabulary budget.
    fn generate_codec_helpers(max_vocab: u32) -> TokenStream {
        render_codec_helpers(max_vocab)
    }

    /// Generate vocabulary-constrained collection encoder implementations
    ///
    /// These implementations override the default Vec/HashMap/String encoders
    /// to use vocabulary-constrained length encoding.
    fn generate_collection_encoders(max_collection_length: usize) -> TokenStream {
        render_collection_encoders(max_collection_length)
    }

    /// Generate vocabulary-constrained number encoder implementations
    ///
    /// These implementations override the default i32/i64/f64 encoders from primitives.rs
    /// to use vocabulary-constrained encoding for all numeric values. With
    /// `decimal_number_encoding`, unbounded i64/u64/f64 use digit-aligned
    /// decimal text encoding instead (see runtime_support.rs).
    fn generate_number_encoders(decimal_number_encoding: bool) -> TokenStream {
        render_number_encoders(decimal_number_encoding)
    }

    /// Generate partial encoding support data structures
    ///
    /// These structures enable two encoding modes:
    /// - Mode 1: Complete instances (existing encode/decode, zero overhead)
    /// - Mode 2: Decode to first gap (decoder-only LM continuation)
    ///
    /// Partial encoding produces a complete default-filled token sequence plus
    /// gap metadata. It is not a live parser state for suffix-constrained infill.
    fn generate_partial_encoding_types() -> TokenStream {
        render_partial_encoding_types()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::encoding_decision::MemberTypeRefinement;
    use crate::generator::schema_discovery::{
        MemberInfo, SchemaDiscovery, SchemaLocation, TypeDiscovery, TypeKind,
    };
    use crate::generator::struct_generator::StructGenerator;
    use crate::generator::type_analysis::{TraitDerivability, TypeAnalysis};
    use quote::quote;
    use syn::punctuated::Punctuated;

    // Helper: Create test CodeGenerator
    fn create_test_generator() -> CodeGenerator {
        let discovery = SchemaDiscovery::new();
        let analysis = TypeAnalysis::default();
        CodeGenerator::new(
            discovery,
            analysis,
            None,
            Default::default(),
            false,
            StructGenerator::DEFAULT_ENCODING_BASE,
            2048,
        )
    }

    fn parse_generated_file(tokens: TokenStream) -> syn::File {
        match syn::parse2(tokens) {
            Ok(file) => file,
            Err(error) => panic!("generated tokens should parse as Rust: {error}"),
        }
    }

    fn derive_traits_from_tokens(tokens: TokenStream) -> Vec<String> {
        let file = parse_generated_file(quote! {
            #tokens
            struct GeneratedDeriveProbe;
        });

        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(item_struct) if item_struct.ident == "GeneratedDeriveProbe" => {
                    Some(derive_traits(&item_struct.attrs))
                }
                _ => None,
            })
            .expect("expected derive probe struct")
    }

    fn derive_traits(attrs: &[syn::Attribute]) -> Vec<String> {
        attrs
            .iter()
            .filter(|attr| attr.path().is_ident("derive"))
            .filter_map(|attr| {
                attr.parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
                    .ok()
            })
            .flat_map(|paths| paths.into_iter())
            .map(|path| {
                path.segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::")
            })
            .collect()
    }

    fn trait_impl_for_self_type_segment<'a>(
        file: &'a syn::File,
        trait_name: &str,
        self_type_segment: &str,
    ) -> Option<&'a syn::ItemImpl> {
        file.items.iter().find_map(|item| match item {
            syn::Item::Impl(item_impl)
                if item_impl.trait_.as_ref().is_some_and(|(_, path, _)| {
                    path.segments
                        .last()
                        .is_some_and(|segment| segment.ident == trait_name)
                }) && impl_self_type_has_segment(item_impl, self_type_segment) =>
            {
                Some(item_impl)
            }
            _ => None,
        })
    }

    fn impl_self_type_has_segment(item_impl: &syn::ItemImpl, expected_segment: &str) -> bool {
        match item_impl.self_ty.as_ref() {
            syn::Type::Path(type_path) => type_path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == expected_segment),
            _ => false,
        }
    }

    fn type_param_has_trait_bound(
        item_impl: &syn::ItemImpl,
        param_name: &str,
        trait_name: &str,
    ) -> bool {
        item_impl.generics.params.iter().any(|param| match param {
            syn::GenericParam::Type(type_param) if type_param.ident == param_name => {
                type_param.bounds.iter().any(|bound| match bound {
                    syn::TypeParamBound::Trait(trait_bound) => trait_bound
                        .path
                        .segments
                        .last()
                        .is_some_and(|segment| segment.ident == trait_name),
                    _ => false,
                })
            }
            _ => false,
        })
    }

    fn impl_has_method(item_impl: &syn::ItemImpl, method_name: &str) -> bool {
        item_impl.items.iter().any(|item| match item {
            syn::ImplItem::Fn(method) => method.sig.ident == method_name,
            _ => false,
        })
    }

    #[test]
    fn test_apply_member_type_refinements_uses_optimization_metadata() {
        let mut type_discovery = TypeDiscovery::new(
            "Config".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/".to_string(),
                description: "test config".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );
        type_discovery.add_member(MemberInfo {
            name: "additional_properties".to_string(),
            member_type: MemberType::HashMap {
                key: "String".to_string(),
                value: "serde_json::Value".to_string(),
            },
            required: false,
            is_nullable: false,
            schema: serde_json::Map::new(),
        });

        let mut discovery = SchemaDiscovery::new();
        discovery
            .discovered_types
            .insert("Config".to_string(), type_discovery);

        let mut strategy = EncodingStrategy::new();
        strategy.add_member_type_refinement(
            "Config",
            "additional_properties",
            MemberTypeRefinement::HashMap {
                key: "String".to_string(),
                value: "String".to_string(),
            },
        );

        let refined = CodeGenerator::apply_member_type_refinements(discovery, Some(&strategy));
        let member_type = &refined
            .get_type("Config")
            .unwrap()
            .members
            .iter()
            .find(|member| member.name == "additional_properties")
            .unwrap()
            .member_type;

        assert_eq!(
            member_type,
            &MemberType::HashMap {
                key: "String".to_string(),
                value: "String".to_string(),
            }
        );
    }

    // ========================================================================================
    // Test Group 1: sanitize_field_name - Reserved Keywords (3 tests)
    // ========================================================================================

    #[test]
    fn test_sanitize_field_name_rust_keywords() {
        let generator = create_test_generator();

        // Strict keywords
        assert_eq!(generator.sanitize_field_name("type"), "type_");
        assert_eq!(generator.sanitize_field_name("struct"), "struct_");
        assert_eq!(generator.sanitize_field_name("fn"), "fn_");
        assert_eq!(generator.sanitize_field_name("impl"), "impl_");
        assert_eq!(generator.sanitize_field_name("enum"), "enum_");
        assert_eq!(generator.sanitize_field_name("trait"), "trait_");

        // Reserved keywords
        assert_eq!(generator.sanitize_field_name("async"), "async_");
        assert_eq!(generator.sanitize_field_name("await"), "await_");
        assert_eq!(generator.sanitize_field_name("dyn"), "dyn_");
    }

    #[test]
    fn test_sanitize_field_name_generated_decoder_locals() {
        let generator = create_test_generator();
        assert_eq!(generator.sanitize_field_name("tokens"), "tokens_");
    }

    #[test]
    fn test_sanitize_field_name_special_prefixes() {
        let generator = create_test_generator();

        // Special character prefixes
        assert_eq!(generator.sanitize_field_name("$schema"), "dollar_schema");
        assert_eq!(generator.sanitize_field_name("+prop"), "plus_prop");
        assert_eq!(
            generator.sanitize_field_name("//comment"),
            "double_slash_comment"
        );
        assert_eq!(generator.sanitize_field_name("/path"), "slash_path");
        assert_eq!(generator.sanitize_field_name("@type"), "at_type");
        assert_eq!(generator.sanitize_field_name("#id"), "hash_id");
    }

    #[test]
    fn test_sanitize_field_name_edge_cases() {
        let generator = create_test_generator();

        // Special single-character mappings
        assert_eq!(generator.sanitize_field_name("."), "dot");
        assert_eq!(generator.sanitize_field_name("*"), "star");
        assert_eq!(generator.sanitize_field_name("^"), "caret");
        assert_eq!(generator.sanitize_field_name("^_"), "caret_underscore");

        // Character replacements
        assert_eq!(generator.sanitize_field_name("kebab-case"), "kebab_case");
        assert_eq!(generator.sanitize_field_name("dot.case"), "dot_case");
        assert_eq!(generator.sanitize_field_name("space case"), "space_case");

        // Numeric start (invalid identifier)
        assert_eq!(generator.sanitize_field_name("123field"), "field_123field");

        // Empty string
        assert_eq!(generator.sanitize_field_name(""), "field");
    }

    #[test]
    fn test_sanitize_field_name_always_yields_an_identifier_quote_accepts() {
        // Property names are arbitrary JSON strings. Control characters,
        // whitespace, symbols, and emoji used to reach `format_ident!` unchanged
        // and panic the compiler with "... is not a valid Ident".
        let generator = create_test_generator();
        let names = [
            "line\nbreak",
            "tab\there",
            "cr\rhere",
            "nul\u{0}x",
            "sep\u{2028}x",
            "\u{85}",
            "ünï✓",
            "🙂",
            "a²",
            "½",
            "\u{200b}zero",
            "x\u{301}",
            "\u{345}lead",
            "_",
            "__",
            "9a",
            "",
            " ",
            "\u{feff}bom",
            "a\u{a0}b",
            "İ",
            "ß",
            "Ⅷ",
        ];
        for name in names {
            let sanitized = generator.sanitize_field_name(name);
            assert!(
                !sanitized.is_empty(),
                "{name:?} sanitized to an empty identifier"
            );
            assert_ne!(sanitized, "_", "{name:?} sanitized to the reserved `_`");
            // `format_ident!` panics on anything that is not a valid identifier.
            let ident = quote::format_ident!("{}", sanitized);
            assert_eq!(ident.to_string(), sanitized);
        }
    }

    #[test]
    fn test_sanitize_field_name_keeps_valid_identifiers_stable() {
        // Names that were already valid keep their (lowercased) spelling, so the
        // generated field names of existing schemas do not change.
        let generator = create_test_generator();
        assert_eq!(generator.sanitize_field_name("ünïcode"), "ünïcode");
        assert_eq!(generator.sanitize_field_name("ß"), "ß");
        assert_eq!(generator.sanitize_field_name("Straße"), "straße");
        assert_eq!(generator.sanitize_field_name("snake_case9"), "snake_case9");
    }

    // ========================================================================================
    // Test Group 2: generate_member_type - Type Generation (3 tests)
    // ========================================================================================

    #[test]
    fn test_generate_member_type_primitives() {
        let generator = create_test_generator();

        // Test primitive type generation
        let string_type = MemberType::Primitive("String".to_string());
        let tokens = generator.generate_member_type(&string_type).unwrap();
        assert_eq!(tokens.to_string(), "String");

        let i32_type = MemberType::Primitive("i32".to_string());
        let tokens = generator.generate_member_type(&i32_type).unwrap();
        assert_eq!(tokens.to_string(), "i32");

        let bool_type = MemberType::Primitive("bool".to_string());
        let tokens = generator.generate_member_type(&bool_type).unwrap();
        assert_eq!(tokens.to_string(), "bool");
    }

    #[test]
    fn test_generate_member_type_containers() {
        let generator = create_test_generator();

        // Test Vec<T> container
        let vec_type = MemberType::Container {
            container: "Vec".to_string(),
            inner: "String".to_string(),
        };
        let tokens = generator.generate_member_type(&vec_type).unwrap();
        assert_eq!(tokens.to_string(), "Vec < String >");

        // Test Option<T> container
        let option_type = MemberType::Container {
            container: "Option".to_string(),
            inner: "i64".to_string(),
        };
        let tokens = generator.generate_member_type(&option_type).unwrap();
        assert_eq!(tokens.to_string(), "Option < i64 >");
    }

    #[test]
    fn test_generate_member_type_complex_containers() {
        let mut discovery = SchemaDiscovery::new();
        let mut analysis = TypeAnalysis::default();

        // Register a type that doesn't need boxing
        let schema_location = crate::generator::schema_discovery::SchemaLocation {
            json_path: "#/definitions/MyType".to_string(),
            description: "Test type".to_string(),
            parent_type: None,
            property_name: None,
        };

        let schema_obj = serde_json::Map::new();
        let type_discovery = TypeDiscovery::new(
            "MyType".to_string(),
            TypeKind::Struct,
            schema_location,
            schema_obj,
        );
        discovery.register_type(type_discovery);

        // Mark as NOT needing boxing
        analysis.boxed_types.clear(); // Ensure it's not in the set

        let generator = CodeGenerator::new(
            discovery.clone(),
            analysis.clone(),
            None,
            Default::default(),
            false,
            StructGenerator::DEFAULT_ENCODING_BASE,
            2048,
        );

        // Test ComplexContainer without boxing
        let complex_type = MemberType::ComplexContainer {
            container: "Vec".to_string(),
            inner: "MyType".to_string(),
        };
        let tokens = generator.generate_member_type(&complex_type).unwrap();
        assert_eq!(tokens.to_string(), "Vec < MyType >");

        // Now test with boxing
        analysis.boxed_types.insert("MyType".to_string());
        let gen_with_boxing = CodeGenerator::new(
            discovery,
            analysis,
            None,
            Default::default(),
            false,
            StructGenerator::DEFAULT_ENCODING_BASE,
            2048,
        );

        let tokens = gen_with_boxing.generate_member_type(&complex_type).unwrap();
        // Note: TokenStream formatting doesn't add space after final angle bracket
        assert_eq!(tokens.to_string(), "Vec < Box < MyType >>");
    }

    // ========================================================================================
    // Test Group 3: generate_derive_traits - Trait Derivation (2 tests)
    // ========================================================================================

    #[test]
    fn test_generate_derive_traits_with_derivability() {
        let generator = create_test_generator();

        // Test with full trait derivability
        let trait_derivability = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "All fields are primitive types".to_string(),
            confidence: 1.0,
            blocking_types: std::collections::HashSet::new(),
        };

        let tokens = generator.generate_derive_traits(Some(&trait_derivability), false);
        let derives = derive_traits_from_tokens(tokens);

        // Should include: Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Hash, Serialize, Deserialize
        assert_eq!(
            derives,
            [
                "Debug",
                "PartialEq",
                "Eq",
                "Clone",
                "PartialOrd",
                "Ord",
                "Hash",
                "serde::Serialize",
                "serde::Deserialize",
            ]
        );
    }

    #[test]
    fn test_generate_derive_traits_without_full_derivability() {
        let generator = create_test_generator();

        // Test with no Eq/Ord/Hash derivability (like f64 fields)
        let mut blocking = std::collections::HashSet::new();
        blocking.insert("f64".to_string());

        let trait_derivability = TraitDerivability {
            can_derive_eq: false,
            can_derive_ord: false,
            can_derive_hash: false,
            reasoning: "Contains f64 field which doesn't implement Eq/Ord/Hash".to_string(),
            confidence: 1.0,
            blocking_types: blocking,
        };

        let tokens = generator.generate_derive_traits(Some(&trait_derivability), false);
        let derives = derive_traits_from_tokens(tokens);

        // Should include: Debug, PartialEq, Clone, Serialize, Deserialize
        assert_eq!(
            derives,
            [
                "Debug",
                "PartialEq",
                "Clone",
                "serde::Serialize",
                "serde::Deserialize",
            ]
        );

        // Should NOT include: Eq, Ord, Hash
        assert!(!derives.iter().any(|trait_name| trait_name == "Eq"));
        assert!(!derives.iter().any(|trait_name| trait_name == "PartialOrd"));
        assert!(!derives.iter().any(|trait_name| trait_name == "Ord"));
        assert!(!derives.iter().any(|trait_name| trait_name == "Hash"));
    }

    // ========================================================================================
    // Test Group 4: parse_rust_type - Type Parsing (2 tests)
    // ========================================================================================

    #[test]
    fn test_parse_rust_type_standard_types() {
        let generator = create_test_generator();

        // Test standard primitive types
        assert_eq!(generator.parse_rust_type("String").to_string(), "String");
        assert_eq!(generator.parse_rust_type("i32").to_string(), "i32");
        assert_eq!(generator.parse_rust_type("i64").to_string(), "i64");
        assert_eq!(generator.parse_rust_type("f64").to_string(), "f64");
        assert_eq!(generator.parse_rust_type("bool").to_string(), "bool");

        // Test serde_json types
        assert_eq!(
            generator.parse_rust_type("serde_json::Value").to_string(),
            "serde_json :: Value"
        );
        assert_eq!(
            generator.parse_rust_type("serde_json::Number").to_string(),
            "serde_json :: Number"
        );
    }

    #[test]
    fn test_parse_rust_type_complex_types() {
        let generator = create_test_generator();

        // Test custom type (simple identifier)
        assert_eq!(generator.parse_rust_type("MyType").to_string(), "MyType");
        assert_eq!(
            generator.parse_rust_type("UserConfig").to_string(),
            "UserConfig"
        );

        // Complex types with angle brackets should parse correctly
        // (These may have whitespace variations due to TokenStream formatting)
        let vec_result = generator.parse_rust_type("Vec<String>");
        assert!(vec_result.to_string().contains("Vec"));
        assert!(vec_result.to_string().contains("String"));
    }

    #[test]
    fn test_parse_rust_type_qualifies_json_integer_in_complex_types() {
        let generator = create_test_generator();

        assert_eq!(
            generator.parse_rust_type("JsonInteger").to_string(),
            "crate :: tokenizer :: JsonInteger"
        );
        assert_eq!(
            generator
                .parse_rust_type("rustc_hash::FxHashMap<String, JsonInteger>")
                .to_string(),
            "rustc_hash :: FxHashMap < String , crate :: tokenizer :: JsonInteger >"
        );
        assert_eq!(
            generator
                .parse_rust_type(
                    "rustc_hash::FxHashMap<String, rustc_hash::FxHashMap<String, JsonInteger>>",
                )
                .to_string(),
            "rustc_hash :: FxHashMap < String , rustc_hash :: FxHashMap < String , crate :: tokenizer :: JsonInteger >>"
        );
    }

    // ========================================================================================
    // Test Group 5: Helper Functions - Codec Generation (3 tests)
    // ========================================================================================

    #[test]
    fn test_generate_codec_helpers_structure() {
        // Test that codec helpers generate expected function signatures
        let tokens = CodeGenerator::generate_codec_helpers(256);
        let token_str = tokens.to_string();

        // Should contain core helper functions
        assert!(token_str.contains("encode_length_constrained"));
        assert!(token_str.contains("decode_length_constrained"));
        assert!(token_str.contains("encode_u32_constrained"));
        assert!(token_str.contains("decode_u32_constrained"));
        assert!(token_str.contains("decode_u32_constrained_with_first"));

        // Should contain MAX_VOCAB constant usage
        assert!(token_str.contains("MAX_VOCAB"));
        assert!(token_str.contains("ESCAPE_TOKEN"));
    }

    #[test]
    fn test_generate_collection_encoders() {
        let tokens = CodeGenerator::generate_collection_encoders(2048);
        let generated_file = parse_generated_file(tokens.clone());
        let token_str = tokens.to_string();

        // Should contain core collection types
        let string_impl =
            trait_impl_for_self_type_segment(&generated_file, "JsonTokenEncoder", "String")
                .expect("expected generated String encoder impl");
        assert!(impl_has_method(string_impl, "encode"));
        assert!(impl_has_method(string_impl, "decode"));
        assert!(impl_has_method(string_impl, "token_count"));

        let vec_impl = trait_impl_for_self_type_segment(&generated_file, "JsonTokenEncoder", "Vec")
            .expect("expected generated Vec encoder impl");
        assert!(type_param_has_trait_bound(
            vec_impl,
            "T",
            "JsonTokenEncoder"
        ));
        assert!(impl_has_method(vec_impl, "encode"));
        assert!(impl_has_method(vec_impl, "decode"));
        assert!(impl_has_method(vec_impl, "token_count"));

        let map_impl =
            trait_impl_for_self_type_segment(&generated_file, "JsonTokenEncoder", "FxHashMap")
                .expect("expected generated FxHashMap encoder impl");
        assert!(impl_has_method(map_impl, "encode"));
        assert!(impl_has_method(map_impl, "decode"));
        assert!(impl_has_method(map_impl, "token_count"));

        assert!(!token_str.contains("crate :: string_dictionary :: lookup"));
        assert!(!token_str.contains("DICT_MARKER"));
    }

    #[test]
    fn test_generate_number_encoders_structure() {
        // Test that number encoders generate expected implementations
        let tokens = CodeGenerator::generate_number_encoders(false);
        let generated_file = parse_generated_file(tokens.clone());
        let token_str = tokens.to_string();

        // Should contain numeric type implementations
        for self_type in ["i32", "i64", "f64"] {
            let encoder_impl =
                trait_impl_for_self_type_segment(&generated_file, "JsonTokenEncoder", self_type)
                    .unwrap_or_else(|| panic!("expected generated {self_type} encoder impl"));
            assert!(impl_has_method(encoder_impl, "encode"));
            assert!(impl_has_method(encoder_impl, "decode"));
            assert!(impl_has_method(encoder_impl, "token_count"));
        }

        // Should contain zigzag encoding for i32
        assert!(token_str.contains("zigzag"));

        // Should contain byte splitting for i64 and f64
        assert!(token_str.contains("to_le_bytes"));
        assert!(token_str.contains("to_bits"));
    }

    #[test]
    fn test_generate_decimal_number_encoders_structure() {
        // Decimal mode: i64/u64/f64 use digit-aligned decimal text encoding;
        // i32 keeps zigzag; no bit-split byte encoding for i64/f64.
        let tokens = CodeGenerator::generate_number_encoders(true);
        let generated_file = parse_generated_file(tokens.clone());
        let token_str = tokens.to_string();

        for self_type in ["i32", "i64", "u64", "f64"] {
            let encoder_impl =
                trait_impl_for_self_type_segment(&generated_file, "JsonTokenEncoder", self_type)
                    .unwrap_or_else(|| panic!("expected generated {self_type} encoder impl"));
            assert!(impl_has_method(encoder_impl, "encode"));
            assert!(impl_has_method(encoder_impl, "decode"));
            assert!(impl_has_method(encoder_impl, "token_count"));
        }

        assert!(token_str.contains("decimal_number_codec"));
        assert!(token_str.contains("zigzag"));
        assert!(!token_str.contains("to_bits"));
    }

    // ========================================================================================
    // Test Group 6: Edge Cases and Integration (2 tests)
    // ========================================================================================

    #[test]
    fn test_sanitize_field_name_unicode_and_mixed_chars() {
        let generator = create_test_generator();

        // Mixed special characters
        assert_eq!(
            generator.sanitize_field_name("field-name.with_many!chars"),
            "field_name_with_many_chars"
        );

        // Comparison operators (get converted by character replacement rules)
        assert_eq!(generator.sanitize_field_name(">="), "gteq");
        assert_eq!(generator.sanitize_field_name("<="), "lteq");
        assert_eq!(generator.sanitize_field_name("=="), "eqeq");

        // Parentheses and brackets
        assert_eq!(generator.sanitize_field_name("func()"), "func__");
        assert_eq!(generator.sanitize_field_name("array[]"), "array__");
        assert_eq!(generator.sanitize_field_name("object{}"), "object__");
    }

    #[test]
    fn test_generate_derive_traits_with_pattern_validators() {
        let generator = create_test_generator();

        // When has_pattern_validators is true, should NOT include serde derives
        let trait_derivability = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "All fields are primitive types".to_string(),
            confidence: 1.0,
            blocking_types: std::collections::HashSet::new(),
        };

        let tokens = generator.generate_derive_traits(Some(&trait_derivability), true);
        let derives = derive_traits_from_tokens(tokens);

        // Should include standard traits
        assert_eq!(
            derives,
            [
                "Debug",
                "PartialEq",
                "Eq",
                "Clone",
                "PartialOrd",
                "Ord",
                "Hash",
            ]
        );

        // Should NOT include serde derives (custom serde needed for pattern validators)
        assert!(
            !derives
                .iter()
                .any(|trait_name| trait_name == "serde::Serialize")
        );
        assert!(
            !derives
                .iter()
                .any(|trait_name| trait_name == "serde::Deserialize")
        );
    }

    // ========================================================================================
    // Test Group: calculate_max_structural_token_id - Bug fix for BPE overlap (TDD Red)
    // ========================================================================================

    #[test]
    fn test_calculate_max_structural_token_id_large_enum() {
        // This test exposes the bug where BASE_VOCAB_SIZE calculation is incorrect
        // when using EncodingStrategy instead of VocabularyAwareEncoder.
        //
        // Scenario: Schema with large enum (513 variants)
        // Expected: Enum with 513 variants uses tokens 0-512, so max_token_id = 512
        //           and BASE_VOCAB_SIZE should be 513 (max_token_id + 1)
        //
        // Bug: EncodingStrategy::update_stats() incorrectly calculates max_token_id
        //      for BoundedInteger encodings (uses value range instead of token range)

        let mut discovery = SchemaDiscovery::new();

        // Create a large enum type with 513 variants (matching kubernetes schema size)
        let mut variants = Vec::new();
        for i in 0..513 {
            variants.push(MemberInfo {
                name: format!("Variant{}", i),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: serde_json::Map::new(),
            });
        }

        let large_enum = TypeDiscovery {
            type_name: "KubernetesKind".to_string(),
            type_kind: TypeKind::Enum,
            schema_location: SchemaLocation {
                json_path: "#/properties/kind".to_string(),
                description: "Kubernetes resource kind".to_string(),
                parent_type: None,
                property_name: Some("kind".to_string()),
            },
            schema_object: serde_json::Map::new(),
            members: variants,
            direct_references: std::collections::HashSet::new(),
            needs_boxing: false,
            discriminator_info: None,
        };

        discovery.register_type(large_enum);

        // Create EncodingStrategy with EnumIndex decision
        let mut encoding_strategy = EncodingStrategy::new();
        let variants: Vec<String> = (0..513).map(|i| format!("Variant{}", i)).collect();
        encoding_strategy.add_decision("KubernetesKind", EncodingDecision::EnumIndex { variants });

        // Pass encoding_strategy to generator
        let generator = CodeGenerator::new(
            discovery,
            TypeAnalysis::default(),
            Some(encoding_strategy),
            Default::default(),
            false,
            StructGenerator::DEFAULT_ENCODING_BASE,
            2048,
        );

        let max_token_id = generator.calculate_max_structural_token_id();

        // 513 variants use tokens 0-512, so max_token_id should be 512
        assert_eq!(
            max_token_id, 512,
            "Enum with 513 variants should have max_token_id = 512"
        );
    }
}
