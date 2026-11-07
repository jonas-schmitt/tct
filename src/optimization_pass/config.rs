//! Optimization configuration and pass management

#![allow(deprecated)]

use super::core::{OptimizationLevel, OptimizationPass};
use super::passes::{
    BoundedIntegerOptimizationPass, EnumIndexOptimizationPass, PatternOptimizationPass,
};
use rustc_hash::FxHashMap;

/// Configuration for the optimization pass system
pub struct OptimizationConfig {
    /// Optimization level
    pub level: OptimizationLevel,
    /// Ordered list of legacy built-in compatibility passes to apply.
    ///
    /// This field remains public for older callers. The maintained compiler
    /// extension model is structured `EncodingStrategy` metadata, not arbitrary
    /// custom passes that return rendered `TokenStream` fragments.
    pub passes: Vec<Box<dyn OptimizationPass>>,
    /// Pass-specific configuration
    pub pass_config: FxHashMap<String, PassConfig>,
    /// Maximum total vocabulary size constraint (None = unlimited)
    /// Limits complete vocabulary: base_vocab + bpe_merges + special tokens
    pub max_vocab: Option<usize>,
    /// Maximum base vocabulary for multi-token encoding (None = default 256)
    ///
    /// Controls the encoding base for all multi-token ranges (primitives, collections, bounded integers).
    /// Each token range uses next power-of-2 ≤ max_base_vocab as its encoding base.
    ///
    /// **Minimum:** 257 (required for UTF-8 strings: 256 bytes + END_STRING_MARKER)
    /// **Default:** None → 256 (maintains current behavior)
    /// **Range:** 257-65536 (smaller values rejected)
    ///
    /// Trade-off: Smaller base → more tokens per value, Larger base → larger vocabulary
    pub max_base_vocab: Option<u32>,
    /// Compilation constraints (includes string encoding and windowed generation)
    pub compilation_constraints:
        Option<crate::generator::encoding_decision::CompilationConstraints>,
    /// Whether bit-packed optional field presence encoding is enabled
    ///
    /// When enabled, structs with ≥4 optional `Option<T>` fields will pack presence flags
    /// into u32 tokens instead of using individual presence tokens.
    /// Enabled by default at O3, Os, and OVocab; can be explicitly controlled via API.
    pub bit_packing_enabled: bool,
    /// Whether streaming-friendly encoding is enabled
    ///
    /// When enabled:
    /// - All types derive Default for partial construction
    /// - decode_prefix() is generated for truncation-tolerant decoding
    ///
    /// Token format is identical to non-streaming mode (full compatibility).
    /// This enables LLM autocompletion workflows with prefix decoding.
    pub streaming_encode: bool,
    /// Maximum collection length for decoded Vec/HashMap (None = 2048 default)
    ///
    /// Prevents memory exhaustion from model-generated garbage tokens that
    /// decode to billion-element lengths. Default matches model context size
    /// (2048 tokens), since each Vec element needs ≥1 token to encode.
    pub max_collection_length: Option<usize>,
    /// Maximum recursive schema traversal depth during discovery.
    ///
    /// Prevents stack exhaustion from pathological schemas with deeply nested
    /// properties, items, compositions, or definitions. None uses the
    /// repository default from `schema_utils`.
    pub max_schema_depth: Option<usize>,
    /// Digit-aligned decimal text encoding for unbounded i64/u64/f64 numbers.
    ///
    /// Opt-in: emitted token streams are incompatible with the default
    /// bit-split encoding, so existing corpora/wheels keep the old layout.
    /// Motivation: digit-aligned numbers are easier for language models to
    /// learn than bit-split integer payloads.
    pub decimal_number_encoding: bool,
}

/// Configuration for individual passes
#[derive(Debug, Clone)]
pub struct PassConfig {
    /// Whether this pass is enabled
    pub enabled: bool,
    /// Pass-specific settings
    pub settings: FxHashMap<String, String>,
}

impl OptimizationConfig {
    /// Create configuration for -O0 level (no optimizations)
    pub fn o0() -> Self {
        Self {
            level: OptimizationLevel::O0,
            passes: Vec::new(), // No passes for -O0
            pass_config: FxHashMap::default(),
            max_vocab: None,
            max_base_vocab: None, // None = default 256
            compilation_constraints: None,
            bit_packing_enabled: false,     // No optimizations at O0
            streaming_encode: false,        // Disabled by default
            max_collection_length: None,    // None = default 2048
            max_schema_depth: None,         // None = default traversal ceiling
            decimal_number_encoding: false, // Opt-in only
        }
    }

