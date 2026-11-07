//! Token-only canonical-render fidelity gate for the Go elided stream.
//!
//! The Go stdlib coverage gate's `render_reparse_equal` is derived from
//! `canonical_render.matches_source`, which the Go projection sidecar computes
//! as `bytes.Equal(format.Source(source), source)` — a gofmt-idempotence check
//! of the *source*, not a token round trip. This suite closes that gap with the
//! round trip the claim actually needs:
//!
//! ```text
//! source -> projection -> canonical elided tokens
//!        -> decode_complete_tree_elided -> render_go_source_from_raw_tree
//!        -> gofmt -> byte compare against the gofmt-canonical source
//! ```
//!
//! Failures are classified into layout-loss buckets so the residue is
//! actionable rather than a single pass/fail number.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use tct::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use tct::provenance::sha256_hex;
use tct::syntax_go_raw_tree_renderer::render_go_source_from_raw_tree;
use tct::syntax_go_tokenizer_generator::{
    PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED,
    build_go_tokenizer_generator_artifacts_with_projection_report_mode,
};
use tct::syntax_go_unified_artifact::{GO_UNIFIED_GRAMMAR_ID, go_unified_grammar_artifact};

/// Pinned GOROOT-relative sample. Kept in a fixture so the denominator of the
/// reported round-trip rate cannot drift between runs.
const CANONICAL_RENDER_FIDELITY_SAMPLE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/canonical-render-fidelity-sample-v1.json";

/// Small always-on subset. One file per layout-loss class observed in the
/// pinned sample, so the fast gate fails for a nameable reason.
const CANONICAL_RENDER_FIDELITY_FOCUS_PATHS: &[&str] = &[
    // one-line function bodies (`func f() bool { return false }`)
    "src/internal/goarch/goarch.go",
    // one-line struct types (`type T struct{ x int }`)
    "src/go/types/typelists.go",
    // blank lines between top-level declarations
    "src/internal/goexperiment/exp_arenas_off.go",
    // import grouping (blank-line-separated import groups)
    "src/bytes/reader.go",
    // trailing/attached comment anchoring
    "src/compress/bzip2/huffman.go",
];

const OUTPUT_DIR: &str = "target/tct-go-canonical-render-fidelity";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Outcome {
    ByteIdentical,
    Mismatch(&'static str),
    RenderError,
    DecodeError,
    ProjectionError,
}

impl Outcome {
    fn bucket(&self) -> &'static str {
        match self {
            Outcome::ByteIdentical => "byte_identical",
            Outcome::Mismatch(class) => class,
            Outcome::RenderError => "render_error",
            Outcome::DecodeError => "decode_error",
            Outcome::ProjectionError => "projection_error",
        }
    }
}

struct FileOutcome {
    path: String,
    outcome: Outcome,
    detail: String,
}

#[test]
#[ignore = "records the current Go elided-stream canonical-render fidelity over the pinned 200-file stdlib sample; slow (one Go projection per file) and reports the layout-loss residue rather than gating it"]
fn go_elided_stream_round_trips_to_canonical_bytes_on_pinned_sample() {
    let paths = match std::env::var("TCT_GO_FIDELITY_PATHS") {
        Ok(value) if !value.trim().is_empty() => value
            .split(',')
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .collect(),
        _ => pinned_sample_paths(),
    };
    let outcomes = round_trip_paths(&paths);
    let report = summarize(&outcomes);
    println!("{report}");
    let failures: Vec<&FileOutcome> = outcomes
        .iter()
        .filter(|outcome| outcome.outcome != Outcome::ByteIdentical)
        .collect();
    assert!(
        failures.is_empty(),
        "Go elided stream is not byte-lossless to gofmt-canonical source for {}/{} pinned sample files\n{report}",
        failures.len(),
        outcomes.len()
    );
}

#[test]
#[ignore = "TDD red state for the Go elided-stream layout-decision fields: un-ignore together with the encoder/renderer change that makes the token-only round trip byte-identical"]
fn go_elided_stream_round_trips_to_canonical_bytes_on_focus_files() {
    let paths: Vec<String> = CANONICAL_RENDER_FIDELITY_FOCUS_PATHS
        .iter()
        .map(|path| (*path).to_string())
        .collect();
    let outcomes = round_trip_paths(&paths);
    let report = summarize(&outcomes);
    println!("{report}");
    let failures: Vec<&FileOutcome> = outcomes
        .iter()
        .filter(|outcome| outcome.outcome != Outcome::ByteIdentical)
        .collect();
    assert!(
        failures.is_empty(),
        "Go elided stream is not byte-lossless to gofmt-canonical source for {}/{} focus files\n{report}",
        failures.len(),
        outcomes.len()
    );
}

