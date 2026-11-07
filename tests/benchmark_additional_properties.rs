//! Benchmark Suite for AdditionalProperties Optimization
//!
//! This test suite provides comprehensive performance comparison between
//! optimization levels (O0, O2, O3) for schemas with additionalProperties.
//! It validates the effectiveness of our optimization strategies.

use serde_json::json;
use std::time::Instant;
use tct::{OptimizationLevel, TctCompiler};

/// Benchmark result for a single schema compilation
#[derive(Debug, Clone)]
struct BenchmarkResult {
    optimization_level: OptimizationLevel,
    compilation_time_ms: u64,
    token_count: Option<usize>,
    vocabulary_reduction: Option<f64>,
    warnings_count: usize,
    success: bool,
    _error_message: Option<String>,
}

/// Comprehensive benchmark suite for additionalProperties optimization
#[derive(Debug)]
struct AdditionalPropertiesBenchmark {
    schema_name: String,
    schema: serde_json::Value,
    results: Vec<BenchmarkResult>,
}

impl AdditionalPropertiesBenchmark {
    fn new(name: &str, schema: serde_json::Value) -> Self {
        Self {
            schema_name: name.to_string(),
            schema,
            results: Vec::new(),
        }
    }

    /// Run benchmark across all optimization levels
    fn run_benchmark(&mut self) {
        let levels = vec![OptimizationLevel::O0, OptimizationLevel::O2];

        for level in levels {
            let result = self.benchmark_single_level(level);
            self.results.push(result);
        }
    }

    /// Benchmark a single optimization level
    fn benchmark_single_level(&self, level: OptimizationLevel) -> BenchmarkResult {
        let start = Instant::now();

        let compiler_result = TctCompiler::new(self.schema.clone(), &self.schema_name)
            .and_then(|compiler| compiler.with_optimization_level(level).compile());

        let compilation_time_ms = start.elapsed().as_millis() as u64;

        match compiler_result {
            Ok(grammar) => {
                // Extract performance metrics if available
                let token_count = grammar
                    .performance_metrics
                    .as_ref()
                    .map(|metrics| metrics.tokens.total_tokens as usize);

                let vocabulary_reduction = grammar
                    .vocabulary_stats
                    .as_ref()
                    .map(|stats| stats.reduction_percentage);

                let warnings_count = grammar
                    .warnings
                    .as_ref()
                    .map(|w| w.warnings.len())
                    .unwrap_or(0);

                BenchmarkResult {
                    optimization_level: level,
                    compilation_time_ms,
                    token_count,
                    vocabulary_reduction,
                    warnings_count,
                    success: true,
                    _error_message: None,
                }
            }
            Err(error) => BenchmarkResult {
                optimization_level: level,
                compilation_time_ms,
                token_count: None,
                vocabulary_reduction: None,
                warnings_count: 0,
                success: false,
                _error_message: Some(error.to_string()),
            },
        }
    }

    /// Generate a performance report
    fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!(
            "\n=== Benchmark Report: {} ===\n",
            self.schema_name
        ));

        // Header
        report.push_str("Level | Time(ms) | Tokens | Vocab Reduction | Warnings | Status\n");
        report.push_str("------|----------|--------|-----------------|----------|--------\n");

        // Results
        for result in &self.results {
            let level_str = format!("{:?}", result.optimization_level);
            let time_str = result.compilation_time_ms.to_string();
            let tokens_str = result
                .token_count
                .map(|t| t.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let vocab_str = result
                .vocabulary_reduction
                .map(|v| format!("{:.1}%", v))
                .unwrap_or_else(|| "N/A".to_string());
            let warnings_str = result.warnings_count.to_string();
            let status_str = if result.success { "✅" } else { "❌" };

            report.push_str(&format!(
                "{:5} | {:8} | {:6} | {:15} | {:8} | {}\n",
                level_str, time_str, tokens_str, vocab_str, warnings_str, status_str
            ));
        }

        // Analysis
        if self.results.len() >= 2 {
            let o0_result = self
                .results
                .iter()
                .find(|r| matches!(r.optimization_level, OptimizationLevel::O0));
            let o2_result = self
                .results
                .iter()
                .find(|r| matches!(r.optimization_level, OptimizationLevel::O2));

            if let (Some(o0), Some(o2)) = (o0_result, o2_result) {
                report.push_str("\n--- Performance Analysis ---\n");

                // Compilation time comparison
                let time_speedup =
                    o0.compilation_time_ms as f64 / o2.compilation_time_ms.max(1) as f64;
                report.push_str(&format!(
                    "Compilation speedup (O2/O0): {:.2}x\n",
                    time_speedup
                ));

                // Token optimization comparison
                if let (Some(o0_tokens), Some(o2_tokens)) = (o0.token_count, o2.token_count) {
                    let token_reduction =
                        (o0_tokens as f64 - o2_tokens as f64) / o0_tokens as f64 * 100.0;
                    report.push_str(&format!(
                        "Token reduction (O0→O2): {:.1}%\n",
                        token_reduction
                    ));
                }

                // Vocabulary optimization
                if let Some(vocab_reduction) = o2.vocabulary_reduction {
                    report.push_str(&format!(
                        "Vocabulary optimization: {:.1}%\n",
                        vocab_reduction
                    ));
                }
            }
        }

        report
    }
}

