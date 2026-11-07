//! Write or validate parser-backed source-corpus evidence for the syntax production route.
//!
//! The command derives records from checked C# and Go parser metadata through
//! the sidecar syntax production route. It writes a volatile report under
//! `target/` by default and does not promote syntax input through `TctCompiler`.

use clap::{Arg, Command, value_parser};
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tct::syntax_production_route::{self, SyntaxProductionRouteImplementationGate};
use tct::syntax_route_manifest::{self, SyntaxProductionRouteManifest};
use tct::syntax_source_corpus_evidence::{self, SyntaxSourceCorpusEvidenceGate};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-syntax-source-corpus-evidence")
        .about("Write syntax source-corpus evidence reports")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("fixture")
                .long("fixture")
                .value_name("PATH")
                .help("Checked source-corpus evidence fixture")
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
            Arg::new("output")
                .long("output")
                .value_name("PATH")
                .help("Report path to write")
                .default_value(syntax_source_corpus_evidence::REPORT_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("validate-report")
                .long("validate-report")
                .value_name("PATH")
                .help("Validate an existing source-corpus evidence report")
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let fixture_path = required_path(&matches, "fixture")?;
    let route_gate_path = required_path(&matches, "route-gate")?;
    let route_manifest_path = required_path(&matches, "route-manifest")?;

    let gate = read_json::<SyntaxSourceCorpusEvidenceGate>(fixture_path)?;
    let route_gate = read_json::<SyntaxProductionRouteImplementationGate>(route_gate_path)?;
    let route_manifest = read_json::<SyntaxProductionRouteManifest>(route_manifest_path)?;
    if let Some(report_path) = matches.get_one::<PathBuf>("validate-report") {
        let report = read_json::<syntax_source_corpus_evidence::SyntaxSourceCorpusEvidenceReport>(
            report_path,
        )?;
        gate.validate_report(&route_gate, &route_manifest, &report)?;
        println!(
            "validated {} records from {}",
            report.record_count,
            report_path.display()
        );
        return Ok(());
    }

    let output_path = required_path(&matches, "output")?;
    let report = gate.build_report(&route_gate, &route_manifest)?;
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
