//! Generate the first Go raw syntax-token tokenizer report.

use clap::parser::ValueSource;
use clap::{Arg, ArgAction, Command, value_parser};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use tct::syntax_go_random_admissible_walk::{
    ARTIFACT_KIND as RANDOM_ADMISSIBLE_WALK_ARTIFACT_KIND, DEFAULT_MAX_ELIDED_STEPS,
    DEFAULT_OUTPUT_PATH as DEFAULT_RANDOM_ADMISSIBLE_WALK_OUTPUT_PATH, DEFAULT_SEED,
    DEFAULT_WALK_COUNT, GoRandomAdmissibleWalkConfig, build_go_random_admissible_walk_report,
    parse_go_random_admissible_walk_report,
};
use tct::syntax_go_source_instance_projection::parse_checked_report;
use tct::syntax_go_tokenizer_generator::{
    DEFAULT_CHECKED_REPORT_PATH, DEFAULT_GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_PATH,
    DEFAULT_MANIFEST_DEPTH_OUTPUT_PATH, DEFAULT_OUTPUT_PATH,
    GoRawTreeAstPrinterOracleCutoverReportInput, GoTokenizerGeneratorManifestDepthReportInput,
    GoTokenizerGeneratorReport, MANIFEST_DEPTH_ARTIFACT_KIND,
    PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED, build_go_tokenizer_generator_artifacts,
    build_go_tokenizer_generator_artifacts_with_projection_report_mode,
    build_go_tokenizer_generator_manifest_depth_report,
    go_tokenizer_generator_report_ingestion_sources,
    parse_go_tokenizer_generator_manifest_depth_report, parse_go_tokenizer_generator_report,
};
use tct::syntax_production_route::syntax_codec_registry_entry;

#[path = "tct_syntax_go_tokenizer_generator/stdlib_coverage.rs"]
mod stdlib_coverage;

