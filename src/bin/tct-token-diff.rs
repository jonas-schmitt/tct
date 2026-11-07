//! Produce token-ID layout diff manifests for old/new schema pairs.

use clap::{Arg, ArgMatches, Command, value_parser};
use serde::Serialize;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;
use tct::bpe::BpeMergeTable;
use tct::provenance::{FileProvenance, file_provenance};
use tct::schema_evolution::{
    BpeArtifactLayout, OptimizationPassOverrides, TokenDiffExperimentGate, TokenIdDiffManifest,
    TokenIdLayout, TokenIdLayoutCompileOptions, compile_token_id_layout_with_options,
    diff_token_id_layouts, summarize_bpe_artifact_layout, token_diff_experiment_gate,
};
use tct::{StringEncoding, TctError};

#[derive(Debug, Serialize)]
struct TokenDiffReport {
    status: TokenDiffReportStatus,
    experiment_gate: TokenDiffExperimentGate,
    command: Vec<String>,
    old_schema: String,
    new_schema: String,
    old_schema_file: FileProvenance,
    new_schema_file: FileProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<TokenIdDiffManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partial: Option<TokenDiffPartialReport>,
    created_at: String,
    tct_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum TokenDiffReportStatus {
    Complete,
    PartialError,
}

#[derive(Debug, Serialize)]
struct TokenDiffPartialReport {
    old: TokenDiffSideEvidence,
    new: TokenDiffSideEvidence,
}

#[derive(Debug, Serialize)]
struct TokenDiffSideEvidence {
    side: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout: Option<TokenIdLayout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout_without_bpe: Option<TokenIdLayout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bpe_artifact: Option<BpeArtifactLayout>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<TokenDiffEvidenceError>,
}

#[derive(Debug, Serialize)]
struct TokenDiffEvidenceError {
    phase: &'static str,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
    message: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let command: Vec<String> = std::env::args().collect();
    let matches = Command::new("tct-token-diff")
        .about("Produce token-ID layout diff manifests for schema evolution")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("old-schema")
                .long("old-schema")
                .help("Old JSON schema file")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("new-schema")
                .long("new-schema")
                .help("New JSON schema file")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .help("Root type name to use for both schemas")
                .default_value("RootType")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("old-name")
                .long("old-name")
                .help("Root type name for the old schema; overrides --name")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("new-name")
                .long("new-name")
                .help("Root type name for the new schema; overrides --name")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("optimization")
                .short('O')
                .long("optimization")
                .help("Optimization profile: O0, O2, O3, Os, or OVocab")
                .default_value("O3")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("enum-optimization")
                .long("enum-optimization")
                .help("Enable enum optimization override for both schemas")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("old-enum-optimization")
                .long("old-enum-optimization")
                .help("Enable enum optimization override for the old schema")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("new-enum-optimization")
                .long("new-enum-optimization")
                .help("Enable enum optimization override for the new schema")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("bounded-int-optimization")
                .long("bounded-int-optimization")
                .help("Enable bounded-integer optimization override for both schemas")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("old-bounded-int-optimization")
                .long("old-bounded-int-optimization")
                .help("Enable bounded-integer optimization override for the old schema")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("new-bounded-int-optimization")
                .long("new-bounded-int-optimization")
                .help("Enable bounded-integer optimization override for the new schema")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("pattern-optimization")
                .long("pattern-optimization")
                .help("Reserved legacy flag for pattern optimization")
                .hide(true)
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("old-pattern-optimization")
                .long("old-pattern-optimization")
                .help("Reserved legacy flag for old-schema pattern optimization")
                .hide(true)
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("new-pattern-optimization")
                .long("new-pattern-optimization")
                .help("Reserved legacy flag for new-schema pattern optimization")
                .hide(true)
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("bit-packing")
                .long("bit-packing")
                .help("Enable bit-packing override for both schemas")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("old-bit-packing")
                .long("old-bit-packing")
                .help("Enable bit-packing override for the old schema")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("new-bit-packing")
                .long("new-bit-packing")
                .help("Enable bit-packing override for the new schema")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("string-encoding")
                .long("string-encoding")
                .help("String encoding for both schemas: utf8 or ascii")
                .default_value("utf8")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("old-string-encoding")
                .long("old-string-encoding")
                .help("String encoding for the old schema; overrides --string-encoding")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("new-string-encoding")
                .long("new-string-encoding")
                .help("String encoding for the new schema; overrides --string-encoding")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("max-base-vocab")
                .long("max-base-vocab")
                .help("Maximum base vocabulary for both schemas")
                .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new("old-max-base-vocab")
                .long("old-max-base-vocab")
                .help("Maximum base vocabulary for the old schema; overrides --max-base-vocab")
                .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new("new-max-base-vocab")
                .long("new-max-base-vocab")
                .help("Maximum base vocabulary for the new schema; overrides --max-base-vocab")
                .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new("old-bpe-merges")
                .long("old-bpe-merges")
                .help("BPE merge file for the old schema")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("new-bpe-merges")
                .long("new-bpe-merges")
                .help("BPE merge file for the new schema")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Write JSON report to this file instead of stdout")
                .value_parser(value_parser!(String)),
        )
        .get_matches();

