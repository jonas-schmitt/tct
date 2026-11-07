//! Grammar structures for generated code output
//!
//! This module defines the core data structures that represent the generated
//! Rust code from JSON schemas, including enums, structs, and complete grammars
//! with tokenization implementations.

use proc_macro2::TokenStream;
use quote::quote;
use rustc_hash::FxHashMap;

mod bpe;
mod sequence;
mod tokenizer;

fn quote_option_u32(value: Option<u32>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value) },
        None => quote! { None },
    }
}

fn quote_option_usize(value: Option<usize>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value) },
        None => quote! { None },
    }
}

fn quote_option_str(value: Option<&str>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value) },
        None => quote! { None },
    }
}

/// A container for a generated enum and its tokenizer implementation.
#[derive(Clone)]
pub struct GeneratedEnum {
    /// Generated enum type name
    pub type_name: String,
    /// The enum type definition
    pub definition: TokenStream,
    /// The tokenizer implementation for the enum
    pub implementation: TokenStream,
}

/// A container for a generated struct and its tokenizer implementation.
#[derive(Clone)]
pub struct GeneratedStruct {
    /// The struct type definition
    pub definition: TokenStream,
    /// The tokenizer implementation for the struct (optional during generation)
    pub implementation: Option<TokenStream>,
}

/// Container for complete generated code from a JSON schema
///
/// Grammar represents the complete compilation result from a JSON schema,
/// containing the root type, all supporting definitions, and optional
/// optimization statistics.
///
/// # Structure
///
/// - **Root**: The main type generated from the schema's top level
/// - **Definitions**: Supporting types, enums, constants, and type aliases
/// - **Statistics**: Optional vocabulary optimization metrics
///
/// # Usage
///
/// After compilation, call [`generate_code()`](Grammar::generate_code) to get
/// the final Rust source code as a string.
///
/// # Examples
///
/// ```
/// use tct::TctCompiler;
/// use serde_json::json;
///
/// let schema = json!({"type": "string", "enum": ["a", "b", "c"]});
/// let grammar = TctCompiler::new(schema, "MyEnum")?.compile()?;
///
/// // Generate the final Rust code
/// let rust_code = grammar.generate_code()?;
/// println!("{}", rust_code);
///
/// // Check optimization statistics
/// if let Some(stats) = &grammar.vocabulary_stats {
///     println!("Vocabulary size: {}", stats.distinct_structural_tokens_count);
///     println!("Reduction: {:.1}%", stats.reduction_percentage);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Grammar {
    /// The root struct generated from the schema
    pub root: GeneratedStruct,
    /// Supporting type definitions
    pub definitions: Definitions,
    /// Root type name (for instance encoding)
    pub root_type_name: String,
    /// Whether the root type is an enum (oneOf/anyOf composition)
    /// If true, partial encoding (Mode 2) requires discriminator detection
    pub is_enum_root: bool,
    /// Whether the enum root has a discriminator field (e.g., Kubernetes `kind`)
    /// If true and is_enum_root, partial encoding (Mode 2) is supported
    pub has_enum_discriminator: bool,
    /// Base vocabulary size (calculated during compilation)
    /// This is the actual base layer size: max(string_vocab, structural_vocab)
    /// Used for string encoding and windowed generation
    pub base_vocab: u32,
    /// Whether streaming-friendly prefix decoding is enabled.
    ///
    /// This controls whether generated types expose `decode_prefix` and is
    /// surfaced in sequence metadata for downstream wrappers.
    pub streaming_encode: bool,
    /// Encoding base for multi-token ranges (primitives, collections, bounded integers)
    /// Computed from OptimizationConfig.max_base_vocab as next power-of-2 ≤ max_base_vocab
    /// Default: 256 (maintains backward compatibility)
    pub encoding_base: u32,
    /// BPE merge table for token compression (if BPE enabled)
    pub bpe_merge_table: Option<crate::bpe::BpeMergeTable>,
    /// Vocabulary optimization statistics (if optimization was enabled)
    pub vocabulary_stats: Option<crate::vocabulary_encoder::VocabularyOptimizationStats>,
    /// Schema quality warnings and optimization suggestions
    pub warnings: Option<crate::warnings::SchemaWarnings>,
    /// Performance metrics for compilation and optimization analysis
    pub performance_metrics: Option<crate::metrics::PerformanceMetrics>,
    /// Profiling code module (if profiling was enabled)
    pub profiling_code: Option<ProfilingModule>,
    /// Compilation constraints (includes windowed generation configuration)
    pub compilation_constraints:
        Option<crate::generator::encoding_decision::CompilationConstraints>,
    /// Maximum collection length for decoded Vec/HashMap (default: 2048)
    /// Prevents memory exhaustion from model-generated garbage tokens
    pub max_collection_length: usize,
    /// Maximum recursive schema traversal depth used during discovery
    pub max_schema_depth: usize,
    /// Encode unbounded i64/u64/f64 as decimal text byte tokens (opt-in).
    /// Token streams are incompatible with the default bit-split encoding.
    pub decimal_number_encoding: bool,
}

/// Container for generated profiling code
#[derive(Clone)]
pub struct ProfilingModule {
    /// The generated profiling module code
    pub module_code: TokenStream,
    /// Optional Criterion benchmark file content
    pub benchmark_file: Option<String>,
}

/// Holds the generated Rust code components from the JSON schema's "definitions" block.
#[derive(Clone)]
pub struct Definitions {
    pub(crate) structs: FxHashMap<String, GeneratedStruct>,
    pub(crate) enums: Vec<GeneratedEnum>,
    pub(crate) constants: Vec<TokenStream>,
    pub(crate) type_aliases: Vec<TokenStream>,
}

impl Grammar {
    /// Get optimization summary as a string
    pub fn optimization_summary(&self) -> String {
        if let Some(stats) = &self.vocabulary_stats {
            stats.summary()
        } else {
            "Vocabulary optimization disabled".to_string()
        }
    }

    /// Get schema quality warnings summary
    pub fn warnings_summary(&self) -> String {
        if let Some(warnings) = &self.warnings {
            warnings.summary()
        } else {
            "✅ No schema quality analysis performed".to_string()
        }
    }

    /// Check if there are any schema quality warnings
    pub fn has_warnings(&self) -> bool {
        self.warnings.as_ref().is_some_and(|w| !w.is_empty())
    }

    /// Get the number of schema quality warnings
    pub fn warning_count(&self) -> usize {
        self.warnings.as_ref().map_or(0, |w| w.len())
    }

    /// Check if the root type supports partial encoding (Mode 2)
    ///
    /// Partial encoding is supported for:
    /// - Struct root types (field-level presence tracking)
    /// - Enum roots with discriminator detection (e.g., Kubernetes `kind` field)
    ///
    /// Enum roots without discriminator don't support partial encoding because
    /// there's no way to determine the variant from incomplete JSON.
    ///
    /// Returns `true` if partial encoding is supported, `false` otherwise.
    pub fn supports_partial_encoding(&self) -> bool {
        if self.is_enum_root {
            // Enum roots need a discriminator field to support partial encoding
            self.has_enum_discriminator
        } else {
            // Struct roots always support partial encoding
            true
        }
    }

    /// Get performance metrics summary
    pub fn performance_summary(&self) -> String {
        if let Some(metrics) = &self.performance_metrics {
            metrics.comprehensive_report()
        } else {
            "Performance metrics not collected".to_string()
        }
    }

    /// Check if performance metrics are available
    pub fn has_performance_metrics(&self) -> bool {
        self.performance_metrics.is_some()
    }

    /// Get token metrics if available
    pub fn token_metrics(&self) -> Option<&crate::metrics::TokenMetrics> {
        self.performance_metrics.as_ref().map(|m| &m.tokens)
    }

    /// Get timing metrics if available
    pub fn timing_metrics(&self) -> Option<&crate::metrics::TimingMetrics> {
        self.performance_metrics.as_ref().map(|m| &m.timing)
    }

