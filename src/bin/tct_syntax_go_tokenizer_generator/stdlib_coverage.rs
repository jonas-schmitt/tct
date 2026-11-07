//! Stdlib-scale Go syntax tokenizer coverage report builder for the generator CLI.
//!
//! Owns the full-denominator / limited-smoke coverage report shape, record
//! construction, and validation used by `--stdlib-coverage-records`. The parent
//! binary keeps clap dispatch and non-coverage generator modes.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tct::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use tct::syntax_go_tokenizer_generator::{
    PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED,
    build_go_tokenizer_generator_artifacts_with_projection_report_mode,
    go_token_only_canonical_render_fidelity,
};
use tct::syntax_go_unified_artifact::{GO_UNIFIED_GRAMMAR_ID, go_unified_grammar_artifact};
use tct::syntax_token_layout_diff::LIVE_RAW_SYNTAX_STREAM_CONTRACT;

use super::{
    ProjectionRegenerationRequest, regenerate_projection_artifact, required_path, write_json,
};

pub(crate) const DEFAULT_STDLIB_COVERAGE_OUTPUT_PATH: &str =
    "target/tct-go-stdlib-syntax-tokenizer-coverage/coverage-report-v1.json";
pub(crate) const DEFAULT_STDLIB_COVERAGE_OUTPUT_DIR: &str =
    "target/tct-go-stdlib-syntax-tokenizer-coverage";
pub(crate) const STDLIB_COVERAGE_ARTIFACT_KIND: &str =
    "tct_go_stdlib_syntax_tokenizer_coverage_report_v1";
const STDLIB_COVERAGE_STATUS_PASSED: &str = "go_stdlib_syntax_tokenizer_coverage_passed";
const STDLIB_COVERAGE_STATUS_BLOCKED: &str = "go_stdlib_syntax_tokenizer_coverage_blocked";
const STDLIB_COVERAGE_STATUS_LIMITED: &str = "go_stdlib_syntax_tokenizer_coverage_limited_smoke";
const STDLIB_COVERAGE_FULL_SCOPE: &str = "full_denominator";
const STDLIB_COVERAGE_LIMITED_SCOPE: &str = "limited_smoke";
const STDLIB_COVERAGE_TARGET_PERCENT: f64 = 95.0;
const STDLIB_COVERAGE_DEFAULT_MAX_JOBS: usize = 8;
const STDLIB_COVERAGE_CLAIM_BOUNDARY: &str = "stdlib-scale Go syntax tokenizer coverage gate: parseable Go stdlib source files are gofmt-canonical-normalized when needed, then projected through the pinned inventory-derived grammar artifact, raw syntax state machine, canonical elided stream, and a token-only decode/render/gofmt round trip back to the canonical source bytes, plus go/parser sidecar validation. The accepted percentage is the token-only round-trip rate, not the Go-side source gofmt-idempotence rate (recorded separately as source_gofmt_reparse_ok). On-disk enumeration hashes remain the stdlib path provenance; projection/working hashes may be the gofmt form. The report is quantified corpus evidence only; it is not a totality claim, not Arena IR or GoIR evidence, not route-handoff provenance, not BPE validity authority, not type correctness, and not stable-backend promotion.";
const STDLIB_COVERAGE_FULL_NEXT_TASK: &str = "iterate on the first bucketed Go stdlib syntax tokenizer coverage gap until token-only render round-trip coverage reaches at least 95% or the residue is documented";
const STDLIB_COVERAGE_LIMITED_NEXT_TASK: &str = "run the Go stdlib syntax tokenizer coverage gate over the full enumerated denominator without --stdlib-coverage-limit";
const STDLIB_COVERAGE_NO_STABLE_CLAIMS: &[&str] = &[
    "no_arena_ir_claim",
    "no_go_ir_claim",
    "no_route_handoff_provenance_claim",
    "no_bpe_layer_validity_claim",
    "no_type_correctness_claim",
    "no_total_go_language_claim",
    "no_paper_facing_claim",
    "no_stable_backend_claim",
];