use stdlib_coverage::{
    DEFAULT_STDLIB_COVERAGE_OUTPUT_DIR, DEFAULT_STDLIB_COVERAGE_OUTPUT_PATH,
    GoStdlibTokenizerCoverageReport, STDLIB_COVERAGE_ARTIFACT_KIND, build_stdlib_coverage_report,
    parse_stdlib_coverage_report,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-syntax-go-tokenizer-generator")
        .about("Generate Go raw syntax-token tokenizer reports")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("random-admissible-walks")
                .long("random-admissible-walks")
                .help("Generate the unified Go artifact random-admissible-walk validity report")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("random-walk-count")
                .long("random-walk-count")
                .value_name("N")
                .help("Number of deterministic random admissible walks to sample")
                .default_value("16")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("random-walk-seed")
                .long("random-walk-seed")
                .value_name("U64")
                .help("Base random seed for --random-admissible-walks")
                .default_value("6071789276072201523")
                .value_parser(value_parser!(u64)),
        )
        .arg(
            Arg::new("random-walk-max-elided-steps")
                .long("random-walk-max-elided-steps")
                .value_name("N")
                .help("Maximum sampled elided tokens per random admissible walk")
                .default_value("512")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("checked-report")
                .long("checked-report")
                .value_name("PATH")
                .help("Checked Go source-instance projection report")
                .default_value(DEFAULT_CHECKED_REPORT_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("report-ingestion-manifest")
                .long("report-ingestion-manifest")
                .value_name("PATH")
                .help("Report-ingestion manifest whose source-instance records should be checked as a tokenizer-generator depth gate")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("checked-report-dir")
                .long("checked-report-dir")
                .value_name("DIR")
                .help("Directory of checked Go source-instance projection reports used by --report-ingestion-manifest")
                .default_value("tests/fixtures/go_syntax_corpus")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("go-ast-printer-oracle-report")
                .long("go-ast-printer-oracle-report")
                .value_name("PATH")
                .help("Checked Go raw-tree AST printer oracle full-adoption report used by --report-ingestion-manifest")
                .default_value(DEFAULT_GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("stdlib-coverage-records")
                .long("stdlib-coverage-records")
                .value_name("PATH")
                .help("Per-file JSONL emitted by go_stdlib_syntax_coverage_scan --enumerate"),
        )
        .arg(
            Arg::new("stdlib-coverage-goroot")
                .long("stdlib-coverage-goroot")
                .value_name("DIR")
                .help("GOROOT containing the stdlib files named by --stdlib-coverage-records")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("stdlib-coverage-output-dir")
                .long("stdlib-coverage-output-dir")
                .value_name("DIR")
                .help("Directory for volatile per-file stdlib projection artifacts")
                .default_value(DEFAULT_STDLIB_COVERAGE_OUTPUT_DIR)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("stdlib-coverage-limit")
                .long("stdlib-coverage-limit")
                .value_name("N")
                .help("Limit parseable stdlib files processed by the coverage gate for smoke validation")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("stdlib-coverage-jobs")
                .long("stdlib-coverage-jobs")
                .value_name("N")
                .help("Number of parallel per-file stdlib coverage jobs; defaults to available parallelism capped at 8")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("projection-output")
                .long("projection-output")
                .value_name("PATH")
                .help("Volatile Go source-instance projection artifact to regenerate")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("source")
                .long("source")
                .value_name("PATH")
                .help("Go source file to project through the registry-selected metadata")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("grammar-id")
                .long("grammar-id")
                .value_name("ID")
                .help("Registered Go syntax grammar id used with --source"),
        )
        .arg(
            Arg::new("manifest-path")
                .long("manifest-path")
                .value_name("PATH")
                .help("Corpus manifest entry path recorded by the source projection report"),
        )
        .arg(
            Arg::new("projection-report-output")
                .long("projection-report-output")
                .value_name("PATH")
                .help("Volatile Go source-instance projection report generated by --source mode")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_name("PATH")
                .help("Tokenizer-generator report path to write")
                .default_value(DEFAULT_OUTPUT_PATH)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("go-bin")
                .long("go-bin")
                .value_name("PATH")
                .help("Go toolchain binary used for the parser-backed projection sidecar"),
        )
        .arg(
            Arg::new("validate-report")
                .long("validate-report")
                .value_name("PATH")
                .help("Validate an existing Go tokenizer-generator report without writing outputs")
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    if let Some(report_path) = matches.get_one::<PathBuf>("validate-report") {
        validate_tokenizer_generator_report(report_path)?;
        return Ok(());
    }

    let go_bin = matches
        .get_one::<String>("go-bin")
        .cloned()
        .or_else(|| std::env::var("TCT_GO_BIN").ok())
        .or_else(|| tct::go_toolchain::go_bin_string().ok())
        .unwrap_or_else(|| "go".to_string());

    let artifacts = if matches.get_flag("random-admissible-walks") {
        if matches.get_one::<PathBuf>("source").is_some()
            || matches
                .get_one::<PathBuf>("report-ingestion-manifest")
                .is_some()
            || matches
                .get_one::<String>("stdlib-coverage-records")
                .is_some()
            || matches.get_one::<PathBuf>("projection-output").is_some()
            || matches
                .get_one::<PathBuf>("projection-report-output")
                .is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--random-admissible-walks cannot be combined with --source, --report-ingestion-manifest, --stdlib-coverage-records, --projection-output, or --projection-report-output",
            )
            .into());
        }
        GeneratorOutput::RandomAdmissibleWalk(Box::new(build_go_random_admissible_walk_report(
            &random_admissible_walk_config(&matches)?,
            &go_bin,
        )?))
    } else if let Some(records_path) = matches.get_one::<String>("stdlib-coverage-records") {
        if matches.get_one::<PathBuf>("source").is_some()
            || matches
                .get_one::<PathBuf>("report-ingestion-manifest")
                .is_some()
            || matches.get_one::<PathBuf>("projection-output").is_some()
            || matches
                .get_one::<PathBuf>("projection-report-output")
                .is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--stdlib-coverage-records cannot be combined with --source, --report-ingestion-manifest, --projection-output, or --projection-report-output",
            )
            .into());
        }
        GeneratorOutput::StdlibCoverage(Box::new(build_stdlib_coverage_report(
            &matches,
            &go_bin,
            Path::new(records_path),
        )?))
    } else if let Some(source_path) = matches.get_one::<PathBuf>("source") {
        if matches
            .get_one::<PathBuf>("report-ingestion-manifest")
            .is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--report-ingestion-manifest cannot be combined with --source",
            )
            .into());
        }
        GeneratorOutput::Single(Box::new(build_source_driven_artifacts(
            &matches,
            &go_bin,
            source_path,
        )?))
    } else if let Some(report_ingestion_manifest_path) =
        matches.get_one::<PathBuf>("report-ingestion-manifest")
    {
        if matches.get_one::<PathBuf>("projection-output").is_some()
            || matches
                .get_one::<PathBuf>("projection-report-output")
                .is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--projection-output and --projection-report-output are not valid with --report-ingestion-manifest",
            )
            .into());
        }
        build_manifest_depth_report(&matches, &go_bin, report_ingestion_manifest_path)?
    } else {
        if matches
            .get_one::<PathBuf>("projection-report-output")
            .is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--projection-report-output is only valid with --source",
            )
            .into());
        }
        GeneratorOutput::Single(Box::new(build_checked_report_artifacts(&matches, &go_bin)?))
    };

    match artifacts {
        GeneratorOutput::Single(artifacts) => {
            write_json(
                &artifacts.generated_codec_report.report_path,
                &artifacts.generated_codec_report,
            )?;
            let output_path = required_path(&matches, "output")?;
            write_json(output_path, &artifacts.report)?;

            println!(
                "wrote Go tokenizer-generator report for {} raw tokens to {}",
                artifacts.report.raw_token_count,
                output_path.display()
            );
        }
        GeneratorOutput::ManifestDepth {
            report,
            generated_codec_reports,
        } => {
            for generated_codec_report in generated_codec_reports {
                write_json(&generated_codec_report.report_path, &generated_codec_report)?;
            }
            let output_path = output_path_for_mode(&matches, DEFAULT_MANIFEST_DEPTH_OUTPUT_PATH)?;
            write_json(&output_path, &report)?;

            println!(
                "wrote Go tokenizer-generator manifest-depth report for {} records to {}",
                report.supported_record_count,
                output_path.display()
            );
        }
        GeneratorOutput::StdlibCoverage(report) => {
            let output_path = output_path_for_mode(&matches, DEFAULT_STDLIB_COVERAGE_OUTPUT_PATH)?;
            write_json(&output_path, &report)?;

            println!(
                "wrote Go stdlib syntax tokenizer coverage report for {}/{} selected parseable files to {}",
                report.attempted_record_count,
                report.input_denominator,
                output_path.display()
            );
        }
        GeneratorOutput::RandomAdmissibleWalk(report) => {
            let output_path =
                output_path_for_mode(&matches, DEFAULT_RANDOM_ADMISSIBLE_WALK_OUTPUT_PATH)?;
            write_json(&output_path, &report)?;

            println!(
                "wrote Go random-admissible-walk validity report for {}/{} passed walks to {}",
                report.validity_passed_count,
                report.walk_count,
                output_path.display()
            );
        }
    }
    Ok(())
}

