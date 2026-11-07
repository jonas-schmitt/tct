//! Profiling Execution Verification Tests
//!
//! This module verifies that generated profiling code actually works during tokenizer execution,
//! not just that it compiles. We create temporary Cargo projects with profiling enabled,
//! execute them, and verify that timing telemetry is emitted without asserting local scheduler
//! speed or treating the values as benchmark evidence.

use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::{ProfilingConfig, TctCompiler};

fn generated_profiling_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

#[test]
fn test_profiling_execution_simple_enum() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing profiling execution with simple enum schema");

    // Simple enum schema for reliable testing
    let schema = json!({
        "type": "string",
        "enum": ["red", "green", "blue"]
    });

    let test_instance = json!("green");

    // Generate code with basic profiling enabled
    let compiler = TctCompiler::new(schema, "Color")?.with_profiling(ProfilingConfig::basic());

    let grammar = compiler.compile()?;
    let generated_code = grammar.generate_code()?;

    // Verify profiling code was generated
    assert!(
        generated_code.contains("TimedEncode"),
        "Generated code should contain TimedEncode trait"
    );
    assert!(
        generated_code.contains("encode_with_timing"),
        "Generated code should contain encode_with_timing"
    );
    assert!(
        generated_code.contains("decode_with_timing"),
        "Generated code should contain decode_with_timing"
    );
    assert!(
        generated_code.contains("quick_benchmark"),
        "Generated code should contain quick_benchmark"
    );

    println!("✅ Profiling code generation verified");

    // Create and execute profiling test project
    let test_result = create_and_execute_profiling_test(
        &generated_code,
        &test_instance,
        "color_enum_profiling_test",
        true, // enable profiling feature
    )?;

    // Verify execution results
    assert!(
        test_result.execution_succeeded,
        "Profiling test execution should succeed"
    );
    assert!(
        test_result.encode_time_ns.is_some(),
        "Should capture encode timing"
    );
    assert!(
        test_result.decode_time_ns.is_some(),
        "Should capture decode timing"
    );
    assert!(test_result.round_trip_valid, "Round-trip should be valid");

    // Keep timing as telemetry. Stable tests should validate profiling
    // availability and round-trip behavior, not local scheduler speed.
    if let Some(encode_ns) = test_result.encode_time_ns {
        println!("✅ Encode timing: {}ns ({}µs)", encode_ns, encode_ns / 1000);
    }

    if let Some(decode_ns) = test_result.decode_time_ns {
        println!("✅ Decode timing: {}ns ({}µs)", decode_ns, decode_ns / 1000);
    }

    // Verify benchmark execution if present
    if let Some(benchmark_result) = test_result.benchmark_result {
        assert!(
            benchmark_result.iterations > 0,
            "Benchmark should run multiple iterations"
        );
        assert!(
            benchmark_result.success_rate >= 0.9,
            "Benchmark success rate should be at least 90%"
        );
        println!(
            "✅ Benchmark execution: {} iterations, {:.1}% success rate, avg encode {}ns",
            benchmark_result.iterations,
            benchmark_result.success_rate * 100.0,
            benchmark_result.avg_encode_ns
        );
    }

    println!("🎉 Profiling execution verification passed!");
    Ok(())
}

#[test]
fn test_profiling_feature_gate_behavior() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing profiling feature gate behavior");

    // Note: Use "Choice" instead of "Option" to avoid naming conflict with std::option::Option
    let schema = json!({
        "type": "string",
        "enum": ["option1", "option2"]
    });

    let test_instance = json!("option1");

    // Generate code with profiling
    let compiler = TctCompiler::new(schema, "Choice")?.with_profiling(ProfilingConfig::basic());

    let grammar = compiler.compile()?;
    let generated_code = grammar.generate_code()?;

    // Test 1: With profiling feature enabled - should work
    let with_profiling = create_and_execute_profiling_test(
        &generated_code,
        &test_instance,
        "option_profiling_enabled",
        true, // enable profiling feature
    )?;

    assert!(
        with_profiling.execution_succeeded,
        "Execution with profiling feature should succeed"
    );
    assert!(
        with_profiling.profiling_available,
        "Profiling should be available when feature enabled"
    );
    println!("✅ Profiling feature enabled: execution succeeded");

    // Test 2: Without profiling feature - should compile but profiling unavailable
    let without_profiling = create_and_execute_profiling_test(
        &generated_code,
        &test_instance,
        "option_profiling_disabled",
        false, // disable profiling feature
    )?;

    assert!(
        without_profiling.execution_succeeded,
        "Execution without profiling feature should still succeed"
    );
    assert!(
        !without_profiling.profiling_available,
        "Profiling should be unavailable when feature disabled"
    );
    println!("✅ Profiling feature disabled: execution succeeded, profiling unavailable");

    println!("🎉 Feature gate behavior verification passed!");
    Ok(())
}

