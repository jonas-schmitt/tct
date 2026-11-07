//! Bounded gold-stats report for the SQL schema-linking frontend (T11).
//!
//! Quantifies the schema-linking win: **tokens per schema reference under
//! menu-index encoding versus the open-vocabulary UTF-8 baseline** on the same
//! gold queries. Baseline cost for an n-byte identifier is n+2 tokens
//! (terminal, n byte tokens, payload end) against exactly one menu token.
//!
//! **Claim boundary:** the n+2 open-vocab payload statistic is a
//! **compiler-internal diagnostic only**. It must never become a paper-facing
//! baseline or paper number. Paper residual-compression comparisons use the
//! strongest steelmanned canonical-SQL baseline (**A2**: parser/CFG,
//! jump-forward, and dynamic prefix-time scope masking) from the models-side
//! experiment contracts.
//!
//! Packaging and measurement evidence only — not training, evaluation, or a
//! paper claim. Parse failures are recorded explicitly (**P8**/T11).

use crate::sql_schema_linking::ast::{
    AggregateArg, Expression, SelectItem, SelectList, SelectStatement,
};
use crate::sql_schema_linking::catalog::{SqlCatalog, SqlFixtureBundle, load_fixture_bundle};
use crate::sql_schema_linking::codec::{
    SQL_CODEC_CLAIM_BOUNDARY, SqlSchemaLinkingCodec, count_menu_tokens,
    count_menu_tokens_by_role_replay,
};
use crate::sql_schema_linking::layout::{
    SQL_SYNTAX_LAYOUT_PAD_TOKEN, SQL_SYNTAX_LAYOUT_VOCAB_SIZE, SQL_SYNTAX_TOKEN_LAYOUT_ID,
};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Artifact kind for the bounded gold-stats summary.
pub const SQL_GOLD_STATS_ARTIFACT_KIND: &str = "tct_sql_schema_linking_gold_stats_v0";
/// Explicit claim boundary (not a paper claim).
pub const SQL_GOLD_STATS_CLAIM_BOUNDARY: &str = "compiler_sql_schema_linking_only";
/// Report version.
pub const SQL_GOLD_STATS_VERSION: u32 = 0;

/// One gold query outcome in the stats report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoldQueryStat {
    pub id: String,
    pub sql: String,
    pub parse_ok: bool,
    pub parse_error: Option<String>,
    pub round_trip_ok: bool,
    pub round_trip_error: Option<String>,
    /// Menu-band **digit** tokens in the encoded stream (may be > sites when
    /// multi-digit ordinals are used).
    pub menu_token_count: Option<usize>,
    /// Alias of [`Self::menu_token_count`] (digit tokens, not sites).
    #[serde(default)]
    pub menu_digit_count: Option<usize>,
    /// Total encoded tokens (menu encoding).
    pub stream_token_count: Option<usize>,
    /// Number of table + column menu **sites** resolved from the AST.
    pub schema_ref_site_count: Option<usize>,
    /// Alias of [`Self::schema_ref_site_count`] (sites, not digits).
    #[serde(default)]
    pub menu_site_count: Option<usize>,
    /// Open-vocab UTF-8 baseline tokens for those schema refs (n+2 each).
    pub baseline_schema_ref_tokens: Option<usize>,
    /// Menu encoding cost for those schema refs in the historical 1-digit
    /// accounting (equals site count). Prefer [`Self::menu_digit_count`] for
    /// stream digit cost under multi-digit menus.
    pub menu_schema_ref_tokens: Option<usize>,
}

/// Aggregate bounded gold stats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SqlGoldStatsReport {
    pub artifact_kind: String,
    pub version: u32,
    pub claim_boundary: String,
    pub codec_claim_boundary: String,
    pub no_training_claims: bool,
    pub layout_id: String,
    pub vocab_size: u32,
    pub pad_token: u32,
    pub db_id: String,
    pub tables_json_sha256: String,
    pub gold_queries_json_sha256: String,
    pub repo_commit: Option<String>,
    pub query_count: usize,
    pub parse_ok_count: usize,
    pub parse_fail_count: usize,
    pub round_trip_ok_count: usize,
    pub total_schema_ref_sites: usize,
    /// Alias of [`Self::total_schema_ref_sites`] (menu **sites**).
    #[serde(default)]
    pub total_menu_site_count: usize,
    pub total_menu_schema_ref_tokens: usize,
    /// Sum of menu **digit** tokens over round-tripped gold streams.
    #[serde(default)]
    pub total_menu_digit_count: usize,
    pub total_baseline_schema_ref_tokens: usize,
    /// `baseline / menu` over successful schema-ref sites (integer ratio when possible).
    pub menu_vs_baseline_ratio_numerator: usize,
    pub menu_vs_baseline_ratio_denominator: usize,
    pub queries: Vec<GoldQueryStat>,
}