    /// Create configuration for -O2 level (balanced optimizations)
    pub fn o2() -> Self {
        let mut pass_config = FxHashMap::default();
        // Enable vocabulary optimization by default for O2
        pass_config.insert(
            "vocabulary_optimization".to_string(),
            PassConfig {
                enabled: true,
                settings: FxHashMap::default(),
            },
        );

        Self {
            level: OptimizationLevel::O2,
            passes: vec![
                Box::new(EnumIndexOptimizationPass::new()),
                Box::new(BoundedIntegerOptimizationPass::new()),
            ],
            pass_config,
            max_vocab: None,
            max_base_vocab: None, // None = default 256
            compilation_constraints: None,
            bit_packing_enabled: false, // O2: Production-ready optimizations only (no experimental bit-packing)
            streaming_encode: false,    // Disabled by default
            max_collection_length: None, // None = default 2048
            max_schema_depth: None,     // None = default traversal ceiling
            decimal_number_encoding: false, // Opt-in only
        }
    }

    /// Add a legacy built-in optimization pass to the configuration.
    ///
    /// This compatibility method should not be used for new compiler
    /// extensions. Custom pass implementations are not preserved by
    /// `OptimizationConfig::clone()` and are not accepted by
    /// `TctCompiler::with_optimization_pass`.
    #[doc(hidden)]
    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass>) {
        self.passes.push(pass);
    }

    /// Configure a specific pass
    pub fn configure_pass(&mut self, pass_name: &str, config: PassConfig) {
        self.pass_config.insert(pass_name.to_string(), config);
    }

    /// Create configuration from specific pass names
    pub fn from_pass_names(pass_names: &[String]) -> Result<Self, String> {
        let mut config = Self {
            level: OptimizationLevel::O2, // Use O2 as base level
            passes: Vec::new(),
            pass_config: FxHashMap::default(),
            max_vocab: None,
            max_base_vocab: None, // None = default 256
            compilation_constraints: None,
            bit_packing_enabled: false, // Disabled by default (safer, production-ready)
            streaming_encode: false,    // Disabled by default
            max_collection_length: None, // None = default 2048
            max_schema_depth: None,     // None = default traversal ceiling
            decimal_number_encoding: false, // Opt-in only
        };

        for pass_name in pass_names {
            match pass_name.as_str() {
                "enum_optimization" => {
                    config.add_pass(Box::new(EnumIndexOptimizationPass::new()));
                }
                "integer_optimization" => {
                    config.add_pass(Box::new(BoundedIntegerOptimizationPass::new()));
                }
                "pattern_optimization" => {
                    return Err(
                        "pattern_optimization is reserved but not implemented as a direct \
                         optimization pass; patternProperties validation is always schema-driven"
                            .to_string(),
                    );
                }
                "vocabulary_optimization" => {
                    // Vocabulary optimization is handled differently - enable through config
                    config.configure_pass(
                        "vocabulary_optimization",
                        PassConfig {
                            enabled: true,
                            settings: FxHashMap::default(),
                        },
                    );
                }
                _ => {
                    return Err(format!("Unknown optimization pass: '{}'", pass_name));
                }
            }
        }

        // Always enable vocabulary optimization when any passes are specified
        // This ensures statistics are available for analysis and testing
        if !pass_names.is_empty() {
            config.configure_pass(
                "vocabulary_optimization",
                PassConfig {
                    enabled: true,
                    settings: FxHashMap::default(),
                },
            );
        }

        Ok(config)
    }

    /// Set maximum total vocabulary size constraint
    ///
    /// Limits the complete vocabulary including all layers:
    /// VOCAB_SIZE = base_vocab + bpe_merges + special tokens
    pub fn with_max_vocabulary(mut self, max_size: usize) -> Result<Self, String> {
        if max_size == 0 {
            return Err("Vocabulary size must be greater than zero".to_string());
        }
        self.max_vocab = Some(max_size);
        Ok(self)
    }

    /// Create configuration with all available passes (equivalent to O3)
    pub fn with_all_passes() -> Self {
        let pass_names = vec![
            "enum_optimization".to_string(),
            "integer_optimization".to_string(),
            "vocabulary_optimization".to_string(),
        ];
        let mut config =
            Self::from_pass_names(&pass_names).expect("All built-in pass names should be valid");
        // Set level to O3 for aggressive optimizations (including additionalProperties type inference)
        config.level = OptimizationLevel::O3;
        // Enable experimental bit-packing for O3 (aggressive optimization)
        config.bit_packing_enabled = true;
        config
    }

    /// Create configuration for `Os`: minimize generated token sequence size.
    ///
    /// This profile enables all currently available compression passes and
    /// preserves the distinct level so downstream optimization decisions can
    /// prefer token-count reductions.
    pub fn os() -> Self {
        let mut config = Self::with_all_passes();
        config.level = OptimizationLevel::Os;
        config.bit_packing_enabled = true;
        config
    }

    /// Create configuration for `OVocab`: minimize generated vocabulary size.
    ///
    /// This profile enables vocabulary-aware passes while preserving the
    /// distinct level so downstream decisions can prefer lower vocabulary
    /// pressure. It also keeps bit-packed optional field presence because that
    /// structural compression uses existing base tokens and does not increase
    /// generated vocabulary size.
    pub fn ovocab() -> Self {
        let mut config = Self::with_all_passes();
        config.level = OptimizationLevel::OVocab;
        config.bit_packing_enabled = true;
        config
    }

    /// Check if vocabulary optimization is enabled
    pub fn is_vocabulary_optimization_enabled(&self) -> bool {
        self.pass_config
            .get("vocabulary_optimization")
            .map(|config| config.enabled)
            .unwrap_or(false)
    }

    /// Enable bit-packed optional field presence encoding
    ///
    /// When enabled, structs with ≥4 optional `Option<T>` fields will pack presence flags
    /// into u32 tokens.
    /// Enabled by default at O3, Os, and OVocab.
    pub fn with_bit_packing(mut self, enabled: bool) -> Self {
        self.bit_packing_enabled = enabled;
        self
    }

    /// Check if bit-packed presence encoding is enabled
    pub fn is_bit_packing_enabled(&self) -> bool {
        self.bit_packing_enabled
    }

    /// Enable streaming-friendly encoding for prefix decoding
    ///
    /// When enabled:
    /// - All types derive Default for partial construction
    /// - decode_prefix() is generated for truncation-tolerant decoding
    ///
    /// Token format is identical to non-streaming mode (full compatibility).
    pub fn with_streaming_encode(mut self, enabled: bool) -> Self {
        self.streaming_encode = enabled;
        self
    }

    /// Check if streaming-friendly encoding is enabled
    pub fn is_streaming_encode_enabled(&self) -> bool {
        self.streaming_encode
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
    pub fn with_max_collection_length(mut self, max_len: usize) -> Self {
        self.max_collection_length = Some(max_len);
        self
    }

    /// Get the maximum collection length (default 2048)
    pub fn get_max_collection_length(&self) -> usize {
        self.max_collection_length.unwrap_or(2048)
    }

    /// Set maximum recursive schema traversal depth during discovery.
    ///
    /// Use this for tests or trusted schemas with unusual nesting. The default
    /// is high enough for ordinary real-world schemas and rejects pathological
    /// inputs before they can exhaust the call stack.
    pub fn with_max_schema_depth(mut self, max_depth: usize) -> Self {
        self.max_schema_depth = Some(max_depth);
        self
    }

    /// Get the maximum schema traversal depth.
    pub fn get_max_schema_depth(&self) -> usize {
        self.max_schema_depth
            .unwrap_or(crate::schema_utils::DEFAULT_MAX_SCHEMA_TRAVERSAL_DEPTH)
    }

    /// Set maximum base vocabulary for multi-token encoding
    ///
    /// Controls the encoding base for all token ranges (primitives, collections, bounded integers).
    /// The actual encoding base will be the largest power-of-2 that doesn't exceed `max_base_vocab`.
    ///
    /// **Minimum:** 257 (required for UTF-8 strings: 256 bytes + END_STRING_MARKER)
    ///
    /// # Arguments
    /// * `max_base_vocab` - Maximum vocabulary size for encoding base (must be >= 257)
    ///
    /// # Example
    /// ```ignore
    /// use tct::optimization_pass::OptimizationConfig;
    ///
    /// let config = OptimizationConfig::o2()
    ///     .with_max_base_vocab(512)?;  // Uses base-512 encoding
    /// # Ok::<(), String>(())
    /// ```
    pub fn with_max_base_vocab(mut self, max_base_vocab: u32) -> Result<Self, String> {
        if max_base_vocab < 257 {
            return Err(format!(
                "max_base_vocab must be at least 257 (required for UTF-8 strings), got {}",
                max_base_vocab
            ));
        }
        self.max_base_vocab = Some(max_base_vocab);
        Ok(self)
    }

    /// Get the encoding base for multi-token encoding
    ///
    /// Returns the largest power-of-2 that doesn't exceed `max_base_vocab`.
    /// If `max_base_vocab` is None, returns the default of 256.
    ///
    /// # Examples
    /// - `None` → 256 (default)
    /// - `Some(300)` → 256 (largest power-of-2 ≤ 300)
    /// - `Some(512)` → 512 (exact power-of-2)
    /// - `Some(1000)` → 512 (largest power-of-2 ≤ 1000)
    pub fn get_encoding_base(&self) -> u32 {
        Self::compute_encoding_base(self.max_base_vocab)
    }

    /// Compute encoding base from max_base_vocab constraint
    ///
    /// Returns the largest power-of-2 that doesn't exceed the constraint.
    ///
    /// # Arguments
    /// * `max_base_vocab` - Optional maximum base vocabulary (None = 256)
    ///
    /// # Examples
    /// ```ignore
    /// use tct::optimization_pass::OptimizationConfig;
    ///
    /// assert_eq!(OptimizationConfig::compute_encoding_base(None), 256);
    /// assert_eq!(OptimizationConfig::compute_encoding_base(Some(300)), 256);
    /// assert_eq!(OptimizationConfig::compute_encoding_base(Some(512)), 512);
    /// assert_eq!(OptimizationConfig::compute_encoding_base(Some(1000)), 512);
    /// ```
    pub fn compute_encoding_base(max_base_vocab: Option<u32>) -> u32 {
        match max_base_vocab {
            None => 256, // Default base
            Some(max) => {
                // Find largest power-of-2 <= max
                if max <= 256 {
                    256 // Minimum is 256 (even if max_base_vocab is set to 257-300)
                } else {
                    // Find highest set bit position
                    let msb = 31 - max.leading_zeros();
                    1u32 << msb
                }
            }
        }
    }

    /// Check if a specific legacy built-in pass type exists in the configuration
    ///
    /// # Example
    /// ```ignore
    /// use tct::optimization_pass::{OptimizationConfig, EnumIndexOptimizationPass};
    ///
    /// let config = OptimizationConfig::o2();
    /// assert!(config.has_pass_by_type::<EnumIndexOptimizationPass>());
    /// ```
    #[doc(hidden)]
    pub fn has_pass_by_type<T: 'static>(&self) -> bool {
        self.passes.iter().any(|p| p.as_any().is::<T>())
    }

    /// Ensure a specific legacy built-in pass type is present in the configuration
    ///
    /// If the pass type is not already present, it will be added.
    ///
    /// # Example
    /// ```ignore
    /// use tct::optimization_pass::{OptimizationConfig, EnumIndexOptimizationPass};
    ///
    /// let mut config = OptimizationConfig::o0();
    /// config.ensure_pass(EnumIndexOptimizationPass::new());
    /// assert!(config.has_pass_by_type::<EnumIndexOptimizationPass>());
    /// ```
    #[doc(hidden)]
    pub fn ensure_pass<T: OptimizationPass + 'static>(&mut self, pass: T) {
        if !self.has_pass_by_type::<T>() {
            self.add_pass(Box::new(pass));
        }
    }

    /// Remove all legacy built-in passes of a specific type from the configuration
    ///
    /// # Example
    /// ```ignore
    /// use tct::optimization_pass::{OptimizationConfig, EnumIndexOptimizationPass};
    ///
    /// let mut config = OptimizationConfig::o2();
    /// config.remove_pass_by_type::<EnumIndexOptimizationPass>();
    /// assert!(!config.has_pass_by_type::<EnumIndexOptimizationPass>());
    /// ```
    #[doc(hidden)]
    pub fn remove_pass_by_type<T: 'static>(&mut self) {
        self.passes.retain(|p| !p.as_any().is::<T>());
    }
}

