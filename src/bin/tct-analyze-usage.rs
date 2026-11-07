//! Vocabulary Usage Analysis Binary
//!
//! Analyzes actual token usage from JSON instances to identify vocabulary optimization opportunities.
//!
//! # Workflow
//! 1. Compile schema to generate tokenizer
//! 2. Encode all instances to token sequences
//! 3. Analyze token usage patterns (frequency, categories, rare tokens)
//! 4. Generate optimization recommendations
//! 5. Output analysis report (text or JSON)
//!
//! # Usage
//! ```bash
//! tct-analyze-usage \
//!   --schema schemas/popular/github-workflow.json \
//!   --instances data/workflows/*.json \
//!   --format text
//! ```

use clap::{Arg, ArgMatches, Command, value_parser};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;
use tct::TctCompiler;
use tct::instance_encoder::{InstanceEncoder, expand_glob_pattern};
use tct::provenance::{FileProvenance, file_provenance};

fn main() -> Result<(), Box<dyn Error>> {
    let command: Vec<String> = std::env::args().collect();
    let matches = Command::new("tct-analyze-usage")
        .about("Analyze vocabulary usage from JSON instances")
        .version("0.1.0")
        .arg(
            Arg::new("schema")
                .short('s')
                .long("schema")
                .help("JSON schema file")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("instances")
                .short('i')
                .long("instances")
                .help("Instance JSON files (glob pattern)")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .help("Output format: text or json")
                .default_value("text")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .help("Schema name for analysis")
                .value_parser(value_parser!(String)),
        )
        .get_matches();

    let schema_path = required_arg::<String>(&matches, "schema")?;
    let instances_pattern = required_arg::<String>(&matches, "instances")?;
    let format = required_arg::<String>(&matches, "format")?;
    let schema_name = matches.get_one::<String>("name");

    eprintln!("🔧 Vocabulary Usage Analysis Configuration");
    eprintln!("  Schema: {}", schema_path);
    eprintln!("  Instances: {}", instances_pattern);
    eprintln!("  Format: {}", format);
    eprintln!();

    // Step 1: Load and compile schema
    eprintln!("📖 Loading schema...");
    let schema_content = fs::read_to_string(schema_path)?;
    let schema: Value = serde_json::from_str(&schema_content)?;

    let name = if let Some(n) = schema_name {
        n.clone()
    } else {
        // Derive from schema file name
        Path::new(schema_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("schema")
            .to_string()
    };

    eprintln!("🔨 Compiling schema '{}'...", name);
    let compiler =
        TctCompiler::new(schema, "TestType")?.with_optimization_level(tct::OptimizationLevel::O3);
    let grammar = compiler.compile()?;

    // Generate code
    eprintln!("🔧 Generating TCT tokenizer code...");
    let code = grammar.generate_code()?;

    eprintln!("✅ Schema compiled successfully");
    eprintln!();

    // Step 2: Load instances
    eprintln!("📂 Loading instances from pattern: {}", instances_pattern);
    let instance_paths = expand_glob_pattern(instances_pattern)?;
    eprintln!("  Found {} instance files", instance_paths.len());

    if instance_paths.is_empty() {
        return Err("No instance files found matching pattern".into());
    }

    // Step 3: Encode instances
    eprintln!("🔢 Encoding instances to tokens...");
    eprintln!("  Creating instance encoder...");
    let encoder = InstanceEncoder::new(&code.to_string(), &grammar.root_type_name)?;
    eprintln!("  Encoding {} instances...", instance_paths.len());
    let token_sequences = encoder.encode_batch(&instance_paths)?;
    eprintln!("  Encoded {} instances", token_sequences.len());
    eprintln!();

    // Step 4: Analyze usage
    eprintln!("📊 Analyzing token usage...");
    let base_vocab_size = grammar.base_vocab;
    let input_files = instance_paths
        .iter()
        .map(file_provenance)
        .collect::<Result<Vec<_>, _>>()?;
    let provenance = UsageProvenance {
        command,
        schema_name: name.clone(),
        schema: schema_path.clone(),
        instances_pattern: instances_pattern.clone(),
        schema_file: file_provenance(schema_path)?,
        input_files,
        bpe_merges: None,
        bpe_merges_file: None,
        base_vocab_size,
        base_vocab_source: "grammar".to_string(),
        optimization_level: tct::OptimizationLevel::O3.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        tct_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let analysis = analyze_token_usage(&token_sequences, base_vocab_size, Some(provenance));
    eprintln!("  Analyzed {} unique tokens", analysis.unique_tokens_used);
    eprintln!();

    // Step 5: Output results
    match format.as_str() {
        "json" => {
            let output = serde_json::to_string_pretty(&analysis)?;
            println!("{}", output);
        }
        "text" => {
            print_text_report(&name, &analysis);
        }
        _ => {
            return Err(format!("Unknown format: {}", format).into());
        }
    }

    Ok(())
}

fn required_arg<'a, T>(matches: &'a ArgMatches, name: &str) -> Result<&'a T, Box<dyn Error>>
where
    T: Clone + Send + Sync + 'static,
{
    matches.get_one::<T>(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("missing required or defaulted CLI argument {name:?}"),
        )
        .into()
    })
}

