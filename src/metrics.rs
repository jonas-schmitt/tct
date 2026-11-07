//! Performance and optimization metrics
//!
//! This module provides comprehensive metrics for analyzing the performance
//! characteristics of schema compilation and token optimization.

use rustc_hash::FxHashMap;
use std::time::{Duration, Instant};

/// Metrics for token usage and optimization effectiveness
#[derive(Debug, Clone)]
pub struct TokenMetrics {
    /// Total number of tokens required to encode schema instances
    pub total_tokens: u64,
    /// Number of tokens for structure (objects, arrays, separators)
    pub structure_tokens: u64,
    /// Number of tokens for primitive values (strings, numbers, etc.)
    pub value_tokens: u64,
    /// Number of tokens saved through optimization
    pub tokens_saved: u64,
    /// Estimated unoptimized token count for comparison
    pub unoptimized_tokens: u64,
    /// Breakdown by type for detailed analysis
    pub type_breakdown: FxHashMap<String, u64>,
}

impl TokenMetrics {
    /// Create new empty token metrics
    pub fn new() -> Self {
        Self {
            total_tokens: 0,
            structure_tokens: 0,
            value_tokens: 0,
            tokens_saved: 0,
            unoptimized_tokens: 0,
            type_breakdown: FxHashMap::default(),
        }
    }

    /// Calculate compression ratio as a percentage
    pub fn compression_ratio(&self) -> f64 {
        if self.unoptimized_tokens == 0 {
            0.0
        } else {
            (self.tokens_saved as f64 / self.unoptimized_tokens as f64) * 100.0
        }
    }

    /// Add tokens for a specific type
    pub fn add_tokens(&mut self, type_name: &str, count: u64) {
        self.total_tokens += count;
        *self
            .type_breakdown
            .entry(type_name.to_string())
            .or_insert(0) += count;
    }

    /// Add structure tokens (brackets, separators, etc.)
    pub fn add_structure_tokens(&mut self, count: u64) {
        self.structure_tokens += count;
        self.total_tokens += count;
        self.add_tokens("structure", count);
    }

    /// Add value tokens (actual data content)
    pub fn add_value_tokens(&mut self, count: u64) {
        self.value_tokens += count;
        self.total_tokens += count;
        self.add_tokens("values", count);
    }

    /// Set the baseline unoptimized token count for comparison
    pub fn set_unoptimized_baseline(&mut self, count: u64) {
        self.unoptimized_tokens = count;
        if count > self.total_tokens {
            self.tokens_saved = count - self.total_tokens;
        }
    }

    /// Generate a summary report of token usage
    pub fn summary(&self) -> String {
        format!(
            "Token Metrics:\n\
             - Total tokens: {}\n\
             - Structure: {}\n\
             - Values: {}\n\
             - Compression: {:.1}% ({} tokens saved)\n\
             - Top types: {}",
            self.total_tokens,
            self.structure_tokens,
            self.value_tokens,
            self.compression_ratio(),
            self.tokens_saved,
            self.top_types_summary()
        )
    }

    /// Get summary of top token-consuming types
    fn top_types_summary(&self) -> String {
        let mut types: Vec<_> = self.type_breakdown.iter().collect();
        types.sort_by(|a, b| b.1.cmp(a.1));
        types
            .iter()
            .take(3)
            .map(|(name, count)| format!("{}: {}", name, count))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Default for TokenMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics for compilation timing and performance
#[derive(Debug, Clone)]
pub struct TimingMetrics {
    /// Time spent parsing the JSON schema
    pub schema_parsing: Duration,
    /// Time spent analyzing schema structure and constraints
    pub schema_analysis: Duration,
    /// Time spent generating Rust code
    pub code_generation: Duration,
    /// Time spent on optimization passes
    pub optimization: Duration,
    /// Total compilation time
    pub total_time: Duration,
    /// Detailed timing for individual phases
    pub phase_timings: FxHashMap<String, Duration>,
}

impl TimingMetrics {
    /// Create new empty timing metrics
    pub fn new() -> Self {
        Self {
            schema_parsing: Duration::ZERO,
            schema_analysis: Duration::ZERO,
            code_generation: Duration::ZERO,
            optimization: Duration::ZERO,
            total_time: Duration::ZERO,
            phase_timings: FxHashMap::default(),
        }
    }

    /// Record timing for a specific phase
    pub fn record_phase(&mut self, phase: &str, duration: Duration) {
        self.phase_timings.insert(phase.to_string(), duration);
        self.total_time += duration;
    }

    /// Set main phase timings
    pub fn set_main_phases(
        &mut self,
        parsing: Duration,
        analysis: Duration,
        generation: Duration,
        optimization: Duration,
    ) {
        self.schema_parsing = parsing;
        self.schema_analysis = analysis;
        self.code_generation = generation;
        self.optimization = optimization;
        self.total_time = parsing + analysis + generation + optimization;
    }

    /// Generate a summary report of compilation timing
    pub fn summary(&self) -> String {
        format!(
            "Timing Metrics:\n\
             - Total: {:.2}ms\n\
             - Parsing: {:.2}ms ({:.1}%)\n\
             - Analysis: {:.2}ms ({:.1}%)\n\
             - Generation: {:.2}ms ({:.1}%)\n\
             - Optimization: {:.2}ms ({:.1}%)",
            self.total_time.as_secs_f64() * 1000.0,
            self.schema_parsing.as_secs_f64() * 1000.0,
            self.percentage_of_total(self.schema_parsing),
            self.schema_analysis.as_secs_f64() * 1000.0,
            self.percentage_of_total(self.schema_analysis),
            self.code_generation.as_secs_f64() * 1000.0,
            self.percentage_of_total(self.code_generation),
            self.optimization.as_secs_f64() * 1000.0,
            self.percentage_of_total(self.optimization)
        )
    }

    fn percentage_of_total(&self, duration: Duration) -> f64 {
        if self.total_time == Duration::ZERO {
            0.0
        } else {
            (duration.as_secs_f64() / self.total_time.as_secs_f64()) * 100.0
        }
    }
}

impl Default for TimingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer helper for measuring execution time
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Start a new timer
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Stop the timer and return elapsed duration
    pub fn stop(self) -> Duration {
        self.start.elapsed()
    }
}

/// Combined performance metrics for schema compilation
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Token usage and optimization metrics
    pub tokens: TokenMetrics,
    /// Compilation timing metrics
    pub timing: TimingMetrics,
    /// Schema complexity indicators
    pub complexity: ComplexityMetrics,
}

