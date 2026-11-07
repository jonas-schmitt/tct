//! # Profiling Configuration for TCT Code Generation
//!
//! This module provides comprehensive profiling and benchmarking capabilities for TCT-generated
//! tokenizers. When enabled, TCT generates feature-gated profiling infrastructure that provides
//! zero-overhead timing measurements, benchmark helpers, and optional Criterion integration.
//!
//! ## Features
//!
//! - **Zero-overhead profiling**: generated code is only compiled when the generated crate enables its `profiling` feature
//! - **Multiple profiling levels**: Basic timing, detailed analysis, and benchmarking
//! - **Criterion integration**: Automatic generation of comprehensive benchmark files
//! - **Builder pattern**: Fluent API for configuration
//! - **Feature-gated**: generated crates use `#[cfg(feature = "profiling")]` for conditional compilation
//!
//! ## Usage Examples
//!
//! ### Basic Profiling
//!
//! ```rust
//! use tct::{TctCompiler, ProfilingConfig};
//! use serde_json::json;
//!
//! let schema = json!({"type": "string", "enum": ["red", "green", "blue"]});
//! let grammar = TctCompiler::new(schema, "Color")?
//!     .with_profiling(ProfilingConfig::basic())
//!     .compile()?;
//!
//! // Generated code will include TimedEncode trait with encode_with_timing()
//! let code = grammar.generate_code()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Detailed Profiling
//!
//! ```rust
//! use tct::{TctCompiler, ProfilingConfig};
//! use serde_json::json;
//!
//! let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
//! let grammar = TctCompiler::new(schema, "Person")?
//!     .with_profiling(ProfilingConfig::detailed())
//!     .compile()?;
//!
//! // Generated code includes detailed timing with operation breakdowns.
//! // Use tct-measure-tokens --track-allocations for measured allocation counts.
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Criterion Benchmark Generation
//!
//! ```no_run
//! use tct::{TctCompiler, ProfilingConfig};
//! use serde_json::json;
//!
//! let schema = json!({"type": "string", "enum": ["small", "medium", "large"]});
//! let profiling = ProfilingConfig::benchmarking()
//!     .with_criterion_sample("small", json!("small"))
//!     .with_criterion_sample("large", json!("large"));
//! let grammar = TctCompiler::new(schema, "Size")?
//!     .with_profiling(profiling)
//!     .compile()?;
//!
//! // Generates both profiling code AND Criterion benchmark file (requires benches/ directory)
//! if let Some(profiling) = &grammar.profiling_code {
//!     if let Some(benchmark_file) = &profiling.benchmark_file {
//!         std::fs::write("benches/size_benchmarks.rs", benchmark_file)?;
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Custom Configuration
//!
//! ```rust
//! use tct::{TctCompiler, ProfilingConfig};
//! use serde_json::json;
//!
//! let config = ProfilingConfig::basic()
//!     .with_feature_gate("custom-profiling")
//!     .with_criterion(true)
//!     .with_allocation_tracking(true);
//!
//! let schema = json!({"type": "number", "minimum": 0, "maximum": 100});
//! let grammar = TctCompiler::new(schema, "Percentage")?
//!     .with_profiling(config)
//!     .compile()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Generated Profiling API
//!
//! When profiling is enabled, the generated code includes:
//!
//! - **`TimedEncode` trait**: Provides `encode_with_timing()`, `decode_with_timing()`, and `token_count_with_timing()`
//! - **`quick_benchmark()` function**: Runs multiple iterations and returns `BenchmarkResults`
//! - **`benchmark_round_trip()` function**: Tests encode → decode → validate cycle
//! - **`DetailedProfiler` trait** (detailed level): Per-operation timing breakdowns
//!
//! Allocation counts are not returned by the generated profiling helper. Use
//! the maintained `tct-measure-tokens --track-allocations` path when generated
//! helper subprocess allocation counts are required.
//!
//! Generated Criterion benchmark files require schema-valid sample JSON values.
//! Add them with [`ProfilingConfig::with_criterion_sample`]. Without samples,
//! the generated benchmark file fails at runtime with an explicit message
//! instead of using placeholder data.
//!
//! ## Feature Gate Usage
//!
//! Generated profiling code is feature-gated for zero-overhead when disabled:
//!
//! ```toml
//! # Cargo.toml
//! [features]
//! profiling = []
//! ```
//!
//! ```bash
//! # Enable profiling during compilation
//! cargo build --features profiling
//! ```