/// Token category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TokenCategory {
    Byte,       // 0-255
    Structural, // 256 to u32::MAX-1
    Special,    // u32::MAX
}

impl TokenCategory {
    fn from_token(token: u32) -> Self {
        if token <= 255 {
            TokenCategory::Byte
        } else if token == u32::MAX {
            TokenCategory::Special
        } else {
            TokenCategory::Structural
        }
    }
}

/// Information about a rare token
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RareTokenInfo {
    token: u32,
    frequency: usize,
    category: TokenCategory,
}

/// Category breakdown statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CategoryBreakdown {
    byte_tokens_used: usize,
    byte_tokens_available: usize,
    structural_tokens_used: usize,
    structural_tokens_available: usize,
    special_tokens_used: usize,
}

/// Reproducibility metadata for a usage analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsageProvenance {
    command: Vec<String>,
    schema_name: String,
    schema: String,
    instances_pattern: String,
    schema_file: FileProvenance,
    input_files: Vec<FileProvenance>,
    bpe_merges: Option<String>,
    bpe_merges_file: Option<FileProvenance>,
    base_vocab_size: u32,
    base_vocab_source: String,
    optimization_level: String,
    created_at: String,
    tct_version: String,
}

/// Complete usage analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsageAnalysis {
    total_instances: usize,
    total_tokens: usize,
    unique_tokens_used: usize,
    base_vocab_size: u32,
    unused_tokens: usize,
    token_frequencies: FxHashMap<u32, usize>,
    category_breakdown: CategoryBreakdown,
    rare_tokens: Vec<RareTokenInfo>,
    top_tokens: Vec<(u32, usize)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provenance: Option<UsageProvenance>,
}

/// Analyze token usage patterns from sequences
fn analyze_token_usage(
    sequences: &[Vec<u32>],
    base_vocab_size: u32,
    provenance: Option<UsageProvenance>,
) -> UsageAnalysis {
    let mut token_frequencies: FxHashMap<u32, usize> = FxHashMap::default();
    let mut total_tokens = 0;

    // Count frequencies
    for sequence in sequences {
        for &token in sequence {
            *token_frequencies.entry(token).or_insert(0) += 1;
            total_tokens += 1;
        }
    }

    let unique_tokens_used = token_frequencies.len();
    let unused_tokens = (base_vocab_size as usize).saturating_sub(unique_tokens_used);

    // Categorize tokens
    let mut byte_tokens_used = 0;
    let mut structural_tokens_used = 0;
    let mut special_tokens_used = 0;

    for &token in token_frequencies.keys() {
        match TokenCategory::from_token(token) {
            TokenCategory::Byte => byte_tokens_used += 1,
            TokenCategory::Structural => structural_tokens_used += 1,
            TokenCategory::Special => special_tokens_used += 1,
        }
    }

    // Calculate available tokens per category
    let byte_tokens_available = 256;
    let structural_tokens_available = (base_vocab_size as usize).saturating_sub(257); // 256 bytes + 1 special (u32::MAX)
    let category_breakdown = CategoryBreakdown {
        byte_tokens_used,
        byte_tokens_available,
        structural_tokens_used,
        structural_tokens_available,
        special_tokens_used,
    };

    // Identify rare tokens (used < 10 times)
    let rare_threshold = 10;
    let mut rare_tokens: Vec<RareTokenInfo> = token_frequencies
        .iter()
        .filter(|(_, freq)| **freq < rare_threshold)
        .map(|(&token, &frequency)| RareTokenInfo {
            token,
            frequency,
            category: TokenCategory::from_token(token),
        })
        .collect();
    rare_tokens.sort_by_key(|r| r.frequency);

    // Get top N most frequent tokens
    let mut top_tokens: Vec<(u32, usize)> = token_frequencies
        .iter()
        .map(|(&token, &freq)| (token, freq))
        .collect();
    top_tokens.sort_by_key(|(_, frequency)| std::cmp::Reverse(*frequency));
    top_tokens.truncate(20);

    UsageAnalysis {
        total_instances: sequences.len(),
        total_tokens,
        unique_tokens_used,
        base_vocab_size,
        unused_tokens,
        token_frequencies,
        category_breakdown,
        rare_tokens,
        top_tokens,
        provenance,
    }
}

