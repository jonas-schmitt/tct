#![recursion_limit = "256"]

//! # TCT: Type-Constrained Tokenization
//!
//! TCT is a toolkit for creating highly efficient, schema-aware tokenizers for Large Language Models (LLMs).
//! Instead of general-purpose tokenization, TCT uses JSON schemas to define precise vocabularies and structures,
//! creating dense, context-dependent encodings whose decoded values always have the schema's structure.
//!
//! ## Quick Start
//!
//! ```
//! use tct::{TctCompiler, OptimizationLevel};
//! use serde_json::json;
//!
//! // Define your data schema
//! let schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "status": {"enum": ["active", "pending", "completed"]},
//!         "priority": {"type": "integer", "minimum": 1, "maximum": 5}
//!     }
//! });
//!
//! // Compile to optimized tokenizer
//! let grammar = TctCompiler::new(schema, "Task")?
//!     .with_optimization_level(OptimizationLevel::O2)
//!     .compile()?;
//!
//! // Generate Rust code
//! let rust_code = grammar.generate_code()?;
//! println!("{}", rust_code);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//!
//! ## Key Benefits
//!
//! - **Schema-Shaped Output**: every successfully decoded value has the schema's structure (value constraints such as patterns and length bounds are not enforced)
//! - **Compact Sequences**: structure that the schema already fixes costs no tokens
//! - **Schema-Specific Vocabularies**: Only the tokens your data actually needs
//! - **LLM-Robust Encoding**: Graceful handling of out-of-vocabulary tokens
//!
//! ## API Overview
//!
//! The main API centers around [`TctCompiler`], which uses a builder pattern for configuration:
//!
//! - **[`TctCompiler`]**: Main entry point for schema compilation
//! - **[`Grammar`]**: Result of compilation containing generated code
//! - **[`OptimizationLevel`]**: Configure compression vs debuggability trade-offs
//! - **[`TctError`]**: Comprehensive error handling with actionable suggestions
//!
//! ## Optimization Levels
//!
//! - **O0**: Development mode - no optimization, maximum debuggability
//! - **O2**: Production mode - balanced optimization (recommended)
//! - **O3**: Aggressive optimization with format-aware encodings (default)
//! - **Os**: Size-focused structural compression profile
//! - **OVocab**: Vocabulary-size-focused profile
//!
//! ## Profiling and Benchmarking
//!
//! TCT includes comprehensive profiling capabilities for performance analysis:
//!
//! ```no_run
//! use tct::{TctCompiler, ProfilingConfig};
//! use serde_json::json;
//!
//! let schema = json!({"type": "string", "enum": ["red", "green", "blue"]});
//! let profiling = ProfilingConfig::benchmarking()
//!     .with_criterion_sample("red", json!("red"))
//!     .with_criterion_sample("blue", json!("blue"));
//! let grammar = TctCompiler::new(schema, "Color")?
//!     .with_profiling(profiling)
//!     .compile()?;
//!
//! // Generate code with built-in profiling
//! let code = grammar.generate_code()?;
//!
//! // Generate Criterion benchmark file (requires benches/ directory)
//! if let Some(profiling) = &grammar.profiling_code {
//!     if let Some(benchmark_file) = &profiling.benchmark_file {
//!         std::fs::write("benches/color_benchmarks.rs", benchmark_file)?;
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Profiling Features
//!
//! - **Zero-overhead**: generated crates gate profiling code behind their own `profiling` Cargo feature
//! - **Multiple levels**: Basic timing, detailed analysis, benchmarking
//! - **Criterion integration**: Auto-generated comprehensive benchmarks
//! - **Multiple data sizes**: Small, medium, large test scenarios
//! - **Performance metrics**: Encode/decode times, token counts, success rates
//!
//! ## Error Handling
//!
//! ```
//! use tct::{TctCompiler, TctError};
//! use serde_json::json;
//!
//! let invalid_schema = json!({"type": "invalid"});
//! match TctCompiler::new(invalid_schema, "Test")?.compile() {
//!     Ok(grammar) => println!("Success: {}", grammar.generate_code()?),
//!     Err(TctError::SchemaValidation { message, path, suggestion, .. }) => {
//!         eprintln!("Schema error at {}: {}", path, message);
//!         eprintln!("💡 {}", suggestion);
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Experimental Frontends
//!
//! The crate also contains experimental non-JSON frontends: a SQL
//! schema-linking frontend, a syntax/grammar layer (`GrammarIR`,
//! `StructuralModel`, token layout, raw syntax state machine, generated codec)
//! with Go and C# syntax targets, tree-sitter subset codecs, and an exact-mask
//! contract. They are compiled only with the `experimental` Cargo feature,
//! carry no stability guarantees, and may change or be removed in any release.
//!
//! ## Architecture
//!
//! TCT follows a clean separation of concerns:
//!
//! - **Schema Processing**: Parse and validate JSON schemas
//! - **Code Generation**: Transform schemas to optimized Rust types
//! - **Optimization Passes**: Apply compression strategies based on level
//! - **Error Handling**: Provide actionable feedback with recovery suggestions
//!
//! For more details, see the [developer guide] and [architecture documentation].
//!
//! [developer guide]: https://github.com/jonas-schmitt/tct/blob/main/DEVELOPMENT.md
//! [architecture documentation]: https://github.com/jonas-schmitt/tct/blob/main/ARCHITECTURE.md

// Declare the modules that make up this library.