    /// Get complexity metrics if available
    pub fn complexity_metrics(&self) -> Option<&crate::metrics::ComplexityMetrics> {
        self.performance_metrics.as_ref().map(|m| &m.complexity)
    }

    /// Generates a single formatted string of Rust code from all the generated components.
    pub fn generate_code(&self) -> Result<String, crate::error::TctError> {
        let mut combined_tokens = TokenStream::new();
        let defs = &self.definitions;

        // ALWAYS generate standalone tokenizer module for complete portability
        let tokenizer_module = self.generate_tokenizer_module();
        combined_tokens.extend(tokenizer_module);

        // Add constants
        for constant in &defs.constants {
            combined_tokens.extend(quote! { #constant });
        }

        // Add type aliases
        for alias in &defs.type_aliases {
            combined_tokens.extend(quote! { #alias });
        }

        // Add enums and their impls
        for enm in &defs.enums {
            combined_tokens.extend(enm.definition.clone());
            combined_tokens.extend(enm.implementation.clone());
        }

        // Add structs from definitions and their impls
        let mut sorted_structs: Vec<_> = defs.structs.values().collect();
        sorted_structs.sort_by_key(|a| a.definition.to_string());
        for s in sorted_structs {
            combined_tokens.extend(s.definition.clone());
            if let Some(implementation) = &s.implementation {
                combined_tokens.extend(implementation.clone());
            }
        }

        // Add the main root struct at the end
        combined_tokens.extend(self.root.definition.clone());
        if let Some(implementation) = &self.root.implementation {
            combined_tokens.extend(implementation.clone());
        }

        // Add BPE module if merge table is present
        if let Some(merge_table) = &self.bpe_merge_table {
            let bpe_code = self.generate_bpe_module(merge_table)?;
            combined_tokens.extend(bpe_code);

            // Add convenience wrapper functions
            let wrappers = self.generate_bpe_convenience_wrappers();
            combined_tokens.extend(wrappers);
        }

        // Generate VOCAB_SIZE constant (total vocabulary including BPE and PAD)
        // This represents the maximum possible vocabulary when all merges are applied
        let bpe_max_merges = self
            .bpe_merge_table
            .as_ref()
            .map(|b| b.merges.len() as u32)
            .unwrap_or(0);

        // Use the pre-calculated base vocabulary from compilation
        // This was calculated during Grammar construction using the same logic
        let base_vocab = self.base_vocab;

        // Total before special tokens
        let total_base = base_vocab + bpe_max_merges;

        // VOCAB_SIZE = base_vocab + bpe_merges + 1 (for PAD token)
        let final_vocab_size = total_base + 1;

        let vocab_size_constant = quote! {
            /// Total vocabulary size (maximum possible tokens)
            ///
            /// Formula: VOCAB_SIZE = base_vocab + bpe_merges + 1
            /// where base_vocab includes string bytes + END_STRING_MARKER + structural tokens
            ///
            /// Token allocation:
            /// - Base layer: [0, base_vocab-1] (strings + structural)
            /// - BPE merges: [base_vocab, base_vocab+bpe_merges-1]
            /// - PAD_TOKEN: at (base_vocab + bpe_merges)
            pub const VOCAB_SIZE: u32 = #final_vocab_size;
        };
        combined_tokens.extend(vocab_size_constant);

        // Note: MAX_BASE_VOCAB constant is now generated inside tokenizer module
        // (see generate_tokenizer_module() for the constant definition)

        // Add PAD_TOKEN constant
        let pad_token_value = total_base; // PAD at (bpe_start + bpe_merges)

        let pad_token_constant = quote! {
            /// PAD token for batching
            /// Position: base_vocab + bpe_merges
            /// Used to pad sequences to uniform length in batched training/inference
            pub const PAD_TOKEN: u32 = #pad_token_value;
        };
        combined_tokens.extend(pad_token_constant);

        let sequence_pad_token = 0u32;
        let sequence_bos_token = 1u32;
        let sequence_eos_token = 2u32;
        let sequence_schema_token = 3u32;
        let sequence_payload_offset = 4u32;
        let sequence_vocab_size = sequence_payload_offset + pad_token_value;
        let sequence_schema_id = self.root_type_name.as_str();

        let sequence_boundary_version = crate::sequence_bundle::CURRENT_SEQUENCE_BOUNDARY_VERSION;
        let sequence_token_constants = quote! {
            /// Version of the model-facing sequence-boundary token policy.
            ///
            /// Raw schema-local encode/decode APIs remain content-only. Boundary
            /// tokens are reserved for wrapper APIs that prepare complete model
            /// sequences.
            pub const SEQUENCE_BOUNDARY_VERSION: u32 = #sequence_boundary_version;

            /// Padding token for model-facing TCT sequences.
            ///
            /// This is distinct from raw schema-local `PAD_TOKEN`; raw payload
            /// tokens are offset before they enter a model-facing sequence.
            pub const SEQUENCE_PAD_TOKEN: u32 = #sequence_pad_token;

            /// Beginning-of-sequence token for model-facing TCT sequences.
            pub const SEQUENCE_BOS_TOKEN: u32 = #sequence_bos_token;

            /// Alias for [`SEQUENCE_BOS_TOKEN`].
            pub const SEQUENCE_START_TOKEN: u32 = SEQUENCE_BOS_TOKEN;

            /// End-of-sequence token for model-facing TCT sequences.
            pub const SEQUENCE_EOS_TOKEN: u32 = #sequence_eos_token;

            /// Alias for [`SEQUENCE_EOS_TOKEN`].
            pub const SEQUENCE_END_TOKEN: u32 = SEQUENCE_EOS_TOKEN;

            /// Single-schema decision token for model-facing TCT sequences.
            ///
            /// Multi-schema bundles generalize this to one schema-decision token
            /// per bundled schema.
            pub const SEQUENCE_SCHEMA_TOKEN: u32 = #sequence_schema_token;

            /// Schema identifier for this generated single-schema wrapper.
            pub const SEQUENCE_SCHEMA_ID: &str = #sequence_schema_id;

            /// Offset applied to raw schema-local content tokens in model-facing
            /// sequences.
            ///
            /// Local content tokens in `[0, PAD_TOKEN)` map to
            /// `[SEQUENCE_PAYLOAD_OFFSET, SEQUENCE_VOCAB_SIZE)`. Raw
            /// schema-local `PAD_TOKEN` is not a valid payload token.
            pub const SEQUENCE_PAYLOAD_OFFSET: u32 = #sequence_payload_offset;

            /// Vocabulary size for model-facing sequences that may include
            /// sequence special tokens and offset payload tokens.
            ///
            /// `VOCAB_SIZE` continues to describe the raw generated tokenizer
            /// vocabulary, including raw `PAD_TOKEN`.
            pub const SEQUENCE_VOCAB_SIZE: u32 = #sequence_vocab_size;
        };
        combined_tokens.extend(sequence_token_constants);

        combined_tokens.extend(self.generate_sequence_boundary_wrappers());

        // Add profiling module if present
        if let Some(profiling) = &self.profiling_code {
            combined_tokens.extend(profiling.module_code.clone());
        }

        // Parse the combined token stream into a syntax tree.
        let parsed_file = syn::parse2::<syn::File>(combined_tokens.clone()).map_err(|e| {
            crate::error::TctError::code_generation(
                format!("Failed to parse generated token stream: {}", e),
                "root",
                "Check that generated structs and enums have valid Rust syntax",
                crate::error::GenerationPhase::CodeFormatting,
            )
        })?;

        // Add attribution after formatting: ordinary comments are not retained
        // when code passes through proc_macro2 token streams and syn.
        let code = prettyplease::unparse(&parsed_file);
        Ok(format!(
            "// Generated by TCT: Type-Constrained Tokenization.\n\
             // TCT implementation portions: Copyright 2026 Jonas Schmitt.\n\
             // TCT implementation license: Apache-2.0.\n\
             // Preserve the applicable LICENSE and NOTICE when redistributing.\n\
             // Source and notices: https://github.com/jonas-schmitt/tct\n\
             // Input schemas and data retain their separate rights.\n\
             // Embedded third-party schema components retain their own licenses.\n\
             // Redistribute their exact source and applicable notices with this code.\n\n{code}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TctCompiler;
    use quote::quote;
    use serde_json::json;

    /// Helper function to create a simple Grammar for testing (no optimizations or stats)
    fn create_simple_grammar() -> Grammar {
        let schema = json!({"type": "string", "enum": ["a", "b", "c"]});
        TctCompiler::new(schema, "TestEnum")
            .unwrap()
            .with_no_optimization() // O0: No optimizations, no metrics
            .compile()
            .unwrap()
    }

    /// Helper function to create a Grammar with optimization stats
    fn create_grammar_with_stats() -> Grammar {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]},
                "count": {"type": "integer", "minimum": 0, "maximum": 100}
            }
        });
        TctCompiler::new(schema, "TestObject")
            .unwrap()
            .with_stats()
            .compile()
            .unwrap()
    }

    fn parse_generated_file(code: &str) -> syn::File {
        syn::parse_file(code).expect("generated code should parse as a Rust file")
    }

    fn generated_struct<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemStruct {
        find_generated_struct(&file.items, name)
            .unwrap_or_else(|| panic!("expected generated struct {name}"))
    }

    fn generated_enum<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemEnum {
        find_generated_enum(&file.items, name)
            .unwrap_or_else(|| panic!("expected generated enum {name}"))
    }

    fn find_generated_struct<'a>(
        items: &'a [syn::Item],
        name: &str,
    ) -> Option<&'a syn::ItemStruct> {
        items.iter().find_map(|item| match item {
            syn::Item::Struct(item_struct) if item_struct.ident == name => Some(item_struct),
            syn::Item::Mod(item_mod) => item_mod
                .content
                .as_ref()
                .and_then(|(_, module_items)| find_generated_struct(module_items, name)),
            _ => None,
        })
    }