enum GeneratorOutput {
    Single(Box<tct::syntax_go_tokenizer_generator::GoTokenizerGeneratorArtifacts>),
    ManifestDepth {
        report: Box<tct::syntax_go_tokenizer_generator::GoTokenizerGeneratorManifestDepthReport>,
        generated_codec_reports:
            Vec<tct::syntax_go_source_instance_projection::SourceInstanceGeneratedCodecReport>,
    },
    StdlibCoverage(Box<GoStdlibTokenizerCoverageReport>),
    RandomAdmissibleWalk(Box<tct::syntax_go_random_admissible_walk::GoRandomAdmissibleWalkReport>),
}

fn random_admissible_walk_config(
    matches: &clap::ArgMatches,
) -> Result<GoRandomAdmissibleWalkConfig, Box<dyn Error>> {
    let walk_count = matches
        .get_one::<usize>("random-walk-count")
        .copied()
        .unwrap_or(DEFAULT_WALK_COUNT);
    let seed = matches
        .get_one::<u64>("random-walk-seed")
        .copied()
        .unwrap_or(DEFAULT_SEED);
    let max_elided_steps = matches
        .get_one::<usize>("random-walk-max-elided-steps")
        .copied()
        .unwrap_or(DEFAULT_MAX_ELIDED_STEPS);
    if walk_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--random-walk-count must be greater than zero",
        )
        .into());
    }
    if max_elided_steps == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--random-walk-max-elided-steps must be greater than zero",
        )
        .into());
    }
    let output_path = output_path_for_mode(matches, DEFAULT_RANDOM_ADMISSIBLE_WALK_OUTPUT_PATH)?;
    Ok(GoRandomAdmissibleWalkConfig {
        seed,
        walk_count,
        max_elided_steps,
        output_path: output_path.display().to_string(),
    })
}

