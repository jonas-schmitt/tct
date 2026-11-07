//! TctCompiler Builder API
//!
//! This module provides the new builder pattern API for TCT compilation.
//! The TctCompiler struct offers a fluent, discoverable interface for
//! configuring and executing schema compilation with optimization.

// Sub-modules for organized functionality
pub mod build_helpers;
mod grammar_conversion;
pub mod schema_processors;
pub mod schema_validation;
mod schema_warnings;

use crate::error::{TctError, TctResult};
use crate::generator::encoding_decision::EncodingStrategy;
use crate::generator::generation_output::CodeGeneration;
use crate::generator::schema_discovery::SchemaDiscovery;
use crate::generator::type_analysis::TypeAnalysis;
use crate::grammar::Grammar;
use crate::optimization_pass::OptimizationLevel;
use crate::profiling_config::ProfilingConfig;
use serde_json::Value;

// Import utilities from sub-modules
use build_helpers::{map_compilation_error, validate_configuration};
use schema_validation::{
    count_constrained_values, count_domain_types, count_optimizable_fields,
    schema_has_optimizable_content, validate_schema,
};

/// Builder for configuring and executing TCT schema compilation
///
/// The TctCompiler provides a fluent interface for configuring compilation
/// options and executing schema-to-tokenizer generation with optimization.
/// It converts JSON schemas into optimized Rust types with tokenization support.
///
/// ## Features
///
/// - **Optimization Levels**: O0 (development), O2 (production balanced), O3 (experimental)
/// - **Base Vocabulary Controls**: Tune the multi-token encoding base without
///   changing schema semantics
/// - **Custom Optimization Passes**: Fine-grained control over specific optimizations
/// - **Statistics Generation**: Optional vocabulary optimization statistics
/// - **Error Recovery**: Comprehensive error messages with suggestions
///
/// ## Optimization Levels
///
/// - **O0**: No optimization, preserves schema structure exactly
/// - **O2**: Balanced optimization with enum indices and bounded integers
/// - **O3**: Full maintained optimization profile, including vocabulary analysis
///   and bit-packing where applicable (default)
///
/// ## Builder Methods
///
/// - [`with_optimization_level()`](TctCompiler::with_optimization_level): Set optimization level
/// - [`with_max_base_vocab()`](TctCompiler::with_max_base_vocab): Tune the multi-token encoding base
/// - [`with_stats()`](TctCompiler::with_stats): Include optimization statistics in output
/// - [`with_no_optimization()`](TctCompiler::with_no_optimization): Disable all optimizations
/// - [`compile()`](TctCompiler::compile): Execute compilation and return Grammar
///
/// # Examples
///
/// Basic usage:
/// ```
/// use tct::{TctCompiler, OptimizationLevel};
/// use serde_json::json;
///
/// let schema = json!({"type": "string", "enum": ["a", "b", "c"]});
/// let grammar = TctCompiler::new(schema, "MyEnum")?
///     .with_optimization_level(OptimizationLevel::O2)
///     .compile()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// With statistics:
/// ```
/// use tct::TctCompiler;
/// use serde_json::json;
///
/// let schema = json!({"type": "object", "properties": {"status": {"enum": ["active", "inactive"]}}});
/// let grammar = TctCompiler::new(schema, "Status")?
///     .with_stats()
///     .compile()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Error handling:
/// ```
/// use tct::{TctCompiler, TctError};
/// use serde_json::json;
///
/// let invalid_schema = json!({"type": "invalid"});
/// match TctCompiler::new(invalid_schema, "Test")?.compile() {
///     Ok(grammar) => {
///         // Use grammar.generate_code()?
///     }
///     Err(TctError::SchemaValidation { message, path, suggestion, .. }) => {
///         eprintln!("Schema error at {}: {}", path, message);
///         eprintln!("💡 {}", suggestion);
///     }
///     Err(e) => eprintln!("Other error: {}", e),
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct TctCompiler {
    schema: Value,
    type_name: String,
    optimization_level: OptimizationLevel,
    bpe_merges_path: Option<std::path::PathBuf>,
    include_stats: bool,
    custom_passes: Vec<String>, // Store pass names for backward compatibility
    profiling_config: ProfilingConfig,
    compilation_constraints: crate::generator::encoding_decision::CompilationConstraints,
    // Individual pass controls (None = use level default, Some(true) = force enable)
    enum_optimization: Option<bool>,
    bounded_int_optimization: Option<bool>,
    pattern_optimization: Option<bool>,
    bit_packing_optimization: Option<bool>,
    max_base_vocab: Option<u32>, // Override for maximum base vocabulary size (P1-2)
    /// Enable streaming-friendly encoding for prefix decoding
    ///
    /// When enabled:
    /// - All types derive Default
    /// - decode_prefix() is generated for truncation-tolerant decoding
    ///
    /// Token format is identical to non-streaming mode (full compatibility).
    streaming_encode: bool,
    /// Digit-aligned decimal text encoding for unbounded i64/u64/f64 numbers
    /// (opt-in; token streams incompatible with the default bit-split encoding)
    decimal_number_encoding: bool,
    /// Maximum collection length for decoded Vec/HashMap (None = use default 2048)
    max_collection_length: Option<usize>,
    /// Maximum recursive schema traversal depth during discovery
    max_schema_depth: Option<usize>,
}

/// Complete result of compilation with the pass-owned artifacts preserved.
///
/// This is crate-internal so reporting tools can inspect structured compiler
/// metadata without re-walking schemas or parsing generated Rust source.
#[doc(hidden)]
pub(crate) struct CompiledSchema {
    pub grammar: Grammar,
    pub artifacts: CompilerPassArtifacts,
}

/// Outputs produced by the four compiler passes for one schema compilation.
#[doc(hidden)]
#[allow(dead_code)]
pub(crate) struct CompilerPassArtifacts {
    pub schema_discovery: SchemaDiscovery,
    pub type_analysis: TypeAnalysis,
    pub encoding_strategy: Option<EncodingStrategy>,
    pub code_generation: CodeGeneration,
}

