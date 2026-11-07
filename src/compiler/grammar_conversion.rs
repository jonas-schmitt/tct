//! Final compiler handoff from generated Rust fragments to the runtime
//! [`Grammar`](crate::grammar::Grammar) artifact.
//!
//! This module keeps the conversion boundary in `TctCompiler` focused on
//! packaging pass outputs, vocabulary layout, optional BPE/profiling metadata,
//! and compiler metrics after generation has already produced structured
//! `CodeGeneration` results.

use super::TctCompiler;
use crate::error::{TctError, TctResult};
use crate::generator::encoding_decision::EncodingStrategy;
use crate::generator::generation_output::CodeGeneration;
use crate::grammar::Grammar;
use crate::metrics::TimingMetrics;
use rustc_hash::FxHashMap;
use std::time::{Duration, Instant};

impl TctCompiler {
    /// Convert CodeGeneration results to Grammar.
    pub(super) fn convert_to_grammar(
        &self,
        code_generation: CodeGeneration,
        encoding_strategy: Option<EncodingStrategy>,
        compilation_start: Instant,
        pass_timings: FxHashMap<String, Duration>,
        has_enum_discriminator: bool,
        vocabulary_stats_enabled: bool,
    ) -> TctResult<Grammar> {
        use crate::grammar::GeneratedStruct;
        use crate::metrics::{ComplexityMetrics, PerformanceMetrics, TokenMetrics};

        // Extract root generation - guaranteed to exist or return error
        let (root, is_enum_root, root_type_name) = if let Some(root_gen) =
            code_generation.root_generation
        {
            // Capture whether root is a composition (oneOf/anyOf = enum) before conversion
            let is_enum = root_gen.is_composition;
            let root_type_name = root_gen.type_name.clone();

            // Convert RootCodeGeneration to GeneratedStruct
            let root_struct = GeneratedStruct {
                definition: root_gen.definition_tokens,
                implementation: Some(root_gen.implementation_tokens),
            };

            (root_struct, is_enum, root_type_name)
        } else {
            // Look for root type in generated types - use sanitized name
            use crate::utils::to_upper_camel_case;
            let sanitized_root_name = to_upper_camel_case(&self.type_name);

            if let Some(root_type_gen) = code_generation.get_type_generation(&sanitized_root_name) {
                let root_struct = GeneratedStruct {
                    definition: root_type_gen.definition_tokens.clone(),
                    implementation: Some(root_type_gen.implementation_tokens.clone()),
                };
                // Fallback path: assume struct root (not enum) for safety
                (root_struct, false, root_type_gen.type_name.clone())
            } else {
                return Err(TctError::CodeGeneration {
                    message: format!(
                        "Root type '{}' (sanitized: '{}') was not generated",
                        self.type_name, sanitized_root_name
                    ),
                    schema_path: "#/".to_string(),
                    suggestion: "Ensure the root type is properly discovered and generated"
                        .to_string(),
                    phase: crate::error::GenerationPhase::TypeGeneration,
                });
            }
        };

        // Use the definitions from CodeGeneration (already built)
        let mut definitions = code_generation.definitions;

        // CRITICAL FIX: Remove root type from definitions to prevent duplication
        // The root struct will be added separately, so it shouldn't be in definitions.structs
        if definitions.structs.remove(&root_type_name).is_some() {
            crate::log_debug!(
                "🔧 Removed root type '{}' from definitions to prevent duplication",
                root_type_name
            );
        }
        let enum_count_before = definitions.enums.len();
        definitions
            .enums
            .retain(|generated_enum| generated_enum.type_name != root_type_name);
        if definitions.enums.len() != enum_count_before {
            crate::log_debug!(
                "🔧 Removed root enum '{}' from definitions to prevent duplication",
                root_type_name
            );
        }

        // Create performance metrics
        let total_time = compilation_start.elapsed();
        let performance_metrics = Some(PerformanceMetrics {
            tokens: TokenMetrics::default(),
            timing: compiler_timing_metrics(total_time, pass_timings),
            complexity: ComplexityMetrics {
                object_count: definitions.structs.len() as u32,
                enum_count: definitions.enums.len() as u32,
                max_depth: 5, // Could calculate from schema
                constraint_count: code_generation.stats.total_types as u32,
                additional_properties_count: 0,
                unconstrained_additional_properties: 0,
            },
        });

        // Get vocabulary optimization stats from the effective pass
        // configuration. Named built-in passes override the builder's
        // optimization level, so this cannot rely on `self.optimization_level`.
        let vocabulary_stats = if vocabulary_stats_enabled {
            encoding_strategy
                .as_ref()
                .map(|strategy| strategy.stats.clone())
        } else {
            None
        };

        // Detect schema quality warnings
        let warnings = self.detect_warnings();

        // Load BPE merge table if provided
        let bpe_merge_table = if let Some(ref path) = self.bpe_merges_path {
            use crate::bpe::BpeMergeTable;
            Some(BpeMergeTable::from_file(path).map_err(|e| {
                TctError::InvalidConfiguration {
                    field: "bpe_merges".to_string(),
                    reason: format!("Failed to load BPE merge file: {}", e),
                    suggestion:
                        "Ensure the BPE merge file is valid JSON matching the expected format"
                            .to_string(),
                }
            })?)
        } else {
            None
        };

        // Generate profiling code if enabled
        let profiling_code = if !self.profiling_config.is_disabled() {
            use crate::generator::profiling_generator::ProfilingGenerator;
            use crate::utils::to_upper_camel_case;

            let sanitized_type_name = to_upper_camel_case(&self.type_name);
            let generator =
                ProfilingGenerator::new(sanitized_type_name, self.profiling_config.clone());
            Some(generator.generate())
        } else {
            None
        };

        // Compute encoding_base from max_base_vocab if provided, otherwise use default (256)
        let encoding_base = if let Some(max_base_vocab) = self.max_base_vocab {
            crate::optimization_pass::config::OptimizationConfig::compute_encoding_base(Some(
                max_base_vocab,
            ))
        } else {
            crate::optimization_pass::config::OptimizationConfig::compute_encoding_base(None)
        };

        // Use the generation pass's canonical base vocabulary calculation.
        // This keeps Grammar.base_vocab, generated BASE_VOCAB_SIZE, BPE_START,
        // VOCAB_SIZE, and PAD_TOKEN on the same token layout.
        let base_vocab = code_generation.stats.base_vocab_size;

        if let Some(bpe) = &bpe_merge_table {
            if bpe.dict_size() > 0 {
                return Err(TctError::InvalidConfiguration {
                    field: "bpe_merges".to_string(),
                    reason: format!(
                        "BPE merge table was trained with legacy dictionary size {}",
                        bpe.dict_size()
                    ),
                    suggestion:
                        "Retrain BPE on plain TCT token sequences without dictionary compression"
                            .to_string(),
                });
            }
            if bpe.base_vocab_size != base_vocab {
                return Err(TctError::InvalidConfiguration {
                    field: "bpe_merges".to_string(),
                    reason: format!(
                        "BPE was trained with base_vocab_size {} but current generated base vocabulary is {}",
                        bpe.base_vocab_size, base_vocab
                    ),
                    suggestion:
                        "Retrain BPE with the current schema, string encoding, and max_base_vocab settings"
                            .to_string(),
                });
            }
            let expected_bpe_start = crate::bpe::bpe_start_token(base_vocab);
            if bpe.bpe_start() != expected_bpe_start {
                return Err(TctError::InvalidConfiguration {
                    field: "bpe_merges".to_string(),
                    reason: format!(
                        "BPE starts at token {} but current generated vocabulary layout starts BPE at {}",
                        bpe.bpe_start(),
                        expected_bpe_start
                    ),
                    suggestion:
                        "Regenerate the BPE merge table so merge token IDs match the current vocabulary layout"
                            .to_string(),
                });
            }
        }

        Ok(Grammar {
            root,
            definitions,
            root_type_name: self.type_name.clone(),
            is_enum_root,
            has_enum_discriminator,
            base_vocab,
            streaming_encode: self.streaming_encode,
            encoding_base,
            bpe_merge_table,
            vocabulary_stats,
            warnings,
            performance_metrics,
            profiling_code,
            compilation_constraints: Some(self.compilation_constraints.clone()),
            max_collection_length: self.max_collection_length.unwrap_or(2048),
            max_schema_depth: self
                .max_schema_depth
                .unwrap_or(crate::schema_utils::DEFAULT_MAX_SCHEMA_TRAVERSAL_DEPTH),
            decimal_number_encoding: self.decimal_number_encoding,
        })
    }
}

fn compiler_timing_metrics(
    total_time: Duration,
    pass_timings: FxHashMap<String, Duration>,
) -> TimingMetrics {
    let discovery_time = pass_timings
        .get("Discovery")
        .copied()
        .unwrap_or(Duration::ZERO);
    let analysis_time = pass_timings
        .get("Analysis")
        .copied()
        .unwrap_or(Duration::ZERO);
    let optimization_time = pass_timings
        .get("Optimization")
        .copied()
        .unwrap_or(Duration::ZERO);
    let generation_time = pass_timings
        .get("Generation")
        .copied()
        .unwrap_or(Duration::ZERO);

    TimingMetrics {
        schema_parsing: Duration::ZERO,
        schema_analysis: discovery_time + analysis_time,
        code_generation: generation_time,
        optimization: optimization_time,
        total_time,
        phase_timings: pass_timings,
    }
}
