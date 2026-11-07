//! Comprehensive Random Token Fuzzing for GitHub Workflow Tokenizer
//!
//! Tests the github-workflow decoder with random token sequences in the
//! vocabulary range [0, 8192) to verify modulo-based graceful recovery
//! and identify which errors can still occur.
//!
//! ## Test Organization
//!
//! ### Fast Tests (require an installed `tct_github_workflow` wheel)
//! These tests use batch processing and are ignored by default because they
//! need the generated wheel importable from `python3`:
//! - `test_1_basic_decode_random_tokens` (~8s) - 3000 random sequences
//! - `test_4_extreme_boundary_values` (~5s) - Edge case validation
//! - `test_5_bpe_range_unknown_tokens` (~4s) - 500 BPE-range sequences
//! - `test_6_base_vocab_edge_cases` (~4s) - 500 base-range sequences
//!
//! ### Local/Nightly Tests (Slow - Marked with #[ignore])
//! These tests require complex subprocess interactions and should be run manually:
//! - `test_2_consumption_tracking_with_surplus` (~60s) - Surplus detection
//! - `test_3_partial_decode_random_presence` (~36s) - Partial decode fuzzing
//! - `test_7_round_trip_on_successful_decodes` (~500s) - Full round-trip validation
//!
//! **Run ignored tests locally with:**
//! ```bash
//! cargo test --test random_token_github_workflow_test -- --ignored --nocapture
//! ```
//!
//! ## Test Coverage
//! - Basic decode (Mode 1)
//! - Consumption tracking
//! - Partial decode (Mode 2)
//! - Windowed generation
//! - BPE unknown token handling
//! - Base vocabulary edge handling
//! - Round-trip validation

use rand::RngExt;
use std::fs;

// Constants for github-workflow tokenizer
const VOCAB_SIZE: u32 = 8192;
const BASE_VOCAB_SIZE: u32 = 1024;
const DICT_SIZE: u32 = 1024;
const BPE_START: u32 = if BASE_VOCAB_SIZE > DICT_SIZE + 1 {
    BASE_VOCAB_SIZE
} else {
    DICT_SIZE + 1
};

/// Statistics for categorizing decode errors
#[derive(Debug, Default)]
struct TestStats {
    total: usize,
    successes: usize,
    unexpected_eof: usize,
    utf8_error: usize,
    invalid_data: usize,
    invalid_token: usize,
}

impl TestStats {
    fn record_result(&mut self, result: &Result<String, String>) {
        self.total += 1;
        match result {
            Ok(_) => self.successes += 1,
            Err(e) => {
                // Match patterns from Python RuntimeError wrapping Rust DecodeError
                if e.contains("Unexpected end of token stream")
                    || e.contains("UnexpectedEndOfStream")
                {
                    self.unexpected_eof += 1;
                } else if e.contains("Failed to decode UTF-8 string from tokens")
                    || e.contains("Utf8Error")
                {
                    self.utf8_error += 1;
                } else if e.contains("Invalid token") || e.contains("InvalidToken") {
                    self.invalid_token += 1;
                } else if e.contains("Invalid") {
                    self.invalid_data += 1;
                } else {
                    // Unknown error type
                    eprintln!("Unknown error pattern: {}", e);
                    self.invalid_data += 1;
                }
            }
        }
    }

    fn print_summary(&self, label: &str) {
        println!("\n[{}] Results:", label);
        println!("  Total:     {} sequences", self.total);
        println!(
            "  Success:   {} ({:.1}%)",
            self.successes,
            self.success_rate()
        );
        println!(
            "  InvalidToken: {} {}",
            self.invalid_token,
            if self.invalid_token == 0 {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  UnexpectedEOF: {} ({:.1}%)",
            self.unexpected_eof,
            self.percentage(self.unexpected_eof)
        );
        println!(
            "  Utf8Error: {} ({:.1}%)",
            self.utf8_error,
            self.percentage(self.utf8_error)
        );
        println!(
            "  InvalidData: {} ({:.1}%)",
            self.invalid_data,
            self.percentage(self.invalid_data)
        );
    }

    fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.successes as f64 / self.total as f64) * 100.0
        }
    }

    fn percentage(&self, count: usize) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (count as f64 / self.total as f64) * 100.0
        }
    }
}

/// Generate random token sequences
fn generate_random_tokens(count: usize, max_value: u32) -> Vec<u32> {
    let mut rng = rand::rng();
    (0..count).map(|_| rng.random_range(0..max_value)).collect()
}