fn build_checked_report_artifacts(
    matches: &clap::ArgMatches,
    go_bin: &str,
) -> Result<tct::syntax_go_tokenizer_generator::GoTokenizerGeneratorArtifacts, Box<dyn Error>> {
    let checked_report_path = required_path(matches, "checked-report")?;
    let checked_report_contents = fs::read_to_string(checked_report_path)?;
    let checked_report = parse_checked_report(&checked_report_contents)?;
    let projection_output = matches
        .get_one::<PathBuf>("projection-output")
        .cloned()
        .unwrap_or_else(|| PathBuf::from(&checked_report.projection_output));

    regenerate_projection_artifact(ProjectionRegenerationRequest {
        go_bin,
        source_path: Path::new(&checked_report.source.path),
        metadata_path: Path::new(&checked_report.metadata.path),
        metadata_smoke_fixture: None,
        manifest_path: Some(&checked_report.manifest_path),
        grammar_id: &checked_report.candidate_grammar_id,
        projection_output: &projection_output,
        projection_report_output: None,
        route_registered: false,
    })?;
    let projection_artifact_contents = fs::read_to_string(&projection_output)?;
    Ok(build_go_tokenizer_generator_artifacts(
        checked_report_path,
        &projection_output,
        &projection_artifact_contents,
    )?)
}

fn build_source_driven_artifacts(
    matches: &clap::ArgMatches,
    go_bin: &str,
    source_path: &Path,
) -> Result<tct::syntax_go_tokenizer_generator::GoTokenizerGeneratorArtifacts, Box<dyn Error>> {
    let grammar_id = matches.get_one::<String>("grammar-id").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "--source requires --grammar-id",
        )
    })?;
    let entry = syntax_codec_registry_entry(grammar_id).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{grammar_id} is not registered in SYNTAX_CODEC_REGISTRY"),
        )
    })?;
    if entry.target_id != "go" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "--source requires a Go syntax registry entry, got {}",
                entry.target_id
            ),
        )
        .into());
    }

    let projection_output = matches
        .get_one::<PathBuf>("projection-output")
        .cloned()
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "target/tct-syntax-go-tokenizer-generator/{grammar_id}-source-instance-projection-v1.json"
            ))
        });
    let projection_report_output = matches
        .get_one::<PathBuf>("projection-report-output")
        .cloned()
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "target/tct-syntax-go-tokenizer-generator/{grammar_id}-source-instance-projection-report-v1.json"
            ))
        });
    let manifest_path = matches.get_one::<String>("manifest-path").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "--source requires --manifest-path so the projection report records the corpus entry",
        )
    })?;
    let metadata_smoke_fixture = go_metadata_smoke_fixture_from_metadata_path(entry.metadata_path)?;

    regenerate_projection_artifact(ProjectionRegenerationRequest {
        go_bin,
        source_path,
        metadata_path: Path::new(entry.metadata_path),
        metadata_smoke_fixture: Some(&metadata_smoke_fixture),
        manifest_path: Some(manifest_path),
        grammar_id,
        projection_output: &projection_output,
        projection_report_output: Some(&projection_report_output),
        route_registered: true,
    })?;
    let projection_artifact_contents = fs::read_to_string(&projection_output)?;
    Ok(
        build_go_tokenizer_generator_artifacts_with_projection_report_mode(
            &projection_report_output,
            &projection_output,
            &projection_artifact_contents,
            PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED,
        )?,
    )
}