    let old_schema_path = required_string_arg(&matches, "old-schema")?;
    let new_schema_path = required_string_arg(&matches, "new-schema")?;
    let default_name = required_string_arg(&matches, "name")?;
    let old_name = matches
        .get_one::<String>("old-name")
        .unwrap_or(default_name);
    let new_name = matches
        .get_one::<String>("new-name")
        .unwrap_or(default_name);
    let optimization = required_string_arg(&matches, "optimization")?;
    let optimization_level = tct::OptimizationLevel::parse_cli(optimization)
        .map_err(|error| format!("invalid --optimization: {error}"))?;
    let shared_pass_overrides = pass_overrides_from_matches(&matches, "")?;
    let old_pass_overrides =
        shared_pass_overrides.union(pass_overrides_from_matches(&matches, "old-")?);
    let new_pass_overrides =
        shared_pass_overrides.union(pass_overrides_from_matches(&matches, "new-")?);
    let default_string_encoding =
        parse_string_encoding(required_string_arg(&matches, "string-encoding")?)?;
    let old_string_encoding = if let Some(value) = matches.get_one::<String>("old-string-encoding")
    {
        parse_string_encoding(value)?
    } else {
        default_string_encoding
    };
    let new_string_encoding = if let Some(value) = matches.get_one::<String>("new-string-encoding")
    {
        parse_string_encoding(value)?
    } else {
        default_string_encoding
    };
    let default_max_base_vocab = matches.get_one::<u32>("max-base-vocab").copied();
    let old_max_base_vocab = matches
        .get_one::<u32>("old-max-base-vocab")
        .copied()
        .or(default_max_base_vocab);
    let new_max_base_vocab = matches
        .get_one::<u32>("new-max-base-vocab")
        .copied()
        .or(default_max_base_vocab);
    let old_bpe_path = matches.get_one::<String>("old-bpe-merges").map(Path::new);
    let new_bpe_path = matches.get_one::<String>("new-bpe-merges").map(Path::new);

    let old_schema = load_schema(old_schema_path)?;
    let new_schema = load_schema(new_schema_path)?;
    let old_options = TokenIdLayoutCompileOptions::new(optimization_level)
        .with_optimization_pass_overrides(old_pass_overrides)
        .with_bpe_merges_path(old_bpe_path)
        .with_max_base_vocab(old_max_base_vocab)
        .with_string_encoding(old_string_encoding);
    let new_options = TokenIdLayoutCompileOptions::new(optimization_level)
        .with_optimization_pass_overrides(new_pass_overrides)
        .with_bpe_merges_path(new_bpe_path)
        .with_max_base_vocab(new_max_base_vocab)
        .with_string_encoding(new_string_encoding);
    let old_evidence = compile_side_evidence("old", old_schema, old_name, old_options);
    let new_evidence = compile_side_evidence("new", new_schema, new_name, new_options);
    let manifest = if let (Some(old_layout), Some(new_layout)) =
        (old_evidence.layout.clone(), new_evidence.layout.clone())
    {
        Some(diff_token_id_layouts(old_layout, new_layout))
    } else {
        None
    };
    let status = if manifest.is_some() {
        TokenDiffReportStatus::Complete
    } else {
        TokenDiffReportStatus::PartialError
    };
    let partial = if manifest.is_none() {
        Some(TokenDiffPartialReport {
            old: old_evidence,
            new: new_evidence,
        })
    } else {
        None
    };

    let report = TokenDiffReport {
        status,
        experiment_gate: token_diff_experiment_gate(),
        command,
        old_schema: old_schema_path.clone(),
        new_schema: new_schema_path.clone(),
        old_schema_file: file_provenance(old_schema_path)?,
        new_schema_file: file_provenance(new_schema_path)?,
        manifest,
        partial,
        created_at: chrono::Utc::now().to_rfc3339(),
        tct_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let output = serde_json::to_string_pretty(&report)?;

    if let Some(path) = matches.get_one::<String>("output") {
        fs::write(path, output)?;
    } else {
        println!("{output}");
    }

    if status == TokenDiffReportStatus::PartialError {
        std::process::exit(1);
    }

    Ok(())
}

fn required_string_arg<'a>(
    matches: &'a ArgMatches,
    name: &str,
) -> Result<&'a String, Box<dyn Error>> {
    matches
        .get_one::<String>(name)
        .ok_or_else(|| format!("missing required or defaulted CLI argument: {name}").into())
}

