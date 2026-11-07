//! Honest Spider-scale producer parse coverage diagnostic.
//!
//! Measures what fraction of host-mounted gold queries the subset frontend
//! (`sql_schema_linking_m0_m5_select_v0`) can **parse** under real catalogs.
//! This is a diagnostic only — not Primary metrics, not paper extractable, not
//! a reason to soften the subset grammar.

use crate::sql_schema_linking::catalog::SqlCatalog;
use crate::sql_schema_linking::codec::SqlSchemaLinkingCodec;
use crate::sql_schema_linking::render::is_sql_subset_keyword;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Artifact kind for producer parse coverage reports.
pub const SQL_SPIDER_PARSE_COVERAGE_ARTIFACT: &str = "tct_sql_spider_parse_coverage_v0";
/// Explicit non-claim boundary.
pub const SQL_SPIDER_PARSE_COVERAGE_CLAIM_BOUNDARY: &str = "compiler_sql_schema_linking_only";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpiderParseCoverageReport {
    pub artifact_kind: String,
    pub version: u32,
    pub claim_boundary: String,
    pub evidence_class: String,
    pub claim_bearing: bool,
    pub paper_extractable: bool,
    pub tables_json_path: String,
    pub examples_json_path: String,
    pub n_total: usize,
    pub n_parse_ok: usize,
    pub n_encode_ok: usize,
    pub n_missing_catalog: usize,
    pub n_parse_err: usize,
    pub n_empty_query: usize,
    pub parseable_fraction: f64,
    pub encode_fraction_of_total: f64,
    pub encode_fraction_of_parse_ok: f64,
    pub reason_counts: BTreeMap<String, usize>,
    pub sample_failures: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SpiderExampleRow {
    db_id: Option<String>,
    query: Option<String>,
    #[serde(default)]
    sql: Option<String>,
    #[serde(default, rename = "SQL")]
    sql_upper: Option<String>,
}

/// Measure producer parse rate over Spider-style examples + multi-db tables.json.
pub fn measure_spider_parse_coverage(
    tables_path: &Path,
    examples_path: &Path,
    max_examples: Option<usize>,
) -> Result<SpiderParseCoverageReport, String> {
    let raw = std::fs::read_to_string(examples_path).map_err(|e| format!("read examples: {e}"))?;
    let rows: Vec<SpiderExampleRow> =
        serde_json::from_str(&raw).map_err(|e| format!("parse examples json: {e}"))?;
    let limit = max_examples.unwrap_or(rows.len()).min(rows.len());

    let mut codecs: HashMap<String, Result<SqlSchemaLinkingCodec, String>> = HashMap::new();
    let mut n_parse_ok = 0usize;
    let mut n_encode_ok = 0usize;
    let mut n_missing_catalog = 0usize;
    let mut n_parse_err = 0usize;
    let mut n_empty_query = 0usize;
    let mut reason_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut sample_failures: Vec<String> = Vec::new();
    // Cap per reason so residual taxonomies get diverse samples (not only the
    // first 12 global failures, which are usually the same early DBs).
    let mut samples_per_reason: BTreeMap<String, usize> = BTreeMap::new();
    const MAX_SAMPLES_PER_REASON: usize = 3;
    const MAX_SAMPLES_TOTAL: usize = 48;

    for row in rows.into_iter().take(limit) {
        let db_id = row.db_id.unwrap_or_default();
        let query = row.query.or(row.sql).or(row.sql_upper).unwrap_or_default();
        if db_id.is_empty() {
            n_parse_err += 1;
            *reason_counts.entry("missing_db_id".into()).or_default() += 1;
            continue;
        }
        if query.trim().is_empty() {
            n_empty_query += 1;
            *reason_counts.entry("empty_query".into()).or_default() += 1;
            continue;
        }

        let codec = codecs.entry(db_id.clone()).or_insert_with(|| {
            SqlCatalog::load_unpinned_tables_json(tables_path, &db_id)
                .map_err(|e| format!("catalog load: {e}"))
                .and_then(|cat| {
                    SqlSchemaLinkingCodec::for_catalog(cat).map_err(|e| format!("codec: {e}"))
                })
        });

        match codec {
            Err(msg) => {
                n_missing_catalog += 1;
                let key = if msg.contains("no entry for db_id") {
                    "missing_catalog".to_string()
                } else {
                    format!("catalog_error:{}", truncate_reason(msg))
                };
                *reason_counts.entry(key).or_default() += 1;
            }
            Ok(codec) => match codec.parse(&query) {
                Ok(stmt) => {
                    n_parse_ok += 1;
                    *reason_counts.entry("parse_ok".into()).or_default() += 1;
                    match codec.encode(&stmt) {
                        Ok(tokens) if !tokens.is_empty() => {
                            n_encode_ok += 1;
                            *reason_counts.entry("encode_ok".into()).or_default() += 1;
                        }
                        Ok(_) => {
                            *reason_counts.entry("encode_empty".into()).or_default() += 1;
                        }
                        Err(err) => {
                            let key = format!("encode_err:{}", truncate_reason(&err.to_string()));
                            *reason_counts.entry(key).or_default() += 1;
                        }
                    }
                }
                Err(err) => {
                    n_parse_err += 1;
                    let msg = err.to_string();
                    let key = refine_blocker_with_query(&classify_parse_error(&msg), &query);
                    *reason_counts.entry(key.clone()).or_default() += 1;
                    let reason_n = samples_per_reason.entry(key.clone()).or_default();
                    if *reason_n < MAX_SAMPLES_PER_REASON
                        && sample_failures.len() < MAX_SAMPLES_TOTAL
                    {
                        *reason_n += 1;
                        sample_failures
                            .push(format!("{db_id}: {key}: {}", truncate_chars(&query, 80)));
                    }
                }
            },
        }
    }

    let n_total = limit;
    let parseable_fraction = if n_total == 0 {
        0.0
    } else {
        n_parse_ok as f64 / n_total as f64
    };
    let encode_fraction_of_total = if n_total == 0 {
        0.0
    } else {
        n_encode_ok as f64 / n_total as f64
    };
    let encode_fraction_of_parse_ok = if n_parse_ok == 0 {
        0.0
    } else {
        n_encode_ok as f64 / n_parse_ok as f64
    };

    Ok(SpiderParseCoverageReport {
        artifact_kind: SQL_SPIDER_PARSE_COVERAGE_ARTIFACT.to_string(),
        version: 0,
        claim_boundary: SQL_SPIDER_PARSE_COVERAGE_CLAIM_BOUNDARY.to_string(),
        evidence_class: "dry-run integration".to_string(),
        claim_bearing: false,
        paper_extractable: false,
        tables_json_path: tables_path.display().to_string(),
        examples_json_path: examples_path.display().to_string(),
        n_total,
        n_parse_ok,
        n_encode_ok,
        n_missing_catalog,
        n_parse_err,
        n_empty_query,
        parseable_fraction,
        encode_fraction_of_total,
        encode_fraction_of_parse_ok,
        reason_counts,
        sample_failures,
        notes: vec![
            "Producer parse/encode coverage under sql_schema_linking_m0_m5_select_v0 only.".into(),
            "Low coverage is a finding to report, not a bar to soften.".into(),
            "Not Primary metrics; not paper-extractable; not Spider 2.0.".into(),
            "Training should prefer grammar-synthesized valid queries over real catalogs.".into(),
        ],
    })
}

/// One producer-encoded example for dry-run A5 stream prep (not claim-bearing).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpiderEncodedExample {
    pub db_id: String,
    pub query: String,
    pub tokens: Vec<u32>,
    pub n_tokens: usize,
}