#[test]
fn test_quick_benchmark_execution() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing quick_benchmark execution");

    let schema = json!({
        "type": "string",
        "enum": ["pending", "active", "completed", "cancelled"]
    });

    let test_instance = json!("active");

    // Generate code with profiling
    let compiler = TctCompiler::new(schema, "Status")?.with_profiling(ProfilingConfig::basic());

    let grammar = compiler.compile()?;
    let generated_code = grammar.generate_code()?;

    // Create test that specifically exercises quick_benchmark
    let test_result = create_and_execute_benchmark_test(
        &generated_code,
        &test_instance,
        "benchmark_test",
        50, // run 50 benchmark iterations
    )?;

    assert!(
        test_result.execution_succeeded,
        "Benchmark test should execute successfully"
    );

    if let Some(benchmark) = test_result.benchmark_result {
        assert_eq!(benchmark.iterations, 50, "Should run exactly 50 iterations");
        assert!(
            benchmark.success_rate >= 0.95,
            "Success rate should be at least 95% (was {:.1}%)",
            benchmark.success_rate * 100.0
        );
        if benchmark.iterations >= 10 {
            let encode_cv = calculate_coefficient_of_variation(&benchmark.encode_times_ns);
            let decode_cv = calculate_coefficient_of_variation(&benchmark.decode_times_ns);

            println!(
                "✅ Timing consistency: encode CV={:.2}, decode CV={:.2}",
                encode_cv, decode_cv
            );
        }

        println!(
            "✅ Benchmark completed: {} iterations, {:.1}% success",
            benchmark.iterations,
            benchmark.success_rate * 100.0
        );
        println!(
            "   Average encode: {}ns, decode: {}ns",
            benchmark.avg_encode_ns, benchmark.avg_decode_ns
        );
    } else {
        return Err("Benchmark result missing from test output".into());
    }

    println!("🎉 Quick benchmark execution verification passed!");
    Ok(())
}

/// Results from executing a profiling test
#[derive(Debug)]
struct ProfilingTestResult {
    execution_succeeded: bool,
    profiling_available: bool,
    encode_time_ns: Option<u64>,
    decode_time_ns: Option<u64>,
    round_trip_valid: bool,
    benchmark_result: Option<BenchmarkTestResult>,
    _compilation_output: String,
    _execution_output: String,
}

/// Results from benchmark execution
#[derive(Debug)]
struct BenchmarkTestResult {
    iterations: usize,
    success_rate: f64,
    avg_encode_ns: u64,
    avg_decode_ns: u64,
    encode_times_ns: Vec<u64>,
    decode_times_ns: Vec<u64>,
}

