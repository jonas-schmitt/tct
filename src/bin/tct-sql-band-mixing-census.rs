//! Band-mixing census over Spider SQL streams (shared-payload-pool soundness).
//!
//! At every free (non-singleton-mask) decision on an expanded encode walk,
//! classify the admissible set by which layout bands appear. The shared
//! payload pool collapse is sound only when no free decision admits two
//! payload roles that would map to the same post-collapse token id.
//!
//! Report-only. Not a paper claim.

use clap::{Arg, Command, value_parser};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tct::provenance::sha256_hex;
use tct::sql_schema_linking::layout::{
    SQL_MENU_INDEX_PAYLOAD_ROLE, sql_syntax_token_layout, u8_integer_band_range,
};
use tct::sql_schema_linking::runtime::SqlSchemaLinkingRuntime;
use tct::sql_schema_linking::{SQL_SYNTAX_LAYOUT_PAD_TOKEN, SQL_SYNTAX_LAYOUT_VOCAB_SIZE};

#[derive(Debug, serde::Deserialize)]
struct SpiderRow {
    db_id: Option<String>,
    /// Surface SQL string (Spider train/dev). Preferred over structured `sql`.
    query: Option<String>,
}

#[derive(Debug, Serialize)]
struct CensusReport {
    artifact_kind: &'static str,
    claim_bearing: bool,
    paper_extractable: bool,
    tables_path: String,
    tables_sha256: String,
    examples_path: String,
    sample_limit: usize,
    n_attempted: usize,
    n_encoded: usize,
    n_encode_failed: usize,
    n_free_decisions: usize,
    n_single_band: usize,
    n_multi_band: usize,
    pure_band_counts: BTreeMap<String, usize>,
    mixed_band_counts: BTreeMap<String, usize>,
    /// Multi-band patterns that would collide under a shared payload pool
    /// (two or more of utf8/u8_integer/menu in the same free mask).
    unsound_shared_pool_patterns: BTreeMap<String, usize>,
    layout_vocab_size: u32,
    layout_pad_token: u32,
    layout_ranges: Vec<LayoutRange>,
    sound_for_shared_payload_pool: bool,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LayoutRange {
    role: String,
    start: u32,
    end_exclusive: u32,
}

fn band_for_token(
    token: u32,
    struct_end: u32,
    utf8: (u32, u32),
    integer: (u32, u32),
    menu: (u32, u32),
    pad: u32,
) -> Option<&'static str> {
    if token >= pad {
        return None;
    }
    if token < struct_end {
        return Some("structural_decisions");
    }
    if token >= utf8.0 && token < utf8.1 {
        return Some("utf8_byte_payload");
    }
    if token >= integer.0 && token < integer.1 {
        return Some("u8_integer_payload");
    }
    if token >= menu.0 && token < menu.1 {
        return Some(SQL_MENU_INDEX_PAYLOAD_ROLE);
    }
    None
}