/// Encode every parseable example; skip failures. Also returns a coverage report.
pub fn encode_spider_examples(
    tables_path: &Path,
    examples_path: &Path,
    max_examples: Option<usize>,
) -> Result<(Vec<SpiderEncodedExample>, SpiderParseCoverageReport), String> {
    let report = measure_spider_parse_coverage(tables_path, examples_path, max_examples)?;
    let raw = std::fs::read_to_string(examples_path).map_err(|e| format!("read examples: {e}"))?;
    let rows: Vec<SpiderExampleRow> =
        serde_json::from_str(&raw).map_err(|e| format!("parse examples json: {e}"))?;
    let limit = max_examples.unwrap_or(rows.len()).min(rows.len());

    let mut codecs: HashMap<String, Result<SqlSchemaLinkingCodec, String>> = HashMap::new();
    let mut out: Vec<SpiderEncodedExample> = Vec::new();

    for row in rows.into_iter().take(limit) {
        let db_id = row.db_id.unwrap_or_default();
        let query = row.query.or(row.sql).or(row.sql_upper).unwrap_or_default();
        if db_id.is_empty() || query.trim().is_empty() {
            continue;
        }
        let codec = codecs.entry(db_id.clone()).or_insert_with(|| {
            SqlCatalog::load_unpinned_tables_json(tables_path, &db_id)
                .map_err(|e| format!("catalog load: {e}"))
                .and_then(|cat| {
                    SqlSchemaLinkingCodec::for_catalog(cat).map_err(|e| format!("codec: {e}"))
                })
        });
        let Ok(codec) = codec else {
            continue;
        };
        let Ok(stmt) = codec.parse(&query) else {
            continue;
        };
        let Ok(tokens) = codec.encode(&stmt) else {
            continue;
        };
        if tokens.is_empty() {
            continue;
        }
        out.push(SpiderEncodedExample {
            db_id,
            query,
            n_tokens: tokens.len(),
            tokens,
        });
    }
    Ok((out, report))
}

/// Round-trip report for producer-encoded host examples (dry-run; not claim-bearing).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpiderRoundTripReport {
    pub artifact_kind: String,
    pub version: u32,
    pub claim_boundary: String,
    pub evidence_class: String,
    pub claim_bearing: bool,
    pub paper_extractable: bool,
    pub n_attempted: usize,
    pub n_round_trip_ok: usize,
    pub n_round_trip_err: usize,
    pub round_trip_fraction_of_attempted: f64,
    pub sample_failures: Vec<String>,
    pub notes: Vec<String>,
}

