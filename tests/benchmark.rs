use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tempfile::TempDir;

fn generated_benchmark_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

/// Generated-tokenizer smoke benchmark with a clearly synthetic baseline.
///
/// The TCT encode/decode timings come from generated code. The traditional
/// tokenization comparison is a rough whitespace/JSON-punctuation estimate and
/// must not be used as a measured tokenizer baseline.
#[test]
fn test_benchmark_compression_benchmarks() {
    let test_cases = vec![
        ("Simple", create_simple_schema(), create_simple_data()),
        ("Medium", create_medium_schema(), create_medium_data()),
        ("Complex", create_complex_schema(), create_complex_data()),
    ];

    println!("\n=== TCT Generated Tokenizer Smoke Measurements ===");
    println!(
        "{:<15} {:<12} {:<12} {:<12} {:<12}",
        "Schema", "JSON Bytes", "TCT Tokens", "TCT Bytes", "Ratio"
    );
    println!("{:-<65}", "");

    for (name, schema, test_data) in test_cases {
        let results = benchmark_schema(&schema, &test_data, name);
        println!(
            "{:<15} {:<12} {:<12} {:<12} {:<12.2}",
            name,
            results.json_bytes,
            results.token_count,
            results.token_bytes,
            results.compression_ratio
        );
    }

    println!("\n=== Measurement Notes ===");
    run_performance_benchmark();
}

#[derive(Debug)]
struct BenchmarkResults {
    json_bytes: usize,
    token_count: usize,
    token_bytes: usize,
    compression_ratio: f64,
    encode_time_us: u128,
    decode_time_us: u128,
}

fn benchmark_schema(
    schema: &serde_json::Value,
    test_data: &serde_json::Value,
    name: &str,
) -> BenchmarkResults {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let crate_path = temp_dir.path();

    // Generate tokenizer code
    let grammar = TctCompiler::new(schema.clone(), name)
        .unwrap()
        .compile()
        .expect("Failed to generate grammar");
    let generated_code = grammar.generate_code().expect("Failed to generate code");

    // Create benchmark crate
    create_benchmark_crate(crate_path, &generated_code, test_data, name)
        .expect("Failed to create benchmark crate");

    // Run benchmark
    let output = compile_and_run_crate(crate_path).expect("Failed to compile and run benchmark");
    parse_benchmark_results(&output)
}

fn create_benchmark_crate(
    crate_path: &Path,
    generated_code: &str,
    test_data: &serde_json::Value,
    schema_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let src_dir = crate_path.join("src");
    fs::create_dir_all(&src_dir)?;

    // Create Cargo.toml with optimization
    let cargo_toml = GeneratedCargoToml::new("tct-benchmark")
        .with_release_lto()
        .with_preserve_order()
        .with_rustc_hash()
        .render();
    fs::write(crate_path.join("Cargo.toml"), cargo_toml)?;
    let tct_path = std::env::current_dir()?;
    copy_workspace_cargo_lock(&tct_path, crate_path)?;

    // Create benchmark main.rs
    let test_data_json = serde_json::to_string_pretty(test_data)?;

    // Write the main.rs file in parts to avoid complex string escaping
    // Note: generated_code includes "pub mod tokenizer { ... }" so we don't need a separate module
    let main_start = format!(
        "use tokenizer::JsonTokenEncoder;\nuse std::time::Instant;\n\n{}\n\n",
        generated_code
    );

    let main_middle = format!(
        "fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let test_json = r###\"{}\"###;\n    let original: {} = serde_json::from_str(test_json)?;\n",
        test_data_json, schema_name
    );

    let main_end = format!(
        r#"
    // Measure JSON serialization size
    let json_str = serde_json::to_string(&original)?;
    let json_bytes = json_str.len();
    
    // Benchmark encoding (multiple iterations for accuracy)
    let iterations = 100;
    let encode_start = Instant::now();
    let mut final_tokens = Vec::new();
    for _ in 0..iterations {{
        let mut tokens = Vec::new();
        original.encode(&mut tokens);
        if final_tokens.is_empty() {{
            final_tokens = tokens;
        }}
    }}
    let encode_time = encode_start.elapsed();
    let encode_time_us = encode_time.as_micros() / iterations;
    
    // Benchmark decoding
    let decode_start = Instant::now();
    let mut final_decoded = None;
    for _ in 0..iterations {{
        let mut token_iter = final_tokens.iter().cloned();
        let decoded = {}::decode(&mut token_iter)?;
        if final_decoded.is_none() {{
            final_decoded = Some(decoded);
        }}
    }}
    let decode_time = decode_start.elapsed();
    let decode_time_us = decode_time.as_micros() / iterations;
    
    // Verify round-trip integrity
    assert_eq!(original, final_decoded.unwrap());
    
    // Calculate statistics
    let token_count = final_tokens.len();
    let token_bytes = token_count * 4; // 4 bytes per u32
    let compression_ratio = json_bytes as f64 / token_bytes as f64;
    
    // Output results in parseable format
    println!("BENCHMARK_RESULTS");
    println!("JSON_BYTES: {{}}", json_bytes);
    println!("TOKEN_COUNT: {{}}", token_count);
    println!("TOKEN_BYTES: {{}}", token_bytes);
    println!("COMPRESSION_RATIO: {{:.4}}", compression_ratio);
    println!("ENCODE_TIME_US: {{}}", encode_time_us);
    println!("DECODE_TIME_US: {{}}", decode_time_us);
    
    // Synthetic traditional-tokenization comparison. This is a rough estimate,
    // not a measured baseline from an external tokenizer.
    let traditional_tokens = traditional_tokenize(&json_str);
    let traditional_bytes = traditional_tokens * 4;
    let traditional_ratio = json_bytes as f64 / traditional_bytes as f64;
    
    println!("SYNTHETIC_TRADITIONAL_TOKENS_ESTIMATE: {{}}", traditional_tokens);
    println!("SYNTHETIC_TRADITIONAL_RATIO_ESTIMATE: {{:.4}}", traditional_ratio);
    println!("SYNTHETIC_TRADITIONAL_IMPROVEMENT_ESTIMATE: {{:.2}}", traditional_ratio / compression_ratio);
    
    Ok(())
}}

