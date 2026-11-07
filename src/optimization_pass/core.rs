//! Legacy optimization pass traits and types.
//!
//! These types are retained for compatibility with the historic
//! `tct::optimization_pass` API. They are not the active extension point for new
//! compiler work: the maintained four-pass pipeline carries optimization
//! results as structured encoding metadata, and generation is the only phase
//! that should emit Rust token streams.

#![allow(deprecated)]

use proc_macro2::TokenStream;
use rustc_hash::FxHashMap;
use serde_json::Value;

/// Compatibility-only trait for historic optimization passes.
///
/// New optimization work should extend structured discovery, analysis, or
/// `EncodingStrategy` metadata instead of returning generated `TokenStream`
/// fragments from this trait. `TctCompiler::with_optimization_pass` accepts
/// only named built-in passes, and `OptimizationConfig::clone()` preserves only
/// known built-ins.
#[deprecated(
    since = "0.1.0",
    note = "Compatibility-only TokenStream hook. New optimizer work must use structured metadata or EncodingStrategy decisions in the four-pass compiler pipeline."
)]
pub trait OptimizationPass: std::fmt::Debug + Send + Sync {
    /// Name of this optimization pass
    fn name(&self) -> &'static str;

    /// Description of what this pass does
    fn description(&self) -> &'static str;

    /// Downcast to Any for type-based pass identification
    ///
    /// This allows checking if a pass is of a specific type and removing
    /// specific passes from the optimization pipeline.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Apply this optimization pass to enum generation
    /// Returns None if this pass doesn't modify enum generation
    fn optimize_enum_encoding(
        &self,
        enum_name: &syn::Ident,
        enum_values: &[String],
        _context: &OptimizationContext,
    ) -> Option<TokenStream> {
        // Default: no optimization applied
        let _ = (enum_name, enum_values);
        None
    }

    /// Apply this optimization pass to struct field encoding
    /// Returns None if this pass doesn't modify field encoding
    fn optimize_field_encoding(
        &self,
        field_name: &str,
        field_type: &str,
        field_schema: &Value,
        _context: &OptimizationContext,
    ) -> Option<TokenStream> {
        // Default: no optimization applied
        let _ = (field_name, field_type, field_schema);
        None
    }

    /// Apply this optimization pass to integer type encoding
    /// Returns None if this pass doesn't modify integer encoding
    fn optimize_integer_encoding(
        &self,
        integer_constraints: &IntegerConstraints,
        _context: &OptimizationContext,
    ) -> Option<TokenStream> {
        // Default: no optimization applied
        let _ = integer_constraints;
        None
    }
}

/// Context information available to optimization passes
#[derive(Debug)]
pub struct OptimizationContext {
    /// The full schema being processed
    pub schema: Value,
    /// Current optimization level
    pub optimization_level: OptimizationLevel,
    /// Maximum vocabulary size constraint (if specified)
    pub max_vocabulary_size: Option<usize>,
    /// Schema analysis results (vocabulary domains, etc.)
    pub analysis_results: FxHashMap<String, AnalysisResult>,
}

/// Integer constraint information for optimization
#[derive(Debug)]
pub struct IntegerConstraints {
    /// Minimum value (inclusive)
    pub minimum: Option<i64>,
    /// Maximum value (inclusive)
    pub maximum: Option<i64>,
    /// Minimum value (exclusive)
    pub exclusive_minimum: Option<i64>,
    /// Maximum value (exclusive)
    pub exclusive_maximum: Option<i64>,
}