/// Parse → encode → decode → render → reparse for every encodeable example.
pub fn round_trip_spider_examples(
    tables_path: &Path,
    examples_path: &Path,
    max_examples: Option<usize>,
) -> Result<SpiderRoundTripReport, String> {
    let (streams, _coverage) = encode_spider_examples(tables_path, examples_path, max_examples)?;
    let mut codecs: HashMap<String, Result<SqlSchemaLinkingCodec, String>> = HashMap::new();
    let mut n_ok = 0usize;
    let mut n_err = 0usize;
    let mut samples: Vec<String> = Vec::new();

    for ex in &streams {
        let codec = codecs.entry(ex.db_id.clone()).or_insert_with(|| {
            SqlCatalog::load_unpinned_tables_json(tables_path, &ex.db_id)
                .map_err(|e| format!("catalog load: {e}"))
                .and_then(|cat| {
                    SqlSchemaLinkingCodec::for_catalog(cat).map_err(|e| format!("codec: {e}"))
                })
        });
        let Ok(codec) = codec else {
            n_err += 1;
            continue;
        };
        match codec.round_trip_source(&ex.query) {
            Ok(_) => n_ok += 1,
            Err(e) => {
                n_err += 1;
                if samples.len() < 12 {
                    samples.push(format!(
                        "{}: {}: {}",
                        ex.db_id,
                        truncate_reason(&e.to_string()),
                        truncate_reason(&ex.query)
                    ));
                }
            }
        }
    }
    let n_attempted = streams.len();
    let frac = if n_attempted == 0 {
        0.0
    } else {
        n_ok as f64 / n_attempted as f64
    };
    Ok(SpiderRoundTripReport {
        artifact_kind: "tct_sql_spider_round_trip_v0".into(),
        version: 0,
        claim_boundary: SQL_SPIDER_PARSE_COVERAGE_CLAIM_BOUNDARY.into(),
        evidence_class: "dry-run integration".into(),
        claim_bearing: false,
        paper_extractable: false,
        n_attempted,
        n_round_trip_ok: n_ok,
        n_round_trip_err: n_err,
        round_trip_fraction_of_attempted: frac,
        sample_failures: samples,
        notes: vec![
            "Producer parse→encode→decode→render→reparse on encodeable examples only.".into(),
            "Not claim-bearing; not Spider 2.0; not paper-extractable.".into(),
        ],
    })
}

fn truncate_reason(msg: &str) -> String {
    truncate_chars(msg.lines().next().unwrap_or(msg), 80)
}

/// Truncate to at most `max_chars` **characters** (never bytes).
///
/// Byte slicing panics on any Spider query or error message that carries
/// non-ASCII text and happens to split a UTF-8 sequence at the limit.
fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn classify_parse_error(msg: &str) -> String {
    let lower = msg.to_ascii_lowercase();
    // Order matters: avoid false positives (e.g. "unknown table X in catalog"
    // must not match a bare " in " as SQL IN).
    // Integer capacity / legacy u8-band failures must not fall into opaque
    // blocker:other:… buckets. Over-capacity uses integer_capacity; residual
    // messages that still say "u8 band" / "exceeds 255" keep the legacy label
    // for historical residual joins until rematerialize reclassifies.
    if lower.contains("integer_capacity")
        || lower.contains("exceeds integer capacity")
        || lower.contains("exceeds integer_capacity")
    {
        return "blocker:integer_capacity".to_string();
    }
    if lower.contains("integer_literal")
        || lower.contains("u8 integer")
        || lower.contains("u8 band")
        || lower.contains("above 255")
        || lower.contains("exceeds 255")
    {
        // Prefer capacity bucket when the message also names the new ceiling.
        if lower.contains("16777215") || lower.contains("16_777_215") || lower.contains("capacity")
        {
            return "blocker:integer_capacity".to_string();
        }
        return "blocker:integer_u8_band".to_string();
    }
    // Aggregate ORDER BY / HAVING often surfaces as "column count is not in
    // scope" rather than an "order by count" feature string — reclassify before
    // the generic column_not_in_scope needle so residual taxonomies stay honest.
    if lower.contains("is not in scope") && message_mentions_aggregate_ident(&lower) {
        return "blocker:ORDER_BY_AGG".to_string();
    }
    let needles = [
        ("unknown table", "blocker:unknown_table"),
        ("unknown column", "blocker:unknown_column"),
        ("is not in scope", "blocker:column_not_in_scope"),
        ("is ambiguous", "blocker:column_ambiguous"),
        ("union", "blocker:UNION"),
        ("intersect", "blocker:INTERSECT"),
        ("except", "blocker:EXCEPT"),
        ("between", "blocker:BETWEEN"),
        // Require '(' so "not in scope" / "in catalog" do not false-positive as SQL IN.
        (" not in (", "blocker:IN"),
        (" not in(", "blocker:IN"),
        (" in (", "blocker:IN"),
        (" in(", "blocker:IN"),
        ("unexpected token: in", "blocker:IN"),
        ("case ", "blocker:CASE"),
        ("with ", "blocker:CTE_WITH"),
        ("window", "blocker:WINDOW"),
        ("over (", "blocker:WINDOW"),
        ("alias", "blocker:alias"),
        ("join", "blocker:join_shape"),
        ("limit", "blocker:limit"),
        ("group by", "blocker:group_by"),
        // Prefer specific ORDER BY aggregate before generic order_by / aggregate.
        ("order by count", "blocker:ORDER_BY_AGG"),
        ("order by sum", "blocker:ORDER_BY_AGG"),
        ("order by avg", "blocker:ORDER_BY_AGG"),
        ("order by min", "blocker:ORDER_BY_AGG"),
        ("order by max", "blocker:ORDER_BY_AGG"),
        ("order by", "blocker:order_by"),
        ("having", "blocker:having"),
        ("subquery", "blocker:subquery"),
        // COUNT(DISTINCT …) is outside the subset; prefer a precise residual
        // bucket over generic aggregate before the aggregate needle.
        ("count(distinct", "blocker:COUNT_DISTINCT"),
        ("count (distinct", "blocker:COUNT_DISTINCT"),
        ("distinct", "blocker:DISTINCT_EXTRA"),
        ("aggregate", "blocker:aggregate"),
        ("unexpected", "blocker:unexpected_token"),
        ("expected", "blocker:expected_token"),
    ];
    for (needle, label) in needles {
        if lower.contains(needle) {
            return label.to_string();
        }
    }
    format!("blocker:other:{}", truncate_reason(msg))
}