    fn find_generated_enum<'a>(items: &'a [syn::Item], name: &str) -> Option<&'a syn::ItemEnum> {
        items.iter().find_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident == name => Some(item_enum),
            syn::Item::Mod(item_mod) => item_mod
                .content
                .as_ref()
                .and_then(|(_, module_items)| find_generated_enum(module_items, name)),
            _ => None,
        })
    }

    fn generated_const(file: &syn::File, name: &str) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Const(item_const) => item_const.ident == name,
            _ => false,
        })
    }

    fn generated_module(file: &syn::File, name: &str) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Mod(item_mod) => item_mod.ident == name,
            _ => false,
        })
    }

    fn generated_module_items<'a>(file: &'a syn::File, name: &str) -> &'a [syn::Item] {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Mod(item_mod) if item_mod.ident == name => {
                    item_mod.content.as_ref().map(|(_, items)| items.as_slice())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected generated module {name}"))
    }

    fn module_trait(file: &syn::File, module: &str, name: &str) -> bool {
        generated_module_items(file, module)
            .iter()
            .any(|item| matches!(item, syn::Item::Trait(item_trait) if item_trait.ident == name))
    }

    fn module_enum(file: &syn::File, module: &str, name: &str) -> bool {
        generated_module_items(file, module)
            .iter()
            .any(|item| matches!(item, syn::Item::Enum(item_enum) if item_enum.ident == name))
    }

    fn module_struct(file: &syn::File, module: &str, name: &str) -> bool {
        generated_module_items(file, module)
            .iter()
            .any(|item| matches!(item, syn::Item::Struct(item_struct) if item_struct.ident == name))
    }

    fn module_trait_impl_for_type(
        file: &syn::File,
        module: &str,
        trait_name: &str,
        type_name: &str,
    ) -> bool {
        generated_module_items(file, module).iter().any(|item| {
            let syn::Item::Impl(item_impl) = item else {
                return false;
            };
            let Some((_, trait_path, _)) = &item_impl.trait_ else {
                return false;
            };
            let trait_matches = trait_path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == trait_name);
            let type_matches = match item_impl.self_ty.as_ref() {
                syn::Type::Path(type_path) => type_path.path.is_ident(type_name),
                _ => false,
            };
            trait_matches && type_matches
        })
    }

    fn trait_impl_for_type(file: &syn::File, trait_name: &str, type_name: &str) -> bool {
        trait_impl_for_type_in_items(&file.items, trait_name, type_name)
    }

    fn trait_impl_for_type_in_items(
        items: &[syn::Item],
        trait_name: &str,
        type_name: &str,
    ) -> bool {
        items.iter().any(|item| match item {
            syn::Item::Impl(item_impl) => {
                let Some((_, trait_path, _)) = &item_impl.trait_ else {
                    return false;
                };
                let trait_matches = trait_path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == trait_name);
                let type_matches = match item_impl.self_ty.as_ref() {
                    syn::Type::Path(type_path) => type_path
                        .path
                        .segments
                        .last()
                        .is_some_and(|segment| segment.ident == type_name),
                    _ => false,
                };
                trait_matches && type_matches
            }
            syn::Item::Mod(item_mod) => item_mod.content.as_ref().is_some_and(|(_, items)| {
                trait_impl_for_type_in_items(items, trait_name, type_name)
            }),
            _ => false,
        })
    }

    fn module_trait_method(file: &syn::File, module: &str, trait_name: &str, method: &str) -> bool {
        generated_module_items(file, module)
            .iter()
            .any(|item| match item {
                syn::Item::Trait(item_trait) if item_trait.ident == trait_name => {
                    item_trait.items.iter().any(|trait_item| match trait_item {
                        syn::TraitItem::Fn(function) => function.sig.ident == method,
                        _ => false,
                    })
                }
                _ => false,
            })
    }

    fn generated_nested_module_items<'a>(file: &'a syn::File, path: &[&str]) -> &'a [syn::Item] {
        find_nested_module_items(&file.items, path)
            .unwrap_or_else(|| panic!("expected generated module path {}", path.join("::")))
    }

    fn find_nested_module_items<'a>(
        items: &'a [syn::Item],
        path: &[&str],
    ) -> Option<&'a [syn::Item]> {
        let Some((module, rest)) = path.split_first() else {
            return Some(items);
        };
        items.iter().find_map(|item| match item {
            syn::Item::Mod(item_mod) if item_mod.ident == *module => item_mod
                .content
                .as_ref()
                .and_then(|(_, module_items)| find_nested_module_items(module_items, rest)),
            _ => None,
        })
    }

    fn module_function(file: &syn::File, path: &[&str], name: &str) -> bool {
        generated_nested_module_items(file, path)
            .iter()
            .any(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == name))
    }

    fn generated_enum_variant(file: &syn::File, variant: &str) -> bool {
        generated_enum_variant_in_items(&file.items, variant)
    }

    fn generated_enum_variant_in_items(items: &[syn::Item], variant: &str) -> bool {
        items.iter().any(|item| match item {
            syn::Item::Enum(item_enum) => item_enum
                .variants
                .iter()
                .any(|item_variant| item_variant.ident == variant),
            syn::Item::Mod(item_mod) => item_mod
                .content
                .as_ref()
                .is_some_and(|(_, items)| generated_enum_variant_in_items(items, variant)),
            _ => false,
        })
    }

    fn generated_enum_variants(item_enum: &syn::ItemEnum) -> Vec<String> {
        item_enum
            .variants
            .iter()
            .map(|variant| variant.ident.to_string())
            .collect()
    }

    fn generated_struct_fields(item_struct: &syn::ItemStruct) -> Vec<String> {
        match &item_struct.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|field| field.ident.as_ref())
                .map(ToString::to_string)
                .collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn test_optimization_summary_disabled() {
        let grammar = create_simple_grammar();
        let summary = grammar.optimization_summary();
        assert!(
            summary.contains("disabled") || summary.contains("not enabled"),
            "Expected disabled message, got: {}",
            summary
        );
    }

    #[test]
    fn test_optimization_summary_enabled() {
        let grammar = create_grammar_with_stats();
        let summary = grammar.optimization_summary();
        // Should contain vocabulary stats information
        assert!(
            !summary.contains("disabled"),
            "Expected enabled stats, got: {}",
            summary
        );
    }

    #[test]
    fn test_warnings_summary_no_analysis() {
        let grammar = create_simple_grammar();
        let summary = grammar.warnings_summary();
        assert!(
            summary.contains("No schema quality analysis performed"),
            "Expected no analysis message, got: {}",
            summary
        );
    }

    #[test]
    fn test_has_warnings_false() {
        let grammar = create_simple_grammar();
        assert!(!grammar.has_warnings());
        assert_eq!(grammar.warning_count(), 0);
    }

    #[test]
    fn test_warning_count_zero() {
        let grammar = create_simple_grammar();
        assert_eq!(grammar.warning_count(), 0);
    }

    #[test]
    fn test_supports_partial_encoding_struct_root() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let grammar = TctCompiler::new(schema, "TestStruct")
            .unwrap()
            .compile()
            .unwrap();

        // Struct roots support partial encoding
        assert!(grammar.supports_partial_encoding());
    }

    #[test]
    fn test_supports_partial_encoding_enum_root() {
        let schema = json!({
            "oneOf": [
                {"type": "string"},
                {"type": "number"}
            ]
        });
        let grammar = TctCompiler::new(schema, "TestEnum")
            .unwrap()
            .compile()
            .unwrap();

        // Enum roots (oneOf/anyOf) do NOT support partial encoding
        assert!(!grammar.supports_partial_encoding());
    }

    #[test]
    fn test_performance_summary_collected() {
        let grammar = create_simple_grammar();
        let summary = grammar.performance_summary();
        // Performance metrics are always collected
        assert!(
            summary.contains("Token Metrics") || summary.contains("Timing Metrics"),
            "Expected performance summary, got: {}",
            summary
        );
    }

    #[test]
    fn test_has_performance_metrics_true() {
        let grammar = create_simple_grammar();
        // Performance metrics are always collected during compilation
        assert!(grammar.has_performance_metrics());
    }

    #[test]
    fn test_token_metrics_some() {
        let grammar = create_simple_grammar();
        // Performance metrics are always collected
        assert!(grammar.token_metrics().is_some());
    }

    #[test]
    fn test_timing_metrics_some() {
        let grammar = create_simple_grammar();
        // Performance metrics are always collected
        assert!(grammar.timing_metrics().is_some());
    }

    #[test]
    fn test_complexity_metrics_some() {
        let grammar = create_simple_grammar();
        // Performance metrics are always collected
        assert!(grammar.complexity_metrics().is_some());
    }

    #[test]
    fn test_generated_runtime_carries_attribution() {
        let grammar = TctCompiler::new(json!({"type": "string", "enum": ["red", "blue"]}), "Color")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();
        assert!(code.starts_with("// Generated by TCT"));
        assert!(code.contains("TCT implementation portions: Copyright 2026 Jonas Schmitt."));
        assert!(code.contains("TCT implementation license: Apache-2.0."));
        assert!(code.contains("Input schemas and data retain their separate rights."));
        assert!(code.contains("Embedded third-party schema components retain their own licenses."));
        assert!(
            code.contains("Redistribute their exact source and applicable notices with this code.")
        );
        parse_generated_file(&code);
    }

    #[test]
    fn test_generate_code_basic_enum() {
        let schema = json!({"type": "string", "enum": ["hello", "world"]});
        let grammar = TctCompiler::new(schema, "TestEnum")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);
        let enum_item = generated_enum(&file, "TestEnum");

        assert_eq!(
            generated_enum_variants(enum_item),
            vec!["Hello".to_string(), "World".to_string()]
        );
    }

    #[test]
    fn test_generate_code_basic_struct() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });
        let grammar = TctCompiler::new(schema, "Person")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);
        let fields = generated_struct_fields(generated_struct(&file, "Person"));

        assert!(fields.contains(&"name".to_string()));
        assert!(fields.contains(&"age".to_string()));
    }

    #[test]
    fn test_generate_code_includes_tokenizer_module() {
        let grammar = create_simple_grammar();
        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        assert!(generated_module(&file, "tokenizer"));
        assert!(module_trait(&file, "tokenizer", "JsonTokenEncoder"));
        assert!(module_enum(&file, "tokenizer", "DecodeError"));
        assert!(module_struct(&file, "tokenizer", "EOFContext"));
    }

    #[test]
    fn test_generate_code_includes_role_export_module() {
        let grammar = create_simple_grammar();
        let code = grammar.generate_code().unwrap();
        // syn parse ensures the module is syntactically sound; content
        // checks pin the pieces role consumers depend on.
        parse_generated_file(&code);
        assert!(
            code.contains("pub mod role_export"),
            "role_export module missing"
        );
        assert!(code.contains("ROLE_STRUCTURE"), "role constants missing");
        assert!(
            code.contains("fn token_roles"),
            "token_roles entry point missing"
        );
        assert!(
            code.contains("fn strict_prefix_roles"),
            "strict prefix role entry point missing"
        );
        assert!(
            code.contains("T :: decode_strict") || code.contains("T::decode_strict"),
            "strict prefix roles must replay strict decode"
        );
        // the String decode loop must mark value bytes and the terminator
        assert!(
            code.contains("role_export :: mark_value ()")
                || code.contains("role_export::mark_value()"),
            "String decode does not mark value bytes"
        );
        assert!(
            code.contains("role_export :: mark_value_end ()")
                || code.contains("role_export::mark_value_end()"),
            "String decode does not mark the end-of-value marker"
        );
    }

    #[test]
    fn test_generate_code_decimal_codec_marks_roles() {
        let schema = json!({
            "type": "object",
            "properties": {"rate": {"type": "number"}},
            "required": ["rate"],
        });
        let grammar = TctCompiler::new(schema, "RateHolder")
            .unwrap()
            .with_decimal_number_encoding()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();
        parse_generated_file(&code);
        let codec_start = code
            .find("mod decimal_number_codec")
            .expect("decimal codec missing under with_decimal_number_encoding");
        let codec_slice = &code[codec_start..];
        assert!(
            codec_slice.contains("mark_value_end"),
            "decimal codec does not mark the end-of-value marker"
        );
        assert!(
            codec_slice.contains("mark_value"),
            "decimal codec does not mark digit bytes"
        );
    }

    #[test]
    fn test_generate_code_oneof_marks_discriminator() {
        // A oneOf/enum root decodes a variant-selector token; that token must be
        // tagged ROLE_DISCRIMINATOR so consumers (aux tool-discriminant loss) can
        // find it — its value alone collides with byte/key tokens.
        let schema = json!({
            "oneOf": [
                {"type": "string"},
                {"type": "number"}
            ]
        });
        let grammar = TctCompiler::new(schema, "PickOne")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();
        parse_generated_file(&code);
        assert!(
            code.contains("ROLE_DISCRIMINATOR"),
            "ROLE_DISCRIMINATOR constant missing from role_export"
        );
        // Must be a CALL site (role_export::mark_discriminator()), not merely the
        // fn definition — string|number is a complex (data-variant) enum, so this
        // pins the complex decode path that the agentic tool oneOf actually uses.
        assert!(
            code.contains("role_export :: mark_discriminator ()")
                || code.contains("role_export::mark_discriminator()"),
            "enum decode does not CALL mark_discriminator on the variant-selector"
        );
    }

    #[test]
    fn test_generate_code_includes_vocab_size_constant() {
        let grammar = create_simple_grammar();
        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        assert!(generated_const(&file, "VOCAB_SIZE"));
    }

    #[test]
    fn test_generate_code_includes_pad_token() {
        let grammar = create_simple_grammar();
        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        assert!(generated_const(&file, "PAD_TOKEN"));
    }

    #[test]
    fn test_generated_enum_clone() {
        let definition = quote! { pub enum Test { A, B } };
        let implementation = quote! { impl Test { fn new() -> Self { Test::A } } };

        let enum1 = GeneratedEnum {
            type_name: "Test".to_string(),
            definition: definition.clone(),
            implementation: implementation.clone(),
        };

        let enum2 = enum1.clone();

        // Verify clone works
        assert_eq!(enum1.definition.to_string(), enum2.definition.to_string());
        assert_eq!(
            enum1.implementation.to_string(),
            enum2.implementation.to_string()
        );
    }

    #[test]
    fn test_generated_struct_clone() {
        let definition = quote! { pub struct Test { field: String } };
        let implementation =
            Some(quote! { impl Test { fn new() -> Self { Test { field: String::new() } } } });

        let struct1 = GeneratedStruct {
            definition: definition.clone(),
            implementation: implementation.clone(),
        };

        let struct2 = struct1.clone();

        // Verify clone works
        assert_eq!(
            struct1.definition.to_string(),
            struct2.definition.to_string()
        );
    }

    #[test]
    fn test_generated_struct_without_implementation() {
        let definition = quote! { pub struct Test { field: String } };

        let generated_struct = GeneratedStruct {
            definition: definition.clone(),
            implementation: None,
        };

        // Verify None implementation works
        assert!(generated_struct.implementation.is_none());
    }

    #[test]
    fn test_profiling_module_clone() {
        let module_code = quote! {
            pub mod profiling {
                pub fn benchmark() {}
            }
        };
        let benchmark_file = Some("benchmark content".to_string());

        let prof1 = ProfilingModule {
            module_code: module_code.clone(),
            benchmark_file: benchmark_file.clone(),
        };

        let prof2 = prof1.clone();

        // Verify clone works
        assert_eq!(prof1.module_code.to_string(), prof2.module_code.to_string());
        assert_eq!(prof1.benchmark_file, prof2.benchmark_file);
    }

    #[test]
    fn test_definitions_clone() {
        let mut structs = FxHashMap::default();
        structs.insert(
            "Test".to_string(),
            GeneratedStruct {
                definition: quote! { pub struct Test {} },
                implementation: None,
            },
        );

        let enums = vec![GeneratedEnum {
            type_name: "E".to_string(),
            definition: quote! { pub enum E { A } },
            implementation: quote! { impl E {} },
        }];

        let constants = vec![quote! { pub const C: u32 = 42; }];
        let type_aliases = vec![quote! { pub type T = String; }];

        let defs1 = Definitions {
            structs,
            enums,
            constants,
            type_aliases,
        };

        let defs2 = defs1.clone();

        // Verify clone works
        assert_eq!(defs1.structs.len(), defs2.structs.len());
        assert_eq!(defs1.enums.len(), defs2.enums.len());
        assert_eq!(defs1.constants.len(), defs2.constants.len());
        assert_eq!(defs1.type_aliases.len(), defs2.type_aliases.len());
    }

    #[test]
    fn test_generate_code_with_nested_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "email": {"type": "string"}
                    }
                }
            }
        });
        let grammar = TctCompiler::new(schema, "NestedTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        generated_struct(&file, "NestedTest");
        generated_struct(&file, "NestedTestUser");
    }

    #[test]
    fn test_generate_code_with_array() {
        let schema = json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            }
        });
        let grammar = TctCompiler::new(schema, "ArrayTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        generated_struct(&file, "ArrayTest");
        assert!(code.contains("Vec<String>"), "Expected Vec type");
    }

    #[test]
    fn test_generate_code_with_definitions() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"$ref": "#/definitions/Status"}
            },
            "definitions": {
                "Status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                }
            }
        });
        let grammar = TctCompiler::new(schema, "WithDefs")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);
        let variants = generated_enum_variants(generated_enum(&file, "Status"));

        generated_struct(&file, "WithDefs");
        assert!(variants.contains(&"Active".to_string()));
        assert!(variants.contains(&"Inactive".to_string()));
    }

    #[test]
    fn test_generate_code_with_required_fields() {
        let schema = json!({
            "type": "object",
            "properties": {
                "required_field": {"type": "string"},
                "optional_field": {"type": "integer"}
            },
            "required": ["required_field"]
        });
        let grammar = TctCompiler::new(schema, "RequiredTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);
        let fields = generated_struct_fields(generated_struct(&file, "RequiredTest"));

        assert!(fields.contains(&"required_field".to_string()));
        assert!(fields.contains(&"optional_field".to_string()));
    }

    #[test]
    fn test_generate_code_with_multiple_types_in_definitions() {
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {"$ref": "#/definitions/Config"},
                "status": {"$ref": "#/definitions/Status"}
            },
            "definitions": {
                "Config": {
                    "type": "object",
                    "properties": {
                        "enabled": {"type": "boolean"}
                    }
                },
                "Status": {
                    "type": "string",
                    "enum": ["on", "off"]
                }
            }
        });
        let grammar = TctCompiler::new(schema, "MultiDefs")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        generated_struct(&file, "MultiDefs");
        generated_struct(&file, "Config");
        generated_enum(&file, "Status");
    }

    #[test]
    fn test_generate_code_integer_type() {
        let schema = json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 100
        });
        let grammar = TctCompiler::new(schema, "IntegerRoot")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();

        // Should generate type for bounded integer
        assert!(code.contains("IntegerRoot"), "Expected IntegerRoot type");
    }

    #[test]
    fn test_generate_code_boolean_type() {
        let schema = json!({"type": "boolean"});
        let grammar = TctCompiler::new(schema, "BoolRoot")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();

        // Should generate code for boolean
        assert!(code.contains("BoolRoot"), "Expected BoolRoot type");
    }

    #[test]
    fn test_generate_code_string_type() {
        let schema = json!({"type": "string"});
        let grammar = TctCompiler::new(schema, "StringRoot")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();

        // Should generate code for string
        assert!(code.contains("StringRoot"), "Expected StringRoot type");
    }

    #[test]
    fn test_generate_code_includes_max_base_vocab() {
        let grammar = create_simple_grammar();
        let code = grammar.generate_code().unwrap();

        // Should include MAX_BASE_VOCAB constant in tokenizer module
        assert!(
            code.contains("pub const MAX_BASE_VOCAB:"),
            "Expected MAX_BASE_VOCAB constant"
        );
    }

    #[test]
    fn test_generate_code_includes_optional_field_type() {
        let schema = json!({
            "type": "object",
            "properties": {
                "nullable_field": {"type": ["string", "null"]}
            }
        });
        let grammar = TctCompiler::new(schema, "NullableTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);
        let variants = generated_enum_variants(generated_enum(&file, "OptionalField"));

        assert!(variants.contains(&"Absent".to_string()));
        assert!(variants.contains(&"Null".to_string()));
        assert!(variants.contains(&"Present".to_string()));
    }

    #[test]
    fn test_generate_code_parsing_error() {
        // This test verifies error handling in generate_code
        // We can't easily trigger a parsing error with valid schemas,
        // so this test documents the expected error type

        let schema = json!({"type": "string"});
        let grammar = TctCompiler::new(schema, "ValidSchema")
            .unwrap()
            .compile()
            .unwrap();

        // Should succeed for valid grammar
        let result = grammar.generate_code();
        assert!(
            result.is_ok(),
            "Valid grammar should generate code successfully"
        );
    }

    // Phase 3: Tests for helper method code paths via public API

    #[test]
    fn test_generate_code_with_profiling_basic() {
        // Test basic profiling module generation
        let schema = json!({
            "type": "object",
            "properties": {
                "value": {"type": "integer"}
            }
        });

        let grammar = TctCompiler::new(schema, "ProfilingTest")
            .unwrap()
            .with_profiling(crate::profiling_config::ProfilingConfig::basic())
            .compile()
            .unwrap();

        // Verify profiling module is present in grammar
        assert!(
            grammar.profiling_code.is_some(),
            "Expected profiling code to be generated"
        );

        let code = grammar.generate_code().unwrap();

        // Verify profiling-specific code elements
        assert!(
            code.contains("pub mod profiling"),
            "Expected profiling module in generated code"
        );
    }

    #[test]
    fn test_generate_code_with_profiling_detailed() {
        // Test detailed profiling configuration
        let schema = json!({
            "type": "object",
            "properties": {
                "data": {"type": "string"}
            }
        });

        let grammar = TctCompiler::new(schema, "DetailedProfilingTest")
            .unwrap()
            .with_profiling(crate::profiling_config::ProfilingConfig::detailed())
            .compile()
            .unwrap();

        // Verify profiling code is present
        assert!(
            grammar.profiling_code.is_some(),
            "Expected detailed profiling code to be generated"
        );
    }

    #[test]
    fn test_generate_code_with_benchmarking() {
        // Test benchmark file generation
        let schema = json!({
            "type": "object",
            "properties": {
                "data": {"type": "string"}
            }
        });

        let grammar = TctCompiler::new(schema, "BenchmarkTest")
            .unwrap()
            .with_profiling(
                crate::profiling_config::ProfilingConfig::benchmarking()
                    .with_criterion_sample("basic", json!({"data": "example"})),
            )
            .compile()
            .unwrap();

        // Verify profiling code with benchmark file is present
        assert!(
            grammar.profiling_code.is_some(),
            "Expected profiling code with benchmarking"
        );
        assert!(
            grammar
                .profiling_code
                .as_ref()
                .and_then(|p| p.benchmark_file.as_ref())
                .is_some(),
            "Expected benchmark file content"
        );
    }

    #[test]
    fn test_definitions_with_constants() {
        // Test that constants from definitions are included in generated code
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                }
            }
        });

        let grammar = TctCompiler::new(schema, "ConstantsTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Constants should be generated for enum values
        assert!(
            !grammar.definitions.constants.is_empty() || generated_enum_variant(&file, "Active"),
            "Expected constants or enum definitions"
        );
    }

    #[test]
    fn test_root_type_name_preservation() {
        // Test that root type name is preserved correctly
        let schema = json!({
            "type": "object",
            "properties": {
                "field": {"type": "string"}
            }
        });

        let grammar = TctCompiler::new(schema, "CustomTypeName")
            .unwrap()
            .compile()
            .unwrap();

        assert_eq!(
            grammar.root_type_name, "CustomTypeName",
            "Root type name should be preserved"
        );

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);
        generated_struct(&file, "CustomTypeName");
    }

    #[test]
    fn test_base_vocab_size_calculation() {
        // Test that base_vocab is calculated correctly
        let schema = json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"}
            }
        });

        let grammar = TctCompiler::new(schema, "VocabTest")
            .unwrap()
            .compile()
            .unwrap();

        // Base vocab should be non-zero for schemas with strings
        assert!(
            grammar.base_vocab > 0,
            "Base vocabulary should be calculated for string-containing schemas"
        );
    }

    #[test]
    fn test_generate_code_formatting_valid_rust() {
        // Test that generated code is valid Rust syntax (no obvious formatting errors)
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "active": {"type": "boolean"}
            },
            "required": ["name"]
        });

        let grammar = TctCompiler::new(schema, "FormattingTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Basic syntax checks
        generated_struct(&file, "FormattingTest");
        assert!(trait_impl_for_type(
            &file,
            "JsonTokenEncoder",
            "FormattingTest"
        ));
        assert!(
            code.contains("fn encode") && code.contains("tokens: &mut Vec<u32>"),
            "Should have encode method"
        );
        assert!(
            code.contains("fn decode") && code.contains("Iterator<Item = u32>"),
            "Should have decode method"
        );
        assert!(
            generated_module(&file, "tokenizer"),
            "Should have tokenizer module"
        );
    }

    // ========================================================================================
    // New Tests for Uncovered Code Paths
    // ========================================================================================

    #[test]
    fn test_optimization_summary_with_stats() {
        let schema = json!({"type": "string", "enum": ["a", "b", "c"]});
        let mut grammar = TctCompiler::new(schema, "TestEnum")
            .unwrap()
            .compile()
            .unwrap();

        // Add mock vocabulary stats
        grammar.vocabulary_stats = Some(crate::vocabulary_encoder::VocabularyOptimizationStats {
            optimized_fields: 5,
            total_constrained_values: 20,
            naive_vocabulary_size: 1000,
            distinct_structural_tokens_count: 500,
            tokens_saved: 500,
            reduction_percentage: 50.0,
            pattern_domains: 0,
            pattern_vocabulary_reduction: 0.0,
            pattern_tokens_saved: 0,
            pattern_tokens_allocated: 0,
            max_structural_token_id_plus_one: 500,
            estimated_unoptimized_tokens: 1000,
            total_vocabulary_reduction: 50.0,
            enum_domains: 3,
            bounded_integer_domains: 2,
            optimization_breakdown: rustc_hash::FxHashMap::default(),
        });

        let summary = grammar.optimization_summary();
        assert!(summary.contains("50"));
        assert!(summary.contains("500"));
    }

    #[test]
    fn test_optimization_summary_without_stats() {
        let schema = json!({"type": "string"});
        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let summary = grammar.optimization_summary();
        // When no vocabulary_stats, returns "Vocabulary optimization disabled"
        assert!(summary.contains("disabled") || summary.contains("Vocabulary"));
    }

    #[test]
    fn test_warnings_summary_with_warnings() {
        let schema = json!({"type": "string", "enum": ["a", "b", "c"]});
        let mut grammar = TctCompiler::new(schema, "TestEnum")
            .unwrap()
            .compile()
            .unwrap();

        // Add mock warnings
        grammar.warnings = Some(crate::warnings::SchemaWarnings {
            warnings: vec![crate::warnings::SchemaWarning {
                warning_type: crate::warnings::WarningType::MissedOptimization,
                severity: crate::warnings::WarningSeverity::Warning,
                schema_path: "/test".to_string(),
                message: "Test warning".to_string(),
                suggestion: "Test suggestion".to_string(),
                example: None,
                impact: crate::warnings::OptimizationImpact {
                    token_reduction: Some(0.1),
                    compilation_impact: crate::warnings::CompilationImpact::Neutral,
                    type_safety: crate::warnings::TypeSafetyImpact::None,
                },
            }],
        });

        let summary = grammar.warnings_summary();
        assert!(summary.contains("Test warning") || summary.contains("1"));
    }

    #[test]
    fn test_warnings_summary_without_warnings() {
        let schema = json!({"type": "string"});
        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let summary = grammar.warnings_summary();
        // When no warnings analysis, returns "✅ No schema quality analysis performed"
        assert!(summary.contains("No") || summary.contains("analysis"));
    }

    #[test]
    fn test_has_warnings() {
        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        assert!(!grammar.has_warnings());

        grammar.warnings = Some(crate::warnings::SchemaWarnings {
            warnings: vec![crate::warnings::SchemaWarning {
                warning_type: crate::warnings::WarningType::GenericValueUsage,
                severity: crate::warnings::WarningSeverity::Info,
                schema_path: "/test".to_string(),
                message: "Test".to_string(),
                suggestion: "Consider adding constraints".to_string(),
                example: None,
                impact: crate::warnings::OptimizationImpact {
                    token_reduction: None,
                    compilation_impact: crate::warnings::CompilationImpact::Neutral,
                    type_safety: crate::warnings::TypeSafetyImpact::None,
                },
            }],
        });

        assert!(grammar.has_warnings());
    }

    #[test]
    fn test_warning_count() {
        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        assert_eq!(grammar.warning_count(), 0);

        grammar.warnings = Some(crate::warnings::SchemaWarnings {
            warnings: vec![
                crate::warnings::SchemaWarning {
                    warning_type: crate::warnings::WarningType::GenericValueUsage,
                    severity: crate::warnings::WarningSeverity::Info,
                    schema_path: "/test1".to_string(),
                    message: "Test1".to_string(),
                    suggestion: "Add constraints".to_string(),
                    example: None,
                    impact: crate::warnings::OptimizationImpact {
                        token_reduction: None,
                        compilation_impact: crate::warnings::CompilationImpact::Neutral,
                        type_safety: crate::warnings::TypeSafetyImpact::None,
                    },
                },
                crate::warnings::SchemaWarning {
                    warning_type: crate::warnings::WarningType::MissedOptimization,
                    severity: crate::warnings::WarningSeverity::Warning,
                    schema_path: "/test2".to_string(),
                    message: "Test2".to_string(),
                    suggestion: "Optimize schema".to_string(),
                    example: None,
                    impact: crate::warnings::OptimizationImpact {
                        token_reduction: Some(0.2),
                        compilation_impact: crate::warnings::CompilationImpact::Neutral,
                        type_safety: crate::warnings::TypeSafetyImpact::None,
                    },
                },
            ],
        });

        assert_eq!(grammar.warning_count(), 2);
    }

    #[test]
    fn test_supports_partial_encoding_struct() {
        let schema = json!({
            "type": "object",
            "properties": {"value": {"type": "string"}}
        });
        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        assert!(grammar.supports_partial_encoding());
    }

    #[test]
    fn test_supports_partial_encoding_enum() {
        let schema = json!({
            "oneOf": [
                {"type": "string"},
                {"type": "integer"}
            ]
        });
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        grammar.is_enum_root = true;
        assert!(!grammar.supports_partial_encoding());
    }

    #[test]
    fn test_supports_partial_encoding_enum_with_discriminator() {
        // Enum root with discriminator (like Kubernetes) SHOULD support partial encoding
        let schema = json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "kind": {"enum": ["Pod"]},
                        "name": {"type": "string"}
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "kind": {"enum": ["Service"]},
                        "port": {"type": "integer"}
                    }
                }
            ]
        });
        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        // This schema has a discriminator pattern, so has_enum_discriminator should be true
        // The compiler sets this from discriminator detection
        assert!(grammar.is_enum_root);
        assert!(grammar.has_enum_discriminator);
        assert!(grammar.supports_partial_encoding());
    }

    #[test]
    fn test_supports_partial_encoding_enum_without_discriminator() {
        // Enum root without discriminator should NOT support partial encoding
        let mut grammar = create_simple_grammar();
        grammar.is_enum_root = true;
        grammar.has_enum_discriminator = false;
        assert!(!grammar.supports_partial_encoding());
    }

    #[test]
    fn test_performance_summary_with_metrics() {
        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        grammar.performance_metrics = Some(crate::metrics::PerformanceMetrics {
            tokens: crate::metrics::TokenMetrics {
                total_tokens: 100,
                structure_tokens: 30,
                value_tokens: 70,
                tokens_saved: 10,
                unoptimized_tokens: 110,
                type_breakdown: rustc_hash::FxHashMap::default(),
            },
            timing: crate::metrics::TimingMetrics::new(),
            complexity: crate::metrics::ComplexityMetrics::new(),
        });

        let summary = grammar.performance_summary();
        assert!(summary.contains("100") || summary.contains("10"));
    }

    #[test]
    fn test_performance_summary_without_metrics() {
        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        // Clear any default performance metrics
        grammar.performance_metrics = None;

        let summary = grammar.performance_summary();
        // When no performance_metrics, returns "Performance metrics not collected"
        assert!(summary.contains("not collected") || summary.contains("Performance"));
    }

    #[test]
    fn test_has_performance_metrics() {
        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        // Clear any default performance metrics
        grammar.performance_metrics = None;
        assert!(!grammar.has_performance_metrics());

        // Set performance metrics
        grammar.performance_metrics = Some(crate::metrics::PerformanceMetrics {
            tokens: crate::metrics::TokenMetrics::new(),
            timing: crate::metrics::TimingMetrics::new(),
            complexity: crate::metrics::ComplexityMetrics::new(),
        });

        assert!(grammar.has_performance_metrics());
    }

    #[test]
    fn test_token_metrics_accessor() {
        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        // Set custom token metrics
        let mut metrics = crate::metrics::TokenMetrics::new();
        metrics.total_tokens = 50;
        metrics.structure_tokens = 10;
        metrics.value_tokens = 40;

        grammar.performance_metrics = Some(crate::metrics::PerformanceMetrics {
            tokens: metrics,
            timing: crate::metrics::TimingMetrics::new(),
            complexity: crate::metrics::ComplexityMetrics::new(),
        });

        // Verify accessor returns the metrics
        assert!(grammar.token_metrics().is_some());
        assert_eq!(grammar.token_metrics().unwrap().total_tokens, 50);
        assert_eq!(grammar.token_metrics().unwrap().structure_tokens, 10);
    }

    #[test]
    fn test_timing_metrics_accessor() {
        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        // Set custom timing metrics
        let mut timing = crate::metrics::TimingMetrics::new();
        timing.total_time = std::time::Duration::from_millis(100);

        grammar.performance_metrics = Some(crate::metrics::PerformanceMetrics {
            tokens: crate::metrics::TokenMetrics::new(),
            timing,
            complexity: crate::metrics::ComplexityMetrics::new(),
        });

        // Verify accessor returns the metrics
        assert!(grammar.timing_metrics().is_some());
        assert_eq!(
            grammar.timing_metrics().unwrap().total_time,
            std::time::Duration::from_millis(100)
        );
    }

    #[test]
    fn test_complexity_metrics_accessor() {
        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        // Set custom complexity metrics
        let mut complexity = crate::metrics::ComplexityMetrics::new();
        complexity.max_depth = 5;
        complexity.object_count = 10;

        grammar.performance_metrics = Some(crate::metrics::PerformanceMetrics {
            tokens: crate::metrics::TokenMetrics::new(),
            timing: crate::metrics::TimingMetrics::new(),
            complexity,
        });

        // Verify accessor returns the metrics
        assert!(grammar.complexity_metrics().is_some());
        assert_eq!(grammar.complexity_metrics().unwrap().max_depth, 5);
        assert_eq!(grammar.complexity_metrics().unwrap().object_count, 10);
    }

    #[test]
    fn test_generate_code_with_vocab_constraints() {
        let schema = json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"}
            }
        });

        let grammar = TctCompiler::new(schema, "VocabTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        assert!(generated_module(&file, "tokenizer"));
        assert!(module_trait(&file, "tokenizer", "JsonTokenEncoder"));
    }

    #[test]
    fn test_generate_code_tokenizer_module_primitives() {
        let schema = json!({
            "type": "object",
            "properties": {
                "flag": {"type": "boolean"},
                "count": {"type": "integer"},
                "value": {"type": "number"}
            }
        });

        let grammar = TctCompiler::new(schema, "PrimitivesTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        assert!(generated_module(&file, "tokenizer"));
        assert!(module_trait_impl_for_type(
            &file,
            "tokenizer",
            "JsonTokenEncoder",
            "bool"
        ));
    }

    #[test]
    fn test_generate_code_with_pad_token() {
        let schema = json!({"type": "string"});

        let grammar = TctCompiler::new(schema, "PadTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        assert!(generated_const(&file, "PAD_TOKEN"));
        assert!(generated_const(&file, "VOCAB_SIZE"));
    }

    #[test]
    fn test_generate_code_buffered_iterator() {
        let schema = json!({"type": "string"});

        let grammar = TctCompiler::new(schema, "BufferedTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();

        // Should generate BufferedTokenIterator
        assert!(code.contains("BufferedTokenIterator"));
        assert!(code.contains("fn from_vec"));
        assert!(code.contains("fn remaining"));
        assert!(code.contains("BufferedTokenIterator::from_vec"));
    }

    #[test]
    fn test_generate_code_with_constants() {
        let schema = json!({
            "type": "object",
            "properties": {
                "value": {"type": "integer", "minimum": 0, "maximum": 10}
            }
        });

        let grammar = TctCompiler::new(schema, "ConstTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        generated_struct(&file, "ConstTest");
    }

    #[test]
    fn test_generate_code_struct_ordering() {
        let schema = json!({
            "type": "object",
            "properties": {
                "first": {"$ref": "#/definitions/TypeA"},
                "second": {"$ref": "#/definitions/TypeB"}
            },
            "definitions": {
                "TypeA": {"type": "string"},
                "TypeB": {"type": "integer"}
            }
        });

        let grammar = TctCompiler::new(schema, "OrderTest")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();

        // Definitions should be ordered for deterministic output
        assert!(code.contains("TypeA") || code.contains("TypeB"));
    }

    // ========================================================================================
    // Additional Tests for Uncovered Code Paths (Targeting 85% Coverage)
    // ========================================================================================

    #[test]
    fn test_generate_tokenizer_module_with_string_encoding() {
        let schema = json!({"type": "string"});
        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Should generate standard String encoding
        assert!(module_trait_impl_for_type(
            &file,
            "tokenizer",
            "JsonTokenEncoder",
            "String"
        ));
        assert!(code.contains("END_STRING_MARKER"));
        assert!(!code.contains("string_dictionary"));
        assert!(!code.contains("DICT_MARKER"));
    }

    #[test]
    fn test_generate_code_with_utf8_string_encoding() {
        use crate::generator::encoding_decision::{CompilationConstraints, StringEncoding};

        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        // Set UTF-8 encoding
        grammar.compilation_constraints = Some(CompilationConstraints {
            string_encoding: StringEncoding::Utf8,
            max_vocab: None,
        });

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Should reference UTF-8 byte handling
        assert!(module_trait(&file, "tokenizer", "JsonTokenEncoder"));
        assert!(code.contains("END_STRING_MARKER"));
    }

    #[test]
    fn test_generate_code_with_bpe_merges() {
        use crate::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};

        let schema = json!({"type": "string"});
        let mut grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        // Create BPE merge table manually
        let merge_table = BpeMergeTable {
            schema: "test".to_string(),
            base_vocab_size: 1000,
            merges: vec![BpeMerge {
                pair: [97, 98],
                new_token: 1000,
                frequency: 100,
            }],
            metadata: BpeMetadata {
                trained_on_instances: 1000,
                training_date: "2024-01-01".to_string(),
                max_merges: 1,
                dict_size: None,
            },
        };
        grammar.bpe_merge_table = Some(merge_table);

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Should generate BPE module
        assert!(generated_module(&file, "bpe"));
        assert!(code.contains("BufferedTokenIterator::from_vec(base_tokens)"));
        assert!(code.contains("codec::decode::<T>(payload)"));
    }

    #[test]
    fn test_generate_tokenizer_module_error_types() {
        let schema = json!({"type": "object", "properties": {"val": {"type": "string"}}});
        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Should embed error type definitions for standalone compilation
        assert!(module_enum(&file, "tokenizer", "DecodeError"));
        assert!(module_struct(&file, "tokenizer", "EOFContext"));
    }

    #[test]
    fn test_generate_tokenizer_module_trait_definition() {
        let schema = json!({"type": "boolean"});
        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Should generate JsonTokenEncoder trait
        assert!(module_trait(&file, "tokenizer", "JsonTokenEncoder"));
        assert!(module_trait_method(
            &file,
            "tokenizer",
            "JsonTokenEncoder",
            "encode"
        ));
        assert!(module_trait_method(
            &file,
            "tokenizer",
            "JsonTokenEncoder",
            "decode"
        ));
        assert!(module_trait_method(
            &file,
            "tokenizer",
            "JsonTokenEncoder",
            "token_count"
        ));
    }

    #[test]
    fn test_generated_variable_base_hot_paths_use_fixed_base_helpers() {
        let schema = json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {"type": "integer"}
                },
                "count": {"type": "integer"}
            },
            "required": ["items", "count"]
        });

        let grammar = TctCompiler::new(schema, "FixedBaseHotPaths")
            .unwrap()
            .with_max_base_vocab(512)
            .unwrap()
            .compile()
            .unwrap();
        assert_eq!(grammar.encoding_base, 512);

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        assert!(code.contains("pub const GENERATED_ENCODING_BASE: u32 = 512u32;"));
        assert!(module_function(
            &file,
            &["tokenizer", "variable_base"],
            "encode_u32_fixed_base"
        ));
        assert!(module_function(
            &file,
            &["tokenizer", "variable_base"],
            "decode_u32_fixed_base"
        ));
        assert!(module_function(
            &file,
            &["tokenizer", "variable_base"],
            "token_count_u32_fixed_base"
        ));
        assert!(code.contains("variable_base::encode_u32_fixed_base(self.len() as u32, tokens)"));
        assert!(code.contains("variable_base::decode_u32_fixed_base(tokens, \"Vec<T>\")"));
        assert!(code.contains("variable_base::token_count_u32_fixed_base(self.len() as u32)"));
        assert!(code.contains("variable_base::encode_u32_fixed_base(*self as u32, tokens)"));
        assert!(code.contains("variable_base::decode_u32_fixed_base(tokens, \"i32\")"));
        assert!(code.contains("variable_base::token_count_u32_fixed_base(*self as u32)"));
        assert!(
            !code.contains("variable_base::encode_u32(self.len() as u32, tokens, 512u32)"),
            "Vec/HashMap length hot paths should use the fixed-base generated helper"
        );
        assert!(
            !code.contains("variable_base::token_count_u32(self.len() as u32, 512u32)"),
            "Vec/HashMap token_count hot paths should use the fixed-base generated helper"
        );
        assert!(
            !code.contains("variable_base::encode_u32(*self as u32, tokens, 512u32)"),
            "i32 hot path should use the fixed-base generated helper"
        );
        assert!(
            !code.contains("variable_base::decode_u32(tokens, \"i32\", 512u32)"),
            "i32 decode path should use the fixed-base generated helper"
        );
    }

    #[test]
    fn test_generate_code_with_profiling_comprehensive() {
        use crate::profiling_config::{ProfilingConfig, ProfilingLevel};

        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "value": {"type": "integer"}
            }
        });

        let config = ProfilingConfig {
            level: ProfilingLevel::Detailed,
            track_allocations: true,
            ..Default::default()
        };

        let grammar = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_profiling(config)
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();

        // Should include profiling infrastructure
        assert!(code.contains("profiling"));
    }

    #[test]
    fn test_generate_code_with_enum_root() {
        let schema = json!({
            "oneOf": [
                {"type": "string"},
                {"type": "integer"}
            ]
        });

        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Should generate enum type
        generated_enum(&file, "Test");
    }

    #[test]
    fn test_generate_code_with_complex_nested_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "users": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "email": {"type": "string"}
                        }
                    }
                }
            }
        });

        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Should handle nested structures
        generated_struct(&file, "Test");
        assert!(code.contains("Vec"));
    }

    #[test]
    fn test_generate_code_with_optional_fields() {
        let schema = json!({
            "type": "object",
            "properties": {
                "required_field": {"type": "string"},
                "optional_field": {"type": "integer"}
            },
            "required": ["required_field"]
        });

        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();

        // Should handle optional fields
        assert!(code.contains("Option<"));
    }

    #[test]
    fn test_generate_code_with_all_primitive_types() {
        let schema = json!({
            "type": "object",
            "properties": {
                "bool_val": {"type": "boolean"},
                "int_val": {"type": "integer"},
                "num_val": {"type": "number"},
                "str_val": {"type": "string"}
            }
        });

        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();

        // Should handle all primitive types
        assert!(code.contains("bool"));
        assert!(code.contains("String"));
    }

    #[test]
    fn test_generate_code_formats_valid_rust() {
        let schema = json!({
            "type": "object",
            "properties": {
                "value": {"type": "string"}
            }
        });

        let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);

        // Basic syntax validation - should have balanced braces
        let open_braces = code.matches('{').count();
        let close_braces = code.matches('}').count();
        assert_eq!(open_braces, close_braces);

        // Should have proper module structure
        generated_struct(&file, "Test");
        assert!(code.contains("impl"));
    }
}