/// Create and execute a profiling test project
fn create_and_execute_profiling_test(
    generated_code: &str,
    test_instance: &serde_json::Value,
    test_name: &str,
    enable_profiling_feature: bool,
) -> Result<ProfilingTestResult, Box<dyn std::error::Error>> {
    let test_dir = format!("/tmp/profiling_execution_test_{}", test_name);
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir)?;
    fs::create_dir_all(format!("{}/src", test_dir))?;

    // Create Cargo.toml with profiling feature and rustc-hash dependency
    let cargo_toml = GeneratedCargoToml::new("profiling-execution-test")
        .with_rustc_hash()
        .with_profiling_feature()
        .render();

    fs::write(format!("{}/Cargo.toml", test_dir), cargo_toml)?;
    let tct_path = std::env::current_dir()?;
    copy_workspace_cargo_lock(&tct_path, Path::new(&test_dir))?;

    // Write test instance to file
    fs::write(
        format!("{}/test_instance.json", test_dir),
        serde_json::to_string_pretty(test_instance)?,
    )?;

    // Create main.rs that exercises profiling functionality
    let main_rs = format!(
        r#"
use serde::{{Deserialize, Serialize}};
use serde_json;

// TCT-generated code with profiling (includes tokenizer module)
{generated_code}

// Helper to find the main type - will be replaced by build script
// MAIN_TYPE_PLACEHOLDER

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let test_json = include_str!("../test_instance.json");

    // Parse test instance
    let instance: TestType = serde_json::from_str(test_json)?;
    println!("EXEC:instance_parsed:true");

    // Check if profiling is available (feature-gated)
    #[cfg(feature = "profiling")]
    {{
        println!("EXEC:profiling_available:true");

        // Test encode_with_timing
        let (tokens, encode_duration) = {{
            use profiling::TimedEncode;
            instance.encode_with_timing()
        }};
        println!("EXEC:encode_time_ns:{{}}", encode_duration.as_nanos());
        println!("EXEC:token_count:{{}}", tokens.len());

        // Test decode_with_timing
        let mut token_iter = tokens.into_iter();
        let (decoded, decode_duration) = {{
            use profiling::TimedEncode;
            TestType::decode_with_timing(&mut token_iter)?
        }};
        println!("EXEC:decode_time_ns:{{}}", decode_duration.as_nanos());

        // Verify round-trip
        let round_trip_valid = instance == decoded;
        println!("EXEC:round_trip_valid:{{}}", round_trip_valid);

        // Test quick_benchmark (small number for speed)
        let benchmark_results = {{
            use profiling::quick_benchmark;
            quick_benchmark(&instance, 10)
        }};

        println!("EXEC:benchmark_iterations:{{}}", benchmark_results.iterations);
        println!("EXEC:benchmark_success_rate:{{}}", benchmark_results.success_rate());
        println!("EXEC:benchmark_avg_encode_ns:{{}}", benchmark_results.avg_encode_time().as_nanos());
        println!("EXEC:benchmark_avg_decode_ns:{{}}", benchmark_results.avg_decode_time().as_nanos());
    }}

    #[cfg(not(feature = "profiling"))]
    {{
        use tokenizer::JsonTokenEncoder;

        println!("EXEC:profiling_available:false");

        // Basic tokenization should still work
        let mut tokens = Vec::new();
        instance.encode(&mut tokens);
        println!("EXEC:basic_encode_tokens:{{}}", tokens.len());

        // Basic decode should work
        let mut token_iter = tokens.into_iter();
        let decoded = TestType::decode(&mut token_iter)?;
        let round_trip_valid = instance == decoded;
        println!("EXEC:round_trip_valid:{{}}", round_trip_valid);
    }}

    println!("EXEC:execution_succeeded:true");
    Ok(())
}}
"#
    );

    // Create initial main.rs
    fs::write(format!("{}/src/main.rs", test_dir), main_rs)?;

    // Extract the main type name from generated code
    let main_type_name = extract_main_type_name(generated_code);

    // Replace the placeholder with the actual type alias
    let main_rs_content = fs::read_to_string(format!("{}/src/main.rs", test_dir))?;
    let updated_main_rs = main_rs_content.replace(
        "// MAIN_TYPE_PLACEHOLDER",
        &format!("type TestType = {};", main_type_name),
    );
    fs::write(format!("{}/src/main.rs", test_dir), updated_main_rs)?;

    // Compile the project
    let mut compile_cmd = generated_profiling_cargo_command();
    compile_cmd
        .current_dir(&test_dir)
        .args(["build", "--release"]);

    if enable_profiling_feature {
        compile_cmd.arg("--features").arg("profiling");
    }

    let compile_output = compile_cmd.output()?;
    let compilation_output = String::from_utf8_lossy(&compile_output.stderr).to_string();

    if !compile_output.status.success() {
        return Err(format!("Compilation failed: {}", compilation_output).into());
    }

    // Execute the compiled binary
    let execute_output = generated_profiling_cargo_command()
        .current_dir(&test_dir)
        .args(["run", "--release"])
        .args(if enable_profiling_feature {
            vec!["--features", "profiling"]
        } else {
            vec![]
        })
        .output()?;

    let execution_output = String::from_utf8_lossy(&execute_output.stdout).to_string();
    let execution_succeeded = execute_output.status.success();

    // Parse execution results
    let result = parse_profiling_execution_output(
        &execution_output,
        execution_succeeded,
        &compilation_output,
    );

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    Ok(result)
}