/// True when a not-in-scope message names an aggregate identifier rather than a
/// real catalog column (common for `ORDER BY count(*)` / `HAVING COUNT(*)`).
fn message_mentions_aggregate_ident(lower: &str) -> bool {
    const AGG: &[&str] = &[
        "column count",
        "column sum",
        "column avg",
        "column min",
        "column max",
        "count(*)",
        "sum(",
        "avg(",
        "min(",
        "max(",
        "count(",
    ];
    AGG.iter().any(|n| lower.contains(n))
}

/// Refine generic error-message buckets using the failing query text.
///
/// Some producer errors say only "aggregate" / "unexpected" while the query
/// itself clearly uses an out-of-subset construct (e.g. COUNT(DISTINCT …)).
/// Do **not** override precise SharedAlphabet / catalog / already-specific
/// buckets (e.g. integer_u8_band on `IndepYear < 1930` with COUNT(DISTINCT)).
fn refine_blocker_with_query(key: &str, query: &str) -> String {
    let ql = query.to_ascii_lowercase();
    // Set ops always win from query text (out-of-subset), even when the parse
    // path first labels join_shape / CTE_WITH / expected_token on a multi-issue
    // residual that also contains EXCEPT/INTERSECT/UNION.
    if ql.contains(" intersect ") {
        return "blocker:INTERSECT".to_string();
    }
    if ql.contains(" except ") {
        return "blocker:EXCEPT".to_string();
    }
    if ql.contains(" union ") {
        return "blocker:UNION".to_string();
    }
    // SharedAlphabet / catalog / already-specific set-op keys stay put for
    // multi-issue queries that also mention COUNT(DISTINCT) etc.
    const PRECISE: &[&str] = &[
        "blocker:integer_u8_band",
        "blocker:integer_capacity",
        "blocker:unknown_table",
        "blocker:ORDER_BY_AGG",
        "blocker:IN",
        "blocker:BETWEEN",
        "blocker:UNION",
        "blocker:INTERSECT",
        "blocker:EXCEPT",
        "blocker:COUNT_DISTINCT",
    ];
    if PRECISE.contains(&key) {
        return key.to_string();
    }
    if ql.contains("count(distinct") || ql.contains("count (distinct") {
        return "blocker:COUNT_DISTINCT".to_string();
    }
    // JOIN without ON often surfaces as expected_token ("expected ON") — bucket
    // with join_shape residual rather than opaque expected_token when the query
    // has JOIN … WHERE with no ON clause text.
    if (key == "blocker:expected_token" || key == "blocker:unexpected_token")
        && ql.contains(" join ")
        && !ql.contains(" on ")
        && ql.contains(" where ")
    {
        return "blocker:join_shape".to_string();
    }
    key.to_string()
}

/// Artifact kind for the keyword-named-column census.
pub const SQL_KEYWORD_COLUMN_CENSUS_ARTIFACT: &str = "tct_sql_keyword_column_census_v0";

/// One catalog column whose name collides with a subset-lexer keyword.
///
/// Schema-derived: the hazard set is
/// [`SQL_SUBSET_KEYWORDS`](crate::sql_schema_linking::SQL_SUBSET_KEYWORDS), a
/// superset of the keyword literals the parser lexes, not any observed model
/// error profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeywordColumnCensusRow {
    pub db_id: String,
    pub table: String,
    /// Spider `column_names_original` spelling (the SQL identifier).
    pub column: String,
    /// Which catalog field collided: `"original"`, `"display"`, or `"both"`.
    pub matched_field: String,
    /// Index inside the owning table = menu ordinal in a single-table scope.
    pub per_table_index: u32,
    /// Menu ordinal reached by the bare spelling, when it parses.
    pub bare_ordinal: Option<u32>,
    /// Parse error for the bare spelling, when it does not.
    pub bare_error: Option<String>,
    /// Menu ordinal reached by the double-quoted spelling.
    pub quoted_ordinal: Option<u32>,
    pub quoted_error: Option<String>,
    /// `parse → encode → decode → render → reparse` fixpoint on the bare form,
    /// under both `Flat` and `Factored` column addressing.
    pub round_trip_ok: bool,
    /// Codec construction failure (menu binding / capacity), if any. Never a
    /// keyword finding: [`KeywordColumnCensusRow::survives_to_menu`] ignores it.
    pub codec_error: Option<String>,
}

impl KeywordColumnCensusRow {
    /// The column survives to the menu iff both spellings reach its ordinal and
    /// the bare form round-trips.
    ///
    /// Scope of the claim: the **select-list** position over a single-table
    /// scope, plus the render fixpoint. It is not a claim about every
    /// grammatical position — see the documented residuals in
    /// `docs/sql-schema-linking-frontend-contract.md` (P7).
    pub fn survives_to_menu(&self) -> bool {
        self.codec_error.is_none()
            && self.bare_ordinal == Some(self.per_table_index)
            && self.quoted_ordinal == Some(self.per_table_index)
            && self.round_trip_ok
    }
}

/// A database whose catalog could not be loaded, so it could not be censused.
///
/// Recorded separately from [`KeywordColumnCensusRow`]: a fail-closed catalog is
/// not a keyword finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnloadableCensusDatabase {
    pub db_id: String,
    pub error: String,
}

/// Keyword-named-column census over one Spider-style multi-db `tables.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeywordColumnCensus {
    pub artifact_kind: String,
    pub version: u32,
    pub claim_boundary: String,
    pub evidence_class: String,
    pub claim_bearing: bool,
    pub paper_extractable: bool,
    pub subset_id: String,
    pub tables_json_path: String,
    /// SHA-256 of the censused `tables.json` (from the first loaded catalog).
    pub tables_json_sha256: Option<String>,
    pub n_databases_requested: usize,
    pub n_databases_censused: usize,
    pub rows: Vec<KeywordColumnCensusRow>,
    pub unloadable_databases: Vec<UnloadableCensusDatabase>,
}

