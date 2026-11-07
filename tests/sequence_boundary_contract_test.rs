use std::fs;
use std::process::Command;

use serde_json::json;
use tct::TctCompiler;
use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::sequence_bundle::{
    SequenceBundleManifest, SequenceBundleManifestError, SequenceBundleSchema,
};

fn generated_sequence_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

#[test]
fn multi_schema_sequence_bundle_routes_payloads_and_masks_by_schema() {
    let manifest = SequenceBundleManifest::new(
        34,
        vec![
            SequenceBundleSchema::new("alpha", 3, 6, 5, 10, false)
                .with_schema_sentinel("<|schema:alpha|>"),
            SequenceBundleSchema::new("beta", 4, 8, 7, 20, true)
                .with_schema_sentinel("<|schema:beta|>"),
        ],
    );

    manifest.validate().unwrap();
    assert_eq!(manifest.manifest_sha256().unwrap().len(), 64);

    let alpha = manifest
        .wrap_payload_for_schema_id("alpha", &[0, 4])
        .unwrap();
    let beta = manifest.wrap_payload_for_schema_token(4, &[0, 6]).unwrap();
    assert_eq!(alpha, vec![1, 3, 10, 14, 2]);
    assert_eq!(beta, vec![1, 4, 20, 26, 2]);

    let mut padded_alpha = alpha.clone();
    padded_alpha.extend([0, 0]);
    let alpha_payload = manifest.unwrap_payload(&padded_alpha).unwrap();
    assert_eq!(alpha_payload.schema_id, "alpha");
    assert_eq!(alpha_payload.schema_token, 3);
    assert_eq!(alpha_payload.local_payload, vec![0, 4]);
    assert_eq!(alpha_payload.global_payload, vec![10, 14]);
    assert_eq!(alpha_payload.consumed_tokens, alpha.len());
    assert_eq!(alpha_payload.total_tokens, padded_alpha.len());

    assert!(matches!(
        manifest.wrap_payload_for_schema_id("alpha", &[5]),
        Err(SequenceBundleManifestError::InvalidLocalPayloadToken { .. })
    ));
    assert!(matches!(
        manifest.unwrap_payload(&[1, 4, 10, 2]),
        Err(SequenceBundleManifestError::InvalidGlobalPayloadToken { .. })
    ));
    assert!(matches!(
        manifest.unwrap_payload(&[1, 3, 10, 2, 1]),
        Err(SequenceBundleManifestError::NonPaddingAfterEos { .. })
    ));
    assert!(matches!(
        manifest.generation_mask_for_schema_token(99),
        Err(SequenceBundleManifestError::UnknownSchemaToken { .. })
    ));

    let alpha_mask = manifest.generation_mask_for_schema_token(3).unwrap();
    let beta_mask = manifest.generation_mask_for_schema_token(4).unwrap();
    assert!(alpha_mask.allows(10));
    assert!(alpha_mask.allows(14));
    assert!(alpha_mask.allows(2));
    assert!(!alpha_mask.allows(15));
    assert!(!alpha_mask.allows(20));
    assert!(!alpha_mask.allows(3));
    assert!(beta_mask.allows(20));
    assert!(beta_mask.allows(26));
    assert!(beta_mask.allows(2));
    assert!(!beta_mask.allows(10));

    let metadata = serde_json::to_value(&manifest).unwrap();
    assert_eq!(
        metadata["schemas"][0]["schema_sentinel"],
        "<|schema:alpha|>"
    );
    assert_eq!(metadata["schemas"][1]["bpe_enabled"], true);
    assert_eq!(metadata["masking"]["mask_eos_token_from_labels"], false);
}