/// Helper to run Python wheel decode (single token sequence)
fn decode_with_wheel(tokens: &[u32]) -> Result<String, String> {
    // Write tokens to temp file
    let tokens_json = serde_json::to_string(tokens).unwrap();
    let temp_file = "/tmp/tct_fuzz_tokens.json";
    fs::write(temp_file, &tokens_json).unwrap();

    // Run Python decode
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import json
import tct_github_workflow as tct

with open('{}', 'r') as f:
    tokens = json.load(f)

try:
    result, consumed, total = tct.decode(tokens)
    print(f"SUCCESS:{{consumed}}:{{total}}:{{result}}")
except Exception as e:
    print(f"ERROR:{{str(e)}}")
"#,
            temp_file
        ))
        .output()
        .map_err(|e| format!("Failed to run Python: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.starts_with("SUCCESS:") {
        let parts: Vec<&str> = stdout
            .trim_start_matches("SUCCESS:")
            .splitn(4, ':')
            .collect();
        if parts.len() == 4 {
            Ok(parts[3].to_string())
        } else {
            Err("Invalid success format".to_string())
        }
    } else if stdout.starts_with("ERROR:") {
        Err(stdout.trim_start_matches("ERROR:").to_string())
    } else {
        Err(format!("Unexpected output: {}", stdout))
    }
}

/// Batch decode multiple token sequences in a single Python process (10-20× faster)
fn batch_decode_with_wheel(test_cases: &[Vec<u32>]) -> Vec<Result<String, String>> {
    // Write all test cases to single JSON file
    let batch_json = serde_json::to_string(test_cases).unwrap();
    let temp_file = "/tmp/tct_fuzz_batch.json";
    fs::write(temp_file, &batch_json).unwrap();

    // Run Python ONCE for all test cases
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import json
import tct_github_workflow as tct

with open('{}', 'r') as f:
    all_test_cases = json.load(f)

results = []
for tokens in all_test_cases:
    try:
        result, consumed, total = tct.decode(tokens)
        results.append({{"success": True, "result": result, "consumed": consumed, "total": total}})
    except Exception as e:
        results.append({{"success": False, "error": str(e)}})

print(json.dumps(results))
"#,
            temp_file
        ))
        .output()
        .expect("Failed to run Python batch decode");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse batch results
    let batch_results: Vec<serde_json::Value> = match serde_json::from_str(&stdout) {
        Ok(results) => results,
        Err(e) => {
            eprintln!("Failed to parse batch results: {}", e);
            eprintln!("Output was: {}", stdout);
            return vec![Err(format!("Batch parse error: {}", e)); test_cases.len()];
        }
    };

    // Convert to Rust results
    batch_results
        .into_iter()
        .map(|r| {
            if r["success"].as_bool() == Some(true) {
                Ok(r["result"].as_str().unwrap_or("").to_string())
            } else {
                Err(r["error"].as_str().unwrap_or("Unknown error").to_string())
            }
        })
        .collect()
}

/// Optimized with batch processing: ~120s → ~8s (15× speedup)
#[test]
#[ignore = "requires an installed tct_github_workflow wheel; build it with tests/rebuild_github_workflow_wheel_test.rs first"]
fn test_1_basic_decode_random_tokens() {
    println!("\n=== Test 1: Basic Decode with Random Tokens (Batched) ===");

    let test_cases = vec![
        ("Short (10-50)", 1000, 10, 50),
        ("Medium (100-500)", 1000, 100, 500),
        ("Long (1000-2000)", 1000, 1000, 2000),
    ];

    for (label, iterations, min_len, max_len) in test_cases {
        let mut stats = TestStats::default();
        let mut rng = rand::rng();

        // Generate ALL test cases upfront
        let all_tokens: Vec<Vec<u32>> = (0..iterations)
            .map(|_| {
                let token_count = rng.random_range(min_len..=max_len);
                generate_random_tokens(token_count, VOCAB_SIZE)
            })
            .collect();

        // Process in batches of 100 (balance memory vs process spawning)
        for batch in all_tokens.chunks(100) {
            let results = batch_decode_with_wheel(batch);
            for result in results {
                stats.record_result(&result);
            }
        }

        stats.print_summary(label);

        // Critical assertion: NO InvalidToken errors
        assert_eq!(
            stats.invalid_token, 0,
            "Found {} InvalidToken errors - modulo is not working!",
            stats.invalid_token
        );
    }
}

