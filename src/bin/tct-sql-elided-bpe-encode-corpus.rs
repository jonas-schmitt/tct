//! Encode Spider SQL train examples to elided+BPE token streams with provenance.
//!
//! Each JSONL record carries BPE token ids, merge-table path/sha, base and total
//! vocab sizes, stream contract, and the producing tct commit so the models side
//! can train without guessing runtime versioning.
//!
//! Report-only; not a paper claim. BPE is a downstream artifact over the elided
//! stream (not a token-layout pin change).

use clap::{Arg, Command, value_parser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use tct::bpe::BpeMergeTable;
use tct::provenance::sha256_hex;
use tct::sql_schema_linking::elided_bpe_mask::{
    bpe_codec_for_merges, round_trip_sql_via_elided_bpe,
};
use tct::sql_schema_linking::runtime::SqlSchemaLinkingRuntime;
use tct::sql_schema_linking::{SQL_SYNTAX_LAYOUT_PAD_TOKEN, SQL_SYNTAX_LAYOUT_VOCAB_SIZE};

#[derive(Debug, Deserialize)]
struct CorpusFile {
    examples: Vec<CorpusExample>,
}

#[derive(Debug, Deserialize)]
struct CorpusExample {
    example_id: String,
    db_id: String,
    split: String,
    #[serde(default)]
    excluded: bool,
    #[serde(default)]
    rendered_sql: Option<String>,
    #[serde(default)]
    gold_sql: Option<String>,
    #[serde(default)]
    original_sql: Option<String>,
}

#[derive(Debug, Serialize)]
struct EncodeRecord {
    example_id: String,
    db_id: String,
    split: String,
    stream_contract: &'static str,
    elided_token_ids: Vec<u32>,
    bpe_token_ids: Vec<u32>,
    elided_len: usize,
    bpe_len: usize,
    base_vocab_size: u32,
    pad_token: u32,
    merge_count: usize,
    bpe_vocab_size: u32,
    total_vocab_size: u32,
    merge_table_path: String,
    merge_table_sha256: String,
    merge_table_schema: String,
    catalog_path: String,
    catalog_sha256: String,
    tct_commit: String,
    tool: &'static str,
    version: u32,
    claim_bearing: bool,
}

#[derive(Debug, Serialize)]
struct LengthStats {
    records: usize,
    total_tokens: usize,
    min_tokens: usize,
    max_tokens: usize,
    mean_tokens: f64,
    median_tokens: f64,
    p90_tokens: f64,
}

#[derive(Debug, Serialize)]
struct EncodeReport {
    artifact_kind: &'static str,
    claim_bearing: bool,
    layout_diff_class: &'static str,
    stream_contract: &'static str,
    tct_commit: String,
    catalog_path: String,
    catalog_sha256: String,
    corpus_path: String,
    merge_table_path: String,
    merge_table_sha256: String,
    merge_count: usize,
    base_vocab_size: u32,
    pad_token: u32,
    bpe_vocab_size: u32,
    total_vocab_size: u32,
    split: String,
    n_encoded: usize,
    n_failed: usize,
    n_skipped_excluded: usize,
    n_skipped_wrong_split: usize,
    n_round_trip_verified: usize,
    elided_length_stats: LengthStats,
    bpe_length_stats: LengthStats,
    failure_example_ids: Vec<String>,
    output_jsonl: String,
    output_report: String,
    note: String,
}

fn length_stats(lengths: &[usize]) -> LengthStats {
    if lengths.is_empty() {
        return LengthStats {
            records: 0,
            total_tokens: 0,
            min_tokens: 0,
            max_tokens: 0,
            mean_tokens: 0.0,
            median_tokens: 0.0,
            p90_tokens: 0.0,
        };
    }
    let mut ordered = lengths.to_vec();
    ordered.sort_unstable();
    let total: usize = ordered.iter().sum();
    let n = ordered.len();
    let median = if n.is_multiple_of(2) {
        (ordered[n / 2 - 1] + ordered[n / 2]) as f64 / 2.0
    } else {
        ordered[n / 2] as f64
    };
    let p90_idx = ((n as f64 - 1.0) * 0.90).round() as usize;
    LengthStats {
        records: n,
        total_tokens: total,
        min_tokens: ordered[0],
        max_tokens: ordered[n - 1],
        mean_tokens: total as f64 / n as f64,
        median_tokens: median,
        p90_tokens: ordered[p90_idx.min(n - 1)] as f64,
    }
}

fn git_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn example_sql(ex: &CorpusExample) -> Option<String> {
    ex.rendered_sql
        .as_deref()
        .or(ex.gold_sql.as_deref())
        .or(ex.original_sql.as_deref())
        .map(str::to_string)
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-sql-elided-bpe-encode-corpus")
        .about("Encode Spider train SQL to elided+BPE streams with provenance")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("corpus")
                .long("corpus")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("tables")
                .long("tables")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("bpe-merges")
                .long("bpe-merges")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("merge-count")
                .long("merge-count")
                .value_parser(value_parser!(usize))
                .help("Use first N merges (default: all in table)"),
        )
        .arg(Arg::new("split").long("split").default_value("train"))
        .arg(
            Arg::new("output-jsonl")
                .long("output-jsonl")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output-report")
                .long("output-report")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("verify-round-trip")
                .long("verify-round-trip")
                .action(clap::ArgAction::SetTrue)
                .help("Also run elided+BPE round-trip on every example (slower)"),
        )
        .get_matches();

    let corpus_path = matches.get_one::<PathBuf>("corpus").expect("required");
    let tables_path = matches.get_one::<PathBuf>("tables").expect("required");
    let merges_path = matches.get_one::<PathBuf>("bpe-merges").expect("required");
    let split = matches.get_one::<String>("split").expect("defaulted");
    let out_jsonl = matches
        .get_one::<PathBuf>("output-jsonl")
        .expect("required");
    let out_report = matches
        .get_one::<PathBuf>("output-report")
        .expect("required");
    let verify_rt = matches.get_flag("verify-round-trip");

    let catalog_bytes = fs::read(tables_path)?;
    let catalog_sha = sha256_hex(&catalog_bytes);
    let merge_bytes = fs::read(merges_path)?;
    let merge_sha = sha256_hex(&merge_bytes);
    let table = BpeMergeTable::from_file(merges_path)?;
    let merge_count = matches
        .get_one::<usize>("merge-count")
        .copied()
        .unwrap_or(table.merges.len());
    let (encoder, decoder, bpe_vocab_size) = bpe_codec_for_merges(&table, merge_count)?;
    let tct_commit = git_commit();
    let corpus: CorpusFile = serde_json::from_str(&fs::read_to_string(corpus_path)?)?;

    if let Some(parent) = out_jsonl.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = BufWriter::new(File::create(out_jsonl)?);
    let mut runtimes: HashMap<String, SqlSchemaLinkingRuntime> = HashMap::new();
    let mut n_encoded = 0usize;
    let mut n_failed = 0usize;
    let mut n_skip_excl = 0usize;
    let mut n_skip_split = 0usize;
    let mut n_rt_ok = 0usize;
    let mut elided_lens = Vec::new();
    let mut bpe_lens = Vec::new();
    let mut failures = Vec::new();

    for ex in &corpus.examples {
        if ex.split != *split {
            n_skip_split += 1;
            continue;
        }
        if ex.excluded {
            n_skip_excl += 1;
            continue;
        }
        let sql = match example_sql(ex) {
            Some(s) => s,
            None => {
                n_failed += 1;
                failures.push(ex.example_id.clone());
                continue;
            }
        };
        let rt = match runtimes.get(&ex.db_id) {
            Some(rt) => rt,
            None => {
                let built = SqlSchemaLinkingRuntime::from_catalog_json_bytes(
                    &catalog_bytes,
                    &catalog_sha,
                    &ex.db_id,
                )?;
                runtimes.insert(ex.db_id.clone(), built);
                runtimes.get(&ex.db_id).expect("inserted")
            }
        };

        let elided = match rt.encode_elided(&sql) {
            Ok(e) => e,
            Err(e) => {
                n_failed += 1;
                failures.push(ex.example_id.clone());
                eprintln!("encode_elided {}: {e}", ex.example_id);
                continue;
            }
        };
        let bpe = encoder.encode(&elided);

        if verify_rt {
            match round_trip_sql_via_elided_bpe(rt, &sql, &encoder, &decoder) {
                Ok(rendered) => {
                    let again = rt.encode_elided(&rendered)?;
                    if again != elided {
                        n_failed += 1;
                        failures.push(ex.example_id.clone());
                        eprintln!("round-trip re-encode mismatch {}", ex.example_id);
                        continue;
                    }
                    n_rt_ok += 1;
                }
                Err(e) => {
                    n_failed += 1;
                    failures.push(ex.example_id.clone());
                    eprintln!("round-trip {}: {e}", ex.example_id);
                    continue;
                }
            }
        }

        let rec = EncodeRecord {
            example_id: ex.example_id.clone(),
            db_id: ex.db_id.clone(),
            split: ex.split.clone(),
            stream_contract: "elided",
            elided_len: elided.len(),
            bpe_len: bpe.len(),
            elided_token_ids: elided,
            bpe_token_ids: bpe,
            base_vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
            pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
            merge_count: merge_count.min(table.merges.len()),
            bpe_vocab_size,
            total_vocab_size: bpe_vocab_size,
            merge_table_path: merges_path.display().to_string(),
            merge_table_sha256: merge_sha.clone(),
            merge_table_schema: table.schema.clone(),
            catalog_path: tables_path.display().to_string(),
            catalog_sha256: catalog_sha.clone(),
            tct_commit: tct_commit.clone(),
            tool: "tct-sql-elided-bpe-encode-corpus",
            version: 1,
            claim_bearing: false,
        };
        elided_lens.push(rec.elided_len);
        bpe_lens.push(rec.bpe_len);
        serde_json::to_writer(&mut writer, &rec)?;
        writer.write_all(b"\n")?;
        n_encoded += 1;
        if n_encoded.is_multiple_of(500) {
            eprintln!("  encoded {n_encoded}...");
        }
    }
    writer.flush()?;

    let report = EncodeReport {
        artifact_kind: "sql_elided_bpe_encode_corpus_report_v1",
        claim_bearing: false,
        layout_diff_class: "downstream_bpe_artifact_not_token_layout_change",
        stream_contract: "elided",
        tct_commit,
        catalog_path: tables_path.display().to_string(),
        catalog_sha256: catalog_sha,
        corpus_path: corpus_path.display().to_string(),
        merge_table_path: merges_path.display().to_string(),
        merge_table_sha256: merge_sha,
        merge_count: merge_count.min(table.merges.len()),
        base_vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        bpe_vocab_size,
        total_vocab_size: bpe_vocab_size,
        split: split.clone(),
        n_encoded,
        n_failed,
        n_skipped_excluded: n_skip_excl,
        n_skipped_wrong_split: n_skip_split,
        n_round_trip_verified: n_rt_ok,
        elided_length_stats: length_stats(&elided_lens),
        bpe_length_stats: length_stats(&bpe_lens),
        failure_example_ids: failures,
        output_jsonl: out_jsonl.display().to_string(),
        output_report: out_report.display().to_string(),
        note: "Per-record streams are runtime-versioned via tct_commit + merge_table_sha256. \
               BPE is a downstream artifact; base SQL layout pin is unchanged (exact_reuse)."
            .to_string(),
    };
    if let Some(parent) = out_report.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_report, serde_json::to_string_pretty(&report)?)?;
    eprintln!(
        "encoded={} failed={} elided_median={:.1} bpe_median={:.1} → {}",
        n_encoded,
        n_failed,
        report.elided_length_stats.median_tokens,
        report.bpe_length_stats.median_tokens,
        out_jsonl.display()
    );
    if n_failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}