impl TctCompiler {
    /// Create a new TctCompiler with the given schema and type name
    ///
    /// # Arguments
    ///
    /// * `schema` - The JSON schema to compile
    /// * `type_name` - The name for the root type (used as struct name)
    ///
    /// # Returns
    ///
    /// A configured TctCompiler with default settings, or an error if the
    /// schema is invalid or the type name is empty.
    ///
    /// # Errors
    ///
    /// * `TctError::SchemaValidation` - If the schema is not valid JSON Schema
    /// * `TctError::InvalidConfiguration` - If the type name is empty, or would
    ///   shadow a Rust prelude item (`String`, `Result`, `Default`, ...) in the
    ///   generated code
    pub fn new<S: Into<String>>(schema: Value, type_name: S) -> TctResult<Self> {
        let type_name = type_name.into();

        // Validate type name
        if type_name.is_empty() {
            return Err(TctError::InvalidConfiguration {
                field: "type_name".to_string(),
                reason: "Type name cannot be empty".to_string(),
                suggestion: "Provide a non-empty string for the type name".to_string(),
            });
        }

        // The root type is generated under this name, so it must not shadow a Rust
        // prelude item that the generated code uses unqualified.
        let root_type_name = crate::utils::to_upper_camel_case(&type_name);
        if crate::utils::is_reserved_type_name(&root_type_name) {
            return Err(TctError::InvalidConfiguration {
                field: "type_name".to_string(),
                reason: format!(
                    "`{root_type_name}` is a Rust prelude name that generated code uses \
                     unqualified, so a root type with that name would shadow it"
                ),
                suggestion: format!(
                    "Choose another root type name, for example `{root_type_name}Type` \
                     (`--name` on the command line)"
                ),
            });
        }

        // Basic schema validation
        validate_schema(&schema)?;

        Ok(TctCompiler {
            schema,
            type_name,
            optimization_level: OptimizationLevel::O3, // Default to O3
            bpe_merges_path: None,
            include_stats: false,
            custom_passes: Vec::new(),
            profiling_config: ProfilingConfig::default(),
            compilation_constraints:
                crate::generator::encoding_decision::CompilationConstraints::default(),
            enum_optimization: None,
            bounded_int_optimization: None,
            pattern_optimization: None,
            bit_packing_optimization: None,
            max_base_vocab: None,
            streaming_encode: false,
            decimal_number_encoding: false,
            max_collection_length: None,
            max_schema_depth: None,
        })
    }

    /// Configure the optimization level for compilation
    ///
    /// Controls which optimization passes are applied during compilation.
    /// Higher levels enable more aggressive optimizations for better LLM performance.
    ///
    /// # Arguments
    /// * `level` - The optimization level (O0 for no optimization, O2 for full optimization)
    pub fn with_optimization_level(mut self, level: OptimizationLevel) -> Self {
        self.optimization_level = level;
        self
    }

    // Note: with_max_string_length() removed - end-marker encoding eliminates length constraints
    // Strings now use END_STRING_MARKER to signal end, eliminating need for max length
    /// Cap the raw structural vocabulary using multi-token encodings for wider
    /// finite decisions. UTF-8 requires at least 257 raw symbols.
    pub fn with_max_vocabulary(mut self, max_vocab: usize) -> TctResult<Self> {
        if max_vocab < 257 {
            return Err(TctError::InvalidConfiguration {
                field: "max_vocabulary_size".to_string(),
                reason: format!("max vocabulary must be at least 257, got {max_vocab}"),
                suggestion: "Use 257 or larger so UTF-8 bytes and the value-end marker fit"
                    .to_string(),
            });
        }
        self.compilation_constraints.max_vocab = Some(max_vocab);
        Ok(self)
    }

    /// Enable statistics collection during compilation
    ///
    /// When enabled, the generated Grammar will include detailed statistics
    /// about optimization effectiveness and vocabulary usage.
    pub fn with_stats(mut self) -> Self {
        self.include_stats = true;
        self
    }

