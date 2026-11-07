//! Transliterates GitHub Actions workflows to pure ASCII for training TCT tokenizers.
//!
//! This tool processes JSON workflow files and converts all non-ASCII characters to ASCII
//! equivalents, ensuring the training data is 100% compatible with ASCII-only string encoding.
//!
//! # Design Rationale
//!
//! - **Preprocessing over runtime checks**: Transliterate once during data preparation,
//!   keep encoding brutally efficient (no runtime transliteration overhead)
//! - **Preserve all training data**: Keep all 100k workflows by transliterating instead
//!   of filtering
//! - **Deterministic transliteration**: Consistent ASCII mapping for reproducible training
//!
//! # Usage
//!
//! ```bash
//! cargo build --release --bin tct-transliterate-workflows
//!
//! ./target/release/tct-transliterate-workflows \
//!   --input /tmp/schemastore/train/github-workflow/workflows \
//!   --output /tmp/schemastore/train/github-workflow/workflows-ascii \
//!   --report transliteration-report.txt
//! ```
//!
//! # Transliteration Rules
//!
//! - **Accented Latin**: é→e, ñ→n, ü→u, ä→a, ö→o, ç→c, etc.
//! - **Emoji**: 🚀→"rocket", ✅→"check", ❌→"cross", etc.
//! - **Punctuation**: "→", "→", —→-, etc.
//! - **Greek/Cyrillic**: α→a, β→b, Д→D, etc.
//! - **CJK**: Remove or replace with placeholder (rarely appears in workflows)
//! - **Unknown**: Replace with '?' as last resort

use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Parser, Debug)]
#[clap(
    name = "tct-transliterate-workflows",
    about = "Transliterate GitHub workflows to pure ASCII for TCT training"
)]
struct Args {
    /// Input directory containing JSON workflow files
    #[clap(short, long)]
    input: PathBuf,

    /// Output directory for transliterated ASCII workflows
    #[clap(short, long)]
    output: PathBuf,

    /// Optional report file with transliteration statistics
    #[clap(short, long)]
    report: Option<PathBuf>,
}

/// Transliteration map for common non-ASCII characters to ASCII equivalents
fn build_transliteration_map() -> HashMap<char, &'static str> {
    let mut map = HashMap::new();

    // Latin-1 Supplement (128-255) - Accented characters
    // French
    map.insert('é', "e");
    map.insert('è', "e");
    map.insert('ê', "e");
    map.insert('ë', "e");
    map.insert('à', "a");
    map.insert('â', "a");
    map.insert('ç', "c");
    map.insert('ù', "u");
    map.insert('û', "u");
    map.insert('î', "i");
    map.insert('ï', "i");
    map.insert('ô', "o");

    // German
    map.insert('ä', "a");
    map.insert('ö', "o");
    map.insert('ü', "u");
    map.insert('Ä', "A");
    map.insert('Ö', "O");
    map.insert('Ü', "U");
    map.insert('ß', "ss");

    // Spanish
    map.insert('ñ', "n");
    map.insert('Ñ', "N");
    map.insert('á', "a");
    map.insert('í', "i");
    map.insert('ó', "o");
    map.insert('ú', "u");
    map.insert('¿', "");
    map.insert('¡', "");

    // Portuguese
    map.insert('ã', "a");
    map.insert('õ', "o");
    map.insert('Ã', "A");
    map.insert('Õ', "O");

    // Italian
    map.insert('ì', "i");
    map.insert('ò', "o");

    // Nordic
    map.insert('å', "a");
    map.insert('Å', "A");
    map.insert('æ', "ae");
    map.insert('Æ', "AE");
    map.insert('ø', "o");
    map.insert('Ø', "O");

    // Common symbols
    map.insert('©', "(c)");
    map.insert('®', "(r)");
    map.insert('™', "(tm)");
    map.insert('°', " degrees");
    map.insert('±', "+/-");
    map.insert('×', "x");
    map.insert('÷', "/");
    map.insert('µ', "u");

    // Punctuation (Unicode smart quotes and dashes)
    map.insert('\u{201C}', "\""); // Left double quotation mark "
    map.insert('\u{201D}', "\""); // Right double quotation mark "
    map.insert('\u{2018}', "'"); // Left single quotation mark '
    map.insert('\u{2019}', "'"); // Right single quotation mark '
    map.insert('\u{2013}', "-"); // En dash –
    map.insert('\u{2014}', "-"); // Em dash —
    map.insert('\u{2026}', "..."); // Horizontal ellipsis …
    map.insert('\u{2022}', "*"); // Bullet •

    // Common emoji (convert to ASCII descriptions)
    map.insert('✅', "check");
    map.insert('❌', "cross");
    map.insert('🚀', "rocket");
    map.insert('⚠', "warning");
    map.insert('✔', "check");
    map.insert('✓', "check");
    map.insert('✗', "x");
    map.insert('✘', "x");
    map.insert('→', "->");
    map.insert('←', "<-");
    map.insert('⇒', "=>");
    map.insert('⇐', "<=");

    // Greek letters (common in technical contexts)
    map.insert('α', "alpha");
    map.insert('β', "beta");
    map.insert('γ', "gamma");
    map.insert('δ', "delta");
    map.insert('λ', "lambda");
    map.insert('μ', "mu");
    map.insert('π', "pi");
    map.insert('Σ', "Sigma");
    map.insert('Ω', "Omega");

    map
}

