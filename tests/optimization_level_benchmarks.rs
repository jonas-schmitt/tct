// Legacy optimization level benchmark smoke tests - simplified interface using modules.
// Use tct-measure-tokens for maintained generated-crate measurements.
// Set TCT_BENCHMARK=1 environment variable to run (very slow - compiles multiple Rust crates)

mod optimization_benchmark_schemas;
mod optimization_benchmark_utils;

use optimization_benchmark_schemas::get_all_benchmark_test_cases;
use optimization_benchmark_utils::{
    benchmark_schema_with_level, generate_optimization_comparison_report,
};
use tct::OptimizationLevel;

/// Legacy benchmark smoke suite comparing optimization levels (-O0, -O2, -O3).
///
/// This test compiles and runs multiple Rust projects, which is very slow.
/// Set TCT_BENCHMARK=1 environment variable to run these tests.
#[test]
fn test_optimization_benchmarks_optimization_level_benchmarks() {
    // Skip benchmark tests unless explicitly requested
    if std::env::var("TCT_BENCHMARK").unwrap_or_default() != "1" {
        println!("Skipping benchmark tests. Set TCT_BENCHMARK=1 to run.");
        return;
    }

    let test_cases = get_all_benchmark_test_cases();

    let optimization_levels = [
        (OptimizationLevel::O0, "O0"),
        (OptimizationLevel::O2, "O2"),
        (OptimizationLevel::O3, "O3"),
    ];

    println!("\n=== TCT Legacy Optimization Level Benchmark Smoke Results ===");
    println!(
        "Measurement scope: legacy temp crates with local tokenizer primitives; use tct-measure-tokens for maintained generated-crate results."
    );
    println!(
        "{:<12} {:<6} {:<12} {:<12} {:<12} {:<12} {:<15} {:<15}",
        "Schema",
        "Level",
        "JSON Bytes",
        "TCT Tokens",
        "TCT Bytes",
        "Ratio",
        "Encode (μs)",
        "Decode (μs)"
    );
    println!("{:-<110}", "");

    let mut all_results = Vec::new();

    for (schema_name, schema, test_data) in test_cases.iter() {
        for (opt_level, opt_name) in optimization_levels.iter() {
            println!(
                "Running legacy benchmark smoke: {} with optimization level {}",
                schema_name, opt_name
            );

            let results = benchmark_schema_with_level(schema, test_data, schema_name, *opt_level);

            println!(
                "{:<12} {:<6} {:<12} {:<12} {:<12} {:<12.3} {:<15} {:<15}",
                schema_name,
                opt_name,
                results.json_bytes,
                results.token_count,
                results.token_bytes,
                results.compression_ratio,
                results.encode_time_us,
                results.decode_time_us
            );

            all_results.push((schema_name.to_string(), opt_name.to_string(), results));
        }
    }

    println!("{:-<110}", "");

    // Generate comprehensive analysis report
    generate_optimization_comparison_report(&all_results);

    // Basic validation that benchmarks completed successfully
    assert!(!all_results.is_empty(), "No benchmark results generated");

    // Validate that all optimization levels were tested
    let levels_tested: std::collections::HashSet<String> = all_results
        .iter()
        .map(|(_, level, _)| level.clone())
        .collect();
    assert!(
        levels_tested.contains("O0"),
        "O0 optimization level not tested"
    );
    assert!(
        levels_tested.contains("O2"),
        "O2 optimization level not tested"
    );
    assert!(
        levels_tested.contains("O3"),
        "O3 optimization level not tested"
    );

    // Validate that all test cases were executed
    let schemas_tested: std::collections::HashSet<String> = all_results
        .iter()
        .map(|(schema, _, _)| schema.clone())
        .collect();
    assert!(
        schemas_tested.contains("Simple"),
        "Simple schema not tested"
    );
    assert!(
        schemas_tested.contains("Medium"),
        "Medium schema not tested"
    );
    assert!(
        schemas_tested.contains("Complex"),
        "Complex schema not tested"
    );
    assert!(
        schemas_tested.contains("EnumHeavy"),
        "EnumHeavy schema not tested"
    );

    println!("\n✅ All legacy optimization level benchmark smokes completed successfully!");
}