fn pinned_sample_paths() -> Vec<String> {
    let contents = fs::read_to_string(CANONICAL_RENDER_FIDELITY_SAMPLE_PATH)
        .unwrap_or_else(|error| panic!("{CANONICAL_RENDER_FIDELITY_SAMPLE_PATH}: {error}"));
    let value: Value = serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("{CANONICAL_RENDER_FIDELITY_SAMPLE_PATH}: {error}"));
    value["paths"]
        .as_array()
        .expect("canonical-render fidelity sample must carry a paths array")
        .iter()
        .map(|path| {
            path.as_str()
                .expect("sample paths must be strings")
                .to_string()
        })
        .collect()
}

fn round_trip_paths(paths: &[String]) -> Vec<FileOutcome> {
    let toolchain = tct::go_toolchain::ensure_process_env()
        .expect("pinned Go toolchain must resolve for the canonical-render fidelity gate");
    let goroot = go_env_goroot(&toolchain.go_bin);
    let output_dir = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&output_dir).expect("fidelity output dir");
    paths
        .iter()
        .map(|path| {
            round_trip_one(
                &toolchain.go_bin,
                &toolchain.gofmt_bin,
                &goroot,
                &output_dir,
                path,
            )
        })
        .collect()
}

fn round_trip_one(
    go_bin: &Path,
    gofmt_bin: &Path,
    goroot: &Path,
    output_dir: &Path,
    goroot_relative_path: &str,
) -> FileOutcome {
    match try_round_trip_one(go_bin, gofmt_bin, goroot, output_dir, goroot_relative_path) {
        Ok(outcome) => outcome,
        Err(error) => FileOutcome {
            path: goroot_relative_path.to_string(),
            outcome: Outcome::ProjectionError,
            detail: error.to_string(),
        },
    }
}

fn try_round_trip_one(
    go_bin: &Path,
    gofmt_bin: &Path,
    goroot: &Path,
    output_dir: &Path,
    goroot_relative_path: &str,
) -> Result<FileOutcome, Box<dyn Error>> {
    let source_path = goroot.join(goroot_relative_path);
    let stem = goroot_relative_path.replace(['/', '.'], "-");
    let projection_output = output_dir.join(format!("{stem}-source-instance-projection-v1.json"));
    let projection_report_output =
        output_dir.join(format!("source-instance-projection-report-{stem}-v1.json"));
    let manifest_path = goroot_relative_path
        .strip_prefix("src/")
        .unwrap_or(goroot_relative_path);

    run_projection(
        go_bin,
        &source_path,
        manifest_path,
        &projection_output,
        &projection_report_output,
    )?;

    let projection_contents = fs::read_to_string(&projection_output)?;
    let artifacts = build_go_tokenizer_generator_artifacts_with_projection_report_mode(
        &projection_report_output,
        &projection_output,
        &projection_contents,
        PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED,
    )?;

    // The projection works on the gofmt-canonical form of the source, so the
    // round-trip target is gofmt(source), not the on-disk bytes.
    let source_bytes = fs::read(&source_path)?;
    let canonical = gofmt(gofmt_bin, &source_bytes)?;

    let artifact = go_unified_grammar_artifact()?;
    let tokens = &artifacts.generated_codec_report.tct_tokens;
    let decoded = match artifact.compiled_model.decode_complete_tree_elided(tokens) {
        Ok(decoded) => decoded,
        Err(error) => {
            return Ok(FileOutcome {
                path: goroot_relative_path.to_string(),
                outcome: Outcome::DecodeError,
                detail: format!("{error:?}"),
            });
        }
    };
    let rendered = match render_go_source_from_raw_tree(&decoded) {
        Ok(rendered) => rendered,
        Err(error) => {
            return Ok(FileOutcome {
                path: goroot_relative_path.to_string(),
                outcome: Outcome::RenderError,
                detail: error.to_string(),
            });
        }
    };
    let formatted = gofmt(gofmt_bin, rendered.source.as_bytes())?;
    if std::env::var_os("TCT_GO_FIDELITY_DUMP").is_some() {
        fs::write(output_dir.join(format!("{stem}.rendered.go")), &formatted)?;
        fs::write(output_dir.join(format!("{stem}.canonical.go")), &canonical)?;
    }
    if formatted == canonical {
        return Ok(FileOutcome {
            path: goroot_relative_path.to_string(),
            outcome: Outcome::ByteIdentical,
            detail: format!("sha256 {}", sha256_hex(&canonical)),
        });
    }
    let (class, detail) = classify_mismatch(&canonical, &formatted);
    Ok(FileOutcome {
        path: goroot_relative_path.to_string(),
        outcome: Outcome::Mismatch(class),
        detail,
    })
}

