use std::fs;
use std::process::Command;

use serde_json::json;
use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
use tct::generated_crate::{
    GENERATED_EXTERNAL_BUILD_ENV, GeneratedCargoToml, configure_generated_cargo_command,
    copy_workspace_cargo_lock,
};
use tct::{OptimizationLevel, TctCompiler};

#[test]
fn bitpacked_optional_struct_token_count_matches_encoded_length() {
    let schema = json!({
        "title": "BitpackedTokenCountContract",
        "type": "object",
        "properties": {
            "field_0": {"type": "integer"},
            "field_1": {"type": "integer"},
            "field_2": {"type": "integer"},
            "field_3": {"type": "integer"},
            "field_4": {"type": "integer"},
            "field_5": {"type": "integer"},
            "field_6": {"type": "integer"},
            "field_7": {"type": "integer"},
            "field_8": {"type": "integer"}
        },
        "additionalProperties": false
    });

    for (optimization_level, crate_name) in [
        (OptimizationLevel::Os, "bitpacked-token-count-os"),
        (OptimizationLevel::OVocab, "bitpacked-token-count-ovocab"),
    ] {
        let grammar = TctCompiler::new(schema.clone(), "BitpackedTokenCountContract")
            .unwrap()
            .with_optimization_level(optimization_level)
            .compile()
            .expect("bitpacked token count contract schema should compile");
        let generated_code = grammar
            .generate_code()
            .expect("bitpacked token count contract schema should generate code");

        assert!(
            generated_code.contains("presence_mask"),
            "{optimization_level} should use bit-packed optional-field encoding"
        );

        run_generated_token_count_contract_crate(
            crate_name,
            &generated_code,
            r#"
fn assert_token_count_matches(
    label: &str,
    value: BitpackedTokenCountContract,
) -> Result<(), String> {
    if VOCAB_SIZE != 258 {
        return Err(format!("{} expected VOCAB_SIZE 258, got {}", label, VOCAB_SIZE));
    }
    if PAD_TOKEN != 257 {
        return Err(format!("{} expected PAD_TOKEN 257, got {}", label, PAD_TOKEN));
    }

    let mut tokens = Vec::new();
    value.encode(&mut tokens);
    let estimated = value.token_count();
    if tokens.len() != estimated {
        return Err(format!(
            "{} token_count mismatch: encode produced {}, token_count returned {}, tokens {:?}",
            label,
            tokens.len(),
            estimated,
            tokens
        ));
    }
    if let Some(max_token) = tokens.iter().max() {
        if *max_token >= VOCAB_SIZE {
            return Err(format!(
                "{} token {} is outside VOCAB_SIZE {} in {:?}",
                label,
                max_token,
                VOCAB_SIZE,
                tokens
            ));
        }
    }

    let decoded = BitpackedTokenCountContract::decode(&mut tokens.clone().into_iter())
        .map_err(|error| format!("{} decode failed: {}", label, error))?;
    let decoded_json = serde_json::to_value(decoded)
        .map_err(|error| format!("{} decoded JSON serialization failed: {}", label, error))?;
    let original_json = serde_json::to_value(value)
        .map_err(|error| format!("{} original JSON serialization failed: {}", label, error))?;
    if decoded_json != original_json {
        return Err(format!(
            "{} decode mismatch: expected {:?}, got {:?}",
            label,
            original_json,
            decoded_json
        ));
    }

    Ok(())
}

let empty: BitpackedTokenCountContract = serde_json::from_value(serde_json::json!({}))
    .map_err(|error| format!("empty instance failed to deserialize: {}", error))?;
assert_token_count_matches("empty", empty)?;

let sparse: BitpackedTokenCountContract = serde_json::from_value(serde_json::json!({
    "field_8": 8
}))
    .map_err(|error| format!("sparse instance failed to deserialize: {}", error))?;
assert_token_count_matches("sparse", sparse)?;

let dense: BitpackedTokenCountContract = serde_json::from_value(serde_json::json!({
    "field_0": 0,
    "field_1": 1,
    "field_2": 2,
    "field_3": 3,
    "field_4": 4,
    "field_5": 5,
    "field_6": 6,
    "field_7": 7,
    "field_8": 8
}))
    .map_err(|error| format!("dense instance failed to deserialize: {}", error))?;
assert_token_count_matches("dense", dense)?;

Ok(())
"#,
        );
    }
}