/// Census every keyword-named column in a Spider-style multi-db `tables.json`.
///
/// Each hit is probed through the real frontend path (`SELECT <col> FROM <table>`
/// over a single-table scope, bare and double-quoted) so the report
/// says whether the column is reachable as a menu ordinal, not merely present
/// in the catalog. The round trip runs under both `Flat` and the deployed
/// `Factored` column addressing. When `db_ids` is `Some`, only those databases
/// are censused; unknown ids are reported in `unloadable_databases`.
///
/// Diagnostic only: not claim-bearing and not paper-extractable.
pub fn census_keyword_named_columns(
    tables_path: &Path,
    db_ids: Option<&[String]>,
) -> Result<KeywordColumnCensus, String> {
    use crate::sql_schema_linking::menu_binding::{SqlColumnAddressing, SqlJoinOnPolicy};

    let all_ids = SqlCatalog::list_db_ids(tables_path).map_err(|e| e.to_string())?;
    let selected: Vec<String> = match db_ids {
        Some(wanted) => wanted.to_vec(),
        None => all_ids,
    };
    let n_databases_requested = selected.len();
    let mut n_databases_censused = 0usize;
    let mut tables_json_sha256: Option<String> = None;
    let mut rows = Vec::new();
    let mut unloadable_databases = Vec::new();
    for db_id in selected {
        let catalog = match SqlCatalog::load_unpinned_tables_json(tables_path, &db_id) {
            Ok(catalog) => catalog,
            Err(error) => {
                unloadable_databases.push(UnloadableCensusDatabase {
                    db_id: db_id.clone(),
                    error: error.to_string(),
                });
                continue;
            }
        };
        n_databases_censused += 1;
        if tables_json_sha256.is_none() {
            tables_json_sha256 = Some(catalog.tables_json_sha256.clone());
        }
        let mut hits: Vec<(String, String, String, u32)> = Vec::new();
        for table in &catalog.tables {
            for (index, column) in table.columns.iter().enumerate() {
                let original = is_sql_subset_keyword(&column.name_original);
                let display = is_sql_subset_keyword(&column.name);
                let matched_field = match (original, display) {
                    (true, true) => "both",
                    (true, false) => "original",
                    (false, true) => "display",
                    (false, false) => continue,
                };
                let index = u32::try_from(index)
                    .map_err(|_| format!("column index overflow in {db_id}.{}", table.name))?;
                hits.push((
                    table.name_original.clone(),
                    column.name_original.clone(),
                    matched_field.to_string(),
                    index,
                ));
            }
        }
        if hits.is_empty() {
            continue;
        }
        let flat = SqlSchemaLinkingCodec::for_catalog(catalog.clone());
        let factored = SqlSchemaLinkingCodec::for_catalog_with_policies(
            catalog,
            SqlJoinOnPolicy::Structural,
            SqlColumnAddressing::Factored,
        );
        for (table, column, matched_field, per_table_index) in hits {
            let mut row = KeywordColumnCensusRow {
                db_id: db_id.clone(),
                table: table.clone(),
                column: column.clone(),
                matched_field,
                per_table_index,
                bare_ordinal: None,
                bare_error: None,
                quoted_ordinal: None,
                quoted_error: None,
                round_trip_ok: false,
                codec_error: None,
            };
            match (&flat, &factored) {
                (Err(error), _) => row.codec_error = Some(format!("flat codec: {error}")),
                (_, Err(error)) => row.codec_error = Some(format!("factored codec: {error}")),
                (Ok(flat), Ok(factored)) => {
                    // Quote only the table so the probe isolates the column.
                    let table_ident = quote_probe_ident(&table);
                    let bare = format!("SELECT {column} FROM {table_ident}");
                    let quoted =
                        format!("SELECT {} FROM {table_ident}", quote_probe_ident(&column));
                    match flat.parse(&bare) {
                        Ok(stmt) => row.bare_ordinal = first_select_column_ordinal(&stmt),
                        Err(error) => row.bare_error = Some(error.to_string()),
                    }
                    match flat.parse(&quoted) {
                        Ok(stmt) => row.quoted_ordinal = first_select_column_ordinal(&stmt),
                        Err(error) => row.quoted_error = Some(error.to_string()),
                    }
                    row.round_trip_ok = flat.round_trip_source(&bare).is_ok()
                        && factored.round_trip_source(&bare).is_ok();
                }
            }
            rows.push(row);
        }
    }
    Ok(KeywordColumnCensus {
        artifact_kind: SQL_KEYWORD_COLUMN_CENSUS_ARTIFACT.to_string(),
        version: 0,
        claim_boundary: SQL_SPIDER_PARSE_COVERAGE_CLAIM_BOUNDARY.to_string(),
        evidence_class: "schema-derived diagnostic".to_string(),
        claim_bearing: false,
        paper_extractable: false,
        subset_id: crate::sql_schema_linking::codec::SQL_CODEC_SUBSET_ID.to_string(),
        tables_json_path: tables_path.display().to_string(),
        tables_json_sha256,
        n_databases_requested,
        n_databases_censused,
        rows,
        unloadable_databases,
    })
}