#[derive(Debug, Clone, Deserialize)]
struct GoStdlibCoverageFileRecord {
    path: String,
    sha256: String,
    bytes: usize,
    #[serde(default)]
    is_test: bool,
    #[serde(default)]
    is_build_constrained: bool,
    #[serde(default)]
    is_cgo: bool,
    #[serde(default)]
    is_generated: bool,
    #[serde(default)]
    parse_failed: bool,
    #[serde(default)]
    parse_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GoStdlibTokenizerCoverageReport {
    artifact_kind: String,
    version: u32,
    tool: String,
    status: String,
    target: String,
    route: String,
    coverage_scope: String,
    #[serde(default = "default_stdlib_coverage_parallel_jobs")]
    parallel_jobs: usize,
    goroot: String,
    pub(crate) file_records_path: String,
    file_records_sha256: String,
    metadata_path: String,
    candidate_grammar_id: String,
    grammar_artifact_id: String,
    grammar_artifact_content_sha256: String,
    stream_contract: String,
    coverage_target_percent: f64,
    target_met: bool,
    input_file_count: usize,
    pub(crate) input_denominator: usize,
    parse_failed_count: usize,
    pub(crate) attempted_record_count: usize,
    selected_denominator: usize,
    not_attempted_count: usize,
    supported_record_count: usize,
    blocked_record_count: usize,
    raw_state_machine_complete_count: usize,
    token_only_render_round_trip_equal_count: usize,
    accepted_percent: f64,
    token_only_render_round_trip_equal_percent: f64,
    selected_accepted_percent: f64,
    selected_token_only_render_round_trip_equal_percent: f64,
    total_raw_token_count: usize,
    total_canonical_token_count: usize,
    total_forced_token_count: usize,
    max_raw_token_id: Option<u32>,
    token_layout_vocab_size: u32,
    token_layout_pad_token: u32,
    gap_buckets: Vec<GoStdlibTokenizerCoverageGapBucket>,
    first_blocker: Option<GoStdlibTokenizerCoverageBlocker>,
    records: Vec<GoStdlibTokenizerCoverageRecord>,
    next_ready_task: String,
    validation_commands: Vec<String>,
    claim_boundary: String,
    no_stable_claims: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct GoStdlibTokenizerCoverageGapBucket {
    bucket: String,
    count: usize,
    sample_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct GoStdlibTokenizerCoverageBlocker {
    path: String,
    bucket: String,
    reason: String,
    next_ready_task: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoStdlibTokenizerCoverageRecord {
    path: String,
    source_path: String,
    source_sha256: String,
    bytes: usize,
    is_test: bool,
    is_build_constrained: bool,
    is_cgo: bool,
    is_generated: bool,
    attempted: bool,
    parse_failed: bool,
    status: String,
    bucket: String,
    raw_state_machine_complete: bool,
    /// `gofmt(render(decode(canonical elided tokens)))` equals the checked
    /// gofmt-canonical source bytes. This is the Go elided-stream
    /// losslessness gate; it is computed in Rust from the token stream and
    /// shares nothing with the Go-side source gofmt-idempotence evidence.
    token_only_render_round_trip_equal: bool,
    #[serde(default)]
    token_only_render_diagnostic: Option<String>,
    /// Go-side sidecar evidence only: the *source* is gofmt-idempotent and
    /// reparses. Never a token round trip. Recorded separately so a passing
    /// value can no longer be read as losslessness.
    #[serde(default)]
    source_gofmt_reparse_ok: bool,
    canonical_render_ready: bool,
    gofmt_validation: String,
    go_parser_reparse_validation: String,
    raw_token_count: usize,
    canonical_token_count: usize,
    forced_token_count: usize,
    max_raw_token_id: Option<u32>,
    token_layout_vocab_size: u32,
    token_layout_pad_token: u32,
    canonical_tokens_sha256: String,
    raw_token_ids_sha256: String,
    projection_report_path: String,
    projection_artifact_path: String,
    generated_codec_report_path: String,
    error: Option<String>,
}

pub(crate) fn build_stdlib_coverage_report(
    matches: &clap::ArgMatches,
    go_bin: &str,
    records_path: &Path,
) -> Result<GoStdlibTokenizerCoverageReport, Box<dyn Error>> {
    let goroot = matches
        .get_one::<PathBuf>("stdlib-coverage-goroot")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--stdlib-coverage-records requires --stdlib-coverage-goroot",
            )
        })?;
    let output_dir = required_path(matches, "stdlib-coverage-output-dir")?;
    let limit = matches.get_one::<usize>("stdlib-coverage-limit").copied();
    let parallel_jobs = stdlib_coverage_parallel_jobs(matches)?;

    let records_bytes = fs::read(records_path)?;
    let file_records_sha256 = tct::provenance::sha256_hex(&records_bytes);
    let file_records = parse_stdlib_file_records(&records_bytes)?;
    let input_file_count = file_records.len();
    let parse_failed_count = file_records
        .iter()
        .filter(|record| record.parse_failed)
        .count();
    let input_denominator = input_file_count - parse_failed_count;
    let selected_denominator = limit
        .map(|limit| limit.min(input_denominator))
        .unwrap_or(input_denominator);

    let mut attempted_parseable = 0usize;
    let mut work_records = Vec::new();
    for file_record in file_records {
        if file_record.parse_failed {
            work_records.push(GoStdlibCoverageWorkRecord::ParseFailure(file_record));
            continue;
        }
        if attempted_parseable >= selected_denominator {
            continue;
        }
        attempted_parseable += 1;
        work_records.push(GoStdlibCoverageWorkRecord::Attempt(file_record));
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(parallel_jobs)
        .build()?;
    let coverage_records = pool.install(|| {
        work_records
            .par_iter()
            .map(|work_record| match work_record {
                GoStdlibCoverageWorkRecord::ParseFailure(file_record) => {
                    stdlib_parse_failure_record(goroot, file_record)
                }
                GoStdlibCoverageWorkRecord::Attempt(file_record) => {
                    attempt_stdlib_coverage_record(go_bin, goroot, output_dir, file_record)
                }
            })
            .collect::<Vec<_>>()
    });

    let supported_record_count = coverage_records
        .iter()
        .filter(|record| record.status == "passed")
        .count();
    let blocked_record_count = coverage_records
        .iter()
        .filter(|record| record.status == "blocked")
        .count();
    let raw_state_machine_complete_count = coverage_records
        .iter()
        .filter(|record| record.raw_state_machine_complete)
        .count();
    let token_only_render_round_trip_equal_count = coverage_records
        .iter()
        .filter(|record| record.token_only_render_round_trip_equal)
        .count();
    let not_attempted_count = input_denominator - attempted_parseable;
    let total_raw_token_count = coverage_records
        .iter()
        .map(|record| record.raw_token_count)
        .sum();
    let total_canonical_token_count = coverage_records
        .iter()
        .map(|record| record.canonical_token_count)
        .sum();
    let total_forced_token_count = coverage_records
        .iter()
        .map(|record| record.forced_token_count)
        .sum();
    let max_raw_token_id = coverage_records
        .iter()
        .filter_map(|record| record.max_raw_token_id)
        .max();
    let token_layout_vocab_size = coverage_records
        .iter()
        .find(|record| record.token_layout_vocab_size > 0)
        .map(|record| record.token_layout_vocab_size)
        .unwrap_or(0);
    let token_layout_pad_token = coverage_records
        .iter()
        .find(|record| record.token_layout_pad_token > 0)
        .map(|record| record.token_layout_pad_token)
        .unwrap_or(0);
    let accepted_percent = coverage_percent(supported_record_count, input_denominator);
    let token_only_render_round_trip_equal_percent =
        coverage_percent(token_only_render_round_trip_equal_count, input_denominator);
    let selected_accepted_percent = coverage_percent(supported_record_count, selected_denominator);
    let selected_token_only_render_round_trip_equal_percent = coverage_percent(
        token_only_render_round_trip_equal_count,
        selected_denominator,
    );
    let full_denominator = limit.is_none();
    let target_met = full_denominator
        && token_only_render_round_trip_equal_percent >= STDLIB_COVERAGE_TARGET_PERCENT;
    let status = if !full_denominator {
        STDLIB_COVERAGE_STATUS_LIMITED
    } else if target_met {
        STDLIB_COVERAGE_STATUS_PASSED
    } else {
        STDLIB_COVERAGE_STATUS_BLOCKED
    };
    let coverage_scope = if full_denominator {
        STDLIB_COVERAGE_FULL_SCOPE
    } else {
        STDLIB_COVERAGE_LIMITED_SCOPE
    };
    let gap_buckets = stdlib_gap_buckets(&coverage_records, not_attempted_count);
    let first_blocker = first_stdlib_blocker(&coverage_records, not_attempted_count);
    let unified_artifact = go_unified_grammar_artifact()?;
    let report = GoStdlibTokenizerCoverageReport {
        artifact_kind: STDLIB_COVERAGE_ARTIFACT_KIND.to_string(),
        version: 1,
        tool: "tct-syntax-go-tokenizer-generator".to_string(),
        status: status.to_string(),
        target: "go".to_string(),
        route: "go_parser_metadata_raw_syntax_state_machine".to_string(),
        coverage_scope: coverage_scope.to_string(),
        parallel_jobs,
        goroot: goroot.display().to_string(),
        file_records_path: records_path.display().to_string(),
        file_records_sha256,
        metadata_path: GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH.to_string(),
        candidate_grammar_id: GO_UNIFIED_GRAMMAR_ID.to_string(),
        grammar_artifact_id: unified_artifact.artifact_id.to_string(),
        grammar_artifact_content_sha256: unified_artifact.content_sha256.clone(),
        stream_contract: LIVE_RAW_SYNTAX_STREAM_CONTRACT.to_string(),
        coverage_target_percent: STDLIB_COVERAGE_TARGET_PERCENT,
        target_met,
        input_file_count,
        input_denominator,
        parse_failed_count,
        attempted_record_count: attempted_parseable,
        selected_denominator,
        not_attempted_count,
        supported_record_count,
        blocked_record_count,
        raw_state_machine_complete_count,
        token_only_render_round_trip_equal_count,
        accepted_percent,
        token_only_render_round_trip_equal_percent,
        selected_accepted_percent,
        selected_token_only_render_round_trip_equal_percent,
        total_raw_token_count,
        total_canonical_token_count,
        total_forced_token_count,
        max_raw_token_id,
        token_layout_vocab_size,
        token_layout_pad_token,
        gap_buckets,
        first_blocker,
        records: coverage_records,
        next_ready_task: if full_denominator {
            STDLIB_COVERAGE_FULL_NEXT_TASK.to_string()
        } else {
            STDLIB_COVERAGE_LIMITED_NEXT_TASK.to_string()
        },
        validation_commands: stdlib_coverage_validation_commands(
            records_path,
            goroot,
            limit,
            parallel_jobs,
        ),
        claim_boundary: STDLIB_COVERAGE_CLAIM_BOUNDARY.to_string(),
        no_stable_claims: STDLIB_COVERAGE_NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    };
    validate_stdlib_coverage_report(&report)?;
    Ok(report)
}

#[derive(Debug, Clone)]
enum GoStdlibCoverageWorkRecord {
    ParseFailure(GoStdlibCoverageFileRecord),
    Attempt(GoStdlibCoverageFileRecord),
}

fn stdlib_coverage_parallel_jobs(matches: &clap::ArgMatches) -> Result<usize, Box<dyn Error>> {
    if let Some(jobs) = matches.get_one::<usize>("stdlib-coverage-jobs").copied() {
        if jobs == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--stdlib-coverage-jobs must be greater than zero",
            )
            .into());
        }
        return Ok(jobs);
    }
    Ok(default_stdlib_coverage_parallel_jobs())
}

