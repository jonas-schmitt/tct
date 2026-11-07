//! Report per-sequence length stats for base or BPE-compressed token JSONL.
//!
//! When `--bpe-merges` is set, applies [`SequentialBpeEncoder`] (trainer-equivalent)
//! before measuring. Emits JSON with median / mean / p90.

use clap::{Arg, Command, value_parser};
use serde::Serialize;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tct::bpe::BpeMergeTable;
use tct::bpe::encoder::SequentialBpeEncoder;

#[derive(Debug, Serialize)]
struct LengthStats {
    records: usize,
    total_tokens: usize,
    min_tokens: usize,
    max_tokens: usize,
    mean_tokens: f64,
    median_tokens: f64,
    p90_tokens: f64,
    percentile_method: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    artifact_kind: &'static str,
    encoded_input: String,
    bpe_merges: Option<String>,
    effective_merges: Option<usize>,
    base_vocab_size: Option<u32>,
    bpe_vocab_size: Option<u32>,
    token_length_stats: LengthStats,
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-bpe-length-stats")
        .about("Length stats for token JSONL, optionally after SequentialBpeEncoder")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("encoded")
                .long("encoded")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("bpe-merges")
                .long("bpe-merges")
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let encoded = matches.get_one::<PathBuf>("encoded").expect("required");
    let sequences = load_jsonl(encoded)?;
    if sequences.is_empty() {
        return Err("no sequences loaded".into());
    }

    let (measured, meta) = if let Some(merges_path) = matches.get_one::<PathBuf>("bpe-merges") {
        let table = BpeMergeTable::from_file(merges_path)?;
        let bpe_start = table.bpe_start();
        let bpe_vocab_size = bpe_start
            .checked_add(u32::try_from(table.merges.len()).map_err(|_| "merge count exceeds u32")?)
            .ok_or("BPE vocab overflow")?;
        let encoder = SequentialBpeEncoder::new(&table, bpe_vocab_size);
        let bpe_seqs: Vec<Vec<u32>> = sequences.iter().map(|s| encoder.encode(s)).collect();
        (
            bpe_seqs,
            Some((
                merges_path.display().to_string(),
                table.merges.len(),
                table.base_vocab_size,
                bpe_vocab_size,
            )),
        )
    } else {
        (sequences, None)
    };

    let stats = summarize(&measured.iter().map(|s| s.len()).collect::<Vec<_>>());
    let report = Report {
        artifact_kind: "tct_bpe_length_stats_v1",
        encoded_input: encoded.display().to_string(),
        bpe_merges: meta.as_ref().map(|m| m.0.clone()),
        effective_merges: meta.as_ref().map(|m| m.1),
        base_vocab_size: meta.as_ref().map(|m| m.2),
        bpe_vocab_size: meta.as_ref().map(|m| m.3),
        token_length_stats: stats,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn load_jsonl(path: &PathBuf) -> Result<Vec<Vec<u32>>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut sequences = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let tokens: Vec<u32> =
            serde_json::from_str(&line).map_err(|e| format!("line {}: {e}", i + 1))?;
        sequences.push(tokens);
    }
    Ok(sequences)
}

fn summarize(lengths: &[usize]) -> LengthStats {
    let mut sorted = lengths.to_vec();
    sorted.sort_unstable();
    let records = sorted.len();
    let total: usize = sorted.iter().sum();
    LengthStats {
        records,
        total_tokens: total,
        min_tokens: sorted[0],
        max_tokens: sorted[records - 1],
        mean_tokens: total as f64 / records as f64,
        median_tokens: percentile_linear(&sorted, 0.50),
        p90_tokens: percentile_linear(&sorted, 0.90),
        percentile_method: "linear_interpolation",
    }
}

fn percentile_linear(sorted: &[usize], p: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0] as f64;
    }
    let rank = p * (sorted.len() as f64 - 1.0);
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted[lo] as f64
    } else {
        let w = rank - lo as f64;
        sorted[lo] as f64 * (1.0 - w) + sorted[hi] as f64 * w
    }
}
