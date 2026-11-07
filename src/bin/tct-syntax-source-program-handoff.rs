//! Write or validate source-program claim handoff reports for the syntax route.
//!
//! The command consumes the parser-backed source-corpus evidence report and
//! emits a volatile handoff report under `target/`. The report keeps the claim
//! boundary scoped to generated-codec validity for checked parser-backed source
//! corpus records; it does not promote arena IR or external sidecar controls.

use clap::{Arg, Command, value_parser};
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tct::syntax_production_route::{self, SyntaxProductionRouteImplementationGate};
use tct::syntax_route_manifest::{self, SyntaxProductionRouteManifest};
use tct::syntax_source_corpus_evidence::{
    self, SyntaxSourceCorpusEvidenceGate, SyntaxSourceCorpusEvidenceReport,
};
use tct::syntax_source_program_handoff::{self, SyntaxSourceProgramClaimHandoffGate};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-syntax-source-program-handoff")
        .about("Write syntax source-program handoff reports")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("fixture")
                .long("fixture")
                .value_name("PATH")
                .help("Checked source-program claim handoff fixture")
                .default_value(syntax_source_program_handoff::FIXTURE_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("source-corpus-fixture")
                .long("source-corpus-fixture")
                .value_name("PATH")
                .help("Checked parser-backed source-corpus evidence fixture")
                .default_value(syntax_source_corpus_evidence::FIXTURE_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("route-gate")
                .long("route-gate")
                .value_name("PATH")
                .help("Checked syntax production-route implementation gate")
                .default_value(syntax_production_route::FIXTURE_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("route-manifest")
                .long("route-manifest")
                .value_name("PATH")
                .help("Checked syntax production-route manifest")
                .default_value(syntax_route_manifest::FIXTURE_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("source-corpus-report")
                .long("source-corpus-report")
                .value_name("PATH")
                .help("Generated parser-backed source-corpus evidence report")
                .default_value(syntax_source_corpus_evidence::REPORT_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_name("PATH")
                .help("Report path to write")
                .default_value(syntax_source_program_handoff::REPORT_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("validate-report")
                .long("validate-report")
                .value_name("PATH")
                .help("Validate an existing source-program handoff report")
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let fixture_path = required_path(&matches, "fixture")?;
    let handoff_gate = read_json::<SyntaxSourceProgramClaimHandoffGate>(fixture_path)?;

    if let Some(report_path) = matches.get_one::<PathBuf>("validate-report") {
        let report = read_json::<syntax_source_program_handoff::SyntaxSourceProgramHandoffReport>(
            report_path,
        )?;
        handoff_gate.validate_report(&report)?;
        println!(
            "validated {} records from {}",
            report.record_count,
            report_path.display()
        );
        return Ok(());
    }

    let source_corpus_fixture_path = required_path(&matches, "source-corpus-fixture")?;
    let route_gate_path = required_path(&matches, "route-gate")?;
    let route_manifest_path = required_path(&matches, "route-manifest")?;
    let source_corpus_report_path = required_path(&matches, "source-corpus-report")?;
    let output_path = required_path(&matches, "output")?;

    let source_corpus_gate =
        read_json::<SyntaxSourceCorpusEvidenceGate>(source_corpus_fixture_path)?;
    let route_gate = read_json::<SyntaxProductionRouteImplementationGate>(route_gate_path)?;
    let route_manifest = read_json::<SyntaxProductionRouteManifest>(route_manifest_path)?;
    let source_corpus_report =
        read_json::<SyntaxSourceCorpusEvidenceReport>(source_corpus_report_path)?;
    let report = handoff_gate.build_report(
        &route_gate,
        &route_manifest,
        &source_corpus_gate,
        &source_corpus_report_path.display().to_string(),
        &source_corpus_report,
    )?;
    write_json(output_path, &report)?;

    println!(
        "wrote {} records to {}",
        report.record_count,
        output_path.display()
    );
    Ok(())
}

fn required_path<'a>(
    matches: &'a clap::ArgMatches,
    name: &str,
) -> Result<&'a PathBuf, Box<dyn Error>> {
    matches.get_one::<PathBuf>(name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("missing required argument {name}"),
        )
        .into()
    })
}

fn read_json<T>(path: &Path) -> Result<T, Box<dyn Error>>
where
    T: serde::de::DeserializeOwned,
{
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json<T>(path: &Path, value: &T) -> Result<(), Box<dyn Error>>
where
    T: serde::Serialize,
{
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let mut content = serde_json::to_vec_pretty(value)?;
    content.push(b'\n');
    fs::write(path, content)?;
    Ok(())
}