fn default_stdlib_coverage_parallel_jobs() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .clamp(1, STDLIB_COVERAGE_DEFAULT_MAX_JOBS)
}

fn parse_stdlib_file_records(
    records_bytes: &[u8],
) -> Result<Vec<GoStdlibCoverageFileRecord>, Box<dyn Error>> {
    let mut records = Vec::new();
    for (index, line) in String::from_utf8_lossy(records_bytes).lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: GoStdlibCoverageFileRecord = serde_json::from_str(line).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("decode stdlib per-file JSONL line {}: {error}", index + 1),
            )
        })?;
        if record.path.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("stdlib per-file JSONL line {} has empty path", index + 1),
            )
            .into());
        }
        records.push(record);
    }
    Ok(records)
}

fn attempt_stdlib_coverage_record(
    go_bin: &str,
    goroot: &Path,
    output_dir: &Path,
    file_record: &GoStdlibCoverageFileRecord,
) -> GoStdlibTokenizerCoverageRecord {
    match build_stdlib_coverage_record(go_bin, goroot, output_dir, file_record) {
        Ok(record) => record,
        Err(error) => stdlib_blocked_record(goroot, output_dir, file_record, &error.to_string()),
    }
}

fn build_stdlib_coverage_record(
    go_bin: &str,
    goroot: &Path,
    output_dir: &Path,
    file_record: &GoStdlibCoverageFileRecord,
) -> Result<GoStdlibTokenizerCoverageRecord, Box<dyn Error>> {
    let source_path = goroot.join(filepath_from_slash(&file_record.path));
    let stem = stdlib_safe_stem(&file_record.path);
    let projection_output = output_dir
        .join("projections")
        .join(format!("{stem}-source-instance-projection-v1.json"));
    let projection_report_output = output_dir
        .join("projection-reports")
        .join(format!("source-instance-projection-report-{stem}-v1.json"));
    let manifest_path = file_record
        .path
        .strip_prefix("src/")
        .unwrap_or(&file_record.path);

    // Enumeration provenance is the on-disk stdlib bytes. The projector may
    // gofmt-normalize before projection (gofmt-canonical token space), so the
    // working projection hash can differ from the enumerated hash without
    // meaning the wrong file was selected.
    let original_bytes = fs::read(&source_path)?;
    let original_sha256 = tct::provenance::sha256_hex(&original_bytes);
    if original_sha256 != file_record.sha256 {
        let mut record = stdlib_blocked_record(
            goroot,
            output_dir,
            file_record,
            &format!(
                "on-disk source sha256 {original_sha256} does not match enumerated sha256 {}",
                file_record.sha256
            ),
        );
        record.bucket = "source_hash_mismatch".to_string();
        return Ok(record);
    }

    regenerate_projection_artifact(ProjectionRegenerationRequest {
        go_bin,
        source_path: &source_path,
        metadata_path: Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        metadata_smoke_fixture: None,
        manifest_path: Some(manifest_path),
        grammar_id: GO_UNIFIED_GRAMMAR_ID,
        projection_output: &projection_output,
        projection_report_output: Some(&projection_report_output),
        route_registered: false,
    })?;
    let projection_artifact_contents = fs::read_to_string(&projection_output)?;
    let artifacts = build_go_tokenizer_generator_artifacts_with_projection_report_mode(
        &projection_report_output,
        &projection_output,
        &projection_artifact_contents,
        PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED,
    )?;
    write_json(
        &artifacts.generated_codec_report.report_path,
        &artifacts.generated_codec_report,
    )?;

    // The Go-side `canonical_render.matches_source` evidence only certifies
    // that the source is gofmt-idempotent. The losslessness the elided stream
    // contract promises is a token round trip, so run it here and let it gate
    // the file.
    let fidelity = go_token_only_canonical_render_fidelity(
        &artifacts.generated_codec_report.tct_tokens,
        &artifacts.generated_codec_report.source.sha256,
        artifacts.generated_codec_report.source.bytes,
    );

    let mut record = GoStdlibTokenizerCoverageRecord {
        path: file_record.path.clone(),
        source_path: source_path.display().to_string(),
        source_sha256: file_record.sha256.clone(),
        bytes: file_record.bytes,
        is_test: file_record.is_test,
        is_build_constrained: file_record.is_build_constrained,
        is_cgo: file_record.is_cgo,
        is_generated: file_record.is_generated,
        attempted: true,
        parse_failed: false,
        status: "passed".to_string(),
        bucket: "accepted_token_only_render_round_trip_equal".to_string(),
        raw_state_machine_complete: artifacts.report.raw_state_machine_complete,
        token_only_render_round_trip_equal: fidelity.matches_canonical_source,
        token_only_render_diagnostic: fidelity.diagnostic.clone(),
        source_gofmt_reparse_ok: artifacts.report.canonical_render_ready
            && artifacts.report.gofmt_validation == "passed"
            && artifacts.report.go_parser_reparse_validation == "passed",
        canonical_render_ready: artifacts.report.canonical_render_ready,
        gofmt_validation: artifacts.report.gofmt_validation.clone(),
        go_parser_reparse_validation: artifacts.report.go_parser_reparse_validation.clone(),
        raw_token_count: artifacts.report.raw_token_count,
        canonical_token_count: artifacts.report.canonical_token_count,
        forced_token_count: artifacts.report.forced_token_count,
        max_raw_token_id: artifacts.report.max_raw_token_id,
        token_layout_vocab_size: artifacts.report.token_layout_vocab_size,
        token_layout_pad_token: artifacts.report.token_layout_pad_token,
        canonical_tokens_sha256: artifacts.report.canonical_tokens_sha256.clone(),
        raw_token_ids_sha256: artifacts.report.raw_token_ids_sha256.clone(),
        projection_report_path: projection_report_output.display().to_string(),
        projection_artifact_path: projection_output.display().to_string(),
        generated_codec_report_path: artifacts.generated_codec_report.report_path.clone(),
        error: None,
    };
    if !record.raw_state_machine_complete {
        record.status = "blocked".to_string();
        record.bucket = "raw_state_machine_incomplete".to_string();
        record.error = Some("raw state machine did not complete".to_string());
    } else if !record.source_gofmt_reparse_ok {
        record.status = "blocked".to_string();
        record.bucket = "source_gofmt_reparse_failed".to_string();
        record.error = Some(
            "Go-side canonical render, gofmt, or go/parser sidecar validation did not pass"
                .to_string(),
        );
    } else if !record.token_only_render_round_trip_equal {
        record.status = "blocked".to_string();
        record.bucket = "token_only_render_round_trip_not_equal".to_string();
        record.error = Some(
            record
                .token_only_render_diagnostic
                .clone()
                .unwrap_or_else(|| {
                    "token-only decode/render/gofmt did not reproduce the canonical source bytes"
                        .to_string()
                }),
        );
    }
    Ok(record)
}