/// Double-quote an identifier for a census probe (`"` doubled, as the parser
/// expects). Probes always quote the table so only the column spelling varies.
fn quote_probe_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn first_select_column_ordinal(
    stmt: &crate::sql_schema_linking::ast::SelectStatement,
) -> Option<u32> {
    use crate::sql_schema_linking::ast::{SelectItem, SelectList};
    match &stmt.select_list {
        SelectList::Items(items) => match items.items.first()? {
            SelectItem::Column(column) => Some(column.menu_index),
            _ => None,
        },
        SelectList::Star => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_schema_linking::catalog::fixture_dir;
    use crate::sql_schema_linking::render::SQL_SUBSET_KEYWORDS;
    use std::path::PathBuf;

    #[test]
    fn classify_integer_literal_errors_as_u8_band() {
        let msg =
            "SQL codec error (parse): SQL parse error at offset 48 in rule integer_literal: i…";
        assert_eq!(classify_parse_error(msg), "blocker:integer_u8_band");
        assert_eq!(
            classify_parse_error("literal 2014 exceeds 255 u8 band"),
            "blocker:integer_u8_band"
        );
        assert_eq!(
            classify_parse_error(
                "integer literal 16777216 exceeds integer capacity (16777215); residual bucket integer_capacity"
            ),
            "blocker:integer_capacity"
        );
        // SQL IN list (requires space + "in" + '(') — not confused by integer_literal rule text.
        assert_eq!(
            classify_parse_error("feature not supported: in (subquery)"),
            "blocker:IN"
        );
        // Bare unexpected without IN/join needles.
        assert_eq!(
            classify_parse_error("unexpected token: FOO"),
            "blocker:unexpected_token"
        );
        assert_eq!(
            classify_parse_error("feature not supported: order by count(*)"),
            "blocker:ORDER_BY_AGG"
        );
        // Real parser path: ORDER BY count(*) resolves as a column_ref failure.
        assert_eq!(
            classify_parse_error("SQL parse error: column count is not in scope for catalog x"),
            "blocker:ORDER_BY_AGG"
        );
        assert_eq!(
            classify_parse_error("column COUNT(*) is not in scope for catalog fixture"),
            "blocker:ORDER_BY_AGG"
        );
        // Ordinary column scope failures stay column_not_in_scope.
        assert_eq!(
            classify_parse_error("column year is not in scope for catalog fixture"),
            "blocker:column_not_in_scope"
        );
        assert_eq!(
            classify_parse_error("unexpected token: DISTINCT in aggregate"),
            "blocker:DISTINCT_EXTRA"
        );
        assert_eq!(
            classify_parse_error("feature not supported: count(distinct pettype)"),
            "blocker:COUNT_DISTINCT"
        );
        assert_eq!(
            refine_blocker_with_query(
                "blocker:aggregate",
                "SELECT count(DISTINCT pettype) FROM pets"
            ),
            "blocker:COUNT_DISTINCT"
        );
        // SharedAlphabet u8 failure wins even when the query also has COUNT(DISTINCT).
        assert_eq!(
            refine_blocker_with_query(
                "blocker:integer_u8_band",
                "SELECT count(DISTINCT Language) FROM country WHERE IndepYear < 1930"
            ),
            "blocker:integer_u8_band"
        );
        assert_eq!(
            refine_blocker_with_query("blocker:aggregate", "SELECT COUNT(*) FROM pets"),
            "blocker:aggregate"
        );
        assert_eq!(
            refine_blocker_with_query(
                "blocker:expected_token",
                "SELECT a FROM t JOIN u WHERE x < 1"
            ),
            "blocker:join_shape"
        );
        // Nested INTERSECT must not stay mislabeled as CTE_WITH from message text.
        assert_eq!(
            refine_blocker_with_query(
                "blocker:CTE_WITH",
                "SELECT COUNT(*) FROM (SELECT a FROM t INTERSECT SELECT a FROM u)"
            ),
            "blocker:INTERSECT"
        );
        // EXCEPT after JOIN must not stay join_shape when set op is present.
        assert_eq!(
            refine_blocker_with_query(
                "blocker:join_shape",
                "SELECT a FROM t JOIN u ON t.i = u.i EXCEPT SELECT a FROM v"
            ),
            "blocker:EXCEPT"
        );
        assert_eq!(classify_parse_error("unexpected token: IN"), "blocker:IN");
    }

    #[test]
    fn fixture_examples_parse_at_full_rate() {
        let tables = fixture_dir().join("tables.json");
        // Tiny synthetic examples over the join fixture.
        let examples = fixture_dir().join("gold_queries.json");
        // gold_queries may not be Spider example shape; skip if wrong shape
        let raw = std::fs::read_to_string(&examples).expect("gold");
        if !raw.contains("\"db_id\"") {
            // Build an in-memory temp file
            let dir = std::env::temp_dir().join("tct_sql_coverage_fixture");
            let _ = std::fs::create_dir_all(&dir);
            let ex_path = dir.join("examples.json");
            std::fs::write(
                &ex_path,
                r#"[
                  {"db_id":"tct_fixture_join","query":"SELECT * FROM customers"},
                  {"db_id":"tct_fixture_join","query":"SELECT name FROM customers"}
                ]"#,
            )
            .expect("write");
            let report = measure_spider_parse_coverage(&tables, &ex_path, None).expect("coverage");
            assert_eq!(report.n_total, 2);
            assert_eq!(report.n_parse_ok, 2);
            assert_eq!(report.n_encode_ok, 2);
            assert!((report.parseable_fraction - 1.0).abs() < 1e-9);
            assert!((report.encode_fraction_of_parse_ok - 1.0).abs() < 1e-9);
        }
    }

    /// Spider dev/test host catalogs: every keyword-named column must survive.
    ///
    /// Schema-derived hazard set (`SQL_SUBSET_KEYWORDS`), not an error profile.
    /// Censuses every database in each mounted `tables.json` so the report also
    /// bounds the blast radius outside the dev/test splits. Skips when no host
    /// catalog is mounted; set `TCT_SPIDER_TABLES_JSON` to point at one.
    #[test]
    fn keyword_named_columns_survive_to_the_menu_on_host_spider_catalogs() {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(explicit) = std::env::var_os("TCT_SPIDER_TABLES_JSON") {
            candidates.push(PathBuf::from(explicit));
        }
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            candidates.push(home.join("data/spider-official/tables.json"));
            candidates.push(home.join("data/spider-1.0-full/spider_data/test_tables.json"));
        }
        let mounted: Vec<PathBuf> = candidates.into_iter().filter(|p| p.is_file()).collect();
        if mounted.is_empty() {
            eprintln!(
                "skip keyword_named_columns_survive_to_the_menu_on_host_spider_catalogs: \
                 set TCT_SPIDER_TABLES_JSON or place data/spider-official/tables.json"
            );
            return;
        }
        assert!(
            SQL_SUBSET_KEYWORDS.contains(&"COUNT") && SQL_SUBSET_KEYWORDS.contains(&"FROM"),
            "hazard set must cover the subset lexer's keywords"
        );

        let mut failures: Vec<String> = Vec::new();
        for path in &mounted {
            let census = census_keyword_named_columns(path, None).expect("census");
            eprintln!(
                "keyword-column census {}: {} db(s), {} hit(s), {} unloadable",
                path.display(),
                census.n_databases_censused,
                census.rows.len(),
                census.unloadable_databases.len()
            );
            for unloadable in &census.unloadable_databases {
                // Not a keyword finding: a catalog that fails closed is reported
                // but does not fail this gate.
                eprintln!(
                    "  [uncensusable] {}: {}",
                    unloadable.db_id, unloadable.error
                );
            }
            for row in &census.rows {
                eprintln!(
                    "  {}.{}.{} matched={} per_table_index={} bare={:?} quoted={:?} \
                     round_trip={} {}",
                    row.db_id,
                    row.table,
                    row.column,
                    row.matched_field,
                    row.per_table_index,
                    row.bare_ordinal,
                    row.quoted_ordinal,
                    row.round_trip_ok,
                    row.bare_error.as_deref().unwrap_or("")
                );
                if let Some(error) = &row.codec_error {
                    // Menu-binding / capacity failure, not a keyword finding.
                    eprintln!("  [uncompilable] {}: {error}", row.db_id);
                    continue;
                }
                if row.column.eq_ignore_ascii_case("DISTINCT") {
                    // Documented residual (contract P7): quoted-only. Assert the
                    // escape hatch instead of the bare form.
                    assert_eq!(
                        row.quoted_ordinal,
                        Some(row.per_table_index),
                        "quoted DISTINCT column must still reach its ordinal"
                    );
                    continue;
                }
                if !row.survives_to_menu() {
                    failures.push(format!(
                        "{}.{}.{} (index {}) does not survive to the menu: bare={:?}/{:?} \
                         quoted={:?}/{:?} round_trip={}",
                        row.db_id,
                        row.table,
                        row.column,
                        row.per_table_index,
                        row.bare_ordinal,
                        row.bare_error,
                        row.quoted_ordinal,
                        row.quoted_error,
                        row.round_trip_ok
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "keyword-named columns dropped by the frontend:\n  {}",
            failures.join("\n  ")
        );
    }

    /// Synthetic keyword catalog: the census must find and clear every hazard.
    #[test]
    fn keyword_column_census_clears_a_synthetic_all_keyword_catalog() {
        let mut column_names = vec![serde_json::json!([-1, "*"])];
        let mut column_names_original = vec![serde_json::json!([-1, "*"])];
        let mut column_types = vec![serde_json::json!("text")];
        column_names.push(serde_json::json!([0, "id"]));
        column_names_original.push(serde_json::json!([0, "id"]));
        column_types.push(serde_json::json!("number"));
        for keyword in SQL_SUBSET_KEYWORDS {
            column_names.push(serde_json::json!([0, keyword.to_ascii_lowercase()]));
            column_names_original.push(serde_json::json!([0, keyword]));
            column_types.push(serde_json::json!("number"));
        }
        let raw = serde_json::json!([{
            "db_id": "kw_census",
            "table_names": ["t"],
            "table_names_original": ["t"],
            "column_names": column_names,
            "column_names_original": column_names_original,
            "column_types": column_types,
            "primary_keys": [1],
            "foreign_keys": [],
        }]);
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("tables.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&raw).expect("serialize"))
            .expect("write synthetic tables.json");

        let census = census_keyword_named_columns(&path, None).expect("census");
        assert!(census.unloadable_databases.is_empty());
        let rows = &census.rows;
        assert_eq!(
            rows.len(),
            SQL_SUBSET_KEYWORDS.len(),
            "census must find one row per keyword-spelled column"
        );
        // `DISTINCT` stays ambiguous bare in select-list position: `SELECT
        // DISTINCT FROM t` is the DISTINCT modifier with an empty list, so the
        // quoted spelling is the only reachable one. Everything else must be
        // reachable both ways.
        for row in rows {
            assert_eq!(
                row.quoted_ordinal,
                Some(row.per_table_index),
                "quoted {} must reach its ordinal",
                row.column
            );
            if row.column.eq_ignore_ascii_case("DISTINCT") {
                assert!(
                    row.bare_ordinal != Some(row.per_table_index),
                    "bare DISTINCT is documented as ambiguous; update this test if it is lifted"
                );
                continue;
            }
            assert!(
                row.survives_to_menu(),
                "keyword column {} must survive to the menu: {row:?}",
                row.column
            );
        }
    }

    /// Optional host Spider validation coverage when paths are provided via env.
    #[test]
    fn optional_host_spider_validation_parse_coverage() {
        let tables = std::env::var_os("TCT_SPIDER_TABLES_JSON").map(PathBuf::from);
        let examples = std::env::var_os("TCT_SPIDER_EXAMPLES_JSON").map(PathBuf::from);
        let out = std::env::var_os("TCT_SPIDER_COVERAGE_OUT").map(PathBuf::from);
        let (Some(tables), Some(examples), Some(out)) = (tables, examples, out) else {
            eprintln!(
                "skip optional_host_spider_validation_parse_coverage: set \
                 TCT_SPIDER_TABLES_JSON, TCT_SPIDER_EXAMPLES_JSON, TCT_SPIDER_COVERAGE_OUT"
            );
            return;
        };
        if !tables.is_file() || !examples.is_file() {
            eprintln!("skip: missing tables or examples");
            return;
        }
        let max = std::env::var("TCT_SPIDER_COVERAGE_MAX")
            .ok()
            .and_then(|s| s.parse().ok());
        let report =
            measure_spider_parse_coverage(&tables, &examples, max).expect("measure coverage");
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(&report).expect("serialize");
        std::fs::write(&out, format!("{json}\n")).expect("write coverage");
        eprintln!(
            "spider parse coverage: n_total={} n_ok={} frac={:.4} out={}",
            report.n_total,
            report.n_parse_ok,
            report.parseable_fraction,
            out.display()
        );
        // Always pass: the measured fraction is the finding.
        assert!(report.n_total > 0);
    }

    /// Optional: export A5-shaped token streams for parseable examples.
    #[test]
    fn optional_host_spider_token_stream_export() {
        let tables = std::env::var_os("TCT_SPIDER_TABLES_JSON").map(PathBuf::from);
        let examples = std::env::var_os("TCT_SPIDER_EXAMPLES_JSON").map(PathBuf::from);
        let out = std::env::var_os("TCT_SPIDER_STREAMS_OUT").map(PathBuf::from);
        let (Some(tables), Some(examples), Some(out)) = (tables, examples, out) else {
            eprintln!(
                "skip optional_host_spider_token_stream_export: set \
                 TCT_SPIDER_TABLES_JSON, TCT_SPIDER_EXAMPLES_JSON, TCT_SPIDER_STREAMS_OUT"
            );
            return;
        };
        if !tables.is_file() || !examples.is_file() {
            eprintln!("skip: missing tables or examples");
            return;
        }
        let max = std::env::var("TCT_SPIDER_COVERAGE_MAX")
            .ok()
            .and_then(|s| s.parse().ok());
        let (streams, report) =
            encode_spider_examples(&tables, &examples, max).expect("encode streams");
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let payload = serde_json::json!({
            "artifact_kind": "tct_sql_a5_token_streams_v0",
            "version": 0,
            "claim_boundary": SQL_SPIDER_PARSE_COVERAGE_CLAIM_BOUNDARY,
            "evidence_class": "dry-run integration",
            "claim_bearing": false,
            "paper_extractable": false,
            "n_streams": streams.len(),
            "coverage": report,
            "streams": streams,
            "notes": [
                "Producer A5 token streams for parseable examples only.",
                "Not claim-bearing training data; no optimizer step.",
            ],
        });
        std::fs::write(
            &out,
            format!("{}\n", serde_json::to_string_pretty(&payload).unwrap()),
        )
        .expect("write streams");
        eprintln!(
            "spider token streams: n={} out={}",
            streams.len(),
            out.display()
        );
        assert!(!streams.is_empty() || report.n_parse_ok == 0);
    }

    /// Optional: full producer round-trip on encodeable host examples.
    #[test]
    fn optional_host_spider_round_trip() {
        let tables = std::env::var_os("TCT_SPIDER_TABLES_JSON").map(PathBuf::from);
        let examples = std::env::var_os("TCT_SPIDER_EXAMPLES_JSON").map(PathBuf::from);
        let out = std::env::var_os("TCT_SPIDER_ROUNDTRIP_OUT").map(PathBuf::from);
        let (Some(tables), Some(examples), Some(out)) = (tables, examples, out) else {
            eprintln!(
                "skip optional_host_spider_round_trip: set \
                 TCT_SPIDER_TABLES_JSON, TCT_SPIDER_EXAMPLES_JSON, TCT_SPIDER_ROUNDTRIP_OUT"
            );
            return;
        };
        if !tables.is_file() || !examples.is_file() {
            eprintln!("skip: missing tables or examples");
            return;
        }
        let max = std::env::var("TCT_SPIDER_COVERAGE_MAX")
            .ok()
            .and_then(|s| s.parse().ok());
        let report = round_trip_spider_examples(&tables, &examples, max).expect("round-trip");
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(&report).expect("serialize");
        std::fs::write(&out, format!("{json}\n")).expect("write round-trip");
        eprintln!(
            "spider round-trip: n_attempted={} n_ok={} frac={:.4} out={}",
            report.n_attempted,
            report.n_round_trip_ok,
            report.round_trip_fraction_of_attempted,
            out.display()
        );
        // Always pass: fraction is the finding (encodeable may still fail render identity).
        assert!(report.n_attempted > 0 || report.n_round_trip_ok == 0);
    }
}