#[derive(Debug, Deserialize)]
struct GoldFile {
    queries: Vec<GoldQueryEntry>,
}

#[derive(Debug, Deserialize)]
struct GoldQueryEntry {
    id: String,
    sql: String,
}

/// Build the T11 gold-stats report from the in-repo fixture bundle.
pub fn fixture_gold_stats_report() -> Result<SqlGoldStatsReport, String> {
    let bundle = load_fixture_bundle().map_err(|e| e.to_string())?;
    gold_stats_report_from_bundle(&bundle)
}

/// Build the T11 gold-stats report from a loaded fixture bundle.
pub fn gold_stats_report_from_bundle(
    bundle: &SqlFixtureBundle,
) -> Result<SqlGoldStatsReport, String> {
    let gold: GoldFile = serde_json::from_slice(&bundle.gold_queries_bytes)
        .map_err(|e| format!("gold_queries.json parse: {e}"))?;
    let codec = SqlSchemaLinkingCodec::for_catalog(bundle.catalog.clone())
        .map_err(|e| format!("codec: {e}"))?;
    let mut queries = Vec::with_capacity(gold.queries.len());
    let mut parse_ok_count = 0usize;
    let mut parse_fail_count = 0usize;
    let mut round_trip_ok_count = 0usize;
    let mut total_schema_ref_sites = 0usize;
    let mut total_menu_schema_ref_tokens = 0usize;
    let mut total_menu_digit_count = 0usize;
    let mut total_baseline_schema_ref_tokens = 0usize;

    for entry in &gold.queries {
        let mut stat = GoldQueryStat {
            id: entry.id.clone(),
            sql: entry.sql.clone(),
            parse_ok: false,
            parse_error: None,
            round_trip_ok: false,
            round_trip_error: None,
            menu_token_count: None,
            menu_digit_count: None,
            stream_token_count: None,
            schema_ref_site_count: None,
            menu_site_count: None,
            baseline_schema_ref_tokens: None,
            menu_schema_ref_tokens: None,
        };
        match codec.parse(&entry.sql) {
            Ok(stmt) => {
                parse_ok_count += 1;
                stat.parse_ok = true;
                let (sites, baseline) = schema_ref_costs(&stmt, codec.catalog());
                stat.schema_ref_site_count = Some(sites);
                stat.menu_site_count = Some(sites);
                stat.baseline_schema_ref_tokens = Some(baseline);
                // Site-count accounting (1 per schema ref); digit cost may differ.
                stat.menu_schema_ref_tokens = Some(sites);
                total_schema_ref_sites += sites;
                total_menu_schema_ref_tokens += sites;
                total_baseline_schema_ref_tokens += baseline;
                match codec.round_trip_source(&entry.sql) {
                    Ok(rt) => {
                        round_trip_ok_count += 1;
                        stat.round_trip_ok = true;
                        stat.stream_token_count = Some(rt.tokens.len());
                        // PerGrammar default: dedicated menu band. Fall back to
                        // role replay if range-only is unavailable.
                        let digits = count_menu_tokens(&rt.tokens, codec.model()).or_else(|| {
                            count_menu_tokens_by_role_replay(&rt.tokens, codec.model()).ok()
                        });
                        stat.menu_token_count = digits;
                        stat.menu_digit_count = digits;
                        if let Some(d) = digits {
                            total_menu_digit_count += d;
                        }
                    }
                    Err(e) => {
                        stat.round_trip_error = Some(e.to_string());
                    }
                }
            }
            Err(e) => {
                parse_fail_count += 1;
                stat.parse_error = Some(e.to_string());
            }
        }
        queries.push(stat);
    }

    Ok(SqlGoldStatsReport {
        artifact_kind: SQL_GOLD_STATS_ARTIFACT_KIND.to_string(),
        version: SQL_GOLD_STATS_VERSION,
        claim_boundary: SQL_GOLD_STATS_CLAIM_BOUNDARY.to_string(),
        codec_claim_boundary: SQL_CODEC_CLAIM_BOUNDARY.to_string(),
        no_training_claims: true,
        layout_id: SQL_SYNTAX_TOKEN_LAYOUT_ID.to_string(),
        vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        db_id: bundle.catalog.db_id.clone(),
        tables_json_sha256: bundle.pin.tables_json_sha256.clone(),
        gold_queries_json_sha256: bundle.gold_queries_json_sha256.clone(),
        repo_commit: git_head_sha(),
        query_count: queries.len(),
        parse_ok_count,
        parse_fail_count,
        round_trip_ok_count,
        total_schema_ref_sites,
        total_menu_site_count: total_schema_ref_sites,
        total_menu_schema_ref_tokens,
        total_menu_digit_count,
        total_baseline_schema_ref_tokens,
        menu_vs_baseline_ratio_numerator: total_baseline_schema_ref_tokens,
        menu_vs_baseline_ratio_denominator: total_menu_schema_ref_tokens.max(1),
        queries,
    })
}