fn stdlib_parse_failure_record(
    goroot: &Path,
    file_record: &GoStdlibCoverageFileRecord,
) -> GoStdlibTokenizerCoverageRecord {
    GoStdlibTokenizerCoverageRecord {
        path: file_record.path.clone(),
        source_path: goroot
            .join(filepath_from_slash(&file_record.path))
            .display()
            .to_string(),
        source_sha256: file_record.sha256.clone(),
        bytes: file_record.bytes,
        is_test: file_record.is_test,
        is_build_constrained: file_record.is_build_constrained,
        is_cgo: file_record.is_cgo,
        is_generated: file_record.is_generated,
        attempted: false,
        parse_failed: true,
        status: "parse_failed_excluded".to_string(),
        bucket: "parse_failed_excluded_from_denominator".to_string(),
        raw_state_machine_complete: false,
        token_only_render_round_trip_equal: false,
        token_only_render_diagnostic: None,
        source_gofmt_reparse_ok: false,
        canonical_render_ready: false,
        gofmt_validation: String::new(),
        go_parser_reparse_validation: String::new(),
        raw_token_count: 0,
        canonical_token_count: 0,
        forced_token_count: 0,
        max_raw_token_id: None,
        token_layout_vocab_size: 0,
        token_layout_pad_token: 0,
        canonical_tokens_sha256: String::new(),
        raw_token_ids_sha256: String::new(),
        projection_report_path: String::new(),
        projection_artifact_path: String::new(),
        generated_codec_report_path: String::new(),
        error: Some(file_record.parse_error.clone()),
    }
}

