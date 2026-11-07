//! Validation server for fast batch workflow validation
//!
//! This binary compiles the github-workflow schema ONCE at startup,
//! then validates workflows via stdin interface for maximum efficiency.
//!
//! Protocol:
//! - Input: One JSON file path per line on stdin
//! - Output: `OK <path>` or `ERROR <path> <message>` on stdout
//! - Exit: Send empty line or close stdin
//!
//! Usage:
//! ```bash
//! tct-validate-server --schema schemas/popular/github-workflow.json
//! # Then feed paths via stdin:
//! echo "workflow1.json" | tct-validate-server ...
//! ```

use clap::{Arg, ArgMatches, Command};
use std::error::Error;
use std::fs;
use std::io::{self, BufRead};
use std::path::PathBuf;
use tct::TctCompiler;
use tct::instance_encoder::InstanceEncoder;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct-validate-server")
        .about("Fast batch workflow validator (compiles schema once)")
        .arg(
            Arg::new("schema")
                .short('s')
                .long("schema")
                .help("JSON schema file")
                .required(true),
        )
        .get_matches();

    let schema_path = required_arg::<String>(&matches, "schema")?;

    eprintln!("🚀 Loading and compiling schema: {}", schema_path);

    // Compile schema ONCE
    let schema_content = fs::read_to_string(schema_path)?;
    let schema: serde_json::Value = serde_json::from_str(&schema_content)?;

    let compiler = TctCompiler::new(schema, "GitHubWorkflow")?
        .with_optimization_level(tct::OptimizationLevel::O2);

    let grammar = compiler.compile()?;
    let code = grammar.generate_code()?;

    eprintln!("✅ Schema compiled successfully");

    // Create encoder (this is reusable!)
    let encoder = InstanceEncoder::new(&code.to_string(), &grammar.root_type_name)?;

    eprintln!("📥 Ready to validate workflows (send paths via stdin, empty line to exit)");
    eprintln!();

    // Read workflow paths from stdin and validate
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let path = line.trim();

        if path.is_empty() {
            break;
        }

        // Validate this workflow
        match validate_workflow(&encoder, path) {
            Ok(()) => {
                println!("OK {}", path);
            }
            Err(e) => {
                println!("ERROR {} {}", path, e);
            }
        }
    }

    eprintln!("👋 Validation complete");
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

fn validate_workflow(encoder: &InstanceEncoder, path: &str) -> Result<(), String> {
    // Try to encode the workflow
    let path_buf = PathBuf::from(path);
    let tokens = encoder
        .encode_batch(&[path_buf])
        .map_err(|e| format!("{}", e))?;

    // Verify we actually got tokens (not just a successful but empty encoding)
    if tokens.is_empty() || tokens[0].is_empty() {
        return Err("Encoding produced no tokens (validation failure)".to_string());
    }

    Ok(())
}