// Estimate traditional tokenization with a rough whitespace/punctuation approximation.
fn traditional_tokenize(text: &str) -> usize {{
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut token_count = 0;
    
    for token in tokens {{
        if token.len() > 6 {{
            token_count += (token.len() + 3) / 4;
        }} else {{
            token_count += 1;
        }}
    }}
    
    // Add tokens for JSON structure
    token_count += text.chars().filter(|c| "[]{{}}():,\"".contains(*c)).count();
    token_count
}}
"#,
        schema_name
    );

    let main_code = format!("{}{}{}", main_start, main_middle, main_end);

    fs::write(src_dir.join("main.rs"), main_code)?;
    Ok(())
}

// Note: write_tokenizer_primitives() function removed as the generated code
// from TCT already includes a complete "pub mod tokenizer { ... }" with all primitives

fn parse_benchmark_results(output: &str) -> BenchmarkResults {
    let mut results = BenchmarkResults {
        json_bytes: 0,
        token_count: 0,
        token_bytes: 0,
        compression_ratio: 0.0,
        encode_time_us: 0,
        decode_time_us: 0,
    };

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("JSON_BYTES: ") {
            results.json_bytes = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("TOKEN_COUNT: ") {
            results.token_count = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("TOKEN_BYTES: ") {
            results.token_bytes = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("COMPRESSION_RATIO: ") {
            results.compression_ratio = value.parse().unwrap_or(0.0);
        } else if let Some(value) = line.strip_prefix("ENCODE_TIME_US: ") {
            results.encode_time_us = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("DECODE_TIME_US: ") {
            results.decode_time_us = value.parse().unwrap_or(0);
        }
    }

    results
}

fn compile_and_run_crate(crate_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    // Compile with optimizations
    let compile_output = generated_benchmark_cargo_command()
        .args(["build", "--release"])
        .current_dir(crate_path)
        .output()?;

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        return Err(format!("Compilation failed: {}", stderr).into());
    }

    // Run the benchmark
    let run_output = generated_benchmark_cargo_command()
        .args(["run", "--release"])
        .current_dir(crate_path)
        .output()?;

    if !run_output.status.success() {
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        return Err(format!("Execution failed: {}", stderr).into());
    }

    Ok(String::from_utf8_lossy(&run_output.stdout).to_string())
}

fn run_performance_benchmark() {
    println!("Encoding/Decoding performance will be measured per test case above.");
    println!("Generated tokenizer timings use 100 iterations per test case.");
    println!("Times are microseconds per operation from a local smoke measurement.");
    println!("Synthetic traditional-tokenizer values are estimates, not measured baselines.");
}

// Test case generators
fn create_simple_schema() -> serde_json::Value {
    json!({
        "title": "Simple",
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "active": { "type": "boolean" }
        },
        "required": ["name"]
    })
}

fn create_simple_data() -> serde_json::Value {
    json!({
        "name": "Simple Test Item",
        "active": true
    })
}

fn create_medium_schema() -> serde_json::Value {
    json!({
        "title": "Medium",
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "status": { "enum": ["pending", "active", "completed"] },
            "metadata": {
                "type": "object",
                "properties": {
                    "created_at": { "type": "string" },
                    "updated_at": { "type": "string" },
                    "priority": { "type": "integer" }
                }
            },
            "tags": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["id", "status"]
    })
}

fn create_medium_data() -> serde_json::Value {
    json!({
        "id": "medium-test-12345",
        "status": "active",
        "metadata": {
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z",
            "priority": 5
        },
        "tags": ["important", "test", "benchmark"]
    })
}

fn create_complex_schema() -> serde_json::Value {
    json!({
        "title": "Complex",
        "type": "object",
        "properties": {
            "order_id": { "type": "string" },
            "customer": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "email": { "type": "string" },
                    "preferences": {
                        "type": "object",
                        "properties": {
                            "notifications": { "type": "boolean" },
                            "theme": { "enum": ["light", "dark", "auto"] }
                        }
                    }
                },
                "required": ["id", "email"]
            },
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "sku": { "type": "string" },
                        "quantity": { "type": "integer" },
                        "price": { "type": "number" },
                        "category": { "enum": ["electronics", "books", "clothing", "home"] }
                    },
                    "required": ["sku", "quantity", "price", "category"]
                }
            },
            "shipping": {
                "type": "object",
                "properties": {
                    "method": { "enum": ["standard", "express", "overnight"] },
                    "address": {
                        "type": "object",
                        "properties": {
                            "street": { "type": "string" },
                            "city": { "type": "string" },
                            "country": { "type": "string" },
                            "postal_code": { "type": "string" }
                        },
                        "required": ["street", "city", "country"]
                    }
                },
                "required": ["method", "address"]
            }
        },
        "required": ["order_id", "customer", "items"]
    })
}