fn stdlib_blocked_record(
    goroot: &Path,
    output_dir: &Path,
    file_record: &GoStdlibCoverageFileRecord,
    error: &str,
) -> GoStdlibTokenizerCoverageRecord {
    let stem = stdlib_safe_stem(&file_record.path);
    let projection_output = output_dir
        .join("projections")
        .join(format!("{stem}-source-instance-projection-v1.json"));
    let projection_report_output = output_dir
        .join("projection-reports")
        .join(format!("source-instance-projection-report-{stem}-v1.json"));
    GoStdlibTokenizerCoverageRecord {
        path: file_record.path.clone(),
        source_path: goroot
            .join(filepath_from_slash(&file_record.path))
            .display()
            .to_string(),
        source_sha256: file_record.sha256.clone(),
        bytes: file_record.bytes,
        is_test: file_record.is_test,
        is_build_constrained: file_record.is_build_constrained,
        is_cgo: file_record.is_cgo,
        is_generated: file_record.is_generated,
        attempted: true,
        parse_failed: false,
        status: "blocked".to_string(),
        bucket: stdlib_error_bucket(error),
        raw_state_machine_complete: false,
        token_only_render_round_trip_equal: false,
        token_only_render_diagnostic: None,
        source_gofmt_reparse_ok: false,
        canonical_render_ready: false,
        gofmt_validation: String::new(),
        go_parser_reparse_validation: String::new(),
        raw_token_count: 0,
        canonical_token_count: 0,
        forced_token_count: 0,
        max_raw_token_id: None,
        token_layout_vocab_size: 0,
        token_layout_pad_token: 0,
        canonical_tokens_sha256: String::new(),
        raw_token_ids_sha256: String::new(),
        projection_report_path: projection_report_output.display().to_string(),
        projection_artifact_path: projection_output.display().to_string(),
        generated_codec_report_path: String::new(),
        error: Some(truncate_error(error)),
    }
}

fn stdlib_error_bucket(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("metadata missing rule for ast node") {
        "projection_missing_ast_node_rule".to_string()
    } else if lower.contains("missing metadata field") {
        "projection_missing_metadata_field".to_string()
    } else if lower.contains("source sha-256 drift") {
        "source_hash_mismatch".to_string()
    } else if lower.contains("raw syntax state machine rejected") {
        "raw_state_machine_rejected_stream".to_string()
    } else if lower.contains("token-only") {
        "token_only_render_round_trip_not_equal".to_string()
    } else if lower.contains("canonical render")
        || lower.contains("go/parser")
        || lower.contains("gofmt")
    {
        "source_gofmt_reparse_failed".to_string()
    } else {
        "generated_codec_command_failed".to_string()
    }
}

fn truncate_error(error: &str) -> String {
    const LIMIT: usize = 900;
    if error.len() <= LIMIT {
        error.to_string()
    } else {
        format!("{}... [truncated]", &error[..LIMIT])
    }
}

fn stdlib_gap_buckets(
    records: &[GoStdlibTokenizerCoverageRecord],
    not_attempted_count: usize,
) -> Vec<GoStdlibTokenizerCoverageGapBucket> {
    let mut buckets = BTreeMap::<String, (usize, Vec<String>)>::new();
    for record in records {
        if record.status == "passed" {
            continue;
        }
        let entry = buckets.entry(record.bucket.clone()).or_default();
        entry.0 += 1;
        if entry.1.len() < 5 {
            entry.1.push(record.path.clone());
        }
    }
    if not_attempted_count > 0 {
        buckets.insert(
            "not_attempted".to_string(),
            (not_attempted_count, Vec::new()),
        );
    }
    buckets
        .into_iter()
        .map(
            |(bucket, (count, sample_paths))| GoStdlibTokenizerCoverageGapBucket {
                bucket,
                count,
                sample_paths,
            },
        )
        .collect()
}

