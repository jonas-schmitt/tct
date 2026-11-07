//! Encode Spider SQL train examples to canonical elided TCT streams.
//!
//! Reads a stage-1 indexed corpus (with `db_id` + SQL) and Spider-style
//! `tables.json`, builds a cached [`SqlSchemaLinkingRuntime`] per `db_id`, and
//! writes one JSON array of decision token IDs per line for `tct-train-bpe
//! --encoded`.
//!
//! Report-only stats JSON (median/mean/p90 expanded vs elided lengths) is
//! written beside the JSONL. Not a paper claim.

use clap::{Arg, Command, value_parser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use tct::provenance::sha256_hex;
use tct::sql_schema_linking::menu_binding::{SqlColumnAddressing, SqlJoinOnPolicy};
use tct::sql_schema_linking::runtime::SqlSchemaLinkingRuntime;
use tct::sql_schema_linking::{SQL_SYNTAX_LAYOUT_PAD_TOKEN, SQL_SYNTAX_LAYOUT_VOCAB_SIZE};

#[derive(Debug, Deserialize)]
struct CorpusFile {
    examples: Vec<CorpusExample>,
    #[serde(default)]
    n_train_supported: Option<usize>,
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
    #[serde(default)]
    tct_decision_ids: Option<Vec<u32>>,
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
    catalog_path: String,
    catalog_sha256: String,
    corpus_path: String,
    split: String,
    mode: String,
    base_vocab_size: u32,
    pad_token: u32,
    join_on_policy: &'static str,
    column_addressing: &'static str,
    visit_all_fields: bool,
    stream_contract: &'static str,
    n_requested: usize,
    n_encoded: usize,
    n_failed: usize,
    n_skipped_excluded: usize,
    n_skipped_wrong_split: usize,
    n_runtimes: usize,
    sample_limit: Option<usize>,
    expanded_length_stats: Option<LengthStats>,
    elided_length_stats: LengthStats,
    failure_examples: Vec<FailureExample>,
    output_jsonl: String,
    output_report: String,
}

#[derive(Debug, Serialize)]
struct FailureExample {
    example_id: String,
    db_id: String,
    error: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-sql-elided-encode-corpus")
        .about("Encode Spider train SQL to elided TCT decision-token JSONL")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("corpus")
                .long("corpus")
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help("stage1_corpus_indexed.json (or equivalent)"),
        )
        .arg(
            Arg::new("tables")
                .long("tables")
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help("Spider-style tables.json"),
        )
        .arg(
            Arg::new("output-jsonl")
                .long("output-jsonl")
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help("Output token sequences JSONL (one Vec<u32> per line)"),
        )
        .arg(
            Arg::new("output-report")
                .long("output-report")
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help("JSON report with length stats and failures"),
        )
        .arg(
            Arg::new("split")
                .long("split")
                .default_value("train")
                .help("Corpus split to encode (default: train)"),
        )
        .arg(
            Arg::new("sample-limit")
                .long("sample-limit")
                .value_parser(value_parser!(usize))
                .help("Optional cap on number of successfully encoded examples"),
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .default_value("encode_elided")
                .value_parser(["encode_elided", "elide_decision_ids"])
                .help(
                    "encode_elided: parse+encode SQL then elide; \
                     elide_decision_ids: elide pre-materialized tct_decision_ids",
                ),
        )
        .arg(
            Arg::new("fail-fast")
                .long("fail-fast")
                .action(clap::ArgAction::SetTrue)
                .help("Abort on first encode/elide failure"),
        )
        .arg(
            Arg::new("join-on-policy")
                .long("join-on-policy")
                .default_value("off")
                .value_parser(["off", "structural", "fk_declared"])
                .help(
                    "Join-ON masking policy: off (historical streams), \
                     structural (equality join keys must pair the just-joined \
                     table with an earlier in-scope table), or fk_declared \
                     (structural plus unique declared-FK shrink). Changes the \
                     elided stream contract",
                ),
        )
        .arg(
            Arg::new("column-addressing")
                .long("column-addressing")
                .default_value("flat")
                .value_parser(["flat", "factored"])
                .help(
                    "Column-reference addressing: flat (one scope-relative \
                     ordinal) or factored (two digits - catalog table index, \
                     then column index inside that table - matching the schema \
                     prompt's notation; changes the elided stream contract)",
                ),
        )
        .arg(
            Arg::new("visit-all-fields")
                .long("visit-all-fields")
                .action(clap::ArgAction::SetTrue)
                .help(
                    "Rank-1 compiler merge: force visitation of remaining \
                     optional fields so skip-vs-visit (rule._) is elided. \
                     Stream-contract break; do not mix with historical cells",
                ),
        )
        .get_matches();

    let corpus_path = matches.get_one::<PathBuf>("corpus").expect("required");
    let tables_path = matches.get_one::<PathBuf>("tables").expect("required");
    let output_jsonl = matches
        .get_one::<PathBuf>("output-jsonl")
        .expect("required");
    let output_report = matches
        .get_one::<PathBuf>("output-report")
        .expect("required");
    let split = matches.get_one::<String>("split").expect("defaulted");
    let sample_limit = matches.get_one::<usize>("sample-limit").copied();
    let mode = matches.get_one::<String>("mode").expect("defaulted");
    let fail_fast = matches.get_flag("fail-fast");
    let join_on_policy = SqlJoinOnPolicy::parse(
        matches
            .get_one::<String>("join-on-policy")
            .expect("defaulted"),
    )
    .expect("clap value_parser restricts join-on-policy values");
    let column_addressing = SqlColumnAddressing::parse(
        matches
            .get_one::<String>("column-addressing")
            .expect("defaulted"),
    )
    .expect("clap value_parser restricts column-addressing values");
    let visit_all_fields = matches.get_flag("visit-all-fields");
    // The policies compose, so the recorded contract names the combination.
    let stream_contract_id = SqlSchemaLinkingRuntime::elided_stream_contract_for(
        join_on_policy,
        column_addressing,
        visit_all_fields,
    );

    let catalog_bytes = fs::read(tables_path)?;
    let catalog_sha256 = sha256_hex(&catalog_bytes);
    let corpus_raw = fs::read_to_string(corpus_path)?;
    let corpus: CorpusFile = serde_json::from_str(&corpus_raw)?;

    if let Some(parent) = output_jsonl.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = output_report.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut runtimes: HashMap<String, SqlSchemaLinkingRuntime> = HashMap::new();
    let mut expanded_lengths: Vec<usize> = Vec::new();
    let mut elided_lengths: Vec<usize> = Vec::new();
    let mut failures: Vec<FailureExample> = Vec::new();
    let mut n_skipped_excluded = 0usize;
    let mut n_skipped_wrong_split = 0usize;
    let mut n_encoded = 0usize;

    let jsonl_file = File::create(output_jsonl)?;
    let mut jsonl_out = BufWriter::new(jsonl_file);

    for example in &corpus.examples {
        if example.split != *split {
            n_skipped_wrong_split += 1;
            continue;
        }
        if example.excluded {
            n_skipped_excluded += 1;
            continue;
        }
        if let Some(limit) = sample_limit
            && n_encoded >= limit
        {
            break;
        }

        let runtime = match runtimes.get(&example.db_id) {
            Some(rt) => rt,
            None => {
                let rt = SqlSchemaLinkingRuntime::from_catalog_json_bytes_with_sql_policies(
                    &catalog_bytes,
                    &catalog_sha256,
                    &example.db_id,
                    join_on_policy,
                    column_addressing,
                    visit_all_fields,
                )
                .map_err(|e| {
                    format!("failed to build runtime for db_id {}: {}", example.db_id, e)
                })?;
                runtimes.insert(example.db_id.clone(), rt);
                runtimes.get(&example.db_id).expect("just inserted")
            }
        };

        let result = match mode.as_str() {
            "encode_elided" => {
                let sql = example
                    .rendered_sql
                    .as_deref()
                    .or(example.gold_sql.as_deref())
                    .or(example.original_sql.as_deref())
                    .ok_or_else(|| "example has no SQL field".to_string());
                match sql {
                    Ok(sql) => {
                        // Also record expanded length for the report when cheap.
                        match runtime.encode(sql) {
                            Ok(expanded) => {
                                expanded_lengths.push(expanded.len());
                                runtime
                                    .elide_forced_tokens(&expanded)
                                    .map_err(|e| e.to_string())
                            }
                            Err(e) => Err(e.to_string()),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            "elide_decision_ids" => {
                let Some(ids) = example.tct_decision_ids.as_ref() else {
                    if fail_fast {
                        return Err(format!(
                            "example {} missing tct_decision_ids",
                            example.example_id
                        )
                        .into());
                    }
                    failures.push(FailureExample {
                        example_id: example.example_id.clone(),
                        db_id: example.db_id.clone(),
                        error: "missing tct_decision_ids".to_string(),
                    });
                    continue;
                };
                expanded_lengths.push(ids.len());
                runtime.elide_forced_tokens(ids).map_err(|e| e.to_string())
            }
            other => return Err(format!("unknown mode {other}").into()),
        };

        match result {
            Ok(elided) => {
                // Guard: base-token streams must stay below PAD for BPE base-vocab.
                if let Some(&bad) = elided.iter().find(|&&t| t >= SQL_SYNTAX_LAYOUT_PAD_TOKEN) {
                    let msg = format!(
                        "elided stream contains token {bad} >= PAD {}",
                        SQL_SYNTAX_LAYOUT_PAD_TOKEN
                    );
                    if fail_fast {
                        return Err(format!("{} ({})", msg, example.example_id).into());
                    }
                    failures.push(FailureExample {
                        example_id: example.example_id.clone(),
                        db_id: example.db_id.clone(),
                        error: msg,
                    });
                    continue;
                }
                elided_lengths.push(elided.len());
                serde_json::to_writer(&mut jsonl_out, &elided)?;
                jsonl_out.write_all(b"\n")?;
                n_encoded += 1;
                if n_encoded.is_multiple_of(500) {
                    eprintln!(
                        "  encoded {n_encoded} examples ({} runtimes)...",
                        runtimes.len()
                    );
                }
            }
            Err(error) => {
                if fail_fast {
                    return Err(format!(
                        "encode/elide failed for {} (db={}): {error}",
                        example.example_id, example.db_id
                    )
                    .into());
                }
                failures.push(FailureExample {
                    example_id: example.example_id.clone(),
                    db_id: example.db_id.clone(),
                    error,
                });
            }
        }
    }

    jsonl_out.flush()?;

    let expanded_stats = if expanded_lengths.is_empty() {
        None
    } else {
        Some(summarize_lengths(&mut expanded_lengths))
    };
    let elided_stats = summarize_lengths(&mut elided_lengths);

    // Sidecar metadata for tct-train-bpe auto base-vocab detection.
    if let Some(parent) = output_jsonl.parent() {
        let meta_path = parent.join("metadata.json");
        let meta = serde_json::json!({
            "base_vocab_size": SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
            "pad_token": SQL_SYNTAX_LAYOUT_PAD_TOKEN,
            "schema_name": "sql-schema-linking-elided",
            "stream_contract": "elided",
            "stream_contract_id": stream_contract_id,
            "join_on_policy": join_on_policy.as_str(),
            "column_addressing": column_addressing.as_str(),
            "visit_all_fields": visit_all_fields,
            "source_corpus": corpus_path.display().to_string(),
            "catalog_path": tables_path.display().to_string(),
            "catalog_sha256": catalog_sha256,
            "split": split,
            "mode": mode,
            "n_sequences": n_encoded,
        });
        fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
    }

    let report = EncodeReport {
        artifact_kind: "sql_elided_encode_corpus_report_v1",
        claim_bearing: false,
        catalog_path: tables_path.display().to_string(),
        catalog_sha256,
        corpus_path: corpus_path.display().to_string(),
        split: split.clone(),
        mode: mode.clone(),
        base_vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        join_on_policy: join_on_policy.as_str(),
        column_addressing: column_addressing.as_str(),
        visit_all_fields,
        stream_contract: stream_contract_id,
        n_requested: corpus.n_train_supported.unwrap_or(0),
        n_encoded,
        n_failed: failures.len(),
        n_skipped_excluded,
        n_skipped_wrong_split,
        n_runtimes: runtimes.len(),
        sample_limit,
        expanded_length_stats: expanded_stats,
        elided_length_stats: elided_stats,
        failure_examples: failures.into_iter().take(50).collect(),
        output_jsonl: output_jsonl.display().to_string(),
        output_report: output_report.display().to_string(),
    };

    fs::write(output_report, serde_json::to_string_pretty(&report)?)?;

    eprintln!(
        "Wrote {} elided sequences → {}",
        n_encoded,
        output_jsonl.display()
    );
    eprintln!(
        "Elided length stats: median={:.1} mean={:.2} p90={:.1} (n={})",
        report.elided_length_stats.median_tokens,
        report.elided_length_stats.mean_tokens,
        report.elided_length_stats.p90_tokens,
        report.elided_length_stats.records
    );
    if let Some(ref exp) = report.expanded_length_stats {
        eprintln!(
            "Expanded length stats: median={:.1} mean={:.2} p90={:.1}",
            exp.median_tokens, exp.mean_tokens, exp.p90_tokens
        );
    }
    if report.n_failed > 0 {
        eprintln!("Failures: {} (see report)", report.n_failed);
    }

    Ok(())
}

fn summarize_lengths(lengths: &mut [usize]) -> LengthStats {
    assert!(!lengths.is_empty());
    lengths.sort_unstable();
    let records = lengths.len();
    let total: usize = lengths.iter().sum();
    let mean = total as f64 / records as f64;
    LengthStats {
        records,
        total_tokens: total,
        min_tokens: lengths[0],
        max_tokens: lengths[records - 1],
        mean_tokens: mean,
        median_tokens: percentile_sorted(lengths, 0.50),
        p90_tokens: percentile_sorted(lengths, 0.90),
    }
}

fn percentile_sorted(sorted: &[usize], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
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