fn classify_mask(
    allowed: &[u32],
    struct_end: u32,
    utf8: (u32, u32),
    integer: (u32, u32),
    menu: (u32, u32),
    pad: u32,
) -> BTreeSet<&'static str> {
    let mut bands = BTreeSet::new();
    for &token in allowed {
        if let Some(band) = band_for_token(token, struct_end, utf8, integer, menu, pad) {
            bands.insert(band);
        }
    }
    bands
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-sql-band-mixing-census")
        .about("Classify free-decision admissible sets by SQL layout band")
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
            Arg::new("limit")
                .long("limit")
                .value_parser(value_parser!(usize))
                .default_value("400"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let tables_path = matches.get_one::<PathBuf>("tables").unwrap().clone();
    let examples_path = matches.get_one::<PathBuf>("examples").unwrap().clone();
    let limit = *matches.get_one::<usize>("limit").unwrap();
    let output = matches.get_one::<PathBuf>("output").cloned();

    let tables_bytes = fs::read(&tables_path)?;
    let tables_sha = sha256_hex(&tables_bytes);
    let tables_json = String::from_utf8(tables_bytes.clone())?;

    let examples_raw = fs::read_to_string(&examples_path)?;
    let rows: Vec<SpiderRow> = serde_json::from_str(&examples_raw)?;
    let take = limit.min(rows.len());

    let layout = sql_syntax_token_layout();
    let struct_end = layout
        .ranges
        .iter()
        .find(|r| r.role == "structural_decisions")
        .map(|r| r.end_exclusive)
        .ok_or("missing structural_decisions")?;
    let utf8 = layout
        .ranges
        .iter()
        .find(|r| r.role == "utf8_byte_payload")
        .map(|r| (r.start, r.end_exclusive))
        .ok_or("missing utf8_byte_payload")?;
    let integer = u8_integer_band_range(&layout).ok_or("missing u8_integer_payload")?;
    let menu = layout
        .ranges
        .iter()
        .find(|r| r.role == SQL_MENU_INDEX_PAYLOAD_ROLE)
        .map(|r| (r.start, r.end_exclusive))
        .ok_or("missing menu_index_payload")?;
    let pad = layout.pad_token;

    let mut runtimes: HashMap<String, Result<SqlSchemaLinkingRuntime, String>> = HashMap::new();
    let mut n_encoded = 0usize;
    let mut n_encode_failed = 0usize;
    let mut n_free = 0usize;
    let mut n_single = 0usize;
    let mut n_multi = 0usize;
    let mut pure: BTreeMap<String, usize> = BTreeMap::new();
    let mut mixed: BTreeMap<String, usize> = BTreeMap::new();
    let mut unsound: BTreeMap<String, usize> = BTreeMap::new();

    let payload_roles: BTreeSet<&str> = [
        "utf8_byte_payload",
        "u8_integer_payload",
        SQL_MENU_INDEX_PAYLOAD_ROLE,
    ]
    .into_iter()
    .collect();

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
            Err(_) => {
                n_encode_failed += 1;
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
        n_encoded += 1;

        let mut walker = rt.begin_tct_decode();
        for &token in &tokens {
            let allowed = walker.allowed_token_ids()?;
            // Free decision: mask is not a singleton.
            if allowed.len() != 1 {
                let bands = classify_mask(&allowed, struct_end, utf8, integer, menu, pad);
                n_free += 1;
                let key = bands.iter().copied().collect::<Vec<_>>().join(",");
                if bands.len() <= 1 {
                    n_single += 1;
                    *pure.entry(key.clone()).or_default() += 1;
                } else {
                    n_multi += 1;
                    *mixed.entry(key.clone()).or_default() += 1;
                    let payload_hits: BTreeSet<_> =
                        bands.intersection(&payload_roles).copied().collect();
                    if payload_hits.len() >= 2 {
                        *unsound.entry(key).or_default() += 1;
                    }
                }
            }
            walker.apply(token)?;
        }
    }

    let sound = unsound.is_empty();
    let report = CensusReport {
        artifact_kind: "tct_sql_band_mixing_census_v0",
        claim_bearing: false,
        paper_extractable: false,
        tables_path: tables_path.display().to_string(),
        tables_sha256: tables_sha,
        examples_path: examples_path.display().to_string(),
        sample_limit: take,
        n_attempted: take,
        n_encoded,
        n_encode_failed,
        n_free_decisions: n_free,
        n_single_band: n_single,
        n_multi_band: n_multi,
        pure_band_counts: pure,
        mixed_band_counts: mixed,
        unsound_shared_pool_patterns: unsound,
        layout_vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        layout_pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        layout_ranges: layout
            .ranges
            .iter()
            .map(|r| LayoutRange {
                role: r.role.clone(),
                start: r.start,
                end_exclusive: r.end_exclusive,
            })
            .collect(),
        sound_for_shared_payload_pool: sound,
        notes: vec![
            "Free decision = expanded-stream position whose admissible mask is not a singleton."
                .into(),
            "Unsound for shared payload pool iff any free mask admits two or more of utf8/u8_integer/menu."
                .into(),
            "structural_decisions must stay disjoint from the shared pool; struct+utf8 mixing is expected and safe."
                .into(),
        ],
    };

    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, json.as_bytes())?;
        eprintln!("wrote {}", path.display());
    } else {
        println!("{json}");
    }

    eprintln!(
        "census: encoded={}/{} free={} single={} multi={} sound={}",
        report.n_encoded,
        report.n_attempted,
        report.n_free_decisions,
        report.n_single_band,
        report.n_multi_band,
        report.sound_for_shared_payload_pool
    );
    for (k, v) in &report.pure_band_counts {
        eprintln!("  pure {k}: {v}");
    }
    for (k, v) in &report.mixed_band_counts {
        eprintln!("  mixed {k}: {v}");
    }
    if !report.sound_for_shared_payload_pool {
        eprintln!("UNSOUND patterns:");
        for (k, v) in &report.unsound_shared_pool_patterns {
            eprintln!("  {k}: {v}");
        }
        std::process::exit(2);
    }
    Ok(())
}