/// Profiling level configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfilingLevel {
    /// No profiling code generated (default)
    #[default]
    None,
    /// Basic timing wrappers for encode/decode operations
    Basic,
    /// Detailed per-operation breakdowns and statistics
    Detailed,
}

/// Caller-provided JSON fixture for generated Criterion benchmarks.
#[derive(Debug, Clone)]
pub struct CriterionBenchmarkSample {
    /// Human-readable sample name used in Criterion benchmark IDs.
    pub name: String,
    /// Schema-valid JSON instance for the generated root type.
    pub value: serde_json::Value,
}

/// Profiling configuration for code generation
#[derive(Debug, Clone)]
pub struct ProfilingConfig {
    /// Level of profiling detail to generate
    pub level: ProfilingLevel,
    /// Feature gate name for conditional compilation (e.g., "profiling")
    pub feature_gate: String,
    /// Whether to generate Criterion benchmark files
    pub generate_criterion: bool,
    /// Whether to enable optional heap-profiler hooks in generated timing helpers
    pub track_allocations: bool,
    /// Schema-valid sample JSON values for generated Criterion benchmarks
    pub criterion_samples: Vec<CriterionBenchmarkSample>,
}

impl ProfilingConfig {
    /// Create a basic profiling configuration
    ///
    /// Generates simple timing wrappers with the "profiling" feature gate.
    ///
    /// # Examples
    ///
    /// ```
    /// use tct::profiling_config::ProfilingConfig;
    /// use tct::ProfilingLevel;
    ///
    /// let config = ProfilingConfig::basic();
    /// assert_eq!(config.level, ProfilingLevel::Basic);
    /// assert_eq!(config.feature_gate, "profiling");
    /// ```
    pub fn basic() -> Self {
        Self {
            level: ProfilingLevel::Basic,
            feature_gate: "profiling".to_string(),
            generate_criterion: false,
            track_allocations: false,
            criterion_samples: Vec::new(),
        }
    }

    /// Create a detailed profiling configuration
    ///
    /// Generates comprehensive timing with per-operation breakdowns.
    ///
    /// # Examples
    ///
    /// ```
    /// use tct::profiling_config::ProfilingConfig;
    /// use tct::ProfilingLevel;
    ///
    /// let config = ProfilingConfig::detailed();
    /// assert_eq!(config.level, ProfilingLevel::Detailed);
    /// ```
    pub fn detailed() -> Self {
        Self {
            level: ProfilingLevel::Detailed,
            feature_gate: "profiling".to_string(),
            generate_criterion: false,
            track_allocations: true,
            criterion_samples: Vec::new(),
        }
    }

    /// Create a benchmarking configuration
    ///
    /// Generates profiling code plus Criterion benchmark files. Add
    /// schema-valid benchmark samples with
    /// [`ProfilingConfig::with_criterion_sample`] before treating generated
    /// Criterion output as evidence.
    ///
    /// # Examples
    ///
    /// ```
    /// use tct::profiling_config::ProfilingConfig;
    ///
    /// let config = ProfilingConfig::benchmarking();
    /// assert!(config.generate_criterion);
    /// ```
    pub fn benchmarking() -> Self {
        Self {
            level: ProfilingLevel::Basic,
            feature_gate: "profiling".to_string(),
            generate_criterion: true,
            track_allocations: false,
            criterion_samples: Vec::new(),
        }
    }

    /// Set custom feature gate name
    ///
    /// # Examples
    ///
    /// ```
    /// use tct::profiling_config::ProfilingConfig;
    ///
    /// let config = ProfilingConfig::basic()
    ///     .with_feature_gate("custom-profiling");
    /// assert_eq!(config.feature_gate, "custom-profiling");
    /// ```
    pub fn with_feature_gate<S: Into<String>>(mut self, gate: S) -> Self {
        self.feature_gate = gate.into();
        self
    }