/// Create and execute a benchmark-focused test
fn create_and_execute_benchmark_test(
    generated_code: &str,
    test_instance: &serde_json::Value,
    test_name: &str,
    benchmark_iterations: usize,
) -> Result<ProfilingTestResult, Box<dyn std::error::Error>> {
    let test_dir = format!("/tmp/profiling_benchmark_test_{}", test_name);
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir)?;
    fs::create_dir_all(format!("{}/src", test_dir))?;

    // Create Cargo.toml with profiling feature enabled and rustc-hash dependency
    let cargo_toml = GeneratedCargoToml::new("profiling-benchmark-test")
        .with_rustc_hash()
        .with_profiling_feature()
        .render();

    fs::write(format!("{}/Cargo.toml", test_dir), cargo_toml)?;
    let tct_path = std::env::current_dir()?;
    copy_workspace_cargo_lock(&tct_path, Path::new(&test_dir))?;

    // Write test instance
    fs::write(
        format!("{}/test_instance.json", test_dir),
        serde_json::to_string_pretty(test_instance)?,
    )?;

    // Create main.rs focused on benchmarking
    let main_rs = format!(
        r#"
use serde::{{Deserialize, Serialize}};
use serde_json;

// TCT-generated code with profiling (includes tokenizer module)
{generated_code}

// Helper to find the main type - will be replaced
// MAIN_TYPE_PLACEHOLDER

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let test_json = include_str!("../test_instance.json");
    let instance: TestType = serde_json::from_str(test_json)?;

    #[cfg(feature = "profiling")]
    {{
        use profiling::quick_benchmark;

        println!("EXEC:profiling_available:true");

        // Run comprehensive benchmark
        let results = quick_benchmark(&instance, {benchmark_iterations});

        println!("EXEC:benchmark_iterations:{{}}", results.iterations);
        println!("EXEC:benchmark_success_rate:{{}}", results.success_rate());
        println!("EXEC:benchmark_avg_encode_ns:{{}}", results.avg_encode_time().as_nanos());
        println!("EXEC:benchmark_avg_decode_ns:{{}}", results.avg_decode_time().as_nanos());

        // Output individual timings for consistency analysis
        for (i, &encode_time) in results.encode_times.iter().enumerate() {{
            println!("EXEC:encode_time_{{}}:{{}}", i, encode_time.as_nanos());
        }}

        for (i, &decode_time) in results.decode_times.iter().enumerate() {{
            println!("EXEC:decode_time_{{}}:{{}}", i, decode_time.as_nanos());
        }}
    }}

    println!("EXEC:execution_succeeded:true");
    Ok(())
}}
"#,
        benchmark_iterations = benchmark_iterations
    );

    // Create initial main.rs
    fs::write(format!("{}/src/main.rs", test_dir), main_rs)?;

    // Extract the main type name from generated code and update main.rs
    let main_type_name = extract_main_type_name(generated_code);
    let main_rs_content = fs::read_to_string(format!("{}/src/main.rs", test_dir))?;
    let updated_main_rs = main_rs_content.replace(
        "// MAIN_TYPE_PLACEHOLDER",
        &format!("type TestType = {};", main_type_name),
    );
    fs::write(format!("{}/src/main.rs", test_dir), updated_main_rs)?;

    // Compile and execute with profiling feature
    let compile_output = generated_profiling_cargo_command()
        .current_dir(&test_dir)
        .args(["build", "--release", "--features", "profiling"])
        .output()?;

    if !compile_output.status.success() {
        let compilation_error = String::from_utf8_lossy(&compile_output.stderr);
        return Err(format!("Benchmark compilation failed: {}", compilation_error).into());
    }

    let execute_output = generated_profiling_cargo_command()
        .current_dir(&test_dir)
        .args(["run", "--release", "--features", "profiling"])
        .output()?;

    let execution_output = String::from_utf8_lossy(&execute_output.stdout).to_string();
    let execution_succeeded = execute_output.status.success();
    let compilation_output = String::from_utf8_lossy(&compile_output.stderr).to_string();

    // Parse results with focus on benchmark data
    let result = parse_benchmark_execution_output(
        &execution_output,
        execution_succeeded,
        &compilation_output,
        benchmark_iterations,
    );

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    Ok(result)
}

