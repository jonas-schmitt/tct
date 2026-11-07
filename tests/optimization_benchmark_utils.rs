// Legacy optimization benchmark utilities for crate creation and execution.
// These smoke measurements run generated types in isolated Rust crates with
// local tokenizer primitives. Use tct-measure-tokens for maintained generated
// crate measurements.

use std::fs;
use std::path::Path;
use std::process::Command;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

fn generated_benchmark_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

/// Results from running a legacy benchmark crate.
#[derive(Debug, Clone)]
pub struct OptimizationBenchmarkResults {
    pub json_bytes: usize,
    pub token_count: usize,
    pub token_bytes: usize,
    pub compression_ratio: f64,
    pub encode_time_us: u128,
    pub decode_time_us: u128,
    pub improvement_factor: f64,
}

/// Create a legacy benchmark crate with TCT-generated code and test data.
pub fn create_benchmark_crate(
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
        .render();
    fs::write(crate_path.join("Cargo.toml"), cargo_toml)?;
    let tct_path = std::env::current_dir()?;
    copy_workspace_cargo_lock(&tct_path, crate_path)?;

    // Write tokenizer primitives
    write_tokenizer_primitives(&src_dir)?;

    // Create benchmark main.rs with repeated iterations for local smoke timing.
    let test_data_json = serde_json::to_string_pretty(test_data)?;

    // Split the main code into parts to avoid raw string escaping issues
    let main_start = format!(
        "mod tokenizer;\nuse tokenizer::{{JsonTokenEncoder, DecodeError}};\nuse std::time::Instant;\n\n{}\n\n",
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

    // Benchmark encoding with repeated iterations for local smoke timing.
    let iterations = 1000;
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
    println!("MEASUREMENT_SCOPE: legacy_temp_crate_with_local_tokenizer_primitives");
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

/// Write tokenizer primitive implementations to src/tokenizer.rs
fn write_tokenizer_primitives(src_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let tokenizer_code = r#"
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEndOfStream,
    InvalidToken(u32),
    Utf8Error,
    InvalidData(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::UnexpectedEndOfStream => write!(f, "Unexpected end of token stream"),
            DecodeError::InvalidToken(token) => write!(f, "Invalid token '{}' for this type", token),
            DecodeError::Utf8Error => write!(f, "Failed to decode UTF-8 string from tokens"),
            DecodeError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl Error for DecodeError {}

pub trait JsonTokenEncoder: Sized {
    fn encode(&self, tokens: &mut Vec<u32>);
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError>;
    fn token_count(&self) -> usize;
}

impl JsonTokenEncoder for bool {
    fn encode(&self, tokens: &mut Vec<u32>) {
        tokens.push(if *self { 1 } else { 0 });
    }
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        match tokens.next() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            Some(token) => Err(DecodeError::InvalidToken(token)),
            None => Err(DecodeError::UnexpectedEndOfStream { .. }),
        }
    }
    fn token_count(&self) -> usize { 1 }
}

impl JsonTokenEncoder for i64 {
    fn encode(&self, tokens: &mut Vec<u32>) {
        let bytes = self.to_le_bytes();
        let low = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let high = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        tokens.push(low);
        tokens.push(high);
    }
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let low = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream { .. })?;
        let high = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream { .. })?;
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&low.to_le_bytes());
        bytes[4..8].copy_from_slice(&high.to_le_bytes());
        Ok(i64::from_le_bytes(bytes))
    }
    fn token_count(&self) -> usize { 2 }
}

impl JsonTokenEncoder for f64 {
    fn encode(&self, tokens: &mut Vec<u32>) {
        let bits = self.to_bits();
        let low = bits as u32;
        let high = (bits >> 32) as u32;
        tokens.push(low);
        tokens.push(high);
    }
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let low = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream { .. })?;
        let high = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream { .. })?;
        let bits = (high as u64) << 32 | (low as u64);
        Ok(f64::from_bits(bits))
    }
    fn token_count(&self) -> usize { 2 }
}