impl PerformanceMetrics {
    /// Create new empty performance metrics
    pub fn new() -> Self {
        Self {
            tokens: TokenMetrics::new(),
            timing: TimingMetrics::new(),
            complexity: ComplexityMetrics::new(),
        }
    }

    /// Generate a comprehensive performance report
    pub fn comprehensive_report(&self) -> String {
        format!(
            "{}\n\n{}\n\n{}",
            self.tokens.summary(),
            self.timing.summary(),
            self.complexity.summary()
        )
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics for schema complexity analysis
#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    /// Number of object types in schema
    pub object_count: u32,
    /// Number of enum types in schema
    pub enum_count: u32,
    /// Maximum nesting depth
    pub max_depth: u32,
    /// Number of constraints (min, max, pattern, etc.)
    pub constraint_count: u32,
    /// Number of additionalProperties usages
    pub additional_properties_count: u32,
    /// Number of unconstrained additionalProperties
    pub unconstrained_additional_properties: u32,
}

impl ComplexityMetrics {
    /// Create new empty complexity metrics
    pub fn new() -> Self {
        Self {
            object_count: 0,
            enum_count: 0,
            max_depth: 0,
            constraint_count: 0,
            additional_properties_count: 0,
            unconstrained_additional_properties: 0,
        }
    }

    /// Calculate complexity score (arbitrary units for comparison)
    pub fn complexity_score(&self) -> f64 {
        // Weight different factors for overall complexity
        (self.object_count as f64 * 2.0)
            + (self.enum_count as f64 * 1.5)
            + (self.max_depth as f64 * 3.0)
            + (self.constraint_count as f64 * 1.0)
            + (self.additional_properties_count as f64 * 2.5)
            + (self.unconstrained_additional_properties as f64 * 4.0) // Higher weight for optimization challenges
    }

    /// Generate a summary report of schema complexity
    pub fn summary(&self) -> String {
        format!(
            "Complexity Metrics:\n\
             - Complexity score: {:.1}\n\
             - Objects: {}, Enums: {}\n\
             - Max depth: {}\n\
             - Constraints: {}\n\
             - Additional properties: {} ({} unconstrained)",
            self.complexity_score(),
            self.object_count,
            self.enum_count,
            self.max_depth,
            self.constraint_count,
            self.additional_properties_count,
            self.unconstrained_additional_properties
        )
    }
}

impl Default for ComplexityMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_metrics_compression_ratio() {
        let mut metrics = TokenMetrics::new();
        metrics.total_tokens = 80;
        metrics.set_unoptimized_baseline(100);

        assert_eq!(metrics.compression_ratio(), 20.0);
        assert_eq!(metrics.tokens_saved, 20);
    }

    #[test]
    fn test_token_metrics_type_breakdown() {
        let mut metrics = TokenMetrics::new();
        metrics.add_tokens("string", 10);
        metrics.add_tokens("number", 5);
        metrics.add_tokens("string", 15);

        assert_eq!(metrics.total_tokens, 30);
        assert_eq!(metrics.type_breakdown["string"], 25);
        assert_eq!(metrics.type_breakdown["number"], 5);
    }

    #[test]
    fn test_timing_metrics_percentage() {
        let mut metrics = TimingMetrics::new();
        metrics.set_main_phases(
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
        );

        assert_eq!(metrics.total_time, Duration::from_millis(100));
        assert_eq!(metrics.percentage_of_total(Duration::from_millis(20)), 20.0);
    }

    #[test]
    fn test_complexity_score_calculation() {
        let mut metrics = ComplexityMetrics::new();
        metrics.object_count = 2;
        metrics.enum_count = 1;
        metrics.max_depth = 3;
        metrics.constraint_count = 5;
        metrics.unconstrained_additional_properties = 1;

        // 2*2.0 + 1*1.5 + 3*3.0 + 5*1.0 + 1*4.0 = 4 + 1.5 + 9 + 5 + 4 = 23.5
        assert_eq!(metrics.complexity_score(), 23.5);
    }

    #[test]
    fn test_timer_measures_time() {
        let timer = Timer::start();
        std::thread::sleep(Duration::from_millis(1));
        let elapsed = timer.stop();

        println!("Timer measured elapsed duration: {:?}", elapsed);
    }
}