// ============================================================================
// PUBLIC API - These modules are part of the stable public interface
// ============================================================================

pub mod base256;
pub mod bpe;
pub mod compiler;
pub mod decoder;
pub mod error;
pub mod grammar;
pub mod logger;
pub mod metrics;
pub mod optimization_pass;
/// Performance timing utilities for profiling compilation phases
pub mod performance_timer;
pub mod profiling_config;
pub mod provenance;
pub mod sequence_bundle;
pub mod tokenizer;
pub mod warnings;
pub mod wheel_builder;

// ============================================================================
// INTERNAL MODULES - Not part of public API, exposed only for testing
// ============================================================================

/// Compiler internals used by generated-code errors, diagnostics, and advanced
/// integration tests.
///
/// This module is public because a few exposed types, such as
/// [`StringEncoding`], currently live under `generator`. Treat the module as an
/// unstable implementation surface: prefer [`TctCompiler`] and the top-level
/// re-exports for production use, and do not rely on undocumented generator
/// internals staying source-compatible.
pub mod generator;

// Internal implementation modules (private to library, but utils exposed for binary)
pub mod utils;

// Schema utilities - exposed for benchmarking and testing
pub mod schema_utils;

// Internal implementation modules - NOT PART OF PUBLIC API
// These are exposed only for comprehensive testing and are not documented
// They may change without notice - use TctCompiler API for all production code
#[doc(hidden)]
pub mod bounded_integer;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod csharp_syntax_metadata;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod exact_mask_contract;
#[doc(hidden)]
pub mod generated_crate;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod go_ast_schema_inventory;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod go_syntax_metadata;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod go_toolchain;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod grammar_ir;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod grammar_ir_structural_model;
#[doc(hidden)]
pub mod pattern_analyzer;
#[doc(hidden)]
pub mod schema_analyzer;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod shared_alphabet_layout;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod sql_schema_linking;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_artifact_envelope;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_audit_coverage_routing;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_backend_boundary;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_consumed_token_accounting_routing;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_csharp_grammar_coverage;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_csharp_raw_tree_renderer;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_csharp_real_source_generated_codec_completion_handoff;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_csharp_real_source_generated_codec_corpus;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_csharp_real_source_generated_codec_feasibility;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_final_promotion_decision;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_generated_codec;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_generated_codec_state_api_handoff;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_generated_codec_state_api_integration;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_generated_codec_state_api_route;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_inventory_synthesis;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_random_admissible_walk;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_raw_token_stream;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_raw_tree_renderer;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_real_source_generated_codec_completion_handoff;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_real_source_generated_codec_corpus;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_real_source_program_validity_corpus;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_scratch_seed;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_source_instance_projection;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_source_record_generator_core;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_source_record_raw_token;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_tokenizer_generator;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_go_unified_artifact;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_input_diagnostics;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_json_backend_parity;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_production_route;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_reproducibility_replay;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_reproducibility_routing;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_roadmap_successors;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_route_manifest;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_scratch_generation;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_source_corpus_evidence;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_source_program_external_validation;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_source_program_external_validation_preservation;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_source_program_handoff;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_stable_metadata_promotion;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_stable_promotion_review;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_state_machine;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_state_machine_replay;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_strict_recovery_decode_routing;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_target_agnostic_routing_boundary;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_tct_compiler_routing;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_token_layout;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_token_layout_diff;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_token_vocabulary_bpe_routing;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_validation_hooks;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_validation_matrix;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod syntax_validation_matrix_preservation;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod toy_grammar_frontend;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod tree_sitter_c_subset_codec;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod tree_sitter_json_subset_codec;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod tree_sitter_metadata;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod tree_sitter_subset_framework;
#[cfg(feature = "experimental")]
#[doc(hidden)]
pub mod tree_sitter_terminal_classes;
#[doc(hidden)]
pub mod vocabulary_allocator;
#[doc(hidden)]
pub mod vocabulary_encoder;

// Vocabulary analysis - exposed for CLI and advanced usage
pub mod vocabulary_analyzer;

// Instance encoding infrastructure - exposed for CLI tools (BPE training, usage analysis)
pub mod instance_encoder;

/// Experimental schema-evolution reporting support for the `tct-token-diff`
/// binary.
///
/// This module remains public so repository binaries can consume the library
/// crate the same way external callers do, but it is hidden from generated docs
/// and is not a stable public API. Prefer the `tct-token-diff` CLI unless you
/// are working on TCT internals.
#[doc(hidden)]
pub mod schema_evolution;

// Test utilities - exposed for testing only, not documented
#[doc(hidden)]
pub mod test_utils;

// Re-export the primary public items for consumers of this library.
pub use compiler::TctCompiler;
// GrammarWithStats removed - Grammar now contains optional vocabulary_stats field
pub use generator::encoding_decision::StringEncoding;
pub use grammar::{Definitions, Grammar};
pub use metrics::{ComplexityMetrics, PerformanceMetrics, TimingMetrics, TokenMetrics};
pub use optimization_pass::OptimizationLevel;
pub use profiling_config::{ProfilingConfig, ProfilingLevel};
pub use tokenizer::{DecodeError, EOFContext, JsonTokenEncoder, optional_field::OptionalField};
pub use warnings::{SchemaWarning, SchemaWarnings, WarningSeverity, WarningType};

// Re-export error handling
pub use error::{TctError, TctResult};

// Python bindings (only when python feature is enabled)
#[cfg(feature = "python")]
mod python;