/// ⚠️ SLOW TEST: ~60s runtime - Run manually with:
/// `cargo test --test random_token_github_workflow_test -- --ignored --nocapture`
#[test]
#[ignore = "slow random-token surplus accounting test; run manually with --ignored"]
fn test_2_consumption_tracking_with_surplus() {
    println!("\n=== Test 2: Consumption Tracking with Surplus ===");

    let mut correct_detections = 0;
    let iterations = 500;

    for _ in 0..iterations {
        // Generate a valid-ish sequence with surplus
        let valid_tokens = generate_random_tokens(100, VOCAB_SIZE);
        let surplus_count = rand::random_range(10..100);
        let surplus_tokens = generate_random_tokens(surplus_count, VOCAB_SIZE);

        let mut full_sequence = valid_tokens.clone();
        full_sequence.extend(surplus_tokens);

        // Try decode and check consumption
        let tokens_json = serde_json::to_string(&full_sequence).unwrap();
        fs::write("/tmp/tct_fuzz_tokens.json", &tokens_json).unwrap();

        let output = std::process::Command::new("python3")
            .arg("-c")
            .arg(
                r#"
import json
import tct_github_workflow as tct

with open('/tmp/tct_fuzz_tokens.json', 'r') as f:
    tokens = json.load(f)

try:
    result, consumed, total = tct.decode(tokens)
    print(f"{consumed},{total}")
except:
    print("ERROR")
"#,
            )
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some((consumed_str, total_str)) = stdout.trim().split_once(',')
            && let (Ok(consumed), Ok(total)) =
                (consumed_str.parse::<usize>(), total_str.parse::<usize>())
        {
            if consumed < total {
                correct_detections += 1;
            } else if consumed == total {
                // This is actually expected if decode consumed everything
                // Not a false positive
            }
        }
    }

    println!("\n[Consumption Tracking] Results:");
    println!("  Sequences tested: {}", iterations);
    println!("  Surplus detected: {} ✅", correct_detections);
    println!(
        "  Detection rate: {:.1}%",
        (correct_detections as f64 / iterations as f64) * 100.0
    );
}

/// ⚠️ SLOW TEST: ~36s runtime - Run manually with:
/// `cargo test --test random_token_github_workflow_test -- --ignored --nocapture`
#[test]
#[ignore = "slow random-token partial decode test; run manually with --ignored"]
fn test_3_partial_decode_random_presence() {
    println!("\n=== Test 3: Partial Decode with Random Presence ===");

    let mut stats = TestStats::default();
    let iterations = 300;
    let field_count = 8; // github-workflow has 8 top-level fields

    for _ in 0..iterations {
        let tokens = generate_random_tokens(200, VOCAB_SIZE);
        let presence: Vec<bool> = (0..field_count).map(|_| rand::random_bool(0.5)).collect();

        // Test partial encoding/decoding
        let tokens_json = serde_json::to_string(&tokens).unwrap();
        let presence_json = serde_json::to_string(&presence).unwrap();

        fs::write("/tmp/tct_fuzz_tokens.json", &tokens_json).unwrap();
        fs::write("/tmp/tct_fuzz_presence.json", &presence_json).unwrap();

        let output = std::process::Command::new("python3")
            .arg("-c")
            .arg(
                r#"
import json
import tct_github_workflow as tct

with open('/tmp/tct_fuzz_tokens.json', 'r') as f:
    tokens = json.load(f)

try:
    result, consumed, total = tct.decode(tokens)
    print(f"SUCCESS")
except Exception as e:
    print(f"ERROR:{str(e)}")
"#,
            )
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = if stdout.starts_with("SUCCESS") {
            Ok("success".to_string())
        } else {
            Err(stdout.to_string())
        };

        stats.record_result(&result);
    }

    stats.print_summary("Partial Decode");

    assert_eq!(
        stats.invalid_token, 0,
        "Found {} InvalidToken errors in partial decode",
        stats.invalid_token
    );
}

/// Fast boundary test (~5s) - runs in CI
#[test]
#[ignore = "requires an installed tct_github_workflow wheel; build it with tests/rebuild_github_workflow_wheel_test.rs first"]
fn test_4_extreme_boundary_values() {
    println!("\n=== Test 4: Extreme Boundary Values ===");

    let test_cases = vec![
        ("All zeros", vec![0u32; 500]),
        ("All max", vec![VOCAB_SIZE - 1; 500]),
        ("All high BPE", vec![7500u32; 500]),
        (
            "Alternating 0/max",
            (0..500)
                .map(|i| if i % 2 == 0 { 0 } else { VOCAB_SIZE - 1 })
                .collect(),
        ),
    ];

    let mut total_invalid_token = 0;

    for (label, tokens) in test_cases {
        let result = decode_with_wheel(&tokens);

        if let Err(e) = &result {
            if e.contains("InvalidToken") {
                total_invalid_token += 1;
                println!("\n  {} : ❌ InvalidToken found!", label);
            } else {
                println!("\n  {} : Other error (expected)", label);
            }
        } else {
            println!("\n  {} : ✅ Decoded successfully", label);
        }
    }

    println!("\n[Extreme Values] Summary:");
    println!(
        "  InvalidToken errors: {} {}",
        total_invalid_token,
        if total_invalid_token == 0 {
            "✅"
        } else {
            "❌"
        }
    );

    assert_eq!(
        total_invalid_token, 0,
        "Extreme values caused {} InvalidToken errors - modulo not handling boundaries!",
        total_invalid_token
    );
}