fn first_stdlib_blocker(
    records: &[GoStdlibTokenizerCoverageRecord],
    not_attempted_count: usize,
) -> Option<GoStdlibTokenizerCoverageBlocker> {
    if let Some(record) = records.iter().find(|record| record.status == "blocked") {
        return Some(GoStdlibTokenizerCoverageBlocker {
            path: record.path.clone(),
            bucket: record.bucket.clone(),
            reason: record
                .error
                .clone()
                .unwrap_or_else(|| "stdlib tokenizer coverage record is blocked".to_string()),
            next_ready_task: STDLIB_COVERAGE_FULL_NEXT_TASK.to_string(),
        });
    }
    if not_attempted_count > 0 {
        return Some(GoStdlibTokenizerCoverageBlocker {
            path: String::new(),
            bucket: "not_attempted".to_string(),
            reason: format!("{not_attempted_count} parseable stdlib files were not attempted"),
            next_ready_task: STDLIB_COVERAGE_LIMITED_NEXT_TASK.to_string(),
        });
    }
    None
}

fn stdlib_coverage_validation_commands(
    records_path: &Path,
    goroot: &Path,
    limit: Option<usize>,
    parallel_jobs: usize,
) -> Vec<String> {
    let mut command = format!(
        "cargo run --bin tct-syntax-go-tokenizer-generator -- --stdlib-coverage-records {} --stdlib-coverage-goroot {} --output {} --stdlib-coverage-jobs {}",
        records_path.display(),
        goroot.display(),
        DEFAULT_STDLIB_COVERAGE_OUTPUT_PATH,
        parallel_jobs
    );
    if let Some(limit) = limit {
        command.push_str(&format!(" --stdlib-coverage-limit {limit}"));
    }
    vec![
        command,
        format!(
            "cargo run --bin tct-syntax-go-tokenizer-generator -- --validate-report {}",
            DEFAULT_STDLIB_COVERAGE_OUTPUT_PATH
        ),
        "go test scripts/analysis/go_stdlib_syntax_coverage_scan.go scripts/analysis/go_stdlib_syntax_coverage_scan_test.go".to_string(),
        "git diff --check".to_string(),
    ]
}

fn coverage_percent(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        (numerator as f64 * 100.0) / denominator as f64
    }
}

fn filepath_from_slash(path: &str) -> PathBuf {
    path.split('/').collect()
}

fn stdlib_safe_stem(path: &str) -> String {
    // Preserve underscores so path pairs like `runtime/cgo/mmap.go` and
    // `runtime/cgo_mmap.go` do not collapse to the same artifact stem (which
    // would overwrite projection/codec reports and break M4 extraction).
    let without_src = path.strip_prefix("src/").unwrap_or(path);
    let mut stem = String::with_capacity(without_src.len());
    let mut previous_dash = false;
    for ch in without_src.chars() {
        if ch.is_ascii_alphanumeric() {
            stem.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if ch == '_' {
            stem.push('_');
            previous_dash = false;
        } else if !previous_dash {
            stem.push('-');
            previous_dash = true;
        }
    }
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        "stdlib-record".to_string()
    } else {
        stem.to_string()
    }
}

pub(crate) fn parse_stdlib_coverage_report(
    contents: &str,
) -> Result<GoStdlibTokenizerCoverageReport, Box<dyn Error>> {
    let report: GoStdlibTokenizerCoverageReport = serde_json::from_str(contents)?;
    validate_stdlib_coverage_report(&report)?;
    Ok(report)
}