/// Render a short markdown summary of the gold-stats report.
pub fn gold_stats_markdown(report: &SqlGoldStatsReport) -> String {
    let ratio = if report.total_menu_schema_ref_tokens == 0 {
        0.0
    } else {
        report.total_baseline_schema_ref_tokens as f64 / report.total_menu_schema_ref_tokens as f64
    };
    let mut lines = vec![
        "# SQL schema-linking gold stats (T11)".to_string(),
        String::new(),
        format!("- artifact: `{}`", report.artifact_kind),
        format!("- claim_boundary: `{}`", report.claim_boundary),
        format!("- no_training_claims: {}", report.no_training_claims),
        format!("- db_id: `{}`", report.db_id),
        format!("- layout_id: `{}`", report.layout_id),
        format!(
            "- queries: {} (parse_ok={}, parse_fail={}, round_trip_ok={})",
            report.query_count,
            report.parse_ok_count,
            report.parse_fail_count,
            report.round_trip_ok_count
        ),
        format!(
            "- schema_ref sites: {} menu tokens vs {} baseline tokens (ratio ≈ {:.2}×)",
            report.total_menu_schema_ref_tokens, report.total_baseline_schema_ref_tokens, ratio
        ),
        String::new(),
        "Parse failures are listed explicitly (not filtered out).".to_string(),
        String::new(),
        "| id | parse | round_trip | menu sites | baseline tokens | menu tokens |".to_string(),
        "| --- | --- | --- | ---: | ---: | ---: |".to_string(),
    ];
    for q in &report.queries {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            q.id,
            if q.parse_ok { "ok" } else { "FAIL" },
            if q.round_trip_ok {
                "ok"
            } else if q.parse_ok {
                "FAIL"
            } else {
                "n/a"
            },
            q.schema_ref_site_count
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".into()),
            q.baseline_schema_ref_tokens
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".into()),
            q.menu_schema_ref_tokens
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".into()),
        ));
        if let Some(err) = &q.parse_error {
            lines.push(format!("| ↳ parse_error | `{err}` | | | | |"));
        }
        if let Some(err) = &q.round_trip_error {
            lines.push(format!("| ↳ round_trip_error | `{err}` | | | | |"));
        }
    }
    // Exactly one trailing newline; avoid a blank line at EOF (`git diff --check`).
    lines.join("\n") + "\n"
}

/// Count schema-ref sites and open-vocab baseline tokens for an AST.
///
/// Each table/column menu site contributes **1** menu token and **n+2**
/// baseline tokens where n is the UTF-8 byte length of the resolved name.
fn schema_ref_costs(stmt: &SelectStatement, catalog: &SqlCatalog) -> (usize, usize) {
    let mut sites = 0usize;
    let mut baseline = 0usize;
    walk_select(stmt, catalog, &mut sites, &mut baseline);
    (sites, baseline)
}

fn walk_select(
    stmt: &SelectStatement,
    catalog: &SqlCatalog,
    sites: &mut usize,
    baseline: &mut usize,
) {
    // Table sites: head + each join table.
    add_table_site(stmt.from.head.menu_index, catalog, sites, baseline);
    for join in &stmt.from.joins {
        add_table_site(join.table.menu_index, catalog, sites, baseline);
        walk_expr(&join.on, stmt, catalog, sites, baseline);
    }
    if let Some(ref w) = stmt.where_clause {
        walk_expr(w, stmt, catalog, sites, baseline);
    }
    if let Some(ref g) = stmt.group_by {
        for col in &g.columns {
            add_column_site(col.menu_index, stmt, catalog, sites, baseline);
        }
    }
    if let Some(ref h) = stmt.having {
        walk_expr(h, stmt, catalog, sites, baseline);
    }
    if let Some(ref o) = stmt.order_by {
        for item in &o.items {
            match &item.key {
                crate::sql_schema_linking::ast::OrderKey::Column(col) => {
                    add_column_site(col.menu_index, stmt, catalog, sites, baseline);
                }
                crate::sql_schema_linking::ast::OrderKey::CountStar => {
                    // Structural COUNT(*) — no menu column site.
                }
                crate::sql_schema_linking::ast::OrderKey::Aggregate { arg, .. } => {
                    let crate::sql_schema_linking::ast::AggregateArg::Column(col) = arg;
                    add_column_site(col.menu_index, stmt, catalog, sites, baseline);
                }
            }
        }
    }
    match &stmt.select_list {
        SelectList::Star => {}
        SelectList::Items(items) => {
            for item in &items.items {
                match item {
                    SelectItem::Column(col) => {
                        add_column_site(col.menu_index, stmt, catalog, sites, baseline);
                    }
                    SelectItem::CountStar => {}
                    SelectItem::Aggregate { arg, .. } => {
                        let AggregateArg::Column(col) = arg;
                        add_column_site(col.menu_index, stmt, catalog, sites, baseline);
                    }
                }
            }
        }
    }
}