/// Test basic additionalProperties: true optimization
#[test]
fn test_benchmark_additional_properties_true_basic() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "version": {"type": "string"}
        },
        "additionalProperties": true
    });

    let mut benchmark = AdditionalPropertiesBenchmark::new("basic_additional_true", schema);
    benchmark.run_benchmark();

    let report = benchmark.generate_report();
    println!("{}", report);

    // Validation: Both O0 and O2 should succeed
    assert!(
        benchmark.results.iter().all(|r| r.success),
        "All optimization levels should compile successfully"
    );

    // Warning generation is now implemented - both O0 and O2 should generate warnings about unconstrained additionalProperties
    let o0_result = benchmark
        .results
        .iter()
        .find(|r| matches!(r.optimization_level, OptimizationLevel::O0))
        .expect("O0 result should exist");
    let o2_result = benchmark
        .results
        .iter()
        .find(|r| matches!(r.optimization_level, OptimizationLevel::O2))
        .expect("O2 result should exist");

    // Both optimization levels should warn about unconstrained additionalProperties: true
    assert_eq!(
        o0_result.warnings_count, 1,
        "O0 should generate 1 warning for unconstrained additionalProperties: true"
    );
    assert_eq!(
        o2_result.warnings_count, 1,
        "O2 should generate 1 warning for unconstrained additionalProperties: true"
    );
}

/// Test additionalProperties: false strict validation
#[test]
fn test_benchmark_additional_properties_false_strict() {
    let schema = json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer"},
            "status": {"enum": ["active", "inactive"]}
        },
        "additionalProperties": false
    });

    let mut benchmark = AdditionalPropertiesBenchmark::new("strict_additional_false", schema);
    benchmark.run_benchmark();

    let report = benchmark.generate_report();
    println!("{}", report);

    // Validation: All levels should succeed, O2 should have fewer warnings
    assert!(benchmark.results.iter().all(|r| r.success));

    let o2_result = benchmark
        .results
        .iter()
        .find(|r| matches!(r.optimization_level, OptimizationLevel::O2))
        .expect("O2 result should exist");

    // O2 should have no warnings for strict additionalProperties: false
    assert_eq!(
        o2_result.warnings_count, 0,
        "O2 should not warn about additionalProperties: false"
    );
}

/// Test additionalProperties with typed schema
#[test]
fn test_benchmark_additional_properties_typed_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "config": {"type": "object"}
        },
        "additionalProperties": {
            "type": "string",
            "enum": ["debug", "info", "warn", "error"]
        }
    });

    let mut benchmark = AdditionalPropertiesBenchmark::new("typed_additional_schema", schema);
    benchmark.run_benchmark();

    let report = benchmark.generate_report();
    println!("{}", report);

    // Validation: All should succeed, O2 should optimize the enum in additionalProperties
    assert!(benchmark.results.iter().all(|r| r.success));

    let o2_result = benchmark
        .results
        .iter()
        .find(|r| matches!(r.optimization_level, OptimizationLevel::O2))
        .expect("O2 result should exist");

    // Should have minimal warnings since additionalProperties has constraints
    assert!(
        o2_result.warnings_count <= 1,
        "O2 should have minimal warnings for constrained additionalProperties"
    );
}

/// Test complex nested additionalProperties
#[test]
fn test_benchmark_additional_properties_complex_nested() {
    let schema = json!({
        "type": "object",
        "properties": {
            "metadata": {
                "type": "object",
                "properties": {
                    "version": {"type": "string"}
                },
                "additionalProperties": {
                    "type": "object",
                    "properties": {
                        "type": {"enum": ["config", "data", "temp"]},
                        "size": {"type": "integer", "minimum": 0, "maximum": 1000}
                    },
                    "additionalProperties": true
                }
            }
        },
        "additionalProperties": false
    });

    let mut benchmark = AdditionalPropertiesBenchmark::new("complex_nested_additional", schema);
    benchmark.run_benchmark();

    let report = benchmark.generate_report();
    println!("{}", report);

    // Validation: Complex schemas should compile successfully
    assert!(benchmark.results.iter().all(|r| r.success));

    // O2 should optimize the enum and bounded integer in nested additionalProperties
    let o2_result = benchmark
        .results
        .iter()
        .find(|r| matches!(r.optimization_level, OptimizationLevel::O2))
        .expect("O2 result should exist");

    // Nested constrained additionalProperties currently emits no warnings.
    // Keep this stable unless warning policy starts reporting nested
    // optimization opportunities.
    assert_eq!(
        o2_result.warnings_count, 0,
        "Nested constrained additionalProperties warning policy changed"
    );
}