#[test]
fn generated_sequence_wrapper_offsets_payload_and_preserves_raw_vocab() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "required": ["name"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "Person")
        .unwrap()
        .compile()
        .expect("sequence boundary contract schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("sequence boundary contract schema should generate code");

    assert!(generated_code.contains("pub const SEQUENCE_BOUNDARY_VERSION"));
    assert!(generated_code.contains("pub const SEQUENCE_PAYLOAD_OFFSET"));
    assert!(generated_code.contains("pub mod sequence"));

    run_generated_sequence_contract_crate(
        "sequence-boundary-contract",
        &generated_code,
        r#"
let instance: Person = serde_json::from_value(json!({"name": "alice"}))
    .map_err(|error| format!("instance failed to deserialize: {}", error))?;

if VOCAB_SIZE != PAD_TOKEN + 1 {
    return Err(format!(
        "raw VOCAB_SIZE should remain raw PAD + 1, got VOCAB_SIZE={} PAD_TOKEN={}",
        VOCAB_SIZE, PAD_TOKEN
    ));
}
if SEQUENCE_BOUNDARY_VERSION != 1 {
    return Err(format!("unexpected sequence boundary version {}", SEQUENCE_BOUNDARY_VERSION));
}
if SEQUENCE_PAD_TOKEN != 0 || SEQUENCE_BOS_TOKEN != 1 || SEQUENCE_EOS_TOKEN != 2 {
    return Err(format!(
        "unexpected sequence specials pad={} bos={} eos={}",
        SEQUENCE_PAD_TOKEN, SEQUENCE_BOS_TOKEN, SEQUENCE_EOS_TOKEN
    ));
}
if SEQUENCE_SCHEMA_TOKEN != 3 || SEQUENCE_PAYLOAD_OFFSET != 4 {
    return Err(format!(
        "unexpected schema/offset schema={} offset={}",
        SEQUENCE_SCHEMA_TOKEN, SEQUENCE_PAYLOAD_OFFSET
    ));
}
if SEQUENCE_SCHEMA_ID != "Person" {
    return Err(format!("unexpected sequence schema id {}", SEQUENCE_SCHEMA_ID));
}
if SEQUENCE_VOCAB_SIZE != SEQUENCE_PAYLOAD_OFFSET + PAD_TOKEN {
    return Err(format!(
        "SEQUENCE_VOCAB_SIZE {} should equal offset {} + raw content size {}",
        SEQUENCE_VOCAB_SIZE, SEQUENCE_PAYLOAD_OFFSET, PAD_TOKEN
    ));
}

let metadata = sequence::metadata();
if metadata.artifact_kind != "tct.sequence_boundary.single_schema"
    || metadata.artifact_version != 1
    || metadata.metadata_version != 1
    || metadata.version != SEQUENCE_BOUNDARY_VERSION
    || metadata.schema_id != SEQUENCE_SCHEMA_ID
    || metadata.schema_token != SEQUENCE_SCHEMA_TOKEN
    || metadata.raw_base_vocab_size != BASE_VOCAB_SIZE
    || metadata.raw_base_tokens.start != 0
    || metadata.raw_base_tokens.end_exclusive != BASE_VOCAB_SIZE
    || metadata.raw_bpe_tokens.start != BASE_VOCAB_SIZE
    || metadata.raw_bpe_tokens.end_exclusive != PAD_TOKEN
    || metadata.raw_bpe_merge_count != PAD_TOKEN - BASE_VOCAB_SIZE
    || metadata.raw_vocab_size != VOCAB_SIZE
    || metadata.raw_pad_token != PAD_TOKEN
    || metadata.raw_payload_start != 0
    || metadata.raw_payload_end_exclusive != PAD_TOKEN
    || metadata.payload_offset != SEQUENCE_PAYLOAD_OFFSET
    || metadata.sequence_vocab_size != SEQUENCE_VOCAB_SIZE
    || metadata.sequence_pad_token != SEQUENCE_PAD_TOKEN
    || metadata.sequence_bos_token != SEQUENCE_BOS_TOKEN
    || metadata.sequence_eos_token != SEQUENCE_EOS_TOKEN
    || metadata.sequence_start_token != SEQUENCE_START_TOKEN
    || metadata.sequence_end_token != SEQUENCE_END_TOKEN
    || metadata.sequence_reserved_tokens.reserved_start != SEQUENCE_PAD_TOKEN
    || metadata.sequence_reserved_tokens.reserved_end_exclusive != SEQUENCE_PAYLOAD_OFFSET
    || metadata.sequence_reserved_tokens.pad_token != SEQUENCE_PAD_TOKEN
    || metadata.sequence_reserved_tokens.bos_token != SEQUENCE_BOS_TOKEN
    || metadata.sequence_reserved_tokens.eos_token != SEQUENCE_EOS_TOKEN
    || metadata.sequence_reserved_tokens.schema_token != SEQUENCE_SCHEMA_TOKEN
    || metadata.decode_prefix_supported
    || metadata.bpe_enabled
    || metadata.bpe.enabled
    || metadata.bpe.schema.is_some()
    || metadata.bpe.merge_count != 0
    || metadata.bpe.first_new_token.is_some()
    || metadata.bpe.last_new_token.is_some()
{
    return Err(format!("unexpected sequence metadata: {:?}", metadata));
}
let metadata_json = serde_json::to_value(&metadata)
    .map_err(|error| format!("metadata JSON serialization failed: {}", error))?;
if metadata_json["schema_id"] != "Person"
    || metadata_json["artifact_kind"] != "tct.sequence_boundary.single_schema"
    || metadata_json["payload_offset"] != SEQUENCE_PAYLOAD_OFFSET
    || metadata_json["raw_base_tokens"]["end_exclusive"] != BASE_VOCAB_SIZE
    || metadata_json["raw_bpe_tokens"]["start"] != BASE_VOCAB_SIZE
    || metadata_json["sequence_reserved_tokens"]["schema_token"] != SEQUENCE_SCHEMA_TOKEN
    || metadata_json["decode_prefix_supported"] != false
    || metadata_json["bpe_enabled"] != false
    || metadata_json["bpe"]["enabled"] != false
{
    return Err(format!("unexpected sequence metadata JSON: {}", metadata_json));
}

let mut raw_tokens = Vec::new();
instance.encode(&mut raw_tokens);
if raw_tokens.is_empty() {
    return Err("raw encode unexpectedly produced no tokens".to_string());
}
if raw_tokens.iter().any(|token| *token >= PAD_TOKEN) {
    return Err(format!("raw payload contains reserved token in {:?}", raw_tokens));
}

let wrapped = sequence::encode(&instance);
if wrapped.first().copied() != Some(SEQUENCE_BOS_TOKEN) {
    return Err(format!("wrapped sequence missing BOS: {:?}", wrapped));
}
if wrapped.get(1).copied() != Some(SEQUENCE_SCHEMA_TOKEN) {
    return Err(format!("wrapped sequence missing schema token: {:?}", wrapped));
}
if wrapped.last().copied() != Some(SEQUENCE_EOS_TOKEN) {
    return Err(format!("wrapped sequence missing EOS: {:?}", wrapped));
}

let mapped_payload: Vec<u32> = wrapped[2..wrapped.len() - 1]
    .iter()
    .map(|token| token - SEQUENCE_PAYLOAD_OFFSET)
    .collect();
if mapped_payload != raw_tokens {
    return Err(format!(
        "wrapped payload should be offset raw tokens, got {:?}, expected {:?}",
        mapped_payload, raw_tokens
    ));
}
if wrapped[2..wrapped.len() - 1]
    .iter()
    .any(|token| *token < SEQUENCE_PAYLOAD_OFFSET || *token >= SEQUENCE_VOCAB_SIZE)
{
    return Err(format!("wrapped payload outside sequence vocab: {:?}", wrapped));
}

let (decoded, consumed, total) = sequence::decode::<Person>(&wrapped)
    .map_err(|error| format!("sequence decode failed: {}", error))?;
if consumed != wrapped.len() || total != wrapped.len() {
    return Err(format!("decode consumed/total mismatch: {}/{}", consumed, total));
}
let decoded_json = serde_json::to_value(decoded)
    .map_err(|error| format!("decoded JSON serialization failed: {}", error))?;
if decoded_json != json!({"name": "alice"}) {
    return Err(format!("decoded JSON mismatch: {}", decoded_json));
}

let mut padded = wrapped.clone();
padded.push(SEQUENCE_PAD_TOKEN);
padded.push(SEQUENCE_PAD_TOKEN);
let (_, padded_consumed, padded_total) = sequence::decode::<Person>(&padded)
    .map_err(|error| format!("padded sequence decode failed: {}", error))?;
if padded_consumed != wrapped.len() || padded_total != padded.len() {
    return Err(format!(
        "padded decode consumed/total mismatch: {}/{} for len {}",
        padded_consumed,
        padded_total,
        padded.len()
    ));
}

let mut missing_bos = wrapped.clone();
missing_bos[0] = SEQUENCE_PAD_TOKEN;
if sequence::decode::<Person>(&missing_bos).is_ok() {
    return Err("missing BOS should fail".to_string());
}

let mut bad_schema = wrapped.clone();
bad_schema[1] = SEQUENCE_BOS_TOKEN;
if sequence::decode::<Person>(&bad_schema).is_ok() {
    return Err("bad schema token should fail".to_string());
}

let missing_eos = wrapped[..wrapped.len() - 1].to_vec();
if sequence::decode::<Person>(&missing_eos).is_ok() {
    return Err("missing EOS should fail".to_string());
}

let mut pad_before_eos = wrapped.clone();
pad_before_eos.insert(2, SEQUENCE_PAD_TOKEN);
if sequence::decode::<Person>(&pad_before_eos).is_ok() {
    return Err("sequence PAD before EOS should fail".to_string());
}

let mut non_pad_after_eos = wrapped.clone();
non_pad_after_eos.push(SEQUENCE_BOS_TOKEN);
if sequence::decode::<Person>(&non_pad_after_eos).is_ok() {
    return Err("non-PAD token after EOS should fail".to_string());
}

let mut raw_pad_payload = wrapped.clone();
raw_pad_payload.insert(raw_pad_payload.len() - 1, SEQUENCE_PAYLOAD_OFFSET + PAD_TOKEN);
if sequence::decode::<Person>(&raw_pad_payload).is_ok() {
    return Err("mapped raw PAD token in payload should fail".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn generated_sequence_wrapper_wraps_bpe_payload_after_compression() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(257, 97, 98);

    let grammar = TctCompiler::new(schema, "RootString")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("BPE merge file should load")
        .compile()
        .expect("BPE sequence boundary schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("BPE sequence boundary schema should generate code");

    run_generated_sequence_contract_crate(
        "sequence-boundary-bpe-contract",
        &generated_code,
        r#"
let input = String::from("ab");
let metadata = sequence::metadata();
if metadata.schema_id != "RootString"
    || !metadata.bpe_enabled
    || !metadata.bpe.enabled
    || metadata.raw_bpe_merge_count != 1
    || metadata.raw_bpe_tokens.start != BASE_VOCAB_SIZE
    || metadata.raw_bpe_tokens.end_exclusive != PAD_TOKEN
    || metadata.bpe.schema != Some("sequence-boundary")
    || metadata.bpe.merge_count != 1
    || metadata.bpe.first_new_token != Some(BASE_VOCAB_SIZE)
    || metadata.bpe.last_new_token != Some(BASE_VOCAB_SIZE)
    || metadata.bpe.metadata_trained_on_instances != Some(1)
    || metadata.bpe.metadata_max_merges != Some(1)
{
    return Err(format!("unexpected BPE sequence metadata: {:?}", metadata));
}

let raw_bpe_tokens = codec::encode(&input);
if !raw_bpe_tokens.contains(&BASE_VOCAB_SIZE) {
    return Err(format!("expected BPE merge token {} in {:?}", BASE_VOCAB_SIZE, raw_bpe_tokens));
}
if raw_bpe_tokens.iter().any(|token| *token >= PAD_TOKEN) {
    return Err(format!("BPE payload contains reserved token in {:?}", raw_bpe_tokens));
}

let wrapped = sequence::encode(&input);
let mapped_payload: Vec<u32> = wrapped[2..wrapped.len() - 1]
    .iter()
    .map(|token| token - SEQUENCE_PAYLOAD_OFFSET)
    .collect();
if mapped_payload != raw_bpe_tokens {
    return Err(format!(
        "sequence payload should wrap compressed BPE tokens, got {:?}, expected {:?}",
        mapped_payload, raw_bpe_tokens
    ));
}

let (decoded, consumed, total) = sequence::decode::<String>(&wrapped)
    .map_err(|error| format!("sequence BPE decode failed: {}", error))?;
if decoded != input {
    return Err(format!("decoded {:?}, expected {:?}", decoded, input));
}
if consumed != wrapped.len() || total != wrapped.len() {
    return Err(format!("BPE decode consumed/total mismatch: {}/{}", consumed, total));
}

Ok(())
"#,
    );
}

fn write_bpe_merge_file(base_vocab_size: u32, left: u32, right: u32) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().expect("failed to create temp BPE file");
    let table = BpeMergeTable {
        schema: "sequence-boundary".to_string(),
        base_vocab_size,
        merges: vec![BpeMerge {
            pair: [left, right],
            new_token: base_vocab_size,
            frequency: 10,
        }],
        metadata: BpeMetadata {
            trained_on_instances: 1,
            training_date: "2026-01-01".to_string(),
            max_merges: 1,
            dict_size: None,
        },
    };
    let json = serde_json::to_string_pretty(&table).expect("failed to serialize BPE table");
    fs::write(file.path(), json).expect("failed to write temp BPE file");
    file
}

fn run_generated_sequence_contract_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated sequence-boundary crate");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;
use serde_json::json;

{generated_code}

fn main() -> Result<(), String> {{
{body}
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let target_dir = std::env::current_dir()
        .expect("failed to read current dir")
        .join("target/tct-generated-cargo/sequence-boundary-contracts");
    let output = generated_sequence_cargo_command()
        .arg("run")
        .arg("--quiet")
        .env("CARGO_TARGET_DIR", target_dir)
        .current_dir(crate_dir.path())
        .output()
        .unwrap_or_else(|error| panic!("failed to run generated crate {}: {}", crate_name, error));

    assert!(
        output.status.success(),
        "generated crate {} failed\nstdout:\n{}\nstderr:\n{}",
        crate_name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