fn walk_expr(
    expr: &Expression,
    stmt: &SelectStatement,
    catalog: &SqlCatalog,
    sites: &mut usize,
    baseline: &mut usize,
) {
    match expr {
        Expression::Column(col) => {
            add_column_site(col.menu_index, stmt, catalog, sites, baseline);
        }
        Expression::IntegerLiteral(_) | Expression::StringLiteral(_) | Expression::CountStar => {}
        Expression::Comparison { left, right, .. } | Expression::BinaryBool { left, right, .. } => {
            walk_expr(left, stmt, catalog, sites, baseline);
            walk_expr(right, stmt, catalog, sites, baseline);
        }
        Expression::Subquery(inner) => {
            // Uncorrelated: inner has its own scope / table sites.
            walk_select(inner, catalog, sites, baseline);
        }
        Expression::In { left, right, .. } => {
            use crate::sql_schema_linking::ast::InRhs;
            walk_expr(left, stmt, catalog, sites, baseline);
            match right {
                InRhs::Subquery(inner) => walk_select(inner, catalog, sites, baseline),
                InRhs::List(values) => {
                    for v in values {
                        walk_expr(v, stmt, catalog, sites, baseline);
                    }
                }
            }
        }
        Expression::Aggregate { arg, .. } => {
            let AggregateArg::Column(col) = arg;
            add_column_site(col.menu_index, stmt, catalog, sites, baseline);
        }
        Expression::Between { expr, low, high } => {
            walk_expr(expr, stmt, catalog, sites, baseline);
            walk_expr(low, stmt, catalog, sites, baseline);
            walk_expr(high, stmt, catalog, sites, baseline);
        }
    }
}

fn add_table_site(menu_index: u32, catalog: &SqlCatalog, sites: &mut usize, baseline: &mut usize) {
    *sites += 1;
    if let Some(table) = catalog.tables.get(menu_index as usize) {
        *baseline += table.name.len() + 2;
    }
}

fn add_column_site(
    menu_index: u32,
    stmt: &SelectStatement,
    catalog: &SqlCatalog,
    sites: &mut usize,
    baseline: &mut usize,
) {
    *sites += 1;
    if let Some(name) = resolve_column_name(menu_index, stmt, catalog) {
        *baseline += name.len() + 2;
    }
}

fn resolve_column_name(
    menu_index: u32,
    stmt: &SelectStatement,
    catalog: &SqlCatalog,
) -> Option<String> {
    let mut remaining = menu_index;
    for table_idx in stmt.scope_table_indices() {
        let table = catalog.tables.get(table_idx as usize)?;
        let n = table.columns.len() as u32;
        if remaining < n {
            return Some(table.columns[remaining as usize].name.clone());
        }
        remaining -= n;
    }
    None
}

/// Resolve `HEAD` relative to this crate's manifest directory.
///
/// Report writers and tests may run with a cwd outside the repo (or inside a
/// nested worktree checkout); pin the git invocation to `CARGO_MANIFEST_DIR`
/// so provenance always names the TCT source tree.
fn git_head_sha() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Models handoff artifact kind (T12).
pub const SQL_HANDOFF_ARTIFACT_KIND: &str = "tct_sql_schema_linking_frontend_handoff_v0";
/// Handoff version.
pub const SQL_HANDOFF_VERSION: u32 = 0;