fn validate_stdlib_coverage_report(
    report: &GoStdlibTokenizerCoverageReport,
) -> Result<(), Box<dyn Error>> {
    require_stdlib(
        report.artifact_kind == STDLIB_COVERAGE_ARTIFACT_KIND,
        "artifact_kind",
        "unexpected stdlib coverage artifact kind",
    )?;
    require_stdlib(report.version == 1, "version", "unexpected report version")?;
    require_stdlib(
        report.tool == "tct-syntax-go-tokenizer-generator",
        "tool",
        "unexpected stdlib coverage tool",
    )?;
    require_stdlib(report.target == "go", "target", "unexpected target")?;
    require_stdlib(
        report.route == "go_parser_metadata_raw_syntax_state_machine",
        "route",
        "unexpected route",
    )?;
    require_stdlib(
        report.parallel_jobs > 0,
        "parallel_jobs",
        "parallel job count must be greater than zero",
    )?;
    require_stdlib(
        report.metadata_path == GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
        "metadata_path",
        "stdlib coverage must use the checked Go AST inventory as metadata",
    )?;
    require_stdlib(
        report.candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID,
        "candidate_grammar_id",
        "stdlib coverage must use the unified Go grammar artifact",
    )?;
    let unified_artifact = go_unified_grammar_artifact()?;
    require_stdlib(
        report.grammar_artifact_id == unified_artifact.artifact_id,
        "grammar_artifact_id",
        "grammar artifact ID does not match the live unified artifact",
    )?;
    require_stdlib(
        report.grammar_artifact_content_sha256 == unified_artifact.content_sha256,
        "grammar_artifact_content_sha256",
        "grammar artifact hash does not match the live unified artifact",
    )?;
    require_stdlib(
        report.stream_contract == LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        "stream_contract",
        "stdlib coverage stream contract must match the live raw syntax stream contract",
    )?;
    require_stdlib(
        (report.coverage_target_percent - STDLIB_COVERAGE_TARGET_PERCENT).abs() < f64::EPSILON,
        "coverage_target_percent",
        "unexpected stdlib coverage target",
    )?;

    let parse_failed_count = report
        .records
        .iter()
        .filter(|record| record.parse_failed)
        .count();
    let attempted_record_count = report
        .records
        .iter()
        .filter(|record| record.attempted && !record.parse_failed)
        .count();
    let supported_record_count = report
        .records
        .iter()
        .filter(|record| record.status == "passed")
        .count();
    let blocked_record_count = report
        .records
        .iter()
        .filter(|record| record.status == "blocked")
        .count();
    let raw_state_machine_complete_count = report
        .records
        .iter()
        .filter(|record| record.raw_state_machine_complete)
        .count();
    let token_only_render_round_trip_equal_count = report
        .records
        .iter()
        .filter(|record| record.token_only_render_round_trip_equal)
        .count();
    require_stdlib(
        report.parse_failed_count == parse_failed_count,
        "parse_failed_count",
        "parse failure count does not match records",
    )?;
    require_stdlib(
        report.input_denominator == report.input_file_count - report.parse_failed_count,
        "input_denominator",
        "denominator must exclude parse failures",
    )?;
    require_stdlib(
        report.attempted_record_count == attempted_record_count,
        "attempted_record_count",
        "attempted count does not match records",
    )?;
    require_stdlib(
        report.selected_denominator == report.attempted_record_count,
        "selected_denominator",
        "selected denominator must match attempted parseable records",
    )?;
    require_stdlib(
        report.not_attempted_count == report.input_denominator - report.attempted_record_count,
        "not_attempted_count",
        "not-attempted count must be denominator minus attempted records",
    )?;
    require_stdlib(
        report.supported_record_count == supported_record_count,
        "supported_record_count",
        "supported count does not match records",
    )?;
    require_stdlib(
        report.blocked_record_count == blocked_record_count,
        "blocked_record_count",
        "blocked count does not match records",
    )?;
    require_stdlib(
        report.raw_state_machine_complete_count == raw_state_machine_complete_count,
        "raw_state_machine_complete_count",
        "raw-state complete count does not match records",
    )?;
    require_stdlib(
        report.token_only_render_round_trip_equal_count == token_only_render_round_trip_equal_count,
        "token_only_render_round_trip_equal_count",
        "render/reparse count does not match records",
    )?;
    require_stdlib_f64(
        report.accepted_percent,
        coverage_percent(report.supported_record_count, report.input_denominator),
        "accepted_percent",
    )?;
    require_stdlib_f64(
        report.token_only_render_round_trip_equal_percent,
        coverage_percent(
            report.token_only_render_round_trip_equal_count,
            report.input_denominator,
        ),
        "token_only_render_round_trip_equal_percent",
    )?;
    require_stdlib_f64(
        report.selected_accepted_percent,
        coverage_percent(report.supported_record_count, report.selected_denominator),
        "selected_accepted_percent",
    )?;
    require_stdlib_f64(
        report.selected_token_only_render_round_trip_equal_percent,
        coverage_percent(
            report.token_only_render_round_trip_equal_count,
            report.selected_denominator,
        ),
        "selected_token_only_render_round_trip_equal_percent",
    )?;
    // Passed records must not share projection/codec artifact stems. Collisions
    // overwrite on-disk reports and poison M4 corpus extraction integrity.
    let mut codec_paths = BTreeMap::<&str, &str>::new();
    for record in &report.records {
        if record.status != "passed" || record.generated_codec_report_path.is_empty() {
            continue;
        }
        if let Some(previous) =
            codec_paths.insert(&record.generated_codec_report_path, &record.path)
        {
            return Err(format!(
                "stdlib coverage generated_codec_report_path collision: {} shared by {} and {}",
                record.generated_codec_report_path, previous, record.path
            )
            .into());
        }
    }
    let expected_target_met = report.coverage_scope == STDLIB_COVERAGE_FULL_SCOPE
        && report.token_only_render_round_trip_equal_percent >= STDLIB_COVERAGE_TARGET_PERCENT;
    require_stdlib(
        report.target_met == expected_target_met,
        "target_met",
        "target_met does not match coverage scope and percentage",
    )?;
    let expected_status = if report.coverage_scope == STDLIB_COVERAGE_LIMITED_SCOPE {
        STDLIB_COVERAGE_STATUS_LIMITED
    } else if report.target_met {
        STDLIB_COVERAGE_STATUS_PASSED
    } else {
        STDLIB_COVERAGE_STATUS_BLOCKED
    };
    require_stdlib(
        report.status == expected_status,
        "status",
        "status does not match coverage outcome",
    )?;
    require_stdlib(
        report.gap_buckets == stdlib_gap_buckets(&report.records, report.not_attempted_count),
        "gap_buckets",
        "gap buckets do not match records",
    )?;
    require_stdlib(
        report.first_blocker == first_stdlib_blocker(&report.records, report.not_attempted_count),
        "first_blocker",
        "first blocker does not match records",
    )?;
    let total_raw_token_count = report
        .records
        .iter()
        .map(|record| record.raw_token_count)
        .sum::<usize>();
    let total_canonical_token_count = report
        .records
        .iter()
        .map(|record| record.canonical_token_count)
        .sum::<usize>();
    let total_forced_token_count = report
        .records
        .iter()
        .map(|record| record.forced_token_count)
        .sum::<usize>();
    require_stdlib(
        report.total_raw_token_count == total_raw_token_count,
        "total_raw_token_count",
        "raw token total does not match records",
    )?;
    require_stdlib(
        report.total_canonical_token_count == total_canonical_token_count,
        "total_canonical_token_count",
        "canonical token total does not match records",
    )?;
    require_stdlib(
        report.total_forced_token_count == total_forced_token_count,
        "total_forced_token_count",
        "forced token total does not match records",
    )?;
    let max_raw_token_id = report
        .records
        .iter()
        .filter_map(|record| record.max_raw_token_id)
        .max();
    require_stdlib(
        report.max_raw_token_id == max_raw_token_id,
        "max_raw_token_id",
        "max raw token ID does not match records",
    )?;
    for record in &report.records {
        validate_stdlib_coverage_record(record)?;
    }
    require_stdlib(
        report.claim_boundary.contains("not Arena IR")
            && report.claim_boundary.contains("not BPE")
            && report.claim_boundary.contains("not stable-backend"),
        "claim_boundary",
        "claim boundary must keep non-claims explicit",
    )?;
    for required in STDLIB_COVERAGE_NO_STABLE_CLAIMS {
        require_stdlib(
            report
                .no_stable_claims
                .iter()
                .any(|claim| claim == required),
            "no_stable_claims",
            "missing required no-stable-claim marker",
        )?;
    }
    Ok(())
}