fn build_manifest_depth_report(
    matches: &clap::ArgMatches,
    go_bin: &str,
    report_ingestion_manifest_path: &Path,
) -> Result<GeneratorOutput, Box<dyn Error>> {
    let report_ingestion_manifest_contents = fs::read_to_string(report_ingestion_manifest_path)?;
    let oracle_report_path = required_path(matches, "go-ast-printer-oracle-report")?;
    let oracle_report_contents = fs::read_to_string(oracle_report_path)?;
    let required_sources =
        go_tokenizer_generator_report_ingestion_sources(&report_ingestion_manifest_contents)?;
    let mut required_by_source = required_sources
        .iter()
        .map(|source| {
            (
                (source.source_path.clone(), source.source_sha256.clone()),
                source.manifest_path.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let checked_report_dir = required_path(matches, "checked-report-dir")?;
    let mut checked_report_paths = fs::read_dir(checked_report_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    checked_report_paths.sort();

    let mut artifacts_by_path = Vec::new();
    for checked_report_path in checked_report_paths {
        if !checked_report_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.starts_with("source-instance-projection-report-") && name.ends_with(".json")
            })
        {
            continue;
        }
        let checked_report_contents = fs::read_to_string(&checked_report_path)?;
        let checked_report = parse_checked_report(&checked_report_contents)?;
        let key = (
            checked_report.source.path.clone(),
            checked_report.source.sha256.clone(),
        );
        if !required_by_source.contains_key(&key) {
            continue;
        }

        let projection_output = PathBuf::from(&checked_report.projection_output);
        regenerate_projection_artifact(ProjectionRegenerationRequest {
            go_bin,
            source_path: Path::new(&checked_report.source.path),
            metadata_path: Path::new(&checked_report.metadata.path),
            metadata_smoke_fixture: None,
            manifest_path: Some(&checked_report.manifest_path),
            grammar_id: &checked_report.candidate_grammar_id,
            projection_output: &projection_output,
            projection_report_output: None,
            route_registered: false,
        })?;
        let projection_artifact_contents = fs::read_to_string(&projection_output)?;
        let artifacts = build_go_tokenizer_generator_artifacts(
            &checked_report_path,
            &projection_output,
            &projection_artifact_contents,
        )?;
        required_by_source.remove(&key);
        artifacts_by_path.push((checked_report_path, artifacts));
    }

    if !required_by_source.is_empty() {
        let missing = required_by_source
            .values()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "checked report directory {} is missing manifest-depth sources: {missing}",
                checked_report_dir.display()
            ),
        )
        .into());
    }

    let inputs = artifacts_by_path
        .iter()
        .map(|(checked_projection_report_path, artifacts)| {
            GoTokenizerGeneratorManifestDepthReportInput {
                checked_projection_report_path,
                artifacts,
            }
        })
        .collect::<Vec<_>>();
    let report = build_go_tokenizer_generator_manifest_depth_report(
        report_ingestion_manifest_path,
        &report_ingestion_manifest_contents,
        GoRawTreeAstPrinterOracleCutoverReportInput {
            report_path: oracle_report_path,
            report_contents: &oracle_report_contents,
        },
        &inputs,
    )?;
    let generated_codec_reports = artifacts_by_path
        .into_iter()
        .map(|(_, artifacts)| artifacts.generated_codec_report)
        .collect();
    Ok(GeneratorOutput::ManifestDepth {
        report: Box::new(report),
        generated_codec_reports,
    })
}