impl Clone for OptimizationConfig {
    fn clone(&self) -> Self {
        // Clone the basic fields
        let mut cloned = Self {
            level: self.level,
            passes: Vec::new(), // Start with empty passes
            pass_config: self.pass_config.clone(),
            max_vocab: self.max_vocab,
            max_base_vocab: self.max_base_vocab,
            compilation_constraints: self.compilation_constraints.clone(),
            bit_packing_enabled: self.bit_packing_enabled,
            streaming_encode: self.streaming_encode,
            max_collection_length: self.max_collection_length,
            max_schema_depth: self.max_schema_depth,
            decimal_number_encoding: self.decimal_number_encoding,
        };

        // Recreate the explicit pass list rather than inferring it from the
        // optimization level. `from_pass_names()` can create an O2 config with a
        // deliberate subset, and pass orchestration clones this config between
        // analysis, optimization, and generation.
        for pass in &self.passes {
            match pass.name() {
                "enum_index_optimization" => {
                    cloned.add_pass(Box::new(EnumIndexOptimizationPass::new()));
                }
                "bounded_integer_optimization" => {
                    cloned.add_pass(Box::new(BoundedIntegerOptimizationPass::new()));
                }
                "pattern_optimization" => {
                    cloned.add_pass(Box::new(PatternOptimizationPass::new()));
                }
                _ => {}
            }
        }

        cloned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_encoding_base_default() {
        // None should return default 256
        assert_eq!(OptimizationConfig::compute_encoding_base(None), 256);
    }

    #[test]
    fn test_compute_encoding_base_exact_power_of_2() {
        // Exact powers of 2 should return themselves
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(256)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(512)), 512);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(1024)), 1024);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(2048)), 2048);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(4096)), 4096);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(8192)), 8192);
        assert_eq!(
            OptimizationConfig::compute_encoding_base(Some(16384)),
            16384
        );
        assert_eq!(
            OptimizationConfig::compute_encoding_base(Some(32768)),
            32768
        );
        assert_eq!(
            OptimizationConfig::compute_encoding_base(Some(65536)),
            65536
        );
    }

    #[test]
    fn test_compute_encoding_base_non_power_of_2() {
        // Non-powers of 2 should return largest power-of-2 <= value
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(257)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(300)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(400)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(500)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(511)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(513)), 512);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(1000)), 512);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(1023)), 512);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(1025)), 1024);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(2000)), 1024);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(5000)), 4096);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(10000)), 8192);
    }

    #[test]
    fn test_compute_encoding_base_minimum() {
        // Values <= 256 should all return 256 (minimum)
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(1)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(128)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(255)), 256);
        assert_eq!(OptimizationConfig::compute_encoding_base(Some(256)), 256);
    }

    #[test]
    fn test_with_max_base_vocab_valid() {
        // Valid values should succeed
        let config = OptimizationConfig::o2().with_max_base_vocab(512);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().max_base_vocab, Some(512));

        let config = OptimizationConfig::o2().with_max_base_vocab(257);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().max_base_vocab, Some(257));

        let config = OptimizationConfig::o2().with_max_base_vocab(1000);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().max_base_vocab, Some(1000));
    }

    #[test]
    fn test_with_max_base_vocab_invalid() {
        // Values < 257 should fail
        let result = OptimizationConfig::o2().with_max_base_vocab(256);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("at least 257"));
        }

        assert!(OptimizationConfig::o2().with_max_base_vocab(100).is_err());
        assert!(OptimizationConfig::o2().with_max_base_vocab(0).is_err());
    }

    #[test]
    fn test_get_encoding_base() {
        // Default config should return 256
        let config = OptimizationConfig::o2();
        assert_eq!(config.get_encoding_base(), 256);

        // Config with max_base_vocab should compute correctly
        let config = OptimizationConfig::o2().with_max_base_vocab(512).unwrap();
        assert_eq!(config.get_encoding_base(), 512);

        let config = OptimizationConfig::o2().with_max_base_vocab(1000).unwrap();
        assert_eq!(config.get_encoding_base(), 512);

        let config = OptimizationConfig::o2().with_max_base_vocab(2048).unwrap();
        assert_eq!(config.get_encoding_base(), 2048);
    }

    #[test]
    fn test_max_base_vocab_preserved_in_constructors() {
        // Ensure max_base_vocab field is properly initialized in all constructors
        let config = OptimizationConfig::o0();
        assert_eq!(config.max_base_vocab, None);

        let config = OptimizationConfig::o2();
        assert_eq!(config.max_base_vocab, None);

        let config =
            OptimizationConfig::from_pass_names(&["enum_optimization".to_string()]).unwrap();
        assert_eq!(config.max_base_vocab, None);

        let config = OptimizationConfig::with_all_passes();
        assert_eq!(config.max_base_vocab, None);
    }

    #[test]
    fn test_max_base_vocab_preserved_in_clone() {
        // Ensure max_base_vocab is preserved when cloning
        let original = OptimizationConfig::o2().with_max_base_vocab(1024).unwrap();
        let cloned = original.clone();

        assert_eq!(cloned.max_base_vocab, Some(1024));
        assert_eq!(cloned.get_encoding_base(), 1024);
    }

    #[test]
    fn test_max_collection_length_default() {
        // Default should be 2048 (matches model context size)
        let config = OptimizationConfig::o2();
        assert_eq!(config.get_max_collection_length(), 2048);
    }

    #[test]
    fn test_max_collection_length_custom() {
        // Custom value should override default
        let config = OptimizationConfig::o2().with_max_collection_length(8192);
        assert_eq!(config.get_max_collection_length(), 8192);
    }

    #[test]
    fn test_max_collection_length_preserved_in_constructors() {
        // Ensure max_collection_length field is properly initialized in all constructors
        let config = OptimizationConfig::o0();
        assert_eq!(config.max_collection_length, None);
        assert_eq!(config.get_max_collection_length(), 2048);

        let config = OptimizationConfig::o2();
        assert_eq!(config.max_collection_length, None);
        assert_eq!(config.get_max_collection_length(), 2048);

        let config =
            OptimizationConfig::from_pass_names(&["enum_optimization".to_string()]).unwrap();
        assert_eq!(config.max_collection_length, None);
        assert_eq!(config.get_max_collection_length(), 2048);

        let config = OptimizationConfig::with_all_passes();
        assert_eq!(config.max_collection_length, None);
        assert_eq!(config.get_max_collection_length(), 2048);

        let config = OptimizationConfig::os();
        assert_eq!(config.max_collection_length, None);
        assert_eq!(config.get_max_collection_length(), 2048);

        let config = OptimizationConfig::ovocab();
        assert_eq!(config.max_collection_length, None);
        assert_eq!(config.get_max_collection_length(), 2048);
    }

    #[test]
    fn test_size_and_vocabulary_profiles_preserve_distinct_levels() {
        let os_config = OptimizationConfig::os();
        assert_eq!(os_config.level, OptimizationLevel::Os);
        assert!(os_config.has_pass_by_type::<EnumIndexOptimizationPass>());
        assert!(os_config.has_pass_by_type::<BoundedIntegerOptimizationPass>());
        assert!(!os_config.has_pass_by_type::<PatternOptimizationPass>());
        assert!(os_config.is_vocabulary_optimization_enabled());
        assert!(os_config.is_bit_packing_enabled());

        let ovocab_config = OptimizationConfig::ovocab();
        assert_eq!(ovocab_config.level, OptimizationLevel::OVocab);
        assert!(ovocab_config.has_pass_by_type::<EnumIndexOptimizationPass>());
        assert!(ovocab_config.has_pass_by_type::<BoundedIntegerOptimizationPass>());
        assert!(!ovocab_config.has_pass_by_type::<PatternOptimizationPass>());
        assert!(ovocab_config.is_vocabulary_optimization_enabled());
        assert!(ovocab_config.is_bit_packing_enabled());
    }

    #[test]
    fn test_max_collection_length_preserved_in_clone() {
        // Ensure max_collection_length is preserved when cloning
        let original = OptimizationConfig::o2().with_max_collection_length(4096);
        let cloned = original.clone();

        assert_eq!(cloned.max_collection_length, Some(4096));
        assert_eq!(cloned.get_max_collection_length(), 4096);
    }

    #[test]
    fn test_from_pass_names_builds_expected_passes_and_vocabulary_config() {
        let pass_names = [
            "enum_optimization".to_string(),
            "integer_optimization".to_string(),
        ];
        let config = OptimizationConfig::from_pass_names(&pass_names).unwrap();

        assert_eq!(config.level, OptimizationLevel::O2);
        assert_eq!(config.passes.len(), 2);
        assert!(config.has_pass_by_type::<EnumIndexOptimizationPass>());
        assert!(config.has_pass_by_type::<BoundedIntegerOptimizationPass>());
        assert!(!config.has_pass_by_type::<PatternOptimizationPass>());
        assert!(config.is_vocabulary_optimization_enabled());

        let vocabulary_only =
            OptimizationConfig::from_pass_names(&["vocabulary_optimization".to_string()]).unwrap();
        assert!(vocabulary_only.passes.is_empty());
        assert!(vocabulary_only.is_vocabulary_optimization_enabled());

        let empty = OptimizationConfig::from_pass_names(&[]).unwrap();
        assert!(empty.passes.is_empty());
        assert!(!empty.is_vocabulary_optimization_enabled());
    }

    #[test]
    fn test_clone_preserves_explicit_pass_subset() {
        let config = OptimizationConfig::from_pass_names(&["enum_optimization".to_string()])
            .expect("known pass should build config");
        let cloned = config.clone();

        assert_eq!(cloned.level, OptimizationLevel::O2);
        assert_eq!(cloned.passes.len(), 1);
        assert!(cloned.has_pass_by_type::<EnumIndexOptimizationPass>());
        assert!(!cloned.has_pass_by_type::<BoundedIntegerOptimizationPass>());
        assert!(!cloned.has_pass_by_type::<PatternOptimizationPass>());
        assert!(cloned.is_vocabulary_optimization_enabled());
    }

    #[test]
    fn test_from_pass_names_rejects_unknown_passes() {
        let error = match OptimizationConfig::from_pass_names(&["missing_pass".to_string()]) {
            Ok(_) => panic!("unknown pass should fail"),
            Err(error) => error,
        };

        assert_eq!(error, "Unknown optimization pass: 'missing_pass'");
    }

    #[test]
    fn test_from_pass_names_rejects_reserved_pattern_pass() {
        let error = match OptimizationConfig::from_pass_names(&["pattern_optimization".to_string()])
        {
            Ok(_) => panic!("reserved pattern pass should fail until it is implemented"),
            Err(error) => error,
        };

        assert!(error.contains("pattern_optimization is reserved but not implemented"));
        assert!(error.contains("patternProperties validation is always schema-driven"));
    }

    #[test]
    fn test_max_vocabulary_builder_accepts_positive_values() {
        let config = OptimizationConfig::o2()
            .with_max_vocabulary(1024)
            .expect("positive max vocabulary should be valid");

        assert_eq!(config.max_vocab, Some(1024));
        let error = match OptimizationConfig::o2().with_max_vocabulary(0) {
            Ok(_) => panic!("zero max vocabulary should fail"),
            Err(error) => error,
        };
        assert_eq!(error, "Vocabulary size must be greater than zero");
    }

    #[test]
    fn test_bit_packing_and_streaming_builders_toggle_flags() {
        let config = OptimizationConfig::o2()
            .with_bit_packing(true)
            .with_streaming_encode(true);

        assert!(config.is_bit_packing_enabled());
        assert!(config.is_streaming_encode_enabled());

        let config = config.with_bit_packing(false).with_streaming_encode(false);
        assert!(!config.is_bit_packing_enabled());
        assert!(!config.is_streaming_encode_enabled());
    }

    #[test]
    fn test_pass_type_management_is_idempotent() {
        let mut config = OptimizationConfig::o0();
        assert!(!config.has_pass_by_type::<EnumIndexOptimizationPass>());

        config.ensure_pass(EnumIndexOptimizationPass::new());
        config.ensure_pass(EnumIndexOptimizationPass::new());
        assert!(config.has_pass_by_type::<EnumIndexOptimizationPass>());
        assert_eq!(config.passes.len(), 1);

        config.ensure_pass(BoundedIntegerOptimizationPass::new());
        assert!(config.has_pass_by_type::<BoundedIntegerOptimizationPass>());
        assert_eq!(config.passes.len(), 2);

        config.remove_pass_by_type::<EnumIndexOptimizationPass>();
        assert!(!config.has_pass_by_type::<EnumIndexOptimizationPass>());
        assert!(config.has_pass_by_type::<BoundedIntegerOptimizationPass>());
        assert_eq!(config.passes.len(), 1);

        config.remove_pass_by_type::<PatternOptimizationPass>();
        assert_eq!(config.passes.len(), 1);
    }
}