/// Print human-readable text report
fn print_text_report(schema_name: &str, analysis: &UsageAnalysis) {
    let bloat_pct = (analysis.unused_tokens as f64 / analysis.base_vocab_size as f64) * 100.0;
    let avg_tokens = analysis.total_tokens as f64 / analysis.total_instances as f64;

    println!("Vocabulary Usage Analysis for {}", schema_name);
    println!("{}", "=".repeat(80));
    println!();

    println!(
        "📈 Overall Usage (from {} instances)",
        analysis.total_instances
    );
    println!(
        "  Total tokens encoded: {:>12}",
        format_number(analysis.total_tokens)
    );
    println!(
        "  Unique tokens used:   {:>12} / {} ({:.1}%)",
        format_number(analysis.unique_tokens_used),
        analysis.base_vocab_size,
        (analysis.unique_tokens_used as f64 / analysis.base_vocab_size as f64) * 100.0
    );
    println!(
        "  Unused tokens:        {:>12} ({:.1}% vocabulary bloat)",
        format_number(analysis.unused_tokens),
        bloat_pct
    );
    println!("  Avg tokens/instance:  {:>12.0}", avg_tokens);
    println!();

    println!("📦 Token Categories");
    println!("  Category              Used    Available    Usage %");
    println!("  {}", "-".repeat(54));
    let byte_pct = (analysis.category_breakdown.byte_tokens_used as f64
        / analysis.category_breakdown.byte_tokens_available as f64)
        * 100.0;
    println!(
        "  Byte tokens (0-255)   {:>4}    {:>9}    {:>6.1}%",
        analysis.category_breakdown.byte_tokens_used,
        analysis.category_breakdown.byte_tokens_available,
        byte_pct
    );

    let struct_pct = (analysis.category_breakdown.structural_tokens_used as f64
        / analysis.category_breakdown.structural_tokens_available as f64)
        * 100.0;
    println!(
        "  Structural (256+)     {:>4}    {:>9}    {:>6.1}%",
        analysis.category_breakdown.structural_tokens_used,
        analysis.category_breakdown.structural_tokens_available,
        struct_pct
    );

    println!(
        "  Special (u32::MAX)    {:>4}    {:>9}    {:>6}",
        analysis.category_breakdown.special_tokens_used, 1, "100.0%"
    );
    println!();

    println!("🔥 Top 20 Most Frequent Tokens");
    println!("  Rank   Token          Count        % Total   Category");
    println!("  {}", "-".repeat(59));
    for (i, (token, freq)) in analysis.top_tokens.iter().enumerate() {
        let pct = (*freq as f64 / analysis.total_tokens as f64) * 100.0;
        let category = TokenCategory::from_token(*token);
        let category_str = format!("{:?}", category);
        let display = if *token <= 255 && *token >= 32 && *token <= 126 {
            format!("{} ('{}')", token, *token as u8 as char)
        } else {
            format!("{}", token)
        };
        println!(
            "  {:>4}   {:<14} {:>10}    {:>6.2}%   {}",
            i + 1,
            display,
            format_number(*freq),
            pct,
            category_str
        );
    }
    println!();

    println!("⚠️  Vocabulary Bloat");
    let rare_byte = analysis
        .rare_tokens
        .iter()
        .filter(|r| r.category == TokenCategory::Byte)
        .count();
    let rare_structural = analysis
        .rare_tokens
        .iter()
        .filter(|r| r.category == TokenCategory::Structural)
        .count();

    println!(
        "  Rare tokens (used < 10 times): {} tokens ({:.1}%)",
        analysis.rare_tokens.len(),
        (analysis.rare_tokens.len() as f64 / analysis.unique_tokens_used as f64) * 100.0
    );
    println!("    - {} byte tokens", rare_byte);
    println!("    - {} structural tokens", rare_structural);
    println!();
    println!(
        "  Unused tokens: {} tokens ({:.1}%)",
        analysis.unused_tokens, bloat_pct
    );

    // Calculate unused by category
    let unused_byte = analysis.category_breakdown.byte_tokens_available
        - analysis.category_breakdown.byte_tokens_used;
    let unused_structural = analysis.category_breakdown.structural_tokens_available
        - analysis.category_breakdown.structural_tokens_used;
    println!("    - {} byte tokens", unused_byte);
    println!("    - {} structural tokens", unused_structural);
    println!();

    println!("💡 Optimization Recommendations");
    let mut recommendations = Vec::new();

    // High vocabulary bloat
    if bloat_pct > 15.0 {
        recommendations.push(format!(
            "High vocabulary bloat ({:.1}%): Consider --max-vocab constraint to force multi-token encoding",
            bloat_pct
        ));
    }

    // Many rare structural tokens
    if rare_structural > 50 {
        recommendations.push(format!(
            "Many rare structural tokens ({}): Check for large enums or inefficient types",
            rare_structural
        ));
    }

    // ASCII-heavy data (low byte usage)
    if byte_pct < 70.0 {
        recommendations.push(format!(
            "ASCII-heavy data ({:.1}% byte usage): Excellent candidate for BPE compression",
            byte_pct
        ));
    }

    // Common string opportunity
    if rare_byte > 30 {
        recommendations.push(format!(
            "Many rare byte tokens ({}): Consider common string vocabulary optimization",
            rare_byte
        ));
    }

    for (i, rec) in recommendations.iter().enumerate() {
        println!("  {}. {}", i + 1, rec);
    }

    if recommendations.is_empty() {
        println!("  No major optimization opportunities identified");
    }
}