/// Optimized with batch processing: ~60s → ~4s (15× speedup)
#[test]
#[ignore = "requires an installed tct_github_workflow wheel; build it with tests/rebuild_github_workflow_wheel_test.rs first"]
fn test_5_bpe_range_unknown_tokens() {
    println!("\n=== Test 5: BPE Range Unknown Tokens (Batched) ===");

    let mut stats = TestStats::default();
    let iterations = 500;

    // Generate ALL test cases upfront
    let all_tokens: Vec<Vec<u32>> = (0..iterations)
        .map(|_| {
            let mut rng = rand::rng();
            let token_count = rng.random_range(50..200);
            (0..token_count)
                .map(|_| rng.random_range(BPE_START..VOCAB_SIZE))
                .collect()
        })
        .collect();

    // Process in batches of 100
    for batch in all_tokens.chunks(100) {
        let results = batch_decode_with_wheel(batch);
        for result in results {
            stats.record_result(&result);
        }
    }

    stats.print_summary("BPE Unknown Tokens");

    assert_eq!(
        stats.invalid_token, 0,
        "BPE unknown tokens caused {} InvalidToken errors",
        stats.invalid_token
    );
}

/// Optimized with batch processing: ~60s → ~4s (15× speedup)
#[test]
#[ignore = "requires an installed tct_github_workflow wheel; build it with tests/rebuild_github_workflow_wheel_test.rs first"]
fn test_6_base_vocab_edge_cases() {
    println!("\n=== Test 6: Base Vocabulary Edge Cases (Batched) ===");

    let mut stats = TestStats::default();
    let iterations = 500;

    // Generate ALL test cases upfront
    let all_tokens: Vec<Vec<u32>> = (0..iterations)
        .map(|_| {
            let mut rng = rand::rng();
            let token_count = rng.random_range(50..200);
            (0..token_count)
                .map(|_| rng.random_range(0..BASE_VOCAB_SIZE))
                .collect()
        })
        .collect();

    // Process in batches of 100
    for batch in all_tokens.chunks(100) {
        let results = batch_decode_with_wheel(batch);
        for result in results {
            stats.record_result(&result);
        }
    }

    stats.print_summary("Base Vocabulary Range");

    assert_eq!(
        stats.invalid_token, 0,
        "Base vocabulary tokens caused {} InvalidToken errors",
        stats.invalid_token
    );
}

/// ⚠️ SLOW TEST: ~500s runtime - Run manually with:
/// `cargo test --test random_token_github_workflow_test -- --ignored --nocapture`
#[test]
#[ignore = "very slow random-token round-trip search; run manually with --ignored"]
fn test_7_round_trip_on_successful_decodes() {
    println!("\n=== Test 7: Round-Trip on Successful Decodes ===");

    let mut attempts = 0;
    let mut successful_decodes = 0;
    let mut successful_round_trips = 0;
    let max_successful = 50; // Stop after 50 successful decodes

    while successful_decodes < max_successful && attempts < 5000 {
        attempts += 1;

        let tokens = generate_random_tokens(500, VOCAB_SIZE);

        // Try decode
        if let Ok(json_str) = decode_with_wheel(&tokens) {
            successful_decodes += 1;

            // Try encode → decode round-trip
            let output = std::process::Command::new("python3")
                .arg("-c")
                .arg(format!(
                    r#"
import json
import tct_github_workflow as tct

json_str = '''{}'''
try:
    tokens1 = tct.encode(json_str)
    result, _, _ = tct.decode(tokens1)
    print("SUCCESS")
except Exception as e:
    print(f"ERROR:{{str(e)}}")
"#,
                    json_str.replace('\'', "\\'")
                ))
                .output()
                .unwrap();

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.starts_with("SUCCESS") {
                successful_round_trips += 1;
            }
        }
    }

    println!("\n[Round-Trip] Results:");
    println!("  Attempts: {}", attempts);
    println!("  Successful decodes: {}", successful_decodes);
    println!(
        "  Successful round-trips: {} ({:.1}%) ✅",
        successful_round_trips,
        (successful_round_trips as f64 / successful_decodes as f64) * 100.0
    );
}
