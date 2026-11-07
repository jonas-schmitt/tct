//! Validate serialized direct HF/TCT compatibility profile payloads.

use clap::{Arg, Command, value_parser};
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::process;
use tct::provenance::{FileProvenance, file_provenance};
use tct::sequence_bundle::{
    DirectHfTctCompatibilityProfile, DirectHfTctMaskingPolicy, SequenceGenerationMask, TokenRange,
};

#[derive(Debug, Serialize)]
struct DirectHfProfileValidationReport<'a> {
    status: &'static str,
    command: Vec<String>,
    created_at: String,
    tct_version: &'static str,
    profile_file: FileProvenance,
    profile_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_sha256: Option<&'a str>,
    profile: DirectHfProfileSummary<'a>,
}

#[derive(Debug, Serialize)]
struct DirectHfProfileSummary<'a> {
    profile_name: &'a str,
    profile_version: u32,
    handoff_shape: &'a str,
    schema_id: &'a str,
    schema_sentinel: &'a str,
    schema_sentinel_token: u32,
    tct_start_sentinel: &'a str,
    tct_start_token: u32,
    tct_end_sentinel: &'a str,
    tct_end_token: u32,
    tct_end_role: &'a str,
    local_payload: &'a TokenRange,
    hf_payload: &'a TokenRange,
    raw_pad_token: u32,
    raw_pad_hf_token: u32,
    raw_pad_is_payload: bool,
    masking: &'a DirectHfTctMaskingPolicy,
    generation_mask: &'a SequenceGenerationMask,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let command: Vec<String> = std::env::args().collect();
    let matches = Command::new("tct-validate-direct-hf-profile")
        .about("Validate serialized direct HF/TCT compatibility profiles")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("profile")
                .long("profile")
                .help("Serialized DirectHfTctCompatibilityProfile JSON payload")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("expected-sha256")
                .long("expected-sha256")
                .help("Optional expected DirectHfTctCompatibilityProfile::profile_sha256 value")
                .value_parser(value_parser!(String)),
        )
        .get_matches();

    let profile_path = matches.get_one::<String>("profile").ok_or_else(|| {
        IoError::new(
            ErrorKind::InvalidInput,
            "missing required --profile argument",
        )
    })?;
    let expected_sha256 = matches.get_one::<String>("expected-sha256");
    if let Some(expected) = expected_sha256
        && !is_sha256_hex(expected)
    {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            format!(
                "--expected-sha256 must be 64 lowercase/uppercase hex characters; got {expected}"
            ),
        )
        .into());
    }

    let profile_bytes = fs::read(profile_path)?;
    let profile: DirectHfTctCompatibilityProfile = serde_json::from_slice(&profile_bytes)?;
    profile.validate()?;
    let profile_sha256 = profile.profile_sha256()?;
    if let Some(expected) = expected_sha256
        && profile_sha256 != *expected
    {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            format!(
                "direct HF/TCT profile SHA-256 mismatch: expected {expected}, got {profile_sha256}"
            ),
        )
        .into());
    }

    let report = DirectHfProfileValidationReport {
        status: "validated",
        command,
        created_at: chrono::Utc::now().to_rfc3339(),
        tct_version: env!("CARGO_PKG_VERSION"),
        profile_file: file_provenance(profile_path)?,
        profile_sha256,
        expected_sha256: expected_sha256.map(String::as_str),
        profile: DirectHfProfileSummary {
            profile_name: &profile.profile_name,
            profile_version: profile.profile_version,
            handoff_shape: &profile.handoff_shape,
            schema_id: &profile.schema_id,
            schema_sentinel: &profile.schema_sentinel,
            schema_sentinel_token: profile.schema_sentinel_token,
            tct_start_sentinel: &profile.tct_start_sentinel,
            tct_start_token: profile.tct_start_token,
            tct_end_sentinel: &profile.tct_end_sentinel,
            tct_end_token: profile.tct_end_token,
            tct_end_role: &profile.tct_end_role,
            local_payload: &profile.local_payload,
            hf_payload: &profile.hf_payload,
            raw_pad_token: profile.raw_pad_token,
            raw_pad_hf_token: profile.raw_pad_hf_token,
            raw_pad_is_payload: profile.raw_pad_is_payload,
            masking: &profile.masking,
            generation_mask: &profile.generation_mask,
        },
    };
    println!("{}", serde_json::to_string_pretty(&report)?);

    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}
