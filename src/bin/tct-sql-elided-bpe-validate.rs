//! Validate elided-SQL BPE round-trip and exact-mask contract.
//!
//! For every selected corpus example (default: full Spider train):
//!   decode(expand(unbpe(bpe(elide(encode(sql)))))) re-encodes to the same
//!   elided stream (semantic fixpoint).
//! Optionally builds checked `SqlElidedBpeMaskReport` samples and a merge
//! ladder of mask-construction cost vs stream length.

use clap::{Arg, Command, value_parser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tct::bpe::BpeMergeTable;
use tct::provenance::sha256_hex;
use tct::sql_schema_linking::elided_bpe_mask::{
    bpe_codec_for_merges, build_sql_elided_bpe_mask_report, round_trip_sql_via_elided_bpe,
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
struct FailureDetail {
    example_id: String,
    db_id: String,
    error: String,
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
struct MergeLadderRow {
    merge_count: usize,
    base_vocab_size: u32,
    bpe_vocab_size: u32,
    total_vocab_size: u32,
    bpe_length_stats: LengthStats,
    /// Wall time to build full exact-mask report on the mask sample (seconds).
    mask_build_seconds: Option<f64>,
    mask_prefixes: Option<usize>,
    mask_report_validated: bool,
    /// Mean admitted BPE ids per gold prefix (from the sample report).
    mean_admitted_bpe_ids: Option<f64>,
    /// Mean rejected decisions per gold prefix.
    mean_rejected_bpe_ids: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ValidateReport {
    artifact_kind: &'static str,
    claim_bearing: bool,
    layout_diff_class: &'static str,
    stream_contract: &'static str,
    tct_commit: String,
    catalog_path: String,
    catalog_sha256: String,
    corpus_path: String,
    split: String,
    base_vocab_size: u32,
    pad_token: u32,
    n_round_trip_checked: usize,
    n_round_trip_failed: usize,
    /// Every failing example id (empty on full pass).
    failure_example_ids: Vec<String>,
    /// First few failures with messages (bounded for readability).
    first_failure_details: Vec<FailureDetail>,
    n_mask_sample: usize,
    mask_report_validated: bool,
    full_vocab_scan: bool,
    merge_count: usize,
    bpe_vocab_size: u32,
    mask_report_path: Option<String>,
    merge_ladder: Vec<MergeLadderRow>,
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
    let matches = Command::new("tct-sql-elided-bpe-validate")
        .about("Validate SQL elided BPE round-trip and exact-mask contract")
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
                .value_parser(value_parser!(PathBuf))
                .help("Full merge table (truncated via --merge-count / --ladder)"),
        )
        .arg(
            Arg::new("merge-count")
                .long("merge-count")
                .value_parser(value_parser!(usize))
                .help("Primary merge budget for full-corpus round-trip (default: all)"),
        )
        .arg(
            Arg::new("ladder")
                .long("ladder")
                .default_value("256,512,1024")
                .help("Comma-separated merge budgets for length + mask-cost ladder"),
        )
        .arg(Arg::new("split").long("split").default_value("train"))
        .arg(
            Arg::new("mask-sample")
                .long("mask-sample")
                .value_parser(value_parser!(usize))
                .default_value("32")
                .help("Number of examples for exact-mask report (0 = skip)"),
        )
        .arg(
            Arg::new("full-vocab-scan")
                .long("full-vocab-scan")
                .action(clap::ArgAction::SetTrue)
                .help("Brute-force every BPE id on the mask sample (expensive)"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("mask-report-output")
                .long("mask-report-output")
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let corpus_path = matches.get_one::<PathBuf>("corpus").expect("required");
    let tables_path = matches.get_one::<PathBuf>("tables").expect("required");
    let merges_path = matches.get_one::<PathBuf>("bpe-merges").expect("required");
    let split = matches.get_one::<String>("split").expect("defaulted");
    let mask_sample = *matches.get_one::<usize>("mask-sample").expect("defaulted");
    let full_vocab_scan = matches.get_flag("full-vocab-scan");
    let output = matches.get_one::<PathBuf>("output").expect("required");
    let mask_out = matches.get_one::<PathBuf>("mask-report-output");
    let ladder_raw = matches.get_one::<String>("ladder").expect("defaulted");

    let catalog_bytes = fs::read(tables_path)?;
    let catalog_sha = sha256_hex(&catalog_bytes);
    let corpus: CorpusFile = serde_json::from_str(&fs::read_to_string(corpus_path)?)?;
    let table = BpeMergeTable::from_file(merges_path)?;
    let merge_count = matches
        .get_one::<usize>("merge-count")
        .copied()
        .unwrap_or(table.merges.len());
    let (encoder, decoder, bpe_vocab_size) = bpe_codec_for_merges(&table, merge_count)?;

    let mut runtimes: HashMap<String, SqlSchemaLinkingRuntime> = HashMap::new();
    let mut n_ok = 0usize;
    let mut n_fail = 0usize;
    let mut failure_ids = Vec::new();
    let mut failure_details = Vec::new();
    let mut mask_queries: Vec<(String, String, String)> = Vec::new(); // id, db, sql
    let mut elided_lengths = Vec::new();
    let mut bpe_lengths = Vec::new();
    // Per-example elided streams for ladder length stats without re-encoding SQL.
    let mut train_elided: Vec<(String, String, Vec<u32>)> = Vec::new(); // id, db, elided

    for ex in &corpus.examples {
        if ex.split != *split || ex.excluded {
            continue;
        }
        let sql = match example_sql(ex) {
            Some(s) => s,
            None => {
                n_fail += 1;
                failure_ids.push(ex.example_id.clone());
                if failure_details.len() < 50 {
                    failure_details.push(FailureDetail {
                        example_id: ex.example_id.clone(),
                        db_id: ex.db_id.clone(),
                        error: "missing SQL".to_string(),
                    });
                }
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

        match round_trip_sql_via_elided_bpe(rt, &sql, &encoder, &decoder) {
            Ok(rendered) => {
                let again = rt.encode_elided(&rendered)?;
                let expected = rt.encode_elided(&sql)?;
                if again != expected {
                    n_fail += 1;
                    failure_ids.push(ex.example_id.clone());
                    if failure_details.len() < 50 {
                        failure_details.push(FailureDetail {
                            example_id: ex.example_id.clone(),
                            db_id: ex.db_id.clone(),
                            error: "re-encode mismatch after BPE round-trip".to_string(),
                        });
                    }
                } else {
                    n_ok += 1;
                    elided_lengths.push(expected.len());
                    let bpe = encoder.encode(&expected);
                    bpe_lengths.push(bpe.len());
                    train_elided.push((ex.example_id.clone(), ex.db_id.clone(), expected));
                    if mask_queries.len() < mask_sample {
                        mask_queries.push((ex.example_id.clone(), ex.db_id.clone(), sql));
                    }
                }
            }
            Err(e) => {
                n_fail += 1;
                failure_ids.push(ex.example_id.clone());
                if failure_details.len() < 50 {
                    failure_details.push(FailureDetail {
                        example_id: ex.example_id.clone(),
                        db_id: ex.db_id.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }
        if (n_ok + n_fail).is_multiple_of(500) {
            eprintln!("  checked {} (ok={n_ok} fail={n_fail})...", n_ok + n_fail);
        }
    }

    // Primary mask report (primary merge budget) on the sample.
    let mut mask_validated = false;
    let mut mask_report_path = None;
    if mask_sample > 0 && !mask_queries.is_empty() {
        let mut by_db: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (id, db, sql) in &mask_queries {
            by_db
                .entry(db.clone())
                .or_default()
                .push((id.clone(), sql.clone()));
        }
        for (db_id, qs) in &by_db {
            let rt = runtimes
                .get(db_id)
                .ok_or_else(|| format!("missing runtime for db_id {db_id} in mask sample"))?;
            let report =
                build_sql_elided_bpe_mask_report(rt, qs, &table, merge_count, full_vocab_scan)?;
            report
                .validate()
                .map_err(|e| format!("mask report validate ({db_id}): {e}"))?;
            if let Some(path) = mask_out {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let out_path = if by_db.len() == 1 {
                    path.clone()
                } else {
                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("mask-report");
                    path.with_file_name(format!("{stem}.{db_id}.json"))
                };
                fs::write(&out_path, serde_json::to_string_pretty(&report)?)?;
                mask_report_path = Some(out_path.display().to_string());
            }
        }
        mask_validated = true;
    }

    // Merge ladder: stream lengths over full train + mask build cost on sample.
    let mut ladder_counts: Vec<usize> = ladder_raw
        .split(',')
        .map(|s| s.trim().parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid --ladder: {e}"))?;
    ladder_counts.sort_unstable();
    ladder_counts.dedup();

    let mut merge_ladder = Vec::new();
    for &n_merges in &ladder_counts {
        let n_eff = n_merges.min(table.merges.len());
        let (enc, _dec, vocab) = bpe_codec_for_merges(&table, n_eff)?;
        let lengths: Vec<usize> = train_elided
            .iter()
            .map(|(_, _, elided)| enc.encode(elided).len())
            .collect();
        let stats = length_stats(&lengths);

        let mut mask_build_seconds = None;
        let mut mask_prefixes = None;
        let mut mask_ok = false;
        let mut mean_admitted = None;
        let mut mean_rejected = None;
        if mask_sample > 0 && !mask_queries.is_empty() {
            // Time mask construction on first db group only (representative cost).
            let first_db = &mask_queries[0].1;
            let qs: Vec<(String, String)> = mask_queries
                .iter()
                .filter(|(_, db, _)| db == first_db)
                .map(|(id, _, sql)| (id.clone(), sql.clone()))
                .collect();
            if let Some(rt) = runtimes.get(first_db) {
                let t0 = Instant::now();
                let report =
                    build_sql_elided_bpe_mask_report(rt, &qs, &table, n_eff, full_vocab_scan)?;
                let elapsed = t0.elapsed().as_secs_f64();
                report
                    .validate()
                    .map_err(|e| format!("ladder mask validate m{n_eff}: {e}"))?;
                mask_build_seconds = Some(elapsed);
                mask_prefixes = Some(report.bpe_prefix_masks.len());
                if !report.bpe_prefix_masks.is_empty() {
                    let n = report.bpe_prefix_masks.len() as f64;
                    mean_admitted = Some(
                        report
                            .bpe_prefix_masks
                            .iter()
                            .map(|m| m.allowed_bpe_token_ids.len())
                            .sum::<usize>() as f64
                            / n,
                    );
                    mean_rejected = Some(
                        report
                            .bpe_prefix_masks
                            .iter()
                            .map(|m| m.rejected_count)
                            .sum::<usize>() as f64
                            / n,
                    );
                }
                mask_ok = true;
            }
        }

        eprintln!(
            "  ladder m{n_eff}: median_bpe={} mask_build_s={:?} prefixes={:?}",
            stats.median_tokens, mask_build_seconds, mask_prefixes
        );
        merge_ladder.push(MergeLadderRow {
            merge_count: n_eff,
            base_vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
            bpe_vocab_size: vocab,
            total_vocab_size: vocab, // BPE vocab size already includes base
            bpe_length_stats: stats,
            mask_build_seconds,
            mask_prefixes,
            mask_report_validated: mask_ok,
            mean_admitted_bpe_ids: mean_admitted,
            mean_rejected_bpe_ids: mean_rejected,
        });
    }

    let report = ValidateReport {
        artifact_kind: "sql_elided_bpe_validate_report_v1",
        claim_bearing: false,
        layout_diff_class: "downstream_bpe_artifact_not_token_layout_change",
        stream_contract: "elided",
        tct_commit: git_commit(),
        catalog_path: tables_path.display().to_string(),
        catalog_sha256: catalog_sha,
        corpus_path: corpus_path.display().to_string(),
        split: split.clone(),
        base_vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        n_round_trip_checked: n_ok,
        n_round_trip_failed: n_fail,
        failure_example_ids: failure_ids,
        first_failure_details: failure_details,
        n_mask_sample: mask_queries.len(),
        mask_report_validated: mask_validated,
        full_vocab_scan,
        merge_count,
        bpe_vocab_size,
        mask_report_path,
        merge_ladder,
        note: "BPE IDs are a downstream artifact over the elided SQL stream; not a \
               token-layout pin change (exact_reuse of base SQL layout). Round-trip is \
               semantic fixpoint via re-encode of strict_decode_complete_elided."
            .to_string(),
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, serde_json::to_string_pretty(&report)?)?;
    eprintln!(
        "round-trip ok={} fail={} mask_validated={} elided_median≈{:.1} bpe_median≈{:.1} → {}",
        n_ok,
        n_fail,
        mask_validated,
        length_stats(&elided_lengths).median_tokens,
        length_stats(&bpe_lengths).median_tokens,
        output.display()
    );
    if n_fail > 0 {
        eprintln!(
            "FAILING EXAMPLE IDS ({}): {}",
            n_fail,
            report.failure_example_ids.join(", ")
        );
        std::process::exit(1);
    }
    Ok(())
}