impl JsonTokenEncoder for String {
    fn encode(&self, tokens: &mut Vec<u32>) {
        tokens.push(self.len() as u32);
        tokens.extend(self.as_bytes().iter().map(|&b| b as u32));
    }
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let len = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream { .. })? as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            let byte_val = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream { .. })?;
            bytes.push(byte_val as u8);
        }
        String::from_utf8(bytes).map_err(|_| DecodeError::Utf8Error)
    }
    fn token_count(&self) -> usize { 1 + self.len() }
}

impl<T: JsonTokenEncoder> JsonTokenEncoder for Option<T> {
    fn encode(&self, tokens: &mut Vec<u32>) {
        match self {
            Some(value) => {
                tokens.push(1);
                value.encode(tokens);
            }
            None => tokens.push(0),
        }
    }
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        match tokens.next() {
            Some(0) => Ok(None),
            Some(1) => Ok(Some(T::decode(tokens)?)),
            Some(token) => Err(DecodeError::InvalidToken(token)),
            None => Err(DecodeError::UnexpectedEndOfStream { .. }),
        }
    }
}

impl<T: JsonTokenEncoder> JsonTokenEncoder for Vec<T> {
    fn encode(&self, tokens: &mut Vec<u32>) {
        tokens.push(self.len() as u32);
        for item in self {
            item.encode(tokens);
        }
    }
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let len = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream { .. })? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::decode(tokens)?);
        }
        Ok(vec)
    }
}

use std::collections::HashMap;
impl<V: JsonTokenEncoder> JsonTokenEncoder for HashMap<String, V> {
    fn encode(&self, tokens: &mut Vec<u32>) {
        tokens.push(self.len() as u32);
        for (key, value) in self {
            key.encode(tokens);
            value.encode(tokens);
        }
    }
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let len = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream { .. })? as usize;
        let mut map = HashMap::with_capacity(len);
        for _ in 0..len {
            let key = String::decode(tokens)?;
            let value = V::decode(tokens)?;
            map.insert(key, value);
        }
        Ok(map)
    }
}
"#;
    fs::write(src_dir.join("tokenizer.rs"), tokenizer_code)?;
    Ok(())
}

/// Parse benchmark results from crate output
pub fn parse_benchmark_results(output: &str) -> OptimizationBenchmarkResults {
    let mut results = OptimizationBenchmarkResults {
        json_bytes: 0,
        token_count: 0,
        token_bytes: 0,
        compression_ratio: 0.0,
        encode_time_us: 0,
        decode_time_us: 0,
        improvement_factor: 0.0,
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
        } else if let Some(value) = line
            .strip_prefix("SYNTHETIC_TRADITIONAL_IMPROVEMENT_ESTIMATE: ")
            .or_else(|| line.strip_prefix("IMPROVEMENT_FACTOR: "))
        {
            results.improvement_factor = value.parse().unwrap_or(0.0);
        }
    }

    results
}

/// Compile and run a legacy benchmark crate, returning the output.
pub fn compile_and_run_crate(crate_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    // Compile with optimizations
    let compile_output = generated_benchmark_cargo_command()
        .args(["build", "--release"])
        .current_dir(crate_path)
        .output()?;

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        return Err(format!("Compilation failed: {}", stderr).into());
    }

    // Run the benchmark smoke binary.
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