/// Test NPM package.json-style additionalProperties (real-world scenario)
#[test]
fn test_benchmark_additional_properties_npm_package_json() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "version": {"type": "string"},
            "description": {"type": "string"},
            "main": {"type": "string"},
            "scripts": {
                "type": "object",
                "additionalProperties": {"type": "string"}
            },
            "dependencies": {
                "type": "object",
                "additionalProperties": {"type": "string"}
            },
            "devDependencies": {
                "type": "object",
                "additionalProperties": {"type": "string"}
            }
        },
        "additionalProperties": true
    });

    let mut benchmark = AdditionalPropertiesBenchmark::new("npm_package_json", schema);
    benchmark.run_benchmark();

    let report = benchmark.generate_report();
    println!("{}", report);

    // Validation: Real-world schemas should compile successfully
    assert!(benchmark.results.iter().all(|r| r.success));

    let o0_time = benchmark
        .results
        .iter()
        .find(|r| matches!(r.optimization_level, OptimizationLevel::O0))
        .map(|r| r.compilation_time_ms)
        .expect("O0 benchmark result should exist");

    let o2_time = benchmark
        .results
        .iter()
        .find(|r| matches!(r.optimization_level, OptimizationLevel::O2))
        .map(|r| r.compilation_time_ms)
        .expect("O2 benchmark result should exist");

    println!(
        "O0 compilation: {}ms; O2 compilation: {}ms",
        o0_time, o2_time
    );
}

/// Test performance with varying additionalProperties complexity
#[test]
fn test_benchmark_additional_properties_complexity_scaling() {
    let test_cases = vec![
        // Simple: just boolean
        (
            "simple",
            json!({
                "type": "object",
                "properties": {"id": {"type": "integer"}},
                "additionalProperties": true
            }),
        ),
        // Medium: typed additionalProperties
        (
            "medium",
            json!({
                "type": "object",
                "properties": {"id": {"type": "integer"}},
                "additionalProperties": {"type": "string", "maxLength": 100}
            }),
        ),
        // Complex: nested object additionalProperties
        (
            "complex",
            json!({
                "type": "object",
                "properties": {"id": {"type": "integer"}},
                "additionalProperties": {
                    "type": "object",
                    "properties": {
                        "status": {"enum": ["on", "off"]},
                        "value": {"type": "number", "minimum": 0, "maximum": 100}
                    },
                    "additionalProperties": {"type": "string"}
                }
            }),
        ),
    ];

    println!("\n=== Complexity Scaling Analysis ===");

    for (name, schema) in test_cases {
        let mut benchmark = AdditionalPropertiesBenchmark::new(name, schema);
        benchmark.run_benchmark();

        let report = benchmark.generate_report();
        println!("{}", report);

        // All complexity levels should compile successfully
        assert!(
            benchmark.results.iter().all(|r| r.success),
            "Schema '{}' should compile at all optimization levels",
            name
        );
    }
}

/// Comprehensive optimization effectiveness validation
#[test]
fn test_benchmark_optimization_effectiveness_validation() {
    let schemas = vec![
        (
            "api_response",
            json!({
                "type": "object",
                "properties": {
                    "status": {"enum": ["success", "error", "pending"]},
                    "code": {"type": "integer", "minimum": 100, "maximum": 599},
                    "data": {
                        "type": "object",
                        "additionalProperties": true
                    }
                },
                "additionalProperties": false
            }),
        ),
        (
            "config_file",
            json!({
                "type": "object",
                "properties": {
                    "environment": {"enum": ["dev", "staging", "prod"]},
                    "database": {
                        "type": "object",
                        "properties": {
                            "host": {"type": "string"},
                            "port": {"type": "integer", "minimum": 1, "maximum": 65535}
                        },
                        "additionalProperties": {"type": "string"}
                    }
                },
                "additionalProperties": {"type": "string"}
            }),
        ),
    ];

    println!("\n=== Optimization Effectiveness Summary ===");

    for (name, schema) in schemas {
        let mut benchmark = AdditionalPropertiesBenchmark::new(name, schema);
        benchmark.run_benchmark();

        println!("{}", benchmark.generate_report());

        // Core validation: all optimizations should succeed
        assert!(
            benchmark.results.iter().all(|r| r.success),
            "All optimizations for '{}' should succeed",
            name
        );

        // Performance validation: O2 should provide optimization benefits
        let o2_result = benchmark
            .results
            .iter()
            .find(|r| matches!(r.optimization_level, OptimizationLevel::O2))
            .expect("O2 result should exist");

        // O2 should either reduce vocabulary usage or provide warnings about optimizability
        assert!(
            o2_result.vocabulary_reduction.is_some() || o2_result.warnings_count > 0,
            "O2 should either optimize vocabulary or warn about optimization opportunities"
        );
    }
}
