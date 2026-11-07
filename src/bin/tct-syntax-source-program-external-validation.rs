//! Write or validate syntax source-program external-validation execution reports.
//!
//! The command consumes the generated source-program handoff report plus the
//! checked external-validation controls fixture. The report is volatile output
//! under `target/`: it records observed tool versions and missing downstream
//! controls without promoting parser, formatter, type-checker, functional
//! validator, or arena IR evidence into the syntax-level proof boundary.

use clap::{Arg, Command, value_parser};
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use tct::syntax_source_program_external_validation::{
    self, SyntaxSourceProgramExternalValidationControlsGate,
    SyntaxSourceProgramExternalValidationExecutionReport,
    SyntaxSourceProgramExternalValidationToolEnvelope,
    SyntaxSourceProgramExternalValidationToolObservation,
};
use tct::syntax_source_program_handoff::{
    self, SyntaxSourceProgramClaimHandoffGate, SyntaxSourceProgramHandoffReport,
};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-syntax-source-program-external-validation")
        .about("Write syntax source-program external-validation execution reports")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("fixture")
                .long("fixture")
                .value_name("PATH")
                .help("Checked source-program external-validation controls fixture")
                .default_value(syntax_source_program_external_validation::FIXTURE_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("source-program-handoff-fixture")
                .long("source-program-handoff-fixture")
                .value_name("PATH")
                .help("Checked source-program claim handoff fixture")
                .default_value(syntax_source_program_handoff::FIXTURE_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("source-program-handoff-report")
                .long("source-program-handoff-report")
                .value_name("PATH")
                .help("Generated source-program handoff report")
                .default_value(syntax_source_program_handoff::REPORT_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_name("PATH")
                .help("External-validation execution report path to write")
                .default_value(syntax_source_program_external_validation::EXECUTION_REPORT_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("validate-report")
                .long("validate-report")
                .value_name("PATH")
                .help("Validate an existing external-validation execution report")
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let fixture_path = required_path(&matches, "fixture")?;
    let handoff_fixture_path = required_path(&matches, "source-program-handoff-fixture")?;
    let handoff_report_path = required_path(&matches, "source-program-handoff-report")?;

    let gate = read_json::<SyntaxSourceProgramExternalValidationControlsGate>(fixture_path)?;
    let handoff_gate = read_json::<SyntaxSourceProgramClaimHandoffGate>(handoff_fixture_path)?;
    let handoff_report = read_json::<SyntaxSourceProgramHandoffReport>(handoff_report_path)?;

    if let Some(report_path) = matches.get_one::<PathBuf>("validate-report") {
        let report =
            read_json::<SyntaxSourceProgramExternalValidationExecutionReport>(report_path)?;
        gate.validate_execution_report(&handoff_gate, &handoff_report, &report)?;
        println!(
            "validated {} records from {}",
            report.record_count,
            report_path.display()
        );
        return Ok(());
    }

    let output_path = required_path(&matches, "output")?;
    let tool_envelope = collect_tool_envelope();
    let report = gate.build_execution_report(
        &handoff_gate,
        &fixture_path.display().to_string(),
        &handoff_report_path.display().to_string(),
        &handoff_report,
        tool_envelope,
    )?;
    write_json(output_path, &report)?;

    println!(
        "wrote {} records to {}",
        report.record_count,
        output_path.display()
    );
    Ok(())
}

fn collect_tool_envelope() -> SyntaxSourceProgramExternalValidationToolEnvelope {
    SyntaxSourceProgramExternalValidationToolEnvelope::new(
        env!("CARGO_PKG_VERSION"),
        vec![
            observe_tool("go", "go version", "go", &["version"]),
            observe_tool("dotnet", "dotnet --version", "dotnet", &["--version"]),
            observe_tool("python3", "python3 --version", "python3", &["--version"]),
            observe_tool("cargo", "cargo --version", "cargo", &["--version"]),
        ],
    )
}

fn observe_tool(
    tool_id: &str,
    command_label: &str,
    program: &str,
    args: &[&str],
) -> SyntaxSourceProgramExternalValidationToolObservation {
    match ProcessCommand::new(program).args(args).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = stdout
                .lines()
                .chain(stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("version unavailable")
                .to_string();
            SyntaxSourceProgramExternalValidationToolObservation::available(
                tool_id,
                command_label,
                version,
            )
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let reason = stderr
                .lines()
                .chain(stdout.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("command exited with {}", output.status));
            SyntaxSourceProgramExternalValidationToolObservation::unavailable(
                tool_id,
                command_label,
                reason,
            )
        }
        Err(error) => SyntaxSourceProgramExternalValidationToolObservation::unavailable(
            tool_id,
            command_label,
            error.to_string(),
        ),
    }
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