/// Individual legacy benchmark execution for a specific schema and optimization level.
pub fn benchmark_schema_with_level(
    schema: &serde_json::Value,
    test_data: &serde_json::Value,
    name: &str,
    optimization_level: tct::OptimizationLevel,
) -> OptimizationBenchmarkResults {
    use tct::TctCompiler;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let crate_path = temp_dir.path();

    // Generate tokenizer code with specific optimization level
    let grammar = TctCompiler::new(schema.clone(), name)
        .expect("Failed to create TctCompiler")
        .with_optimization_level(optimization_level)
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

/// Generate a legacy analysis report comparing optimization levels.
pub fn generate_optimization_comparison_report(
    results: &[(String, String, OptimizationBenchmarkResults)],
) {
    println!("\n=== Legacy Optimization Level Benchmark Smoke Analysis ===");
    println!(
        "Measurement scope: legacy temp crates with local tokenizer primitives; use tct-measure-tokens for maintained generated-crate results."
    );
    println!("Synthetic traditional-tokenizer improvement values are estimates.");

    // Group results by schema
    let mut schema_groups = std::collections::HashMap::new();
    for (schema, level, result) in results {
        schema_groups
            .entry(schema.clone())
            .or_insert_with(Vec::new)
            .push((level.clone(), result));
    }

    for (schema_name, schema_results) in schema_groups.iter() {
        println!("\n{} Schema Analysis:", schema_name);

        // Find O0 baseline for comparison
        let o0_result = schema_results.iter().find(|(level, _)| level == "O0");

        if let Some((_, o0_baseline)) = o0_result {
            println!(
                "  Baseline (O0): {} bytes → {} tokens (ratio: {:.3})",
                o0_baseline.json_bytes, o0_baseline.token_count, o0_baseline.compression_ratio
            );

            for (level, result) in schema_results {
                if level != "O0" {
                    let token_reduction =
                        (o0_baseline.token_count as i32 - result.token_count as i32) as f64
                            / o0_baseline.token_count as f64
                            * 100.0;
                    let ratio_improvement =
                        result.compression_ratio / o0_baseline.compression_ratio;
                    let encode_speedup =
                        o0_baseline.encode_time_us as f64 / result.encode_time_us as f64;
                    let decode_speedup =
                        o0_baseline.decode_time_us as f64 / result.decode_time_us as f64;

                    println!("  {} vs O0:", level);
                    println!(
                        "    Token reduction: {:.1}% ({} → {} tokens)",
                        token_reduction, o0_baseline.token_count, result.token_count
                    );
                    println!(
                        "    Compression improvement: {:.2}x better",
                        ratio_improvement
                    );
                    println!("    Legacy encode timing ratio: {:.2}x", encode_speedup);
                    println!("    Legacy decode timing ratio: {:.2}x", decode_speedup);
                }
            }
        }
    }

    // Overall optimization effectiveness summary
    println!("\n=== Overall Optimization Effectiveness ===");
    let mut o2_improvements = Vec::new();
    let mut o3_improvements = Vec::new();

    for (schema_name, schema_results) in schema_groups.iter() {
        if let (Some(o0), Some(o2)) = (
            schema_results.iter().find(|(level, _)| level == "O0"),
            schema_results.iter().find(|(level, _)| level == "O2"),
        ) {
            let improvement = o2.1.compression_ratio / o0.1.compression_ratio;
            o2_improvements.push((schema_name.as_str(), improvement));
        }

        if let (Some(o0), Some(o3)) = (
            schema_results.iter().find(|(level, _)| level == "O0"),
            schema_results.iter().find(|(level, _)| level == "O3"),
        ) {
            let improvement = o3.1.compression_ratio / o0.1.compression_ratio;
            o3_improvements.push((schema_name.as_str(), improvement));
        }
    }

    if !o2_improvements.is_empty() {
        let avg_o2_improvement: f64 =
            o2_improvements.iter().map(|(_, imp)| imp).sum::<f64>() / o2_improvements.len() as f64;
        println!(
            "O2 Average measured token compression improvement: {:.2}x better than O0",
            avg_o2_improvement
        );
        println!(
            "O2 Best case: {:.2}x ({})",
            o2_improvements
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap()
                .1,
            o2_improvements
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap()
                .0
        );
    }

    if !o3_improvements.is_empty() {
        let avg_o3_improvement: f64 =
            o3_improvements.iter().map(|(_, imp)| imp).sum::<f64>() / o3_improvements.len() as f64;
        println!(
            "O3 Average measured token compression improvement: {:.2}x better than O0",
            avg_o3_improvement
        );
        println!(
            "O3 Best case: {:.2}x ({})",
            o3_improvements
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap()
                .1,
            o3_improvements
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap()
                .0
        );
    }
}