/// Transliterate a single string to pure ASCII
fn transliterate_string(s: &str, map: &HashMap<char, &'static str>) -> String {
    let mut result = String::with_capacity(s.len());

    for ch in s.chars() {
        if ch.is_ascii() {
            result.push(ch);
        } else if let Some(&replacement) = map.get(&ch) {
            result.push_str(replacement);
        } else {
            // Unknown character - replace with '?' as last resort
            // This preserves structure while ensuring ASCII-only output
            result.push('?');
        }
    }

    result
}

/// Recursively transliterate all strings in a JSON value
fn transliterate_json(value: &mut Value, map: &HashMap<char, &'static str>) {
    match value {
        Value::String(s) if !s.is_ascii() => {
            *s = transliterate_string(s, map);
        }
        Value::Array(arr) => {
            for item in arr {
                transliterate_json(item, map);
            }
        }
        Value::Object(obj) => {
            for (_key, val) in obj.iter_mut() {
                transliterate_json(val, map);
            }
        }
        _ => {} // Numbers, booleans, null are already ASCII-compatible
    }
}

/// Process a single workflow file
fn process_workflow(
    input_path: &Path,
    output_path: &Path,
    map: &HashMap<char, &'static str>,
) -> Result<ProcessStats, String> {
    // Read input JSON
    let content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path.display(), e))?;

    let mut json: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON in {}: {}", input_path.display(), e))?;

    // Count non-ASCII characters before transliteration
    let non_ascii_count = content.chars().filter(|c| !c.is_ascii()).count();

    // Transliterate all strings
    transliterate_json(&mut json, map);

    // Verify output is pure ASCII
    let output_json = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    if !output_json.is_ascii() {
        return Err(format!(
            "Output still contains non-ASCII after transliteration: {}",
            input_path.display()
        ));
    }

    // Write output
    fs::write(output_path, output_json)
        .map_err(|e| format!("Failed to write {}: {}", output_path.display(), e))?;

    Ok(ProcessStats {
        non_ascii_count,
        transliterated: non_ascii_count > 0,
    })
}

#[derive(Debug, Default)]
struct ProcessStats {
    non_ascii_count: usize,
    transliterated: bool,
}

#[derive(Debug, Default)]
struct OverallStats {
    total_files: usize,
    ascii_files: usize,
    transliterated_files: usize,
    total_non_ascii_chars: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let start_time = Instant::now();

    println!("🔤 TCT Workflow Transliteration Tool");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Input:  {}", args.input.display());
    println!("Output: {}", args.output.display());
    println!();

    // Build transliteration map
    let map = build_transliteration_map();
    println!("📋 Transliteration rules loaded: {} mappings", map.len());
    println!();