#[test]
fn bpe_generated_token_layout_keeps_payload_tokens_below_pad() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(
        257,
        vec![
            BpeMerge {
                pair: [97, 98],
                new_token: 257,
                frequency: 10,
            },
            BpeMerge {
                pair: [257, 99],
                new_token: 258,
                frequency: 5,
            },
        ],
    );

    let grammar = TctCompiler::new(schema, "BpeTokenLayout")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("BPE merge table should load")
        .compile()
        .expect("BPE token layout schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("BPE token layout schema should generate code");

    run_generated_token_count_contract_crate(
        "bpe-token-layout-contract",
        &generated_code,
        r#"
if BASE_VOCAB_SIZE != 257 {
    return Err(format!("expected BASE_VOCAB_SIZE 257, got {}", BASE_VOCAB_SIZE));
}
if PAD_TOKEN + 1 != VOCAB_SIZE {
    return Err(format!("expected PAD_TOKEN to be VOCAB_SIZE - 1, got PAD_TOKEN={PAD_TOKEN} VOCAB_SIZE={VOCAB_SIZE}"));
}
if PAD_TOKEN != 259 || VOCAB_SIZE != 260 {
    return Err(format!("expected 2 BPE merges plus PAD to produce PAD_TOKEN=259 and VOCAB_SIZE=260, got {PAD_TOKEN}/{VOCAB_SIZE}"));
}

let input = "abc".to_string();
let base_tokens = codec::encode_no_bpe(&input);
if base_tokens.iter().any(|token| *token >= BASE_VOCAB_SIZE) {
    return Err(format!("base payload token escaped BASE_VOCAB_SIZE {BASE_VOCAB_SIZE}: {base_tokens:?}"));
}

let bpe_tokens = codec::encode(&input);
if bpe_tokens.iter().any(|token| *token >= VOCAB_SIZE) {
    return Err(format!("BPE payload token escaped VOCAB_SIZE {VOCAB_SIZE}: {bpe_tokens:?}"));
}
if bpe_tokens.contains(&PAD_TOKEN) {
    return Err(format!("BPE payload emitted reserved PAD_TOKEN {PAD_TOKEN}: {bpe_tokens:?}"));
}
if !bpe_tokens.iter().any(|token| *token >= BASE_VOCAB_SIZE && *token < PAD_TOKEN) {
    return Err(format!("BPE payload did not emit a token in the BPE range {BASE_VOCAB_SIZE}..{PAD_TOKEN}: {bpe_tokens:?}"));
}

let (decoded, consumed, total) = codec::decode::<String>(bpe_tokens.clone())
    .map_err(|error| format!("BPE decode failed: {error}"))?;
if decoded != input || consumed != total {
    return Err(format!("BPE decode mismatch decoded={decoded:?} consumed={consumed} total={total}"));
}
let (strict_decoded, strict_consumed, strict_total) = codec::decode_strict::<String>(bpe_tokens)
    .map_err(|error| format!("strict BPE decode failed: {error}"))?;
if strict_decoded != input || strict_consumed != strict_total {
    return Err(format!("strict BPE decode mismatch decoded={strict_decoded:?} consumed={strict_consumed} total={strict_total}"));
}

Ok(())
"#,
    );
}

fn write_bpe_merge_file(base_vocab_size: u32, merges: Vec<BpeMerge>) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().expect("failed to create temp BPE file");
    let max_merges = merges.len();
    let table = BpeMergeTable {
        schema: "token-layout".to_string(),
        base_vocab_size,
        merges,
        metadata: BpeMetadata {
            trained_on_instances: 1,
            training_date: "2026-01-01".to_string(),
            max_merges,
            dict_size: None,
        },
    };
    let json = serde_json::to_string_pretty(&table).expect("failed to serialize BPE table");
    fs::write(file.path(), json).expect("failed to write temp BPE file");
    file
}

fn run_generated_token_count_contract_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let tct_path = std::env::current_dir().unwrap();
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated token-count contract crate");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
{body}
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let target_dir = std::env::current_dir()
        .expect("failed to read current dir")
        .join("target/tct-generated-cargo/token-count-contracts");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    let output = command
        .arg("run")
        .arg("--quiet")
        .env("CARGO_TARGET_DIR", target_dir)
        .env(GENERATED_EXTERNAL_BUILD_ENV, "1")
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