fn run_projection(
    go_bin: &Path,
    source_path: &Path,
    manifest_path: &str,
    projection_output: &Path,
    projection_report_output: &Path,
) -> Result<(), Box<dyn Error>> {
    let output = Command::new(go_bin)
        .arg("run")
        .arg("scripts/analysis/go_syntax_source_instance_project.go")
        .arg("--source")
        .arg(source_path)
        .arg("--metadata")
        .arg(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH)
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--candidate-grammar-id")
        .arg(GO_UNIFIED_GRAMMAR_ID)
        .arg("--projection-output")
        .arg(projection_output)
        .arg("--report-output")
        .arg(projection_report_output)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "Go projection sidecar failed for {}\nstderr:\n{}",
            source_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

fn gofmt(gofmt_bin: &Path, source: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut child = Command::new(gofmt_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .expect("gofmt stdin")
        .write_all(source)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!("gofmt failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(output.stdout)
}

fn go_env_goroot(go_bin: &Path) -> PathBuf {
    let output = Command::new(go_bin)
        .arg("env")
        .arg("GOROOT")
        .output()
        .expect("go env GOROOT");
    PathBuf::from(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Classify the first structural difference between the canonical source and
/// the round-tripped render into a layout-loss bucket.
fn classify_mismatch(canonical: &[u8], rendered: &[u8]) -> (&'static str, String) {
    let canonical_text = String::from_utf8_lossy(canonical);
    let rendered_text = String::from_utf8_lossy(rendered);
    let canonical_lines: Vec<&str> = canonical_text.lines().collect();
    let rendered_lines: Vec<&str> = rendered_text.lines().collect();

    let mut index = 0usize;
    while index < canonical_lines.len()
        && index < rendered_lines.len()
        && canonical_lines[index] == rendered_lines[index]
    {
        index += 1;
    }
    let canonical_line = canonical_lines.get(index).copied().unwrap_or("");
    let rendered_line = rendered_lines.get(index).copied().unwrap_or("");
    let detail = format!(
        "line {}: canonical {:?} vs rendered {:?}",
        index + 1,
        canonical_line,
        rendered_line
    );

    // A one-line body/struct in the canonical source becomes several rendered
    // lines: the canonical line carries a matched `{ ... }` on one line while
    // the render breaks after the brace.
    let canonical_one_line_block = canonical_line.contains('{')
        && canonical_line.contains('}')
        && !canonical_line.trim_start().starts_with("//");
    if canonical_one_line_block && rendered_line.trim_end().ends_with('{') {
        if canonical_line.contains("struct{") || canonical_line.contains("interface{") {
            return ("mismatch_one_line_struct_type", detail);
        }
        return ("mismatch_one_line_body", detail);
    }

    let canonical_blank = canonical_line.trim().is_empty();
    let rendered_blank = rendered_line.trim().is_empty();
    if canonical_blank != rendered_blank {
        let context = canonical_lines[..index]
            .iter()
            .rev()
            .find(|line| !line.trim().is_empty())
            .copied()
            .unwrap_or("");
        let in_import_block = canonical_lines[..index]
            .iter()
            .rev()
            .take_while(|line| !line.starts_with(')'))
            .any(|line| line.starts_with("import ("));
        if in_import_block || context.trim_start().starts_with('"') {
            return ("mismatch_import_grouping", detail);
        }
        return ("mismatch_decl_spacing", detail);
    }

    if canonical_line.contains("//")
        || rendered_line.contains("//")
        || canonical_line.contains("/*")
        || rendered_line.contains("/*")
    {
        return ("mismatch_comment_anchor", detail);
    }
    ("mismatch_other", detail)
}

fn summarize(outcomes: &[FileOutcome]) -> String {
    let mut buckets: BTreeMap<&str, usize> = BTreeMap::new();
    for outcome in outcomes {
        *buckets.entry(outcome.outcome.bucket()).or_default() += 1;
    }
    let identical = buckets.get("byte_identical").copied().unwrap_or(0);
    let mut report = format!(
        "go elided-stream canonical-render fidelity: {identical}/{} byte-identical\n",
        outcomes.len()
    );
    for (bucket, count) in &buckets {
        report.push_str(&format!("  {bucket}: {count}\n"));
    }
    for outcome in outcomes {
        if outcome.outcome != Outcome::ByteIdentical {
            report.push_str(&format!(
                "  {} [{}] {}\n",
                outcome.path,
                outcome.outcome.bucket(),
                outcome.detail
            ));
        }
    }
    report
}