    /// Set string encoding mode (UTF-8 or ASCII restricted-input mode)
    ///
    /// Controls how strings are encoded in the token stream:
    /// - **UTF-8 mode** (default): bytes 0-255, END_MARKER = 256 (257 base tokens)
    ///   - Full Unicode support via UTF-8 byte encoding
    ///   - Handles multi-byte UTF-8 sequences
    ///   - Preserves ordinary JSON strings
    /// - **ASCII mode**: bytes 0-127, END_MARKER = 128 (129 base tokens)
    ///   - Compact vocabulary for efficient LLM training
    ///   - UTF-8 safe (all bytes are valid single-byte UTF-8)
    ///   - Rejects non-ASCII input during generated encoding
    ///
    /// # Arguments
    /// * `encoding` - `StringEncoding::Ascii` or `StringEncoding::Utf8`
    ///
    /// # Example
    /// ```no_run
    /// use tct::TctCompiler;
    /// use tct::generator::encoding_decision::StringEncoding;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
    ///
    /// // ASCII restricted-input mode
    /// let grammar_ascii = TctCompiler::new(schema.clone(), "MyType")?
    ///     .with_string_encoding(StringEncoding::Ascii)
    ///     .compile()?;
    ///
    /// // UTF-8 mode for Unicode support
    /// let grammar_utf8 = TctCompiler::new(schema, "MyType")?
    ///     .with_string_encoding(StringEncoding::Utf8)
    ///     .compile()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_string_encoding(
        mut self,
        encoding: crate::generator::encoding_decision::StringEncoding,
    ) -> Self {
        self.compilation_constraints.string_encoding = encoding;
        self
    }

    /// Specify BPE merge file for token compression
    ///
    /// When provided, the generated code will include BPE encoding/decoding
    /// functions that compress base tokens using the learned merge operations.
    ///
    /// # Arguments
    /// * `path` - Path to BPE merge JSON file
    ///
    /// # Example
    /// ```no_run
    /// use tct::TctCompiler;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "object", "properties": {}});
    /// let grammar = TctCompiler::new(schema, "Workflow")?
    ///     .with_bpe_merges("schemas/bpe/github-workflow-tct-bpe-500.json")?
    ///     .compile()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_bpe_merges<P: Into<std::path::PathBuf>>(mut self, path: P) -> TctResult<Self> {
        let path = path.into();

        // Verify file exists
        if !path.exists() {
            return Err(TctError::InvalidConfiguration {
                field: "bpe_merges".to_string(),
                reason: format!("BPE merge file not found: {:?}", path),
                suggestion: "Ensure the BPE merge file exists at the specified path".to_string(),
            });
        }

        self.bpe_merges_path = Some(path);
        Ok(self)
    }

    /// Set maximum base vocabulary size for multi-token encoding
    ///
    /// Controls the encoding base used for primitives (i32, i64, f64), collections
    /// (Vec, HashMap), and bounded integers. The actual base will be the largest
    /// power-of-2 that is less than or equal to max_base_vocab.
    ///
    /// # Arguments
    /// * `max_base_vocab` - Maximum vocabulary size (must be > 256, recommended: power-of-2)
    ///
    /// # Example
    /// ```
    /// use tct::TctCompiler;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "integer"});
    /// let grammar = TctCompiler::new(schema, "MyInt")?
    ///     .with_max_base_vocab(512)?  // Use base-512 encoding
    ///     .compile()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Trade-offs
    /// - Smaller base (e.g., 256): More tokens per value, larger token streams
    /// - Larger base (e.g., 2048): Fewer tokens per value, larger vocabulary
    ///
    /// # Errors
    /// * `TctError::InvalidConfiguration` - If max_base_vocab <= 256
    pub fn with_max_base_vocab(mut self, max_base_vocab: u32) -> TctResult<Self> {
        // Validate minimum (must be > 256 to allow room for UTF-8 strings)
        if max_base_vocab <= 256 {
            return Err(TctError::InvalidConfiguration {
                field: "max_base_vocab".to_string(),
                reason: format!("max_base_vocab must be > 256, got {}", max_base_vocab),
                suggestion: "Use a value > 256 (recommended: 257-65536, power-of-2 preferred)"
                    .to_string(),
            });
        }

        self.max_base_vocab = Some(max_base_vocab);
        Ok(self)
    }

    /// Add a legacy named built-in optimization pass override.
    ///
    /// When named built-in passes are specified, they override the optimization
    /// level. This allows fine-grained control over the maintained built-in
    /// optimizations without exposing the legacy `OptimizationPass`
    /// `TokenStream` hook as an extension point.
    ///
    /// # Arguments
    /// * `pass_name` - Name of the built-in optimization pass to enable
    ///
    /// # Examples
    /// ```
    /// use tct::TctCompiler;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "string", "enum": ["a", "b"]});
    /// let compiler = TctCompiler::new(schema, "MyEnum")?
    ///     .with_optimization_pass("enum_optimization"); // &str works too
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_optimization_pass<S: Into<String>>(mut self, pass_name: S) -> Self {
        self.custom_passes.push(pass_name.into());
        self
    }

    /// Disable all optimizations (equivalent to O0)
    pub fn with_no_optimization(mut self) -> Self {
        self.optimization_level = OptimizationLevel::O0;
        self
    }

    /// Enable all optimizations (equivalent to O2)
    pub fn with_all_optimizations(mut self) -> Self {
        self.optimization_level = OptimizationLevel::O2;
        self
    }

    /// Configure profiling code generation
    ///
    /// When enabled, generates feature-gated profiling infrastructure alongside
    /// the normal tokenizer code. This provides zero-overhead timing measurements
    /// and optional Criterion benchmark generation.
    ///
    /// # Arguments
    /// * `config` - Profiling configuration specifying level, feature gate, and options
    ///
    /// # Examples
    /// ```
    /// use tct::{TctCompiler, ProfilingConfig};
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "string"});
    ///
    /// // Basic profiling with timing wrappers
    /// let compiler = TctCompiler::new(schema.clone(), "MyType")?
    ///     .with_profiling(ProfilingConfig::basic());
    ///
    /// // Profiling with Criterion benchmark generation
    /// let profiling = ProfilingConfig::benchmarking()
    ///     .with_criterion_sample("example", json!("example"));
    /// let compiler = TctCompiler::new(schema, "MyType")?
    ///     .with_profiling(profiling);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_profiling(mut self, config: ProfilingConfig) -> Self {
        self.profiling_config = config;
        self
    }

    /// Enable enum index optimization
    ///
    /// When enabled, enum values are encoded as indices (0, 1, 2, ...) instead of strings.
    /// This can be used to override the optimization level's default behavior.
    ///
    /// # Example
    /// ```
    /// use tct::TctCompiler;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "string", "enum": ["a", "b", "c"]});
    /// let compiler = TctCompiler::new(schema, "MyEnum")?
    ///     .with_optimization_level(tct::OptimizationLevel::O0)
    ///     .with_enum_optimization(); // Enable only enum optimization
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_enum_optimization(mut self) -> Self {
        self.enum_optimization = Some(true);
        self
    }

    /// Enable bounded integer optimization
    ///
    /// When enabled, integers with min/max constraints use optimized encoding.
    /// This can be used to override the optimization level's default behavior.
    ///
    /// # Example
    /// ```
    /// use tct::TctCompiler;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "integer", "minimum": 0, "maximum": 100});
    /// let compiler = TctCompiler::new(schema, "BoundedInt")?
    ///     .with_optimization_level(tct::OptimizationLevel::O0)
    ///     .with_bounded_int_optimization(); // Enable only bounded int optimization
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_bounded_int_optimization(mut self) -> Self {
        self.bounded_int_optimization = Some(true);
        self
    }

    /// Request pattern optimization.
    ///
    /// This legacy builder is retained so older callers receive a structured
    /// compiler error instead of silently getting a no-op. Direct finite-regex
    /// pattern optimization is not currently implemented as an optimization
    /// pass; `patternProperties` validation remains schema-driven and is not
    /// controlled by this flag.
    ///
    /// # Example
    /// ```
    /// use tct::TctCompiler;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "string", "pattern": "^[a-z]{3}$"});
    /// let result = TctCompiler::new(schema, "Pattern")?
    ///     .with_pattern_optimization()
    ///     .compile();
    /// assert!(result.is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_pattern_optimization(mut self) -> Self {
        self.pattern_optimization = Some(true);
        self
    }

    /// Enable bit-packing optimization
    ///
    /// When enabled, optional field presence flags are packed into u32 tokens.
    /// This experimental optimization is enabled by default at O3, Os, and OVocab.
    ///
    /// # Example
    /// ```
    /// use tct::TctCompiler;
    /// use serde_json::json;
    ///
    /// let schema = json!({
    ///     "type": "object",
    ///     "properties": {
    ///         "a": {"type": "string"},
    ///         "b": {"type": "string"},
    ///         "c": {"type": "string"},
    ///         "d": {"type": "string"}
    ///     }
    /// });
    /// let compiler = TctCompiler::new(schema, "MyStruct")?
    ///     .with_optimization_level(tct::OptimizationLevel::O2)
    ///     .with_bit_packing_optimization(); // Add experimental bit-packing to O2
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_bit_packing_optimization(mut self) -> Self {
        self.bit_packing_optimization = Some(true);
        self
    }

    /// Enable streaming-friendly encoding for prefix decoding
    ///
    /// When enabled, the generated code includes:
    /// - All types derive Default for partial construction
    /// - `decode_prefix()` is generated for truncation-tolerant decoding
    ///
    /// Token format is IDENTICAL to non-streaming mode (full compatibility).
    ///
    /// This enables LLM autocompletion workflows where:
    /// 1. Partial/truncated JSON is encoded
    /// 2. LLM generates continuation tokens
    /// 3. `decode_prefix()` decodes whatever is available
    ///
    /// # Example
    /// ```no_run
    /// use serde_json::json;
    /// use tct::TctCompiler;
    ///
    /// let schema = json!({
    ///     "type": "object",
    ///     "properties": {
    ///         "name": {"type": "string"},
    ///         "age": {"type": "integer"}
    ///     },
    ///     "required": ["name"]
    /// });
    /// let compiler = TctCompiler::new(schema, "Person")?
    ///     .with_streaming_encode();  // Enable streaming format
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_streaming_encode(mut self) -> Self {
        self.streaming_encode = true;
        self
    }

    /// Enable digit-aligned decimal text encoding for unbounded i64/u64/f64.
    ///
    /// Numbers are emitted as their shortest round-trip decimal text (one
    /// byte token per ASCII character, END_STRING_MARKER terminated) instead
    /// of fixed-width bit splits. Opt-in because the token stream layout is
    /// incompatible with the default encoding. Bounded integers keep their
    /// dedicated compact path.
    pub fn with_decimal_number_encoding(mut self) -> Self {
        self.decimal_number_encoding = true;
        self
    }

    /// Set maximum collection length for decoded Vec/HashMap
    ///
    /// Limits the maximum number of elements that can be decoded in a single
    /// collection. Prevents memory exhaustion from invalid token sequences
    /// (model-generated garbage can decode to billion-element lengths).
    ///
    /// Default: 2048 (matches model context size - impossible to legitimately
    /// exceed since each Vec element needs ≥1 token to encode)
    ///
    /// # Arguments
    /// * `max_len` - Maximum elements allowed in decoded collections
    ///
    /// # Example
    /// ```no_run
    /// use tct::TctCompiler;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "array", "items": {"type": "string"}});
    /// let compiler = TctCompiler::new(schema, "MyArray")?
    ///     .with_max_collection_length(4096);  // Allow larger arrays
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_max_collection_length(mut self, max_len: usize) -> Self {
        self.max_collection_length = Some(max_len);
        self
    }

    /// Set maximum recursive schema traversal depth during discovery.
    ///
    /// This protects compilation from pathological schemas whose nested
    /// properties, array items, compositions, or definitions would otherwise
    /// recurse until stack exhaustion. The default is
    /// [`crate::schema_utils::DEFAULT_MAX_SCHEMA_TRAVERSAL_DEPTH`].
    pub fn with_max_schema_depth(mut self, max_depth: usize) -> Self {
        self.max_schema_depth = Some(max_depth);
        self
    }

    /// Compile the schema into a Grammar
    ///
    /// Executes the full compilation pipeline: schema analysis, optimization,
    /// and code generation to produce a Grammar suitable for LLM inference.
    ///
    /// # Returns
    /// * `Grammar` - The compiled grammar with tokenizer and decoder functions
    ///
    /// # Errors
    /// * `TctError::SchemaProcessing` - If schema processing fails
    /// * `TctError::OptimizationFailure` - If optimization passes fail
    /// * `TctError::CodeGeneration` - If code generation fails
    pub fn compile(&self) -> TctResult<Grammar> {
        Ok(self.compile_with_artifacts()?.grammar)
    }

    /// Compile the schema while preserving each pass's structured artifacts.
    ///
    /// This is for crate-internal reporting and compatibility analysis. It
    /// deliberately uses the same pass orchestration as `compile()` so callers
    /// cannot observe a different compiler path.
    #[doc(hidden)]
    pub(crate) fn compile_with_artifacts(&self) -> TctResult<CompiledSchema> {
        use crate::generator::passes::{
            AnalysisPass,
            DiscoveryPass,
            GenerationPass,
            PassOrchestrator, // SemanticAnalysisPass deprecated
        };
        use std::time::Instant;

        // Initialize logger to check environment variables
        crate::logger::Logger::init();

        // Validate configuration before compilation (no legacy max_vocabulary_size)
        validate_configuration(None)?;

        let compilation_start = Instant::now();

        let optimization_config = self.optimization_config()?;
        let vocabulary_stats_enabled = optimization_config.level != OptimizationLevel::O0;

        // Create new multi-pass compiler system
        let mut orchestrator = PassOrchestrator::new();

        // Add the four core passes in dependency order
        // 1. Discovery: Walk schema, identify types
        orchestrator.add_pass(DiscoveryPass::with_root_name(self.type_name.clone()));

        // 2. Analysis: Build dependency graph, determine generation order
        orchestrator.add_pass(AnalysisPass::new());

        // 3. Optimization: Make encoding decisions based on constraints
        orchestrator.add_pass(crate::generator::passes::OptimizationPassWrapper::new(
            optimization_config.clone(),
        ));

        // 4. Generation: Emit Rust code according to optimization decisions
        orchestrator.add_pass(GenerationPass::new());

        // Run all passes to get final context with CodeGeneration
        let context = orchestrator
            .run_all_with_config(&self.schema, optimization_config)
            .map_err(|e| map_compilation_error(e, &self.type_name))?;

        let pass_timings = context.pass_timings.clone();
        let schema_discovery =
            context
                .schema_discovery
                .clone()
                .ok_or_else(|| TctError::CodeGeneration {
                    message: "DiscoveryPass did not produce SchemaDiscovery results".to_string(),
                    schema_path: "#/".to_string(),
                    suggestion: "Ensure the discovery pass ran before later passes".to_string(),
                    phase: crate::error::GenerationPhase::SchemaAnalysis,
                })?;
        let type_analysis =
            context
                .analysis_results
                .clone()
                .ok_or_else(|| TctError::CodeGeneration {
                    message: "AnalysisPass did not produce TypeAnalysis results".to_string(),
                    schema_path: "#/".to_string(),
                    suggestion: "Ensure the analysis pass ran after discovery".to_string(),
                    phase: crate::error::GenerationPhase::SchemaAnalysis,
                })?;

        // Get the code generation results
        let code_generation =
            context
                .code_generation
                .clone()
                .ok_or_else(|| TctError::CodeGeneration {
                    message: "GenerationPass did not produce CodeGeneration results".to_string(),
                    schema_path: "#/".to_string(),
                    suggestion: "Ensure all passes ran correctly".to_string(),
                    phase: crate::error::GenerationPhase::TypeGeneration,
                })?;

        // Get encoding strategy for statistics
        let encoding_strategy = context.encoding_strategy;

        // Check for discriminator info on root enum type (enables partial encoding)
        let has_enum_discriminator = context
            .schema_discovery
            .as_ref()
            .and_then(|sd| {
                // Look up root type's discriminator info
                use crate::utils::to_upper_camel_case;
                let root_type_name = to_upper_camel_case(&self.type_name);
                sd.discovered_types
                    .get(&root_type_name)
                    .and_then(|td| td.discriminator_info.as_ref())
            })
            .is_some();

        // Convert CodeGeneration to Grammar
        let grammar = self.convert_to_grammar(
            code_generation.clone(),
            encoding_strategy.clone(),
            compilation_start,
            pass_timings,
            has_enum_discriminator,
            vocabulary_stats_enabled,
        )?;

        Ok(CompiledSchema {
            grammar,
            artifacts: CompilerPassArtifacts {
                schema_discovery,
                type_analysis,
                encoding_strategy,
                code_generation,
            },
        })
    }

    fn optimization_config(&self) -> TctResult<crate::optimization_pass::OptimizationConfig> {
        use crate::optimization_pass::OptimizationConfig;
        use crate::optimization_pass::passes::{
            BoundedIntegerOptimizationPass, EnumIndexOptimizationPass,
        };

        let mut optimization_config = if self.custom_passes.is_empty() {
            match self.optimization_level {
                OptimizationLevel::O0 => OptimizationConfig::o0(),
                OptimizationLevel::O2 => OptimizationConfig::o2(),
                OptimizationLevel::O3 => OptimizationConfig::with_all_passes(),
                OptimizationLevel::Os => OptimizationConfig::os(),
                OptimizationLevel::OVocab => OptimizationConfig::ovocab(),
            }
        } else {
            OptimizationConfig::from_pass_names(&self.custom_passes).map_err(|reason| {
                TctError::InvalidConfiguration {
                    field: "optimization_pass".to_string(),
                    reason,
                    suggestion: "Use one of: enum_optimization, integer_optimization, vocabulary_optimization. pattern_optimization is reserved until direct finite-regex encoding is implemented.".to_string(),
                }
            })?
        };

        // Thread configurable base-vocabulary encoding through optimization and generation.
        optimization_config.max_base_vocab = self.max_base_vocab;
        optimization_config.max_vocab = self.compilation_constraints.max_vocab;

        // Pass compilation constraints (includes max_string_length)
        optimization_config.compilation_constraints = Some(self.compilation_constraints.clone());

        // Apply individual pass overrides if specified.
        if let Some(true) = self.enum_optimization {
            optimization_config.ensure_pass(EnumIndexOptimizationPass::new());
        }

        if let Some(true) = self.bounded_int_optimization {
            optimization_config.ensure_pass(BoundedIntegerOptimizationPass::new());
        }

        if let Some(true) = self.pattern_optimization {
            return Err(TctError::InvalidConfiguration {
                field: "optimization_pass".to_string(),
                reason: "pattern_optimization is reserved but not implemented as a direct optimization pass".to_string(),
                suggestion: "Do not enable pattern_optimization. patternProperties validation is always schema-driven; add a real finite-regex encoding pass before exposing this as an optimization.".to_string(),
            });
        }

        if let Some(true) = self.bit_packing_optimization {
            optimization_config.bit_packing_enabled = true;
        }

        if self.streaming_encode {
            optimization_config.streaming_encode = true;
        }

        if self.decimal_number_encoding {
            optimization_config.decimal_number_encoding = true;
        }

        if let Some(max_len) = self.max_collection_length {
            optimization_config.max_collection_length = Some(max_len);
        }

        if let Some(max_depth) = self.max_schema_depth {
            optimization_config.max_schema_depth = Some(max_depth);
        }

        Ok(optimization_config)
    }

    // Accessor methods

    /// Get the configured type name
    #[doc(hidden)]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Get the configured optimization level
    #[doc(hidden)]
    pub fn optimization_level(&self) -> OptimizationLevel {
        self.optimization_level
    }

    /// Check if statistics collection is enabled
    #[doc(hidden)]
    pub fn include_stats(&self) -> bool {
        self.include_stats
    }

    // Analysis utilities using extracted modules

    /// Check if schema has content that can be optimized
    pub fn has_optimizable_content(&self) -> bool {
        schema_has_optimizable_content(&self.schema)
    }

    /// Count optimizable fields in the schema
    pub fn optimizable_fields_count(&self) -> usize {
        count_optimizable_fields(&self.schema)
    }

    /// Count total constrained values across all optimizable fields
    pub fn constrained_values_count(&self) -> usize {
        count_constrained_values(&self.schema)
    }

    /// Get domain type counts (enums, integers)
    pub fn domain_type_counts(&self) -> (usize, usize) {
        count_domain_types(&self.schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_generated_file(code: &str) -> syn::File {
        syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
    }

    fn generated_struct<'a>(file: &'a syn::File, type_name: &str) -> &'a syn::ItemStruct {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(item_struct) if item_struct.ident == type_name => {
                    Some(item_struct)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected generated struct {type_name}"))
    }

    fn generated_enum<'a>(file: &'a syn::File, type_name: &str) -> &'a syn::ItemEnum {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Enum(item_enum) if item_enum.ident == type_name => Some(item_enum),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected generated enum {type_name}"))
    }

    fn assert_generated_struct(code: &str, type_name: &str) {
        let file = parse_generated_file(code);
        generated_struct(&file, type_name);
    }

    fn assert_generated_enum(code: &str, type_name: &str) {
        let file = parse_generated_file(code);
        generated_enum(&file, type_name);
    }

    fn enum_has_variant(item_enum: &syn::ItemEnum, variant_name: &str) -> bool {
        item_enum
            .variants
            .iter()
            .any(|variant| variant.ident == variant_name)
    }

    fn struct_has_field(item_struct: &syn::ItemStruct, field_name: &str) -> bool {
        match &item_struct.fields {
            syn::Fields::Named(fields) => fields.named.iter().any(|field| {
                field
                    .ident
                    .as_ref()
                    .is_some_and(|ident| ident == field_name)
            }),
            _ => false,
        }
    }

    #[test]
    fn test_builder_construction() {
        let schema = json!({"type": "string"});
        let compiler = TctCompiler::new(schema, "Test").unwrap();

        assert_eq!(compiler.type_name(), "Test");
        assert_eq!(compiler.optimization_level(), OptimizationLevel::O3);
        assert!(!compiler.include_stats());
    }

    #[test]
    fn test_builder_method_chaining() {
        let schema = json!({"type": "string", "enum": ["a", "b", "c"]});
        let compiler = TctCompiler::new(schema, "TestEnum")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O0)
            .with_stats()
            .with_optimization_pass("enum_optimization".to_string());

        assert_eq!(compiler.optimization_level(), OptimizationLevel::O0);
        assert!(compiler.include_stats());
    }

    #[test]
    fn test_empty_type_name_error() {
        let schema = json!({"type": "string"});
        let result = TctCompiler::new(schema, "");

        assert!(result.is_err());
        if let Err(TctError::InvalidConfiguration { field, .. }) = result {
            assert_eq!(field, "type_name");
        }
    }

    #[test]
    fn test_root_type_name_that_shadows_a_prelude_item_is_rejected() {
        // `pub struct String` as the root type would shadow `String` for the whole
        // generated crate, so the name is refused with a way out instead.
        for name in ["String", "string", "default", "Result", "self"] {
            let result = TctCompiler::new(json!({"type": "string"}), name);

            let Err(TctError::InvalidConfiguration {
                field,
                reason,
                suggestion,
            }) = result
            else {
                panic!("root type name {name:?} should be rejected");
            };
            assert_eq!(field, "type_name");
            assert!(reason.contains("prelude"), "{reason}");
            assert!(suggestion.contains("Type"), "{suggestion}");
        }
    }

    #[test]
    fn test_root_type_names_near_prelude_names_are_accepted() {
        for name in [
            "StringType",
            "Strings",
            "MyResult",
            "Config",
            "default_config",
        ] {
            assert!(
                TctCompiler::new(json!({"type": "string"}), name).is_ok(),
                "root type name {name:?} should be accepted"
            );
        }
    }

    #[test]
    fn test_invalid_schema_error() {
        let invalid_schema = json!("not an object");
        let result = TctCompiler::new(invalid_schema, "Test");

        assert!(result.is_err());
        assert!(matches!(result, Err(TctError::SchemaValidation { .. })));
    }

    #[test]
    fn test_schema_analysis_methods() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]},
                "count": {"type": "integer", "minimum": 0, "maximum": 100}
            }
        });

        let compiler = TctCompiler::new(schema, "TestObject").unwrap();

        assert!(compiler.has_optimizable_content());
        assert_eq!(compiler.optimizable_fields_count(), 2);
        assert_eq!(compiler.constrained_values_count(), 103); // 2 enum + 101 integer values
        assert_eq!(compiler.domain_type_counts(), (1, 1)); // 1 enum, 1 integer
    }

    #[test]
    fn test_clean_api_pattern() {
        let schema = json!({"type": "string", "enum": ["hello", "world"]});
        let compiler = TctCompiler::new(schema, "TestEnum").unwrap();

        // Test the clean, simple pattern: compile then format
        let grammar = compiler.compile().unwrap();
        let code = grammar.generate_code().unwrap();

        // Root-level simple enums should generate enum, not struct
        let file = parse_generated_file(&code);
        let item_enum = generated_enum(&file, "TestEnum");
        assert!(enum_has_variant(item_enum, "Hello"));
        assert!(enum_has_variant(item_enum, "World"));
    }

    #[test]
    fn test_root_enum_duplicate_removed_by_type_metadata() {
        let schema = json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "kind": {"const": "a"},
                        "name": {"type": "string"}
                    },
                    "required": ["kind", "name"]
                },
                {
                    "type": "object",
                    "properties": {
                        "kind": {"const": "b"},
                        "count": {"type": "integer"}
                    },
                    "required": ["kind", "count"]
                }
            ]
        });

        let grammar = TctCompiler::new(schema, "RootChoice")
            .unwrap()
            .compile()
            .unwrap();

        assert!(grammar.is_enum_root);
        assert!(
            grammar
                .definitions
                .enums
                .iter()
                .all(|generated_enum| generated_enum.type_name != "RootChoice"),
            "supporting definitions must not retain the root enum"
        );

        let code = grammar.generate_code().unwrap();
        let root_enum_count = code
            .lines()
            .filter(|line| {
                line.trim_start()
                    .strip_prefix("pub enum ")
                    .and_then(|rest| rest.split(|c: char| c == '{' || c.is_whitespace()).next())
                    == Some("RootChoice")
            })
            .count();
        assert_eq!(root_enum_count, 1, "root enum should be emitted once");
    }

    #[test]
    fn test_with_stats_builder_pattern() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]},
                "priority": {"type": "integer", "minimum": 1, "maximum": 5}
            }
        });

        // Test builder pattern with stats
        let grammar_with_stats = TctCompiler::new(schema.clone(), "WithStats")
            .unwrap()
            .with_stats()
            .compile()
            .unwrap();

        let code_with_stats = grammar_with_stats.generate_code().unwrap();
        assert_generated_struct(&code_with_stats, "WithStats");
        assert!(code_with_stats.len() > 100); // Basic sanity check

        // Test same schema without stats
        let grammar_no_stats = TctCompiler::new(schema.clone(), "WithStats")
            .unwrap()
            .compile()
            .unwrap();

        let code_no_stats = grammar_no_stats.generate_code().unwrap();
        assert_generated_struct(&code_no_stats, "WithStats");

        // Both should generate core types (stats are in Grammar object, not generated code)
        // Note: vocabulary optimization is enabled by default at O3 level,
        // so both will have vocabulary stats since both use default O3 optimization
        assert!(grammar_with_stats.vocabulary_stats.is_some());
        assert!(grammar_no_stats.vocabulary_stats.is_some());

        // Test with O0 to ensure vocabulary optimization can be disabled
        let grammar_o0 = TctCompiler::new(schema.clone(), "WithStats")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O0)
            .compile()
            .unwrap();

        // O0 should not have vocabulary optimization
        assert!(grammar_o0.vocabulary_stats.is_none());
    }

    #[test]
    fn test_file_output_pattern() {
        use std::env;
        use std::fs;

        let schema = json!({"type": "string", "enum": ["test", "file"]});
        let grammar = TctCompiler::new(schema, "FileTest")
            .unwrap()
            .compile()
            .unwrap();

        let temp_dir = env::temp_dir();
        let output_path = temp_dir.join("test_tct_output.rs");

        // Ensure clean state
        let _ = fs::remove_file(&output_path);

        // Test the clean pattern: generate code, then write file manually
        let code = grammar.generate_code().unwrap();
        fs::write(&output_path, code).unwrap();

        // Verify file was created and contains expected content
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();

        // Root-level simple enums should generate enum, not struct
        let file = parse_generated_file(&content);
        let item_enum = generated_enum(&file, "FileTest");
        assert!(enum_has_variant(item_enum, "Test"));
        assert!(enum_has_variant(item_enum, "File"));

        // Clean up
        let _ = fs::remove_file(&output_path);
    }

    // ========== Additional Coverage Tests ==========

    #[test]
    fn test_with_bpe_merges_nonexistent_file() {
        let schema = json!({"type": "string"});
        let result = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_bpe_merges("/nonexistent/path/to/bpe.json");

        assert!(result.is_err());
        if let Err(TctError::InvalidConfiguration { field, reason, .. }) = result {
            assert_eq!(field, "bpe_merges");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected InvalidConfiguration error");
        }
    }

    #[test]
    fn test_with_profiling_basic() {
        let schema = json!({"type": "string"});
        let compiler = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_profiling(ProfilingConfig::basic());

        assert!(!compiler.profiling_config.is_disabled());
    }

    #[test]
    fn test_with_profiling_benchmarking() {
        let schema = json!({"type": "string"});
        let compiler = TctCompiler::new(schema, "Test").unwrap().with_profiling(
            ProfilingConfig::benchmarking().with_criterion_sample("example", json!("example")),
        );

        assert!(!compiler.profiling_config.is_disabled());
        assert_eq!(compiler.profiling_config.criterion_samples.len(), 1);
    }

    #[test]
    fn test_with_no_optimization() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let compiler = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_no_optimization();

        assert_eq!(compiler.optimization_level(), OptimizationLevel::O0);
    }

    #[test]
    fn test_with_all_optimizations() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let compiler = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_all_optimizations();

        assert_eq!(compiler.optimization_level(), OptimizationLevel::O2);
    }

    #[test]
    fn test_optimization_level_o3() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let grammar = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O3)
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        assert_generated_enum(&code, "Test");
    }

    #[test]
    fn test_optimization_level_os() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let grammar = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_optimization_level(OptimizationLevel::Os)
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        assert_generated_enum(&code, "Test");
    }

    #[test]
    fn test_optimization_level_ovocab() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let grammar = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_optimization_level(OptimizationLevel::OVocab)
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        assert_generated_enum(&code, "Test");
    }

    #[test]
    fn test_with_optimization_pass_single() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let compiler = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_optimization_pass("enum_optimization");

        assert_eq!(compiler.custom_passes.len(), 1);
        assert_eq!(compiler.custom_passes[0], "enum_optimization");
    }

    #[test]
    fn test_with_optimization_pass_multiple() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let compiler = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_optimization_pass("enum_optimization")
            .with_optimization_pass("integer_optimization");

        assert_eq!(compiler.custom_passes.len(), 2);
        assert_eq!(compiler.custom_passes[0], "enum_optimization");
        assert_eq!(compiler.custom_passes[1], "integer_optimization");
    }

    #[test]
    fn test_vocabulary_optimization_pass_overrides_o0_for_stats() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]},
                "count": {"type": "integer", "minimum": 1, "maximum": 10}
            }
        });
        let grammar = TctCompiler::new(schema, "VocabularyPassTest")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O0)
            .with_optimization_pass("vocabulary_optimization")
            .compile()
            .unwrap();

        let stats = grammar
            .vocabulary_stats
            .expect("named vocabulary pass should enable vocabulary stats even after O0");
        assert_eq!(stats.optimized_fields, 0);
        assert_eq!(stats.enum_domains, 0);
        assert_eq!(stats.bounded_integer_domains, 0);
    }

    #[test]
    fn test_unknown_optimization_pass_fails_at_compile() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let result = TctCompiler::new(schema, "Test")
            .unwrap()
            .with_optimization_pass("missing_pass")
            .compile();

        match result {
            Err(TctError::InvalidConfiguration { field, reason, .. }) => {
                assert_eq!(field, "optimization_pass");
                assert!(reason.contains("missing_pass"));
            }
            Ok(_) => panic!("expected invalid optimization pass error, got success"),
            Err(error) => panic!("expected invalid optimization pass error, got {error}"),
        }
    }

    #[test]
    fn test_mixed_unknown_optimization_pass_fails_at_compile() {
        let schema = json!({"type": "object", "properties": {
            "status": {"type": "string", "enum": ["active", "inactive"]},
            "count": {"type": "integer", "minimum": 1, "maximum": 10}
        }});
        let result = TctCompiler::new(schema, "MixedPassTest")
            .unwrap()
            .with_optimization_pass("enum_optimization")
            .with_optimization_pass("missing_pass")
            .compile();

        match result {
            Err(TctError::InvalidConfiguration {
                field,
                reason,
                suggestion,
            }) => {
                assert_eq!(field, "optimization_pass");
                assert!(reason.contains("Unknown optimization pass: 'missing_pass'"));
                assert!(suggestion.contains("enum_optimization"));
                assert!(suggestion.contains("pattern_optimization is reserved"));
            }
            Ok(_) => panic!("expected mixed optimization pass error, got success"),
            Err(error) => panic!("expected mixed optimization pass error, got {error}"),
        }
    }

    #[test]
    fn test_pattern_optimization_is_reserved_until_implemented() {
        let schema = json!({"type": "string", "pattern": "^[a-z]{3}$"});
        let result = TctCompiler::new(schema, "Pattern")
            .unwrap()
            .with_pattern_optimization()
            .compile();

        match result {
            Err(TctError::InvalidConfiguration {
                field,
                reason,
                suggestion,
            }) => {
                assert_eq!(field, "optimization_pass");
                assert!(reason.contains("pattern_optimization is reserved"));
                assert!(
                    suggestion.contains("patternProperties validation is always schema-driven")
                );
            }
            Ok(_) => panic!("expected reserved pattern optimization error, got success"),
            Err(error) => panic!("expected reserved pattern optimization error, got {error}"),
        }
    }

    #[test]
    fn test_accessor_methods() {
        let schema = json!({"type": "string"});
        let compiler = TctCompiler::new(schema, "MyType")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O3)
            .with_stats();

        assert_eq!(compiler.type_name(), "MyType");
        assert_eq!(compiler.optimization_level(), OptimizationLevel::O3);
        assert!(compiler.include_stats());
    }

    #[test]
    fn test_compile_with_profiling_enabled() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let grammar = TctCompiler::new(schema, "ProfilingTest")
            .unwrap()
            .with_profiling(ProfilingConfig::basic())
            .compile()
            .unwrap();

        // Profiling code should be generated
        assert!(grammar.profiling_code.is_some());
    }

    #[test]
    fn test_compile_without_profiling() {
        let schema = json!({"type": "string", "enum": ["a", "b"]});
        let grammar = TctCompiler::new(schema, "NoProfilingTest")
            .unwrap()
            .compile()
            .unwrap();

        // No profiling code by default
        assert!(grammar.profiling_code.is_none());
    }

    #[test]
    fn test_schema_analysis_no_optimizable_content() {
        let schema = json!({"type": "string"});
        let compiler = TctCompiler::new(schema, "Plain").unwrap();

        assert!(!compiler.has_optimizable_content());
        assert_eq!(compiler.optimizable_fields_count(), 0);
        assert_eq!(compiler.constrained_values_count(), 0);
        assert_eq!(compiler.domain_type_counts(), (0, 0));
    }

    #[test]
    fn test_schema_analysis_with_enum_only() {
        let schema = json!({"type": "string", "enum": ["a", "b", "c"]});
        let compiler = TctCompiler::new(schema, "EnumTest").unwrap();

        assert!(compiler.has_optimizable_content());
        assert_eq!(compiler.optimizable_fields_count(), 1);
        assert_eq!(compiler.constrained_values_count(), 3);
        assert_eq!(compiler.domain_type_counts(), (1, 0));
    }

    #[test]
    fn test_schema_analysis_with_bounded_integer_only() {
        let schema = json!({"type": "integer", "minimum": 0, "maximum": 10});
        let compiler = TctCompiler::new(schema, "IntegerTest").unwrap();

        assert!(compiler.has_optimizable_content());
        assert_eq!(compiler.optimizable_fields_count(), 1);
        assert_eq!(compiler.constrained_values_count(), 11); // 0 to 10 inclusive
        assert_eq!(compiler.domain_type_counts(), (0, 1));
    }

    #[test]
    fn test_schema_analysis_with_nested_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "nested": {
                    "type": "object",
                    "properties": {
                        "status": {"type": "string", "enum": ["active", "inactive"]},
                        "count": {"type": "integer", "minimum": 1, "maximum": 5}
                    }
                }
            }
        });
        let compiler = TctCompiler::new(schema, "Nested").unwrap();

        assert!(compiler.has_optimizable_content());
        assert_eq!(compiler.optimizable_fields_count(), 2);
        assert_eq!(compiler.constrained_values_count(), 7); // 2 enum + 5 integer
        assert_eq!(compiler.domain_type_counts(), (1, 1));
    }

    #[test]
    fn test_compile_with_o0_no_vocabulary_stats() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]}
            }
        });
        let grammar = TctCompiler::new(schema, "NoStats")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O0)
            .compile()
            .unwrap();

        // O0 should not generate vocabulary stats
        assert!(grammar.vocabulary_stats.is_none());
    }

    #[test]
    fn test_compile_with_o2_has_vocabulary_stats() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]}
            }
        });
        let grammar = TctCompiler::new(schema, "WithStats")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O2)
            .compile()
            .unwrap();

        // O2 should generate vocabulary stats when optimizations are applied
        assert!(grammar.vocabulary_stats.is_some());
    }

    #[test]
    fn test_compile_records_measured_pass_timings() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]},
                "priority": {"type": "integer", "minimum": 1, "maximum": 5}
            },
            "required": ["status"]
        });

        let grammar = TctCompiler::new(schema, "Timing")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O2)
            .compile()
            .unwrap();
        let timing = &grammar
            .performance_metrics
            .as_ref()
            .expect("compiler should emit timing telemetry")
            .timing;

        for pass_name in ["Discovery", "Analysis", "Optimization", "Generation"] {
            assert!(
                timing.phase_timings.contains_key(pass_name),
                "missing timing for {pass_name}"
            );
        }

        let discovery = timing.phase_timings["Discovery"];
        let analysis = timing.phase_timings["Analysis"];
        let optimization = timing.phase_timings["Optimization"];
        let generation = timing.phase_timings["Generation"];

        assert_eq!(timing.schema_analysis, discovery + analysis);
        assert_eq!(timing.optimization, optimization);
        assert_eq!(timing.code_generation, generation);
        assert_eq!(timing.schema_parsing, std::time::Duration::ZERO);
        assert!(timing.total_time >= discovery + analysis + optimization + generation);
    }

    #[test]
    fn test_compile_with_artifacts_preserves_pass_outputs() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]},
                "priority": {"type": "integer", "minimum": 1, "maximum": 5}
            },
            "required": ["status"]
        });

        let compiled = TctCompiler::new(schema, "ArtifactExample")
            .unwrap()
            .with_optimization_level(OptimizationLevel::O3)
            .compile_with_artifacts()
            .unwrap();

        assert_eq!(
            compiled.grammar.base_vocab,
            compiled.artifacts.code_generation.stats.base_vocab_size
        );
        assert!(compiled.artifacts.schema_discovery.root_info.is_some());
        assert!(
            compiled
                .artifacts
                .schema_discovery
                .discovered_types
                .contains_key("ArtifactExample")
        );
        assert!(
            compiled
                .artifacts
                .type_analysis
                .generation_order
                .contains(&"ArtifactExample".to_string())
        );
        assert!(compiled.artifacts.encoding_strategy.is_some());

        let generated_with_metadata = compiled
            .artifacts
            .code_generation
            .type_definitions
            .values()
            .find(|generation| generation.type_name.contains("Status"))
            .expect("inline enum should have generated metadata");
        assert_ne!(
            generated_with_metadata.generated_item_kind,
            crate::generator::generation_output::GeneratedItemKind::Unknown
        );
        assert!(generated_with_metadata.has_token_encoder_impl);

        let timing = &compiled
            .grammar
            .performance_metrics
            .as_ref()
            .expect("compiler should emit timing telemetry")
            .timing;
        for pass_name in ["Discovery", "Analysis", "Optimization", "Generation"] {
            assert!(
                timing.phase_timings.contains_key(pass_name),
                "missing timing for {pass_name}"
            );
        }
    }

    #[test]
    fn test_compile_simple_string_schema() {
        let schema = json!({"type": "string"});
        let grammar = TctCompiler::new(schema, "SimpleString")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        // Simple string generates struct wrapper
        assert!(code.contains("SimpleString"));
    }

    #[test]
    fn test_compile_simple_integer_schema() {
        let schema = json!({"type": "integer"});
        let grammar = TctCompiler::new(schema, "SimpleInteger")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        // Simple integer generates struct wrapper
        assert!(code.contains("SimpleInteger"));
    }

    #[test]
    fn test_compile_simple_boolean_schema() {
        let schema = json!({"type": "boolean"});
        let grammar = TctCompiler::new(schema, "SimpleBoolean")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        // Simple boolean generates struct wrapper
        assert!(code.contains("SimpleBoolean"));
    }

    #[test]
    fn test_compile_object_with_required_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name", "age"]
        });
        let grammar = TctCompiler::new(schema, "Person")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);
        let item_struct = generated_struct(&file, "Person");
        assert!(struct_has_field(item_struct, "name"));
        assert!(struct_has_field(item_struct, "age"));
    }

    #[test]
    fn test_compile_object_with_optional_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "nickname": {"type": "string"}
            },
            "required": ["name"]
        });
        let grammar = TctCompiler::new(schema, "User").unwrap().compile().unwrap();

        let code = grammar.generate_code().unwrap();
        let file = parse_generated_file(&code);
        let item_struct = generated_struct(&file, "User");
        assert!(struct_has_field(item_struct, "name"));
        assert!(struct_has_field(item_struct, "nickname"));
        assert!(code.contains("Option<"));
    }

    #[test]
    fn test_compile_array_schema() {
        let schema = json!({
            "type": "array",
            "items": {"type": "string"}
        });
        let grammar = TctCompiler::new(schema, "StringArray")
            .unwrap()
            .compile()
            .unwrap();

        let code = grammar.generate_code().unwrap();
        // Array generates struct wrapper
        assert!(code.contains("StringArray"));
        assert!(code.contains("Vec"));
    }

    #[test]
    fn max_vocabulary_uses_multitoken_codec_for_wide_enum() {
        let values = (0..385)
            .map(|index| serde_json::Value::String(format!("choice-{index}")))
            .collect::<Vec<_>>();
        let schema = json!({"type": "string", "enum": values});
        let grammar = TctCompiler::new(schema, "WideEnum")
            .unwrap()
            .with_optimization_level(OptimizationLevel::OVocab)
            .with_max_vocabulary(257)
            .unwrap()
            .compile()
            .unwrap();

        assert_eq!(grammar.base_vocab, 257);
        let code = grammar.generate_code().unwrap();
        parse_generated_file(&code);
    }

    #[test]
    fn max_vocabulary_constrains_wide_one_of_discriminator() {
        let variants = (0..385)
            .map(|index| {
                json!({
                    "type": "object",
                    "properties": {"kind": {"const": format!("tool-{index}")}},
                    "required": ["kind"],
                    "additionalProperties": false
                })
            })
            .collect::<Vec<_>>();
        let schema = json!({"oneOf": variants});
        let grammar = TctCompiler::new(schema, "WideToolChoice")
            .unwrap()
            .with_optimization_level(OptimizationLevel::OVocab)
            .with_max_vocabulary(257)
            .unwrap()
            .compile()
            .unwrap();

        assert_eq!(grammar.base_vocab, 257);
        parse_generated_file(&grammar.generate_code().unwrap());
    }

    #[test]
    fn max_vocabulary_rejects_values_below_utf8_floor() {
        let error = match TctCompiler::new(json!({"type": "string"}), "StringValue")
            .unwrap()
            .with_max_vocabulary(256)
        {
            Ok(_) => panic!("UTF-8 requires the 257-symbol floor"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("at least 257"));
    }
}
