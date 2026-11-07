//! Corpus-wide acceptance gate for the SQL shared payload pool collapse.
//!
//! For every encodeable Spider example:
//! 1. `decode(encode(sql))` is a complete fixpoint (`encode(rendered) == tokens`)
//! 2. `expand(elide(tokens)) == tokens` (elided stream contract)
//! 3. Every token id is strictly below PAD
//!
//! Report-only JSON; exit 0 only when all encodeable examples pass.
//! Not a paper claim.

use clap::{Arg, Command, value_parser};
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tct::provenance::sha256_hex;
use tct::sql_schema_linking::runtime::SqlSchemaLinkingRuntime;
use tct::sql_schema_linking::{
    SQL_SYNTAX_LAYOUT_PAD_TOKEN, SQL_SYNTAX_LAYOUT_VOCAB_SIZE, SQL_SYNTAX_TOKEN_LAYOUT_ID,
    sql_syntax_token_layout,
};

#[derive(Debug, serde::Deserialize)]
struct SpiderRow {
    db_id: Option<String>,
    query: Option<String>,
}

#[derive(Debug, Serialize)]
struct Failure {
    db_id: String,
    query_preview: String,
    phase: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct GateReport {
    artifact_kind: &'static str,
    claim_bearing: bool,
    paper_extractable: bool,
    layout_id: &'static str,
    vocab_size: u32,
    pad_token: u32,
    tables_path: String,
    tables_sha256: String,
    examples_path: String,
    n_rows: usize,
    n_encodeable: usize,
    n_encode_failed: usize,
    n_round_trip_ok: usize,
    n_round_trip_err: usize,
    n_elide_expand_ok: usize,
    n_elide_expand_err: usize,
    all_encodeable_passed: bool,
    sample_failures: Vec<Failure>,
    notes: Vec<String>,
}

fn preview(sql: &str) -> String {
    let one = sql.lines().next().unwrap_or(sql).trim();
    if one.len() <= 96 {
        one.to_string()
    } else {
        format!("{}…", &one[..96])
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-sql-shared-pool-corpus-gate")
        .about("Corpus-wide decode(encode) + elide/expand gate for SQL shared payload pool")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("tables")
                .long("tables")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("examples")
                .long("examples")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .value_parser(value_parser!(usize))
                .help("Optional row cap (default: whole file)"),
        )
        .get_matches();

    let tables_path = matches.get_one::<PathBuf>("tables").unwrap().clone();
    let examples_path = matches.get_one::<PathBuf>("examples").unwrap().clone();
    let output = matches.get_one::<PathBuf>("output").cloned();
    let limit = matches.get_one::<usize>("limit").copied();

    let tables_bytes = fs::read(&tables_path)?;
    let tables_sha = sha256_hex(&tables_bytes);
    let tables_json = String::from_utf8(tables_bytes)?;

    let examples_raw = fs::read_to_string(&examples_path)?;
    let rows: Vec<SpiderRow> = serde_json::from_str(&examples_raw)?;
    let take = limit.unwrap_or(rows.len()).min(rows.len());

    let layout = sql_syntax_token_layout();
    layout
        .validate()
        .map_err(|e| format!("layout validate failed: {e}"))?;

    let mut runtimes: HashMap<String, Result<SqlSchemaLinkingRuntime, String>> = HashMap::new();
    let mut n_encodeable = 0usize;
    let mut n_encode_failed = 0usize;
    let mut n_rt_ok = 0usize;
    let mut n_rt_err = 0usize;
    let mut n_ee_ok = 0usize;
    let mut n_ee_err = 0usize;
    let mut failures: Vec<Failure> = Vec::new();

    for row in rows.into_iter().take(take) {
        let db_id = row.db_id.unwrap_or_default();
        let query = row.query.unwrap_or_default();
        if db_id.is_empty() || query.trim().is_empty() {
            n_encode_failed += 1;
            continue;
        }

        let rt = runtimes.entry(db_id.clone()).or_insert_with(|| {
            SqlSchemaLinkingRuntime::from_catalog_json(&tables_json, &tables_sha, &db_id)
                .map_err(|e| e.to_string())
        });
        let rt = match rt {
            Ok(rt) => rt,
            Err(e) => {
                n_encode_failed += 1;
                if failures.len() < 20 {
                    failures.push(Failure {
                        db_id: db_id.clone(),
                        query_preview: preview(&query),
                        phase: "runtime".into(),
                        error: e.clone(),
                    });
                }
                continue;
            }
        };

        let tokens = match rt.encode(&query) {
            Ok(t) => t,
            Err(_) => {
                n_encode_failed += 1;
                continue;
            }
        };
        n_encodeable += 1;

        if tokens.iter().any(|&t| t >= SQL_SYNTAX_LAYOUT_PAD_TOKEN) {
            n_rt_err += 1;
            if failures.len() < 20 {
                failures.push(Failure {
                    db_id: db_id.clone(),
                    query_preview: preview(&query),
                    phase: "pad".into(),
                    error: format!("token >= PAD {}", SQL_SYNTAX_LAYOUT_PAD_TOKEN),
                });
            }
            continue;
        }

        // Gate 1: decode(encode(sql)) is a complete strict fixpoint.
        let rendered = match rt.decode(&tokens) {
            Ok(s) => s,
            Err(e) => {
                n_rt_err += 1;
                if failures.len() < 20 {
                    failures.push(Failure {
                        db_id: db_id.clone(),
                        query_preview: preview(&query),
                        phase: "decode".into(),
                        error: e.to_string(),
                    });
                }
                continue;
            }
        };
        match rt.encode(&rendered) {
            Ok(again) if again == tokens => {
                n_rt_ok += 1;
            }
            Ok(_) => {
                n_rt_err += 1;
                if failures.len() < 20 {
                    failures.push(Failure {
                        db_id: db_id.clone(),
                        query_preview: preview(&query),
                        phase: "reencode_fixpoint".into(),
                        error: "encode(decode(tokens)) diverged from tokens".into(),
                    });
                }
                continue;
            }
            Err(e) => {
                n_rt_err += 1;
                if failures.len() < 20 {
                    failures.push(Failure {
                        db_id: db_id.clone(),
                        query_preview: preview(&query),
                        phase: "reencode".into(),
                        error: e.to_string(),
                    });
                }
                continue;
            }
        }

        // Gate 2: expand(elide(s)) == s over the expanded stream.
        match rt.elide_forced_tokens(&tokens).and_then(|elided| {
            rt.expand_elided_tokens(&elided)
                .map(|expanded| (elided, expanded))
        }) {
            Ok((_elided, expanded)) if expanded == tokens => {
                n_ee_ok += 1;
            }
            Ok(_) => {
                n_ee_err += 1;
                if failures.len() < 20 {
                    failures.push(Failure {
                        db_id: db_id.clone(),
                        query_preview: preview(&query),
                        phase: "elide_expand".into(),
                        error: "expand(elide(tokens)) diverged from tokens".into(),
                    });
                }
            }
            Err(e) => {
                n_ee_err += 1;
                if failures.len() < 20 {
                    failures.push(Failure {
                        db_id: db_id.clone(),
                        query_preview: preview(&query),
                        phase: "elide_expand".into(),
                        error: e.to_string(),
                    });
                }
            }
        }
    }

    let all_ok = n_encodeable > 0 && n_rt_err == 0 && n_ee_err == 0;
    let report = GateReport {
        artifact_kind: "tct_sql_shared_payload_pool_corpus_gate_v0",
        claim_bearing: false,
        paper_extractable: false,
        layout_id: SQL_SYNTAX_TOKEN_LAYOUT_ID,
        vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        tables_path: tables_path.display().to_string(),
        tables_sha256: tables_sha,
        examples_path: examples_path.display().to_string(),
        n_rows: take,
        n_encodeable,
        n_encode_failed,
        n_round_trip_ok: n_rt_ok,
        n_round_trip_err: n_rt_err,
        n_elide_expand_ok: n_ee_ok,
        n_elide_expand_err: n_ee_err,
        all_encodeable_passed: all_ok,
        sample_failures: failures,
        notes: vec![
            "Acceptance: every encodeable Spider row has decode(encode) fixpoint and expand(elide)==s.".into(),
            "Original surface SQL need not equal rendered form; fixpoint is on the expanded token stream.".into(),
            "SQL shared payload pool: structural [0,64) + shared [64,320); PAD=320 VOCAB=321.".into(),
            "breaking_change vs prior four-band pin (833 vocab); rematerialize raw-token corpora and BPE.".into(),
        ],
    };

    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, format!("{json}\n"))?;
        eprintln!("wrote {}", path.display());
    } else {
        println!("{json}");
    }

    eprintln!(
        "corpus gate: encodeable={}/{} rt_ok={} rt_err={} ee_ok={} ee_err={} all_passed={}",
        report.n_encodeable,
        report.n_rows,
        report.n_round_trip_ok,
        report.n_round_trip_err,
        report.n_elide_expand_ok,
        report.n_elide_expand_err,
        report.all_encodeable_passed
    );

    if !all_ok {
        std::process::exit(2);
    }
    Ok(())
}