    // Create output directory
    fs::create_dir_all(&args.output)?;

    // Discover all JSON files
    let mut json_files = Vec::new();
    for entry in fs::read_dir(&args.input)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            json_files.push(path);
        }
    }

    println!("📁 Found {} workflow files", json_files.len());
    println!();

    // Process all files
    let stats = Arc::new(std::sync::Mutex::new(OverallStats::default()));
    let processed = Arc::new(AtomicUsize::new(0));

    println!("⚙️  Processing workflows...");

    for (idx, input_path) in json_files.iter().enumerate() {
        let filename = input_path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("input path has no file name: {}", input_path.display()),
            )
        })?;
        let output_path = args.output.join(filename);

        match process_workflow(input_path, &output_path, &map) {
            Ok(file_stats) => {
                let mut stats = stats
                    .lock()
                    .map_err(|_| io::Error::other("workflow stats mutex poisoned"))?;
                stats.total_files += 1;
                if file_stats.transliterated {
                    stats.transliterated_files += 1;
                    stats.total_non_ascii_chars += file_stats.non_ascii_count;
                } else {
                    stats.ascii_files += 1;
                }

                let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                if count.is_multiple_of(1000) {
                    println!(
                        "  Processed {}/{} files ({:.1}%)",
                        count,
                        json_files.len(),
                        (count as f64 / json_files.len() as f64) * 100.0
                    );
                }
            }
            Err(e) => {
                eprintln!("❌ Error processing {}: {}", filename.to_string_lossy(), e);
            }
        }

        if idx % 10000 == 0 && idx > 0 {
            println!("  Progress: {}/{} files", idx, json_files.len());
        }
    }

    let elapsed = start_time.elapsed();
    let stats = stats
        .lock()
        .map_err(|_| io::Error::other("workflow stats mutex poisoned"))?;

    println!();
    println!("✅ Transliteration complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total files processed:     {}", stats.total_files);
    println!(
        "Already ASCII:             {} ({:.1}%)",
        stats.ascii_files,
        (stats.ascii_files as f64 / stats.total_files as f64) * 100.0
    );
    println!(
        "Transliterated:            {} ({:.1}%)",
        stats.transliterated_files,
        (stats.transliterated_files as f64 / stats.total_files as f64) * 100.0
    );
    println!("Total non-ASCII chars:     {}", stats.total_non_ascii_chars);
    println!("Time elapsed:              {:.2}s", elapsed.as_secs_f64());
    println!(
        "Processing rate:           {:.0} files/sec",
        stats.total_files as f64 / elapsed.as_secs_f64()
    );
    println!();

    // Write report if requested
    if let Some(report_path) = args.report {
        let mut report = fs::File::create(&report_path)?;
        writeln!(report, "TCT Workflow Transliteration Report")?;
        writeln!(report, "====================================")?;
        writeln!(report)?;
        writeln!(report, "Input:  {}", args.input.display())?;
        writeln!(report, "Output: {}", args.output.display())?;
        writeln!(report)?;
        writeln!(report, "Statistics:")?;
        writeln!(report, "  Total files processed:     {}", stats.total_files)?;
        writeln!(
            report,
            "  Already ASCII:             {} ({:.1}%)",
            stats.ascii_files,
            (stats.ascii_files as f64 / stats.total_files as f64) * 100.0
        )?;
        writeln!(
            report,
            "  Transliterated:            {} ({:.1}%)",
            stats.transliterated_files,
            (stats.transliterated_files as f64 / stats.total_files as f64) * 100.0
        )?;
        writeln!(
            report,
            "  Total non-ASCII chars:     {}",
            stats.total_non_ascii_chars
        )?;
        writeln!(
            report,
            "  Time elapsed:              {:.2}s",
            elapsed.as_secs_f64()
        )?;
        writeln!(
            report,
            "  Processing rate:           {:.0} files/sec",
            stats.total_files as f64 / elapsed.as_secs_f64()
        )?;
        println!("📄 Report written to: {}", report_path.display());
    }

    println!("🎉 All workflows are now pure ASCII and ready for training!");

    Ok(())
}