fn load_schema(path: &str) -> Result<Value, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn parse_string_encoding(value: &str) -> Result<StringEncoding, Box<dyn Error>> {
    match value.to_ascii_lowercase().as_str() {
        "ascii" => Ok(StringEncoding::Ascii),
        "utf8" | "utf-8" => Ok(StringEncoding::Utf8),
        _ => Err(format!("invalid string encoding {value:?}; use utf8 or ascii").into()),
    }
}

fn pass_overrides_from_matches(
    matches: &clap::ArgMatches,
    prefix: &str,
) -> Result<OptimizationPassOverrides, Box<dyn Error>> {
    let flag = |name: &str| matches.get_flag(&format!("{prefix}{name}"));

    let mut overrides = OptimizationPassOverrides::default();
    if flag("enum-optimization") {
        overrides = overrides.with_enum_optimization();
    }
    if flag("bounded-int-optimization") {
        overrides = overrides.with_bounded_int_optimization();
    }
    if flag("pattern-optimization") {
        return Err(format!(
            "--{prefix}pattern-optimization is reserved until direct finite-regex encoding is implemented; patternProperties validation is schema-driven"
        )
        .into());
    }
    if flag("bit-packing") {
        overrides = overrides.with_bit_packing();
    }
    Ok(overrides)
}

fn compile_side_evidence(
    side: &'static str,
    schema: Value,
    root_type_name: &str,
    options: TokenIdLayoutCompileOptions<'_>,
) -> TokenDiffSideEvidence {
    match compile_token_id_layout_with_options(schema.clone(), root_type_name, options.clone()) {
        Ok(layout) => TokenDiffSideEvidence {
            side,
            layout: Some(layout),
            layout_without_bpe: None,
            bpe_artifact: None,
            errors: Vec::new(),
        },
        Err(error) => partial_side_evidence(side, schema, root_type_name, options, error),
    }
}

fn partial_side_evidence(
    side: &'static str,
    schema: Value,
    root_type_name: &str,
    options: TokenIdLayoutCompileOptions<'_>,
    error: TctError,
) -> TokenDiffSideEvidence {
    let mut evidence = TokenDiffSideEvidence {
        side,
        layout: None,
        layout_without_bpe: None,
        bpe_artifact: None,
        errors: vec![tct_error_to_evidence("compile", &error)],
    };

    let Some(bpe_path) = options.bpe_merges_path else {
        return evidence;
    };

    let base_options = TokenIdLayoutCompileOptions::new(options.optimization_level)
        .with_optimization_pass_overrides(options.optimization_pass_overrides)
        .with_max_base_vocab(options.max_base_vocab)
        .with_string_encoding(options.string_encoding);
    match compile_token_id_layout_with_options(schema, root_type_name, base_options) {
        Ok(layout_without_bpe) => {
            match BpeMergeTable::from_file(bpe_path) {
                Ok(table) => match summarize_bpe_artifact_layout(
                    bpe_path,
                    &table,
                    layout_without_bpe.base_vocab_size,
                ) {
                    Ok(artifact) => {
                        evidence.bpe_artifact = Some(artifact);
                    }
                    Err(error) => evidence
                        .errors
                        .push(tct_error_to_evidence("bpe_artifact_summary", &error)),
                },
                Err(error) => {
                    evidence.errors.push(TokenDiffEvidenceError {
                        phase: "bpe_artifact_load",
                        kind: "bpe_error",
                        field: None,
                        reason: None,
                        suggestion: None,
                        message: error.to_string(),
                    });
                }
            }
            evidence.layout_without_bpe = Some(layout_without_bpe);
        }
        Err(error) => {
            evidence
                .errors
                .push(tct_error_to_evidence("compile_without_bpe", &error));
        }
    }

    evidence
}

fn tct_error_to_evidence(phase: &'static str, error: &TctError) -> TokenDiffEvidenceError {
    match error {
        TctError::InvalidConfiguration {
            field,
            reason,
            suggestion,
        } => TokenDiffEvidenceError {
            phase,
            kind: "invalid_configuration",
            field: Some(field.clone()),
            reason: Some(reason.clone()),
            suggestion: Some(suggestion.clone()),
            message: error.to_string(),
        },
        _ => TokenDiffEvidenceError {
            phase,
            kind: "tct_error",
            field: None,
            reason: None,
            suggestion: None,
            message: error.to_string(),
        },
    }
}