/// Format number with thousand separators
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_file(path: &str) -> FileProvenance {
        FileProvenance {
            path: path.to_string(),
            bytes: 3,
            sha256: "0".repeat(64),
        }
    }

    #[test]
    fn usage_analysis_uses_authoritative_base_vocab() {
        let provenance = UsageProvenance {
            command: vec!["tct-analyze-usage".to_string()],
            schema_name: "toy".to_string(),
            schema: "schema.json".to_string(),
            instances_pattern: "instances/*.json".to_string(),
            schema_file: fake_file("schema.json"),
            input_files: vec![fake_file("instances/one.json")],
            bpe_merges: None,
            bpe_merges_file: None,
            base_vocab_size: 257,
            base_vocab_source: "grammar".to_string(),
            optimization_level: "O3".to_string(),
            created_at: "2026-04-26T00:00:00Z".to_string(),
            tct_version: "0.1.0".to_string(),
        };

        let analysis = analyze_token_usage(&[vec![97, 97, 256]], 257, Some(provenance));

        assert_eq!(analysis.total_instances, 1);
        assert_eq!(analysis.unique_tokens_used, 2);
        assert_eq!(analysis.base_vocab_size, 257);
        assert_eq!(analysis.unused_tokens, 255);
        assert_eq!(
            analysis.provenance.as_ref().unwrap().base_vocab_source,
            "grammar"
        );
    }

    #[test]
    fn usage_analysis_json_includes_provenance_without_wrapping_report() {
        let analysis = analyze_token_usage(
            &[vec![1, 2]],
            257,
            Some(UsageProvenance {
                command: vec!["tct-analyze-usage".to_string()],
                schema_name: "toy".to_string(),
                schema: "schema.json".to_string(),
                instances_pattern: "instances/*.json".to_string(),
                schema_file: fake_file("schema.json"),
                input_files: vec![fake_file("instances/one.json")],
                bpe_merges: None,
                bpe_merges_file: None,
                base_vocab_size: 257,
                base_vocab_source: "grammar".to_string(),
                optimization_level: "O3".to_string(),
                created_at: "2026-04-26T00:00:00Z".to_string(),
                tct_version: "0.1.0".to_string(),
            }),
        );

        let value = serde_json::to_value(&analysis).unwrap();

        assert_eq!(value["base_vocab_size"], 257);
        assert_eq!(value["provenance"]["schema_name"], "toy");
        assert_eq!(value["provenance"]["schema_file"]["path"], "schema.json");
        assert_eq!(
            value["provenance"]["input_files"][0]["path"],
            "instances/one.json"
        );
        assert_eq!(value["provenance"]["bpe_merges"], serde_json::Value::Null);
    }
}