fn create_complex_data() -> serde_json::Value {
    json!({
        "order_id": "ORD-2024-001234",
        "customer": {
            "id": "CUST-78901",
            "email": "customer@example.com",
            "preferences": {
                "notifications": true,
                "theme": "dark"
            }
        },
        "items": [
            {
                "sku": "ELEC-001",
                "quantity": 1,
                "price": 299.99,
                "category": "electronics"
            },
            {
                "sku": "BOOK-005",
                "quantity": 2,
                "price": 24.99,
                "category": "books"
            }
        ],
        "shipping": {
            "method": "express",
            "address": {
                "street": "123 Main St",
                "city": "New York",
                "country": "USA",
                "postal_code": "10001"
            }
        }
    })
}

#[allow(dead_code)]
fn load_github_workflow_schema() -> serde_json::Value {
    // Load the actual GitHub workflow schema if available, or return a simplified version
    if let Ok(content) = fs::read_to_string("schemas/popular/github-workflow.json") {
        serde_json::from_str(&content).unwrap_or_else(|_| create_github_workflow_schema())
    } else {
        create_github_workflow_schema()
    }
}

#[allow(dead_code)]
fn create_github_workflow_schema() -> serde_json::Value {
    json!({
        "title": "GithubWorkflow",
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "on": {
                "type": "object",
                "properties": {
                    "push": {
                        "type": "object",
                        "properties": {
                            "branches": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        }
                    },
                    "pull_request": {
                        "type": "object",
                        "properties": {
                            "branches": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        }
                    }
                }
            },
            "jobs": {
                "type": "object",
                "additionalProperties": {
                    "type": "object",
                    "properties": {
                        "runs_on": { "type": "string" },
                        "steps": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" },
                                    "run": { "type": "string" },
                                    "uses": { "type": "string" }
                                }
                            }
                        }
                    },
                    "required": ["runs_on"]
                }
            }
        },
        "required": ["on", "jobs"]
    })
}

#[allow(dead_code)]
fn create_github_workflow_data() -> serde_json::Value {
    json!({
        "name": "CI Pipeline",
        "on": {
            "push": {
                "branches": ["main", "develop"]
            },
            "pull_request": {
                "branches": ["main"]
            }
        },
        "jobs": {
            "test": {
                "runs_on": "ubuntu-latest",
                "steps": [
                    {
                        "name": "Checkout",
                        "uses": "actions/checkout@v3"
                    },
                    {
                        "name": "Setup Node",
                        "uses": "actions/setup-node@v3"
                    },
                    {
                        "name": "Install dependencies",
                        "run": "npm install"
                    },
                    {
                        "name": "Run tests",
                        "run": "npm test"
                    }
                ]
            },
            "build": {
                "runs_on": "ubuntu-latest",
                "steps": [
                    {
                        "name": "Build application",
                        "run": "npm run build"
                    }
                ]
            }
        }
    })
}