    /// Enable or disable Criterion benchmark generation
    ///
    /// # Examples
    ///
    /// ```
    /// use tct::profiling_config::ProfilingConfig;
    ///
    /// let config = ProfilingConfig::basic()
    ///     .with_criterion(true);
    /// assert!(config.generate_criterion);
    /// ```
    pub fn with_criterion(mut self, enable: bool) -> Self {
        self.generate_criterion = enable;
        self
    }

    /// Add a schema-valid sample for generated Criterion benchmarks
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::json;
    /// use tct::profiling_config::ProfilingConfig;
    ///
    /// let config = ProfilingConfig::benchmarking()
    ///     .with_criterion_sample("red", json!("red"));
    /// assert_eq!(config.criterion_samples.len(), 1);
    /// ```
    pub fn with_criterion_sample<S: Into<String>>(
        mut self,
        name: S,
        value: serde_json::Value,
    ) -> Self {
        self.criterion_samples.push(CriterionBenchmarkSample {
            name: name.into(),
            value,
        });
        self
    }

    /// Enable or disable allocation tracking
    ///
    /// # Examples
    ///
    /// ```
    /// use tct::profiling_config::ProfilingConfig;
    ///
    /// let config = ProfilingConfig::basic()
    ///     .with_allocation_tracking(true);
    /// assert!(config.track_allocations);
    /// ```
    pub fn with_allocation_tracking(mut self, enable: bool) -> Self {
        self.track_allocations = enable;
        self
    }

    /// Check if profiling is disabled
    pub fn is_disabled(&self) -> bool {
        self.level == ProfilingLevel::None
    }

    /// Check if this is basic level profiling
    pub fn is_basic(&self) -> bool {
        self.level == ProfilingLevel::Basic
    }

    /// Check if this is detailed level profiling
    pub fn is_detailed(&self) -> bool {
        self.level == ProfilingLevel::Detailed
    }
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            level: ProfilingLevel::None,
            feature_gate: "profiling".to_string(),
            generate_criterion: false,
            track_allocations: false,
            criterion_samples: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_config() {
        let config = ProfilingConfig::basic();
        assert_eq!(config.level, ProfilingLevel::Basic);
        assert_eq!(config.feature_gate, "profiling");
        assert!(!config.generate_criterion);
        assert!(!config.track_allocations);
        assert!(config.criterion_samples.is_empty());
    }

    #[test]
    fn test_detailed_config() {
        let config = ProfilingConfig::detailed();
        assert_eq!(config.level, ProfilingLevel::Detailed);
        assert!(config.track_allocations);
    }

    #[test]
    fn test_benchmarking_config() {
        let config = ProfilingConfig::benchmarking();
        assert!(config.generate_criterion);
        assert!(config.criterion_samples.is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let config = ProfilingConfig::basic()
            .with_feature_gate("custom")
            .with_criterion(true)
            .with_criterion_sample("small", serde_json::json!("small"))
            .with_allocation_tracking(true);

        assert_eq!(config.feature_gate, "custom");
        assert!(config.generate_criterion);
        assert!(config.track_allocations);
        assert_eq!(config.criterion_samples.len(), 1);
        assert_eq!(config.criterion_samples[0].name, "small");
        assert_eq!(
            config.criterion_samples[0].value,
            serde_json::json!("small")
        );
    }

    #[test]
    fn test_default() {
        let config = ProfilingConfig::default();
        assert_eq!(config.level, ProfilingLevel::None);
        assert!(config.is_disabled());
    }

    #[test]
    fn test_level_checks() {
        let basic = ProfilingConfig::basic();
        assert!(basic.is_basic());
        assert!(!basic.is_detailed());
        assert!(!basic.is_disabled());

        let detailed = ProfilingConfig::detailed();
        assert!(!detailed.is_basic());
        assert!(detailed.is_detailed());
        assert!(!detailed.is_disabled());

        let none = ProfilingConfig::default();
        assert!(!none.is_basic());
        assert!(!none.is_detailed());
        assert!(none.is_disabled());
    }
}