/// Compiler → models handoff envelope for the SQL schema-linking frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SqlFrontendHandoff {
    pub artifact_kind: String,
    pub version: u32,
    pub claim_boundary: String,
    pub no_training_claims: bool,
    pub not_a_production_route: bool,
    pub repo_commit: Option<String>,
    pub tokenizer_module_id: String,
    pub layout_id: String,
    pub layout_pin_fixture: String,
    pub vocab_size: u32,
    pub pad_token: u32,
    pub menu_band: MenuBandHandoff,
    pub schema_pin: SchemaPinHandoff,
    pub encode_decode: EncodeDecodeHandoff,
    pub shared_alphabet: SharedAlphabetHandoff,
    pub gold_stats_summary: String,
    pub example_token_hashes: Vec<ExampleTokenHash>,
    pub consumer_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuBandHandoff {
    pub role: String,
    /// Menu band width (SharedAlphabet / layout pin: always 256).
    pub width: u32,
    /// Maximum radix digits per menu ordinal. Default **1** preserves old
    /// handoff readers until they learn multi-digit fields.
    #[serde(default = "default_menu_band_max_digits")]
    pub max_digits: u32,
    /// Ordinal capacity = `width.pow(max_digits)`. Default matches width when
    /// `max_digits` is absent (historical single-token menus).
    #[serde(default = "default_menu_band_capacity")]
    pub capacity: u64,
    pub notes: String,
}

fn default_menu_band_max_digits() -> u32 {
    1
}