fn validate_stdlib_coverage_record(
    record: &GoStdlibTokenizerCoverageRecord,
) -> Result<(), Box<dyn Error>> {
    require_stdlib(!record.path.is_empty(), "record.path", "path is empty")?;
    require_stdlib(
        record.source_sha256.len() == 64,
        "record.source_sha256",
        "source sha256 must be hex-encoded",
    )?;
    match record.status.as_str() {
        "passed" => {
            require_stdlib(
                record.attempted,
                "record.attempted",
                "passed record was not attempted",
            )?;
            require_stdlib(
                !record.parse_failed,
                "record.parse_failed",
                "passed record cannot be parse-failed",
            )?;
            require_stdlib(
                record.raw_state_machine_complete,
                "record.raw_state_machine_complete",
                "passed record must complete the raw state machine",
            )?;
            require_stdlib(
                record.token_only_render_round_trip_equal,
                "record.token_only_render_round_trip_equal",
                "passed record must round-trip its canonical elided tokens back to the canonical source bytes",
            )?;
            require_stdlib(
                record.raw_token_count > 0 && record.canonical_token_count > 0,
                "record.raw_token_count",
                "passed record must carry token counts",
            )?;
        }
        "blocked" => {
            require_stdlib(
                record.attempted,
                "record.attempted",
                "blocked record was not attempted",
            )?;
            require_stdlib(
                record.error.is_some(),
                "record.error",
                "blocked record must carry an error",
            )?;
        }
        "parse_failed_excluded" => {
            require_stdlib(
                !record.attempted,
                "record.attempted",
                "parse-failed record must not be attempted",
            )?;
            require_stdlib(
                record.parse_failed,
                "record.parse_failed",
                "parse-failed record must set parse_failed",
            )?;
        }
        other => {
            return Err(stdlib_validation_error(
                "record.status",
                format!("unsupported record status {other}"),
            ));
        }
    }
    Ok(())
}

fn require_stdlib(
    condition: bool,
    field: &'static str,
    message: impl Into<String>,
) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(stdlib_validation_error(field, message))
    }
}

fn require_stdlib_f64(
    actual: f64,
    expected: f64,
    field: &'static str,
) -> Result<(), Box<dyn Error>> {
    if (actual - expected).abs() <= 0.000_000_1 {
        Ok(())
    } else {
        Err(stdlib_validation_error(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn stdlib_validation_error(field: &'static str, message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("stdlib coverage report {field}: {}", message.into()),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::stdlib_safe_stem;
    use std::collections::BTreeMap;

    #[test]
    fn stdlib_safe_stem_preserves_underscore_vs_slash_distinction() {
        // Real stdlib pair that previously collided when '_' was mapped to '-'.
        assert_eq!(
            stdlib_safe_stem("src/runtime/cgo/mmap.go"),
            "runtime-cgo-mmap-go"
        );
        assert_eq!(
            stdlib_safe_stem("src/runtime/cgo_mmap.go"),
            "runtime-cgo_mmap-go"
        );
        assert_ne!(
            stdlib_safe_stem("src/runtime/cgo/mmap.go"),
            stdlib_safe_stem("src/runtime/cgo_mmap.go")
        );
        assert_eq!(
            stdlib_safe_stem("src/runtime/cgo/sigaction.go"),
            "runtime-cgo-sigaction-go"
        );
        assert_eq!(
            stdlib_safe_stem("src/runtime/cgo_sigaction.go"),
            "runtime-cgo_sigaction-go"
        );
    }

    #[test]
    fn stdlib_safe_stem_still_collapses_path_separators_and_dots() {
        assert_eq!(
            stdlib_safe_stem("src/archive/tar/common.go"),
            "archive-tar-common-go"
        );
        assert_eq!(stdlib_safe_stem("src/fmt/print.go"), "fmt-print-go");
    }

    #[test]
    fn stdlib_safe_stem_unique_for_known_collision_pairs() {
        let paths = [
            "src/runtime/cgo/mmap.go",
            "src/runtime/cgo_mmap.go",
            "src/runtime/cgo/sigaction.go",
            "src/runtime/cgo_sigaction.go",
        ];
        let mut stems = BTreeMap::new();
        for path in paths {
            let stem = stdlib_safe_stem(path);
            assert!(
                stems.insert(stem.clone(), path).is_none(),
                "stem {stem} collided for {path}"
            );
        }
    }
}