/// Results from schema analysis that passes can use
#[derive(Debug)]
pub enum AnalysisResult {
    /// Vocabulary domain with known cardinality
    VocabularyDomain {
        /// Total number of possible values in this domain
        cardinality: usize,
    },
    /// Pattern-constrained domain with possible finite enumeration
    PatternDomain {
        /// Regular expression pattern
        pattern: String,
        /// Finite set of values if pattern is enumerable
        finite_values: Option<Vec<String>>,
    },
    /// Integer domain with known bounds
    IntegerDomain {
        /// Minimum value (inclusive)
        min: i64,
        /// Maximum value (inclusive)
        max: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;

    #[derive(Debug)]
    struct NoopOptimizationPass;

    impl OptimizationPass for NoopOptimizationPass {
        fn name(&self) -> &'static str {
            "noop"
        }

        fn description(&self) -> &'static str {
            "No-op optimization pass for default method coverage"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn context() -> OptimizationContext {
        OptimizationContext {
            schema: serde_json::json!({"type": "object"}),
            optimization_level: OptimizationLevel::O2,
            max_vocabulary_size: Some(512),
            analysis_results: FxHashMap::default(),
        }
    }

    #[test]
    fn default_optimization_pass_methods_decline_all_transformations() {
        let pass = NoopOptimizationPass;
        let enum_name = syn::parse_str::<syn::Ident>("Status").unwrap();
        let enum_values = vec!["ready".to_string(), "failed".to_string()];
        let field_schema = serde_json::json!({"type": "string"});
        let integer_constraints = IntegerConstraints {
            minimum: Some(0),
            maximum: Some(10),
            exclusive_minimum: None,
            exclusive_maximum: None,
        };
        let context = context();

        assert_eq!(pass.name(), "noop");
        assert!(pass.description().contains("No-op"));
        assert!(pass.as_any().is::<NoopOptimizationPass>());
        assert!(
            pass.optimize_enum_encoding(&enum_name, &enum_values, &context)
                .is_none()
        );
        assert!(
            pass.optimize_field_encoding("name", "String", &field_schema, &context)
                .is_none()
        );
        assert!(
            pass.optimize_integer_encoding(&integer_constraints, &context)
                .is_none()
        );
    }

    #[test]
    fn optimization_context_and_analysis_results_expose_expected_state() {
        let mut analysis_results = FxHashMap::default();
        analysis_results.insert(
            "status".to_string(),
            AnalysisResult::VocabularyDomain { cardinality: 3 },
        );
        analysis_results.insert(
            "code".to_string(),
            AnalysisResult::PatternDomain {
                pattern: "^(dev|prod)$".to_string(),
                finite_values: Some(vec!["dev".to_string(), "prod".to_string()]),
            },
        );
        analysis_results.insert(
            "port".to_string(),
            AnalysisResult::IntegerDomain {
                min: 1024,
                max: 65535,
            },
        );

        let context = OptimizationContext {
            schema: serde_json::json!({"type": "object"}),
            optimization_level: OptimizationLevel::O3,
            max_vocabulary_size: Some(1024),
            analysis_results,
        };

        assert_eq!(context.optimization_level, OptimizationLevel::O3);
        assert_eq!(context.max_vocabulary_size, Some(1024));
        assert_eq!(context.schema["type"], "object");

        match context.analysis_results.get("status").unwrap() {
            AnalysisResult::VocabularyDomain { cardinality } => assert_eq!(*cardinality, 3),
            other => panic!("unexpected status analysis result: {other:?}"),
        }
        match context.analysis_results.get("code").unwrap() {
            AnalysisResult::PatternDomain {
                pattern,
                finite_values: Some(values),
            } => {
                assert_eq!(pattern, "^(dev|prod)$");
                assert_eq!(values, &["dev", "prod"]);
            }
            other => panic!("unexpected code analysis result: {other:?}"),
        }
        match context.analysis_results.get("port").unwrap() {
            AnalysisResult::IntegerDomain { min, max } => {
                assert_eq!((*min, *max), (1024, 65535));
            }
            other => panic!("unexpected port analysis result: {other:?}"),
        }
    }

    #[test]
    fn optimization_level_parse_cli_accepts_documented_spellings() {
        for (input, expected) in [
            ("0", OptimizationLevel::O0),
            ("O0", OptimizationLevel::O0),
            ("-O0", OptimizationLevel::O0),
            ("2", OptimizationLevel::O2),
            ("O2", OptimizationLevel::O2),
            ("3", OptimizationLevel::O3),
            ("O3", OptimizationLevel::O3),
            ("s", OptimizationLevel::Os),
            ("Os", OptimizationLevel::Os),
            ("os", OptimizationLevel::Os),
            ("size", OptimizationLevel::Os),
            ("vocab", OptimizationLevel::OVocab),
            ("OVocab", OptimizationLevel::OVocab),
            ("ovocabulary", OptimizationLevel::OVocab),
            ("vocabulary", OptimizationLevel::OVocab),
        ] {
            assert_eq!(
                OptimizationLevel::parse_cli(input).unwrap(),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn optimization_level_display_uses_stable_profile_labels() {
        assert_eq!(OptimizationLevel::O0.to_string(), "O0");
        assert_eq!(OptimizationLevel::O2.to_string(), "O2");
        assert_eq!(OptimizationLevel::O3.to_string(), "O3");
        assert_eq!(OptimizationLevel::Os.to_string(), "Os");
        assert_eq!(OptimizationLevel::OVocab.to_string(), "OVocab");
    }

    #[test]
    fn optimization_level_parse_cli_rejects_unknown_profile() {
        let error = OptimizationLevel::parse_cli("O1").unwrap_err();
        assert!(error.contains("invalid optimization level"));
        assert!(error.contains("OVocab"));
    }
}

/// Optimization level configuration
/// Optimization levels for TCT compilation
///
/// These levels provide a compiler-style approach to balancing between development
/// convenience and production efficiency. Higher levels trade some debuggability
/// for better compression and performance.
///
/// ## Comparison
///
/// | Level | What it adds | Use Case |
/// |-------|--------------|----------|
/// | O0    | Nothing: no optimization passes run | Development/Testing |
/// | O2    | Bounded-integer offset encodings and zero-token constants | Balanced |
/// | O3    | O2 plus format-aware strings and bit-packed optional presence (default) | Maximum compression |
/// | Os    | Currently identical to OVocab: O3 without format-aware strings | Context-constrained |
/// | OVocab| Currently identical to Os | Memory-constrained |
///
/// ## Detailed Behavior
///
/// See individual variant documentation for specific optimizations applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// No optimizations applied - preserves schema structure exactly
    ///
    /// **Best for**: Development, debugging, testing, maximum compatibility
    ///
    /// **Behavior**:
    /// - Enum values are still encoded as one-token variant indices, as at every level
    /// - All integers use standard Rust types (`i64`, `f64`)
    /// - No vocabulary sharing or optimization
    /// - Generated code closely mirrors schema structure
    ///
    /// **Trade-offs**: Larger token sequences, more predictable behavior
    O0,

    /// Size optimization - minimize total token count
    ///
    /// **Best for**: Long documents, limited context windows, bandwidth-critical applications
    ///
    /// **Behavior**:
    /// - O2-safe structural optimizations
    /// - Bit-packed optional field presence
    /// - Currently produces the same encodings as [`OptimizationLevel::OVocab`]; the
    ///   labels stay distinct for future tuning
    ///
    /// **Trade-offs**: Potentially larger vocabulary, complex token mappings
    ///
    /// **Status**: Profile implementation active
    Os,

    /// Vocabulary optimization - minimize total vocabulary size
    ///
    /// **Best for**: Memory-constrained environments, vocabulary-sensitive LLM applications
    ///
    /// **Behavior**:
    /// - O2-safe structural optimizations
    /// - Bit-packed optional field presence
    /// - Currently produces the same encodings as [`OptimizationLevel::Os`]; the
    ///   labels stay distinct for future tuning
    ///
    /// **Trade-offs**: None yet; the encodings equal `Os`
    ///
    /// **Status**: Profile implementation active
    OVocab,

    /// Balanced optimization - good compression with reasonable vocabulary
    ///
    /// **Best for**: Production applications, general use cases
    ///
    /// **Behavior**:
    /// - Bounded integers → offset encoding: `range [5,10]` → a width token and one offset token
    /// - Single-variant enums → constant (0 tokens): `enum { Active }` → compile-time constant
    /// - Constant integers → constant (0 tokens): `{min: 42, max: 42}` → compile-time constant
    /// - Enum values stay one-token indices with modulo decoding (`"active"` → `0`,
    ///   `"inactive"` → `1`), exactly as at every other level
    ///
    /// **Trade-offs**: Good balance of efficiency and debuggability
    ///
    /// **Implemented optimizations**:
    /// - Bounded integer optimization
    /// - Single-variant enum constant
    /// - Constant integer (min=max)
    /// - Vocabulary statistics (reporting only)
    ///
    O2,

    /// Maximum optimization - semantic interpretation and experimental techniques
    ///
    /// **Best for**: Research, extreme efficiency requirements, experimental applications
    ///
    /// **Behavior**: All O2 optimizations plus:
    /// - Format-optimized strings: `{type: "string", format: "date-time"}` → Unix timestamp
    ///   - date-time → i64 Unix timestamp for whole-second RFC 3339 values (8 tokens vs
    ///     ~30-40 for the ISO8601 string); values with a sub-second part fall back to the
    ///     string encoding
    ///   - byte → base64-decoded bytes packed into u32 tokens (~25% savings)
    ///   - int32, int64, float, double → numeric encodings of the digit strings
    ///   - other formats (uuid, uri, ipv4, email, ...) use the standard string encoding
    /// - AdditionalProperties type refinement: Infers types beyond schema declarations
    /// - Advanced pattern analysis
    ///
    /// **Trade-offs**: Requires runtime validation, data must conform to declared formats
    ///
    /// **Implemented optimizations**:
    /// - Format-optimized strings
    /// - AdditionalProperties refinement
    ///
    /// This is the default level of `TctCompiler` and the `tct` CLI.
    O3,
}

impl OptimizationLevel {
    /// Stable user-facing label for reports, logs, and metadata.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::O0 => "O0",
            Self::O2 => "O2",
            Self::O3 => "O3",
            Self::Os => "Os",
            Self::OVocab => "OVocab",
        }
    }

    /// Parse command-line optimization profile spelling.
    pub fn parse_cli(value: &str) -> Result<Self, String> {
        let trimmed = value.trim();
        let without_dash_o = trimmed
            .strip_prefix("-O")
            .or_else(|| trimmed.strip_prefix("-o"))
            .unwrap_or(trimmed);
        let lowercase = without_dash_o.to_ascii_lowercase();
        let normalized = lowercase.strip_prefix('o').unwrap_or(&lowercase);

        match normalized {
            "0" => Ok(Self::O0),
            "2" => Ok(Self::O2),
            "3" => Ok(Self::O3),
            "s" | "size" => Ok(Self::Os),
            "vocab" | "vocabulary" => Ok(Self::OVocab),
            _ => Err(format!(
                "invalid optimization level '{}'; valid values are 0, O0, 2, O2, 3, O3, s, Os, size, vocab, or OVocab",
                value
            )),
        }
    }
}

impl std::fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for OptimizationLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_cli(value)
    }
}