fn default_menu_band_capacity() -> u64 {
    // Historical single-token ceiling (= band width).
    256
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaPinHandoff {
    pub db_id: String,
    pub fixture_dir: String,
    pub tables_json_sha256: String,
    pub gold_queries_json_sha256: String,
    pub pin_artifact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodeDecodeHandoff {
    pub generic_codec: bool,
    pub round_trip_claim: String,
    pub decode_to_executable_sql: String,
    pub subset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedAlphabetHandoff {
    pub per_grammar_default: bool,
    pub shared_mode_supported: bool,
    pub menus_under_shared: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExampleTokenHash {
    pub gold_id: String,
    pub token_count: usize,
    pub menu_token_count: usize,
    pub stream_sha256: String,
    pub rendered_sql: String,
}

/// Build the T12 models handoff artifact from in-repo fixtures.
pub fn fixture_sql_frontend_handoff() -> Result<SqlFrontendHandoff, String> {
    use crate::provenance::sha256_hex;
    use crate::sql_schema_linking::codec::SQL_CODEC_SUBSET_ID;
    use crate::sql_schema_linking::layout::{
        SQL_MENU_BAND_WIDTH, SQL_MENU_INDEX_PAYLOAD_ROLE, SQL_TOKEN_LAYOUT_PIN_FIXTURE_PATH,
    };

    let stats = fixture_gold_stats_report()?;
    let bundle = load_fixture_bundle().map_err(|e| e.to_string())?;
    let codec = SqlSchemaLinkingCodec::for_catalog(bundle.catalog.clone())
        .map_err(|e| format!("codec: {e}"))?;

    let mut example_token_hashes = Vec::new();
    for q in &stats.queries {
        if !q.round_trip_ok {
            continue;
        }
        let rt = codec
            .round_trip_source(&q.sql)
            .map_err(|e| format!("{}: {e}", q.id))?;
        let bytes: Vec<u8> = rt.tokens.iter().flat_map(|t| t.to_le_bytes()).collect();
        example_token_hashes.push(ExampleTokenHash {
            gold_id: q.id.clone(),
            token_count: rt.tokens.len(),
            menu_token_count: count_menu_tokens(&rt.tokens, codec.model())
                .or_else(|| count_menu_tokens_by_role_replay(&rt.tokens, codec.model()).ok())
                .ok_or_else(|| format!("{}: menu token count unavailable", q.id))?,
            stream_sha256: sha256_hex(&bytes),
            rendered_sql: rt.rendered,
        });
    }

    Ok(SqlFrontendHandoff {
        artifact_kind: SQL_HANDOFF_ARTIFACT_KIND.to_string(),
        version: SQL_HANDOFF_VERSION,
        claim_boundary: SQL_GOLD_STATS_CLAIM_BOUNDARY.to_string(),
        no_training_claims: true,
        not_a_production_route: true,
        repo_commit: git_head_sha(),
        tokenizer_module_id: "sql_schema_linking_sqlite_subset_v0".to_string(),
        layout_id: SQL_SYNTAX_TOKEN_LAYOUT_ID.to_string(),
        layout_pin_fixture: SQL_TOKEN_LAYOUT_PIN_FIXTURE_PATH.to_string(),
        vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        menu_band: MenuBandHandoff {
            role: SQL_MENU_INDEX_PAYLOAD_ROLE.to_string(),
            width: SQL_MENU_BAND_WIDTH,
            max_digits: crate::sql_schema_linking::menu_binding::SQL_MAX_MENU_DIGITS,
            capacity: crate::sql_schema_linking::menu_binding::sql_menu_ordinal_capacity(),
            notes: "Scope-relative table/column ordinals; VOCAB_SIZE catalog-independent (P4). Band width stays 256 (SharedAlphabet u8 alias, P9); multi-digit radix capacity is width^max_digits (SQL max_digits=2 → 65536).".to_string(),
        },
        schema_pin: SchemaPinHandoff {
            db_id: bundle.catalog.db_id.clone(),
            fixture_dir: "tests/fixtures/sql_schema_linking/".to_string(),
            tables_json_sha256: bundle.pin.tables_json_sha256.clone(),
            gold_queries_json_sha256: bundle.gold_queries_json_sha256.clone(),
            pin_artifact:
                crate::sql_schema_linking::catalog::SQL_SCHEMA_LINKING_FIXTURE_PIN_ARTIFACT
                    .to_string(),
        },
        encode_decode: EncodeDecodeHandoff {
            generic_codec: true,
            round_trip_claim:
                "tree identity + render fixpoint (P8); not byte-identity with arbitrary SQL"
                    .to_string(),
            decode_to_executable_sql: "decode tokens → AST → render_select → canonical SQL"
                .to_string(),
            subset_id: SQL_CODEC_SUBSET_ID.to_string(),
        },
        shared_alphabet: SharedAlphabetHandoff {
            per_grammar_default: true,
            shared_mode_supported: true,
            menus_under_shared:
                "u8_integer_payload band alias (width 256); multi-digit capacity base^max_digits; no SHARED_ALPHABET_* edits"
                    .to_string(),
        },
        gold_stats_summary: "reports/sql-schema-linking-gold-stats-v0.json".to_string(),
        example_token_hashes,
        consumer_notes: vec![
            "Not a production route; do not register in SYNTAX_CODEC_REGISTRY.".to_string(),
            "Column menus are scope-tracking (FROM/JOIN order), not global catalog-only."
                .to_string(),
            "Qualifier uses need no token; star and COUNT(*) are structural (P7).".to_string(),
            "Tokens per schema reference are scope-dependent, not always 1 (multi-digit menu ordinals when live cardinality exceeds band width)."
                .to_string(),
            "Train/eval ownership stays in the downstream training repository after consumer validator lands."
                .to_string(),
        ],
    })
}

/// Markdown summary of the handoff for humans.
pub fn handoff_markdown(h: &SqlFrontendHandoff) -> String {
    format!(
        r#"# SQL schema-linking frontend handoff (T12)

- artifact: `{artifact}`
- claim_boundary: `{claim}`
- no_training_claims: {no_train}
- not_a_production_route: {no_route}
- tokenizer_module_id: `{module}`
- layout_id: `{layout}` (pin: `{layout_pin}`)
- vocab_size / PAD: {vocab} / {pad}
- menu_band: role=`{menu_role}` width={menu_width} max_digits={menu_digits} capacity={menu_capacity}
- schema pin db_id: `{db}` tables_sha={tables} gold_sha={gold}
- gold_stats_summary: `{stats}`
- example streams: {n_examples} gold ids with stream_sha256
- decode → executable SQL: {decode}
- SharedAlphabet: supported (menus on u8 band + multi-digit capacity); PerGrammar remains pin default
- note: tokens per schema reference are scope-dependent (not always 1 digit)

This is compiler packaging for the downstream training repository. It does not
train, evaluate, or support a paper claim.
"#,
        artifact = h.artifact_kind,
        claim = h.claim_boundary,
        no_train = h.no_training_claims,
        no_route = h.not_a_production_route,
        menu_role = h.menu_band.role,
        menu_width = h.menu_band.width,
        menu_digits = h.menu_band.max_digits,
        menu_capacity = h.menu_band.capacity,
        module = h.tokenizer_module_id,
        layout = h.layout_id,
        layout_pin = h.layout_pin_fixture,
        vocab = h.vocab_size,
        pad = h.pad_token,
        db = h.schema_pin.db_id,
        tables = h.schema_pin.tables_json_sha256,
        gold = h.schema_pin.gold_queries_json_sha256,
        stats = h.gold_stats_summary,
        n_examples = h.example_token_hashes.len(),
        decode = h.encode_decode.decode_to_executable_sql,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t11_fixture_gold_stats_records_menu_vs_baseline_and_no_silent_parse_drop() {
        let report = fixture_gold_stats_report().expect("gold stats");
        assert_eq!(report.artifact_kind, SQL_GOLD_STATS_ARTIFACT_KIND);
        assert_eq!(report.claim_boundary, SQL_GOLD_STATS_CLAIM_BOUNDARY);
        assert!(report.no_training_claims);
        assert_eq!(report.parse_fail_count, 0, "fixture golds must all parse");
        assert_eq!(report.parse_ok_count, report.query_count);
        assert_eq!(report.round_trip_ok_count, report.query_count);
        assert!(
            report.total_baseline_schema_ref_tokens > report.total_menu_schema_ref_tokens,
            "baseline open-vocab must cost more tokens than menu indices: baseline={} menu={}",
            report.total_baseline_schema_ref_tokens,
            report.total_menu_schema_ref_tokens
        );
        assert!(report.total_schema_ref_sites > 0);
        // select_one_table: 1 table + 2 columns = 3 sites; names customers(9)+name(4)+city(4) + 3*2 = 23 baseline.
        let one = report
            .queries
            .iter()
            .find(|q| q.id == "select_one_table")
            .expect("select_one_table");
        assert_eq!(one.schema_ref_site_count, Some(3));
        assert_eq!(one.menu_schema_ref_tokens, Some(3));
        assert_eq!(one.baseline_schema_ref_tokens, Some(9 + 2 + 4 + 2 + 4 + 2));
        assert_eq!(one.menu_token_count, Some(3));

        let star = report
            .queries
            .iter()
            .find(|q| q.id == "count_star")
            .expect("count_star");
        assert_eq!(star.schema_ref_site_count, Some(1)); // table only
        assert_eq!(star.menu_token_count, Some(1));

        let md = gold_stats_markdown(&report);
        assert!(md.contains("menu tokens vs"));
        assert!(md.contains("select_one_table"));
    }

    #[test]
    fn t11_write_checked_in_summary_when_env_set() {
        // Opt-in writer for the T11 summary pair under reports/.
        // Usage: TCT_WRITE_SQL_GOLD_STATS=1 cargo test --lib \
        //   sql_schema_linking::report::tests::t11_write_checked_in_summary_when_env_set
        if std::env::var("TCT_WRITE_SQL_GOLD_STATS").ok().as_deref() != Some("1") {
            return;
        }
        let report = fixture_gold_stats_report().expect("gold stats");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let json_path = root.join("reports/sql-schema-linking-gold-stats-v0.json");
        let md_path = root.join("reports/sql-schema-linking-gold-stats-v0.md");
        let json = serde_json::to_string_pretty(&report).expect("json") + "\n";
        std::fs::write(&json_path, json).expect("write json");
        std::fs::write(&md_path, gold_stats_markdown(&report)).expect("write md");
        // Also keep bulky streams out of reports: only summary is checked in.
        let target = root.join("target/sql-schema-linking-gold-stats-v0.json");
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(&json_path, target);
    }

    #[test]
    fn t12_handoff_declares_no_training_claims_and_pins_layout_schema() {
        let handoff = fixture_sql_frontend_handoff().expect("handoff");
        assert_eq!(handoff.artifact_kind, SQL_HANDOFF_ARTIFACT_KIND);
        assert!(handoff.no_training_claims);
        assert!(handoff.not_a_production_route);
        assert_eq!(handoff.vocab_size, SQL_SYNTAX_LAYOUT_VOCAB_SIZE);
        assert_eq!(handoff.pad_token, SQL_SYNTAX_LAYOUT_PAD_TOKEN);
        assert!(!handoff.schema_pin.tables_json_sha256.is_empty());
        assert!(!handoff.schema_pin.gold_queries_json_sha256.is_empty());
        assert!(!handoff.layout_id.is_empty());
        assert!(!handoff.example_token_hashes.is_empty());
        assert!(
            handoff
                .example_token_hashes
                .iter()
                .all(|e| e.stream_sha256.len() == 64)
        );
        assert!(handoff.encode_decode.generic_codec);
        assert!(handoff.shared_alphabet.shared_mode_supported);
        assert_eq!(
            handoff.menu_band.width,
            crate::sql_schema_linking::layout::SQL_MENU_BAND_WIDTH
        );
        assert_eq!(
            handoff.menu_band.max_digits,
            crate::sql_schema_linking::menu_binding::SQL_MAX_MENU_DIGITS
        );
        assert_eq!(
            handoff.menu_band.capacity,
            crate::sql_schema_linking::menu_binding::sql_menu_ordinal_capacity()
        );
        assert!(
            handoff
                .consumer_notes
                .iter()
                .any(|n| n.contains("scope-dependent") && n.contains("not always 1")),
            "handoff must disclose scope-dependent tokens per schema ref"
        );
        let md = handoff_markdown(&handoff);
        assert!(md.contains("no_training_claims: true"));
    }

    /// Checked-in T11 gold-stats must match live compiler semantics.
    ///
    /// `repo_commit` is handled deliberately: the checked-in report pins the
    /// source commit used to *generate* it (normally the parent of the
    /// report-refresh commit). Requiring the artifact to name its containing
    /// commit is impossible / self-referential, so only semantic fields and
    /// provenance shape are compared for that field.
    #[test]
    fn t11_checked_in_gold_stats_match_current_compiler_semantics() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = root.join("reports/sql-schema-linking-gold-stats-v0.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read checked-in gold stats {}: {e}", path.display()));
        let checked: SqlGoldStatsReport =
            serde_json::from_str(&raw).expect("parse checked-in gold stats");
        let live = fixture_gold_stats_report().expect("live gold stats");

        let pinned = checked
            .repo_commit
            .clone()
            .expect("checked-in gold stats must pin generation repo_commit");
        assert!(
            pinned.len() == 40 && pinned.chars().all(|c| c.is_ascii_hexdigit()),
            "repo_commit must be a 40-char hex sha, got {pinned:?}"
        );
        // Live generation records current HEAD when git is available; it is
        // *not* required to equal the pinned generation commit.
        if let Some(live_commit) = live.repo_commit.as_ref() {
            assert!(
                live_commit.len() == 40 && live_commit.chars().all(|c| c.is_ascii_hexdigit()),
                "live repo_commit must be a 40-char hex sha, got {live_commit:?}"
            );
        }

        let mut checked_sem = checked;
        let mut live_sem = live;
        checked_sem.repo_commit = None;
        live_sem.repo_commit = None;
        assert_eq!(
            checked_sem, live_sem,
            "checked-in T11 gold-stats semantic fields must match current compiler output"
        );
        assert!(checked_sem.no_training_claims);
        assert_eq!(checked_sem.claim_boundary, SQL_GOLD_STATS_CLAIM_BOUNDARY);
    }

    /// Checked-in T12 handoff must match live compiler semantics, including
    /// every example stream hash. `repo_commit` is generation provenance only
    /// (see T11 parity test).
    #[test]
    fn t12_checked_in_handoff_matches_current_compiler_semantics() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = root.join("reports/sql-schema-linking-frontend-handoff-v0.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read checked-in handoff {}: {e}", path.display()));
        let checked: SqlFrontendHandoff =
            serde_json::from_str(&raw).expect("parse checked-in handoff");
        let live = fixture_sql_frontend_handoff().expect("live handoff");

        let pinned = checked
            .repo_commit
            .clone()
            .expect("checked-in handoff must pin generation repo_commit");
        assert!(
            pinned.len() == 40 && pinned.chars().all(|c| c.is_ascii_hexdigit()),
            "repo_commit must be a 40-char hex sha, got {pinned:?}"
        );

        let mut checked_sem = checked;
        let mut live_sem = live;
        checked_sem.repo_commit = None;
        live_sem.repo_commit = None;
        assert_eq!(
            checked_sem, live_sem,
            "checked-in T12 handoff semantic fields (incl. example stream hashes) must match current compiler output"
        );
        assert!(checked_sem.no_training_claims);
        assert!(checked_sem.not_a_production_route);
        assert!(
            !checked_sem.example_token_hashes.is_empty(),
            "handoff must retain example stream hashes"
        );
        assert!(
            checked_sem
                .example_token_hashes
                .iter()
                .any(|e| e.gold_id == "sum_aggregate"),
            "handoff must include sum_aggregate stream hash"
        );
    }

    #[test]
    fn t12_write_handoff_when_env_set() {
        // Usage: TCT_WRITE_SQL_HANDOFF=1 cargo test --lib \
        //   sql_schema_linking::report::tests::t12_write_handoff_when_env_set
        if std::env::var("TCT_WRITE_SQL_HANDOFF").ok().as_deref() != Some("1") {
            return;
        }
        let handoff = fixture_sql_frontend_handoff().expect("handoff");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let json_path = root.join("reports/sql-schema-linking-frontend-handoff-v0.json");
        let md_path = root.join("reports/sql-schema-linking-frontend-handoff-v0.md");
        let json = serde_json::to_string_pretty(&handoff).expect("json") + "\n";
        std::fs::write(&json_path, json).expect("write json");
        // handoff_markdown already ends with a single trailing newline.
        std::fs::write(&md_path, handoff_markdown(&handoff)).expect("write md");
    }
}