struct ProjectionRegenerationRequest<'a> {
    go_bin: &'a str,
    source_path: &'a Path,
    metadata_path: &'a Path,
    metadata_smoke_fixture: Option<&'a str>,
    manifest_path: Option<&'a str>,
    grammar_id: &'a str,
    projection_output: &'a Path,
    projection_report_output: Option<&'a Path>,
    route_registered: bool,
}

fn regenerate_projection_artifact(
    request: ProjectionRegenerationRequest<'_>,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = request
        .projection_output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    if let Some(report_output) = request.projection_report_output
        && let Some(parent) = report_output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let mut command = ProcessCommand::new(request.go_bin);
    command
        .arg("run")
        .arg("scripts/analysis/go_syntax_source_instance_project.go")
        .arg("--source")
        .arg(request.source_path)
        .arg("--metadata")
        .arg(request.metadata_path);
    if let Some(metadata_smoke_fixture) = request.metadata_smoke_fixture {
        command
            .arg("--metadata-smoke-fixture")
            .arg(metadata_smoke_fixture);
    }
    if let Some(manifest_path) = request.manifest_path {
        command.arg("--manifest-path").arg(manifest_path);
    }
    command
        .arg("--candidate-grammar-id")
        .arg(request.grammar_id)
        .arg("--projection-output")
        .arg(request.projection_output);
    if let Some(report_output) = request.projection_report_output {
        command.arg("--report-output").arg(report_output);
    }
    if request.route_registered {
        command.arg("--route-registered");
    }

    let output = command.output().map_err(|error| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "failed to run Go projection sidecar with {}: {error}",
                request.go_bin
            ),
        )
    })?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "Go projection sidecar failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
        .into());
    }
    Ok(())
}

fn go_metadata_smoke_fixture_from_metadata_path(path: &str) -> Result<String, Box<dyn Error>> {
    let parent = Path::new(path)
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("failed to derive Go metadata smoke fixture from {path}"),
            )
        })?;
    Ok(parent.replace('_', "-"))
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

fn validate_tokenizer_generator_report(path: &Path) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&contents)?;
    match value
        .get("artifact_kind")
        .and_then(serde_json::Value::as_str)
    {
        Some(MANIFEST_DEPTH_ARTIFACT_KIND) => {
            let report = parse_go_tokenizer_generator_manifest_depth_report(&contents)?;
            println!(
                "validated Go tokenizer-generator manifest-depth report for {} records from {}",
                report.supported_record_count, report.report_ingestion_manifest_path
            );
        }
        Some(STDLIB_COVERAGE_ARTIFACT_KIND) => {
            let report = parse_stdlib_coverage_report(&contents)?;
            println!(
                "validated Go stdlib syntax tokenizer coverage report for {}/{} selected parseable files from {}",
                report.attempted_record_count, report.input_denominator, report.file_records_path
            );
        }
        Some(RANDOM_ADMISSIBLE_WALK_ARTIFACT_KIND) => {
            let report = parse_go_random_admissible_walk_report(&contents)?;
            println!(
                "validated Go random-admissible-walk validity report for {}/{} passed walks",
                report.validity_passed_count, report.walk_count
            );
        }
        _ => {
            let report: GoTokenizerGeneratorReport =
                parse_go_tokenizer_generator_report(&contents)?;
            println!(
                "validated Go tokenizer-generator report for {} raw tokens from {}",
                report.raw_token_count, report.source_path
            );
        }
    }
    Ok(())
}

fn write_json<T>(path: impl AsRef<Path>, value: &T) -> Result<(), Box<dyn Error>>
where
    T: serde::Serialize,
{
    let path = path.as_ref();
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

fn output_path_for_mode(
    matches: &clap::ArgMatches,
    default_path: impl AsRef<Path>,
) -> Result<PathBuf, Box<dyn Error>> {
    if matches.value_source("output") == Some(ValueSource::DefaultValue) {
        return Ok(default_path.as_ref().to_path_buf());
    }
    Ok(required_path(matches, "output")?.clone())
}
