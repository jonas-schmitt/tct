//! Assemble a pure-Python multi-schema TCT bundle wrapper package.
//!
//! The wrapper imports already generated schema-specific Python wheel modules
//! and applies the shared sequence bundle contract around their raw
//! schema-local encode/decode APIs.

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use serde::Deserialize;
use std::error::Error;
use std::fs;
use tct::wheel_builder::{BundleWheelConfig, BundleWheelSchemaConfig};

#[derive(Debug, Deserialize)]
struct BundleWrapperSpec {
    package_name: String,
    version: Option<String>,
    description: Option<String>,
    output_dir: Option<String>,
    vocab_size: Option<u32>,
    schemas: Vec<BundleWrapperSchemaSpec>,
}

#[derive(Debug, Deserialize)]
struct BundleWrapperSchemaSpec {
    schema_id: String,
    module_name: String,
    schema_token: u32,
    raw_vocab_size: u32,
    raw_pad_token: u32,
    global_payload_start: u32,
    bpe_enabled: bool,
    schema_sentinel: Option<String>,
    generated_artifact: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-build-bundle-wrapper")
        .about("Assemble multi-schema TCT bundle wrapper packages")
        .long_about(
            "Assemble a pure-Python package that imports existing generated \
             schema tokenizer wheels and exposes bundle-level sequence APIs. \
             This command writes package source; build the wheel artifact with \
             standard Python packaging tools from the returned directory.",
        )
        .arg(
            Arg::new("spec")
                .long("spec")
                .help("JSON bundle wrapper specification")
                .value_parser(value_parser!(String))
                .required(true),
        )
        .arg(
            Arg::new("output-dir")
                .short('o')
                .long("output-dir")
                .help("Override output directory from the spec")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .help("Suppress informational stderr output")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let spec_path = required_arg::<String>(&matches, "spec")?;
    let output_dir = matches.get_one::<String>("output-dir");
    let quiet = matches.get_flag("quiet");

    let spec_json = fs::read_to_string(spec_path)
        .map_err(|error| format!("failed to read bundle wrapper spec {spec_path}: {error}"))?;
    let spec: BundleWrapperSpec = serde_json::from_str(&spec_json)
        .map_err(|error| format!("failed to parse bundle wrapper spec {spec_path}: {error}"))?;
    let config = spec.into_config(output_dir, quiet);
    let package_dir = tct::wheel_builder::write_bundle_wrapper_package(&config)?;

    println!("{}", package_dir.display());
    Ok(())
}

fn required_arg<'a, T>(matches: &'a ArgMatches, name: &str) -> Result<&'a T, Box<dyn Error>>
where
    T: Clone + Send + Sync + 'static,
{
    matches.get_one::<T>(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("missing required or defaulted CLI argument {name:?}"),
        )
        .into()
    })
}

impl BundleWrapperSpec {
    fn into_config(self, output_dir: Option<&String>, quiet: bool) -> BundleWheelConfig {
        let schemas = self
            .schemas
            .into_iter()
            .map(BundleWrapperSchemaSpec::into_config)
            .collect();
        let mut config = BundleWheelConfig::new(self.package_name, schemas).with_quiet(quiet);
        if let Some(version) = self.version {
            config = config.with_version(version);
        }
        if let Some(description) = self.description {
            config = config.with_description(description);
        }
        if let Some(vocab_size) = self.vocab_size {
            config = config.with_vocab_size(vocab_size);
        }
        if let Some(output_dir) = output_dir.or(self.output_dir.as_ref()) {
            config = config.with_output_dir(output_dir);
        }
        config
    }
}

impl BundleWrapperSchemaSpec {
    fn into_config(self) -> BundleWheelSchemaConfig {
        let mut config = BundleWheelSchemaConfig::new(
            self.schema_id,
            self.module_name,
            self.schema_token,
            self.raw_vocab_size,
            self.raw_pad_token,
            self.global_payload_start,
            self.bpe_enabled,
        );
        if let Some(sentinel) = self.schema_sentinel {
            config = config.with_schema_sentinel(sentinel);
        }
        if let Some(artifact) = self.generated_artifact {
            config = config.with_generated_artifact(artifact);
        }
        config
    }
}