/// Parse execution output for profiling metrics
fn parse_profiling_execution_output(
    output: &str,
    execution_succeeded: bool,
    compilation_output: &str,
) -> ProfilingTestResult {
    let mut encode_time_ns = None;
    let mut decode_time_ns = None;
    let mut round_trip_valid = false;
    let mut profiling_available = false;
    let mut benchmark_result = None;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("EXEC:encode_time_ns:") {
            encode_time_ns = value.parse().ok();
        } else if let Some(value) = line.strip_prefix("EXEC:decode_time_ns:") {
            decode_time_ns = value.parse().ok();
        } else if let Some(value) = line.strip_prefix("EXEC:round_trip_valid:") {
            round_trip_valid = value == "true";
        } else if let Some(value) = line.strip_prefix("EXEC:profiling_available:") {
            profiling_available = value == "true";
        } else if line.contains("EXEC:benchmark_") {
            // Parse benchmark data if present
            benchmark_result = parse_basic_benchmark_data(output);
        }
    }

    ProfilingTestResult {
        execution_succeeded,
        profiling_available,
        encode_time_ns,
        decode_time_ns,
        round_trip_valid,
        benchmark_result,
        _compilation_output: compilation_output.to_string(),
        _execution_output: output.to_string(),
    }
}

/// Parse execution output specifically for benchmark results
fn parse_benchmark_execution_output(
    output: &str,
    execution_succeeded: bool,
    compilation_output: &str,
    _expected_iterations: usize,
) -> ProfilingTestResult {
    let mut profiling_available = false;
    let mut encode_times_ns = Vec::new();
    let mut decode_times_ns = Vec::new();
    let mut iterations = 0;
    let mut success_rate = 0.0;
    let mut avg_encode_ns = 0;
    let mut avg_decode_ns = 0;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("EXEC:profiling_available:") {
            profiling_available = value == "true";
        } else if let Some(value) = line.strip_prefix("EXEC:benchmark_iterations:") {
            iterations = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("EXEC:benchmark_success_rate:") {
            success_rate = value.parse().unwrap_or(0.0);
        } else if let Some(value) = line.strip_prefix("EXEC:benchmark_avg_encode_ns:") {
            avg_encode_ns = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("EXEC:benchmark_avg_decode_ns:") {
            avg_decode_ns = value.parse().unwrap_or(0);
        } else if line.starts_with("EXEC:encode_time_")
            && let Some(time_str) = line.split(':').nth(2)
            && let Ok(time_ns) = time_str.parse::<u64>()
        {
            encode_times_ns.push(time_ns);
        } else if line.starts_with("EXEC:decode_time_")
            && let Some(time_str) = line.split(':').nth(2)
            && let Ok(time_ns) = time_str.parse::<u64>()
        {
            decode_times_ns.push(time_ns);
        }
    }

    let benchmark_result = if iterations > 0 {
        Some(BenchmarkTestResult {
            iterations,
            success_rate,
            avg_encode_ns,
            avg_decode_ns,
            encode_times_ns,
            decode_times_ns,
        })
    } else {
        None
    };

    ProfilingTestResult {
        execution_succeeded,
        profiling_available,
        encode_time_ns: if avg_encode_ns > 0 {
            Some(avg_encode_ns)
        } else {
            None
        },
        decode_time_ns: if avg_decode_ns > 0 {
            Some(avg_decode_ns)
        } else {
            None
        },
        round_trip_valid: success_rate > 0.0,
        benchmark_result,
        _compilation_output: compilation_output.to_string(),
        _execution_output: output.to_string(),
    }
}

/// Parse basic benchmark data from output
fn parse_basic_benchmark_data(output: &str) -> Option<BenchmarkTestResult> {
    let mut iterations = 0;
    let mut success_rate = 0.0;
    let mut avg_encode_ns = 0;
    let mut avg_decode_ns = 0;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("EXEC:benchmark_iterations:") {
            iterations = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("EXEC:benchmark_success_rate:") {
            success_rate = value.parse().unwrap_or(0.0);
        } else if let Some(value) = line.strip_prefix("EXEC:benchmark_avg_encode_ns:") {
            avg_encode_ns = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("EXEC:benchmark_avg_decode_ns:") {
            avg_decode_ns = value.parse().unwrap_or(0);
        }
    }

    if iterations > 0 {
        Some(BenchmarkTestResult {
            iterations,
            success_rate,
            avg_encode_ns,
            avg_decode_ns,
            encode_times_ns: Vec::new(), // Not collected in basic mode
            decode_times_ns: Vec::new(), // Not collected in basic mode
        })
    } else {
        None
    }
}

/// Extract the main type name from generated code
fn extract_main_type_name(generated_code: &str) -> String {
    // Strategy: Extract type from quick_benchmark function signature (most reliable)
    // Example: pub fn quick_benchmark(instance: &Color, iterations: usize)
    //                                              ^^^^^ extract this

    for line in generated_code.lines() {
        let trimmed = line.trim();

        // Look for quick_benchmark function signature
        if trimmed.contains("pub fn quick_benchmark(")
            && trimmed.contains("instance: &")
            && let Some(start_pos) = trimmed.find("instance: &")
        {
            let remainder = &trimmed[start_pos + "instance: &".len()..];
            // Find the end of the type name (comma, space, or closing paren)
            if let Some(end_pos) = remainder.find([',', ' ', ')']) {
                let type_name = &remainder[..end_pos];
                if !type_name.is_empty() {
                    return type_name.to_string();
                }
            }
        }
    }

    // Fallback: Look for pub enum/struct (original logic) skipping known internal types
    let mut inside_tokenizer_module = false;
    let mut brace_depth = 0;

    for line in generated_code.lines() {
        let trimmed = line.trim();

        // Track tokenizer module
        if trimmed.starts_with("pub mod tokenizer") {
            inside_tokenizer_module = true;
            brace_depth = 0;
            continue;
        }

        if inside_tokenizer_module {
            for ch in trimmed.chars() {
                if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        inside_tokenizer_module = false;
                    }
                }
            }
            continue;
        }

        if !inside_tokenizer_module {
            if trimmed.starts_with("pub enum ")
                && let Some(type_name) = extract_type_name_from_line(trimmed, "pub enum ")
                && type_name != "DecodeError"
                && type_name != "OptionalField"
            {
                return type_name;
            }

            if trimmed.starts_with("pub struct ")
                && let Some(type_name) = extract_type_name_from_line(trimmed, "pub struct ")
            {
                let internal_types = [
                    "EOFContext",
                    "BenchmarkResults",
                    "BufferedTokenIterator",
                    "GapInfo",
                    "PartialEncodingResult",
                    "PartialEncoding",
                ];
                if !internal_types.contains(&type_name.as_str()) {
                    return type_name;
                }
            }
        }
    }

    "GeneratedType".to_string()
}

/// Extract type name from a line containing type definition
fn extract_type_name_from_line(line: &str, prefix: &str) -> Option<String> {
    if let Some(remainder) = line.strip_prefix(prefix) {
        // Find the end of the type name (space, {, <, or ()
        let end_chars = [' ', '{', '<', '('];
        if let Some(end_pos) = remainder.find(|c| end_chars.contains(&c)) {
            let type_name = remainder[..end_pos].trim();
            if !type_name.is_empty() {
                return Some(type_name.to_string());
            }
        }
    }
    None
}

/// Calculate coefficient of variation for timing consistency analysis
fn calculate_coefficient_of_variation(values: &[u64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let mean = values.iter().sum::<u64>() as f64 / values.len() as f64;
    let variance = values
        .iter()
        .map(|&x| {
            let diff = x as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;

    let std_dev = variance.sqrt();

    if mean > 0.0 { std_dev / mean } else { 0.0 }
}
