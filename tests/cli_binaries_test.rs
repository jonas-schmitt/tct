//! Fast smoke tests for the maintained Rust CLI binaries.
//!
//! Most tests avoid generated-temp-crate builds and external tools. The focused
//! schema-backed test uses a tiny fixture and a shared generated Cargo target
//! directory so maintained CLI success paths stay visible without turning this
//! suite into a broad generated-crate gate.

use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tct::sequence_bundle::{
    DIRECT_LOCAL_PAYLOAD_HANDOFF_SHAPE, DIRECT_LOCAL_PAYLOAD_PROFILE_NAME,
    DIRECT_LOCAL_PAYLOAD_PROFILE_VERSION, DirectHfTctCompatibilityProfile,
    DirectHfTctMaskingPolicy, GLOBAL_HF_SPECIAL_TCT_END_ROLE, SequenceGenerationMask, TokenRange,
};
use tempfile::TempDir;

fn bin_path(name: &str) -> PathBuf {
    match name {
        "tct-analyze-usage" => PathBuf::from(env!("CARGO_BIN_EXE_tct-analyze-usage")),
        "tct-bpe-lookup-benchmark" => PathBuf::from(env!("CARGO_BIN_EXE_tct-bpe-lookup-benchmark")),
        "tct-build-bundle-wrapper" => PathBuf::from(env!("CARGO_BIN_EXE_tct-build-bundle-wrapper")),
        "tct-measure-tokens" => PathBuf::from(env!("CARGO_BIN_EXE_tct-measure-tokens")),
        "tct-prepare-data" => PathBuf::from(env!("CARGO_BIN_EXE_tct-prepare-data")),
        "tct-token-diff" => PathBuf::from(env!("CARGO_BIN_EXE_tct-token-diff")),
        "tct-train-bpe" => PathBuf::from(env!("CARGO_BIN_EXE_tct-train-bpe")),
        "tct-transliterate-workflows" => {
            PathBuf::from(env!("CARGO_BIN_EXE_tct-transliterate-workflows"))
        }
        "tct-validate-server" => PathBuf::from(env!("CARGO_BIN_EXE_tct-validate-server")),
        "tct-validate-direct-hf-profile" => {
            PathBuf::from(env!("CARGO_BIN_EXE_tct-validate-direct-hf-profile"))
        }
        other => panic!("unknown maintained binary: {other}"),
    }
}

fn read_jsonl_sequences(path: &std::path::Path) -> Vec<Vec<u32>> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Vec<u32>>(line).unwrap())
        .collect()
}

fn assert_sha256_field(value: &serde_json::Value) {
    let digest = value.as_str().expect("sha256 field should be a string");
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
}

fn assert_commit_or_null(value: &serde_json::Value) {
    if value.is_null() {
        return;
    }
    let commit = value
        .as_str()
        .expect("commit field should be a string or null");
    assert_eq!(commit.len(), 40);
    assert!(commit.chars().all(|c| c.is_ascii_hexdigit()));
}

fn assert_bool_or_null(value: &serde_json::Value) {
    assert!(
        value.is_boolean() || value.is_null(),
        "value should be a boolean or null: {value}"
    );
}

fn cli_generated_target_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/tct-maintained-cli-generated")
}

fn simple_schema_fixture(dir: &TempDir) -> (PathBuf, PathBuf, PathBuf, String) {
    let schema_path = dir.path().join("simple-schema.json");
    let instances_dir = dir.path().join("instances");
    fs::create_dir_all(&instances_dir).unwrap();

    fs::write(
        &schema_path,
        serde_json::to_string(&json!({
            "type": "object",
            "required": ["name", "enabled"],
            "additionalProperties": false,
            "properties": {
                "name": {"type": "string"},
                "enabled": {"type": "boolean"}
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let valid_path = instances_dir.join("valid.json");
    fs::write(&valid_path, br#"{"name":"alpha","enabled":true}"#).unwrap();
    let invalid_path = instances_dir.join("invalid.json");
    fs::write(&invalid_path, br#"{"name":"alpha"}"#).unwrap();
    let pattern = instances_dir.join("valid.json").display().to_string();

    (schema_path, valid_path, invalid_path, pattern)
}

fn simple_bpe_merge_fixture(dir: &TempDir) -> PathBuf {
    bpe_merge_fixture(
        dir,
        "simple-bpe.json",
        vec![json!({"pair": [1, 2], "new_token": 257, "frequency": 10})],
    )
}

fn bpe_merge_fixture(dir: &TempDir, file_name: &str, merges: Vec<serde_json::Value>) -> PathBuf {
    let path = dir.path().join(file_name);
    let max_merges = merges.len();
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "schema": "simple-bpe",
            "base_vocab_size": 257,
            "merges": merges,
            "metadata": {
                "trained_on_instances": 2,
                "training_date": "2026-04-26",
                "max_merges": max_merges,
                "dict_size": null
            }
        }))
        .unwrap(),
    )
    .unwrap();
    path
}

fn direct_hf_profile_fixture() -> DirectHfTctCompatibilityProfile {
    DirectHfTctCompatibilityProfile {
        profile_name: DIRECT_LOCAL_PAYLOAD_PROFILE_NAME.to_string(),
        profile_version: DIRECT_LOCAL_PAYLOAD_PROFILE_VERSION,
        handoff_shape: DIRECT_LOCAL_PAYLOAD_HANDOFF_SHAPE.to_string(),
        schema_id: "kubernetes".to_string(),
        schema_sentinel: "<|schema:kubernetes|>".to_string(),
        schema_sentinel_token: 106,
        tct_start_sentinel: "<|tct_start|>".to_string(),
        tct_start_token: 104,
        tct_end_sentinel: "<|tct_end|>".to_string(),
        tct_end_token: 105,
        tct_end_role: GLOBAL_HF_SPECIAL_TCT_END_ROLE.to_string(),
        local_payload: TokenRange::new(0, 17),
        hf_payload: TokenRange::new(2_000, 2_017),
        raw_pad_token: 17,
        raw_pad_hf_token: 2_017,
        raw_pad_is_payload: false,
        masking: DirectHfTctMaskingPolicy::default(),
        generation_mask: SequenceGenerationMask {
            schema_id: "kubernetes".to_string(),
            payload: TokenRange::new(2_000, 2_017),
            eos_token: 105,
        },
    }
}

#[test]
fn direct_hf_profile_validator_accepts_profile_and_hash() {
    let dir = TempDir::new().unwrap();
    let profile = direct_hf_profile_fixture();
    let profile_sha256 = profile.profile_sha256().unwrap();
    let profile_path = dir.path().join("direct-profile.json");
    fs::write(
        &profile_path,
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-validate-direct-hf-profile"))
        .args([
            "--profile",
            profile_path.to_str().unwrap(),
            "--expected-sha256",
            &profile_sha256,
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-validate-direct-hf-profile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "validated");
    assert_eq!(report["profile_sha256"], profile_sha256);
    assert_eq!(report["expected_sha256"], profile_sha256);
    assert_sha256_field(&report["profile_file"]["sha256"]);
    assert_eq!(
        report["profile"]["profile_name"],
        DIRECT_LOCAL_PAYLOAD_PROFILE_NAME
    );
    assert_eq!(report["profile"]["schema_id"], "kubernetes");
    assert_eq!(report["profile"]["raw_pad_is_payload"], false);
    assert_eq!(
        report["profile"]["tct_end_role"],
        GLOBAL_HF_SPECIAL_TCT_END_ROLE
    );
}

#[test]
fn direct_hf_profile_validator_rejects_invalid_profiles_and_hash_mismatch() {
    let dir = TempDir::new().unwrap();
    let mut profile = direct_hf_profile_fixture();
    let profile_path = dir.path().join("direct-profile.json");
    fs::write(
        &profile_path,
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-validate-direct-hf-profile"))
        .args([
            "--profile",
            profile_path.to_str().unwrap(),
            "--expected-sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "hash mismatch should fail validation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("profile SHA-256 mismatch"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    profile.raw_pad_is_payload = true;
    fs::write(
        &profile_path,
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    let output = Command::new(bin_path("tct-validate-direct-hf-profile"))
        .args(["--profile", profile_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success(), "invalid profile should fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must exclude raw PAD token"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn bpe_lookup_benchmark_compares_lookup_strategies() {
    let dir = TempDir::new().unwrap();
    let bpe_path = bpe_merge_fixture(
        &dir,
        "lookup-bpe.json",
        vec![
            json!({"pair": [1, 2], "new_token": 257, "frequency": 10}),
            json!({"pair": [257, 3], "new_token": 258, "frequency": 5}),
            json!({"pair": [4, 5], "new_token": 259, "frequency": 3}),
        ],
    );
    let encoded_path = dir.path().join("encoded.jsonl");
    fs::write(&encoded_path, "[1,2,3,4,5]\n[4,5,99]\n").unwrap();

    let output = Command::new(bin_path("tct-bpe-lookup-benchmark"))
        .args([
            "--bpe-merges",
            bpe_path.to_str().unwrap(),
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--iterations",
            "1",
            "--warmup-iterations",
            "0",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-bpe-lookup-benchmark failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["bpe_start"], 257);
    assert_eq!(report["pad_token"], 260);
    assert_eq!(report["max_vocab_size"], 261);
    assert_eq!(report["lookup_table_entries"], 3);
    assert_eq!(report["active_merges"], 3);
    assert_eq!(report["initial_pair_queries"], 6);
    assert_eq!(report["post_merge_pair_queries"], 4);
    assert_eq!(report["applied_merges"], 4);
    assert_eq!(report["pair_queries"], 10);
    assert_eq!(report["all_strategies_agree"], true);
    assert_eq!(report["input_sequences"], 2);
    assert_eq!(report["runtime_encoded_sequences"], 2);
    assert_eq!(report["runtime_encoded_tokens"], 4);
    assert_eq!(report["all_strategy_outputs_match_runtime"], true);
    let runtime_checksum = report["runtime_encoded_token_checksum"]
        .as_u64()
        .expect("runtime output checksum should be present");
    let strategies = report["strategies"].as_array().unwrap();
    assert_eq!(strategies.len(), 3);
    let names = strategies
        .iter()
        .map(|strategy| strategy["name"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(names.contains("sorted_binary_search"));
    assert!(names.contains("hash_map"));
    assert!(names.contains("bucketed_by_left"));
    for strategy in strategies {
        assert_eq!(strategy["hits"], 4);
        assert_eq!(strategy["encoded_sequences"], 2);
        assert_eq!(strategy["encoded_tokens"], 4);
        assert_eq!(strategy["encoded_token_checksum"], runtime_checksum);
        assert_eq!(strategy["matches_runtime"], true);
        assert!(strategy["best_lookup_time_ns"].as_u64().is_some());
    }
}

#[test]
fn build_bundle_wrapper_cli_writes_package_source() {
    let dir = TempDir::new().unwrap();
    let spec_path = dir.path().join("bundle-spec.json");
    let output_dir = dir.path().join("bundle-output");
    fs::write(
        &spec_path,
        serde_json::to_string_pretty(&json!({
            "package_name": "tct-bundle-demo",
            "version": "0.2.0",
            "description": "Two schema demo bundle",
            "schemas": [
                {
                    "schema_id": "alpha",
                    "module_name": "tct_alpha",
                    "schema_token": 3,
                    "raw_vocab_size": 4,
                    "raw_pad_token": 3,
                    "global_payload_start": 10,
                    "bpe_enabled": false,
                    "schema_sentinel": "<|schema:alpha|>",
                    "generated_artifact": "wheels/tct_alpha.whl"
                },
                {
                    "schema_id": "beta",
                    "module_name": "tct_beta",
                    "schema_token": 4,
                    "raw_vocab_size": 5,
                    "raw_pad_token": 4,
                    "global_payload_start": 20,
                    "bpe_enabled": true
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-build-bundle-wrapper"))
        .args([
            "--spec",
            spec_path.to_str().unwrap(),
            "--output-dir",
            output_dir.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-build-bundle-wrapper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package_dir = output_dir.join("tct-bundle-demo");
    let init_path = package_dir.join("tct_bundle_demo").join("__init__.py");
    let pyproject_path = package_dir.join("pyproject.toml");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        package_dir.display().to_string()
    );
    assert!(init_path.is_file());
    assert!(pyproject_path.is_file());

    let init = fs::read_to_string(init_path).unwrap();
    let pyproject = fs::read_to_string(pyproject_path).unwrap();
    assert!(init.contains("def encode_sequence(schema_id, json_str):"));
    assert!(init.contains("tct_alpha"));
    assert!(init.contains("<|schema:alpha|>"));
    assert!(pyproject.contains("name = \"tct-bundle-demo\""));
    assert!(pyproject.contains("version = \"0.2.0\""));
}

#[test]
fn build_bundle_wrapper_cli_runtime_smoke_with_fake_modules_when_python_available() {
    let python_available = Command::new("python3").arg("--version").output().is_ok();
    if !python_available {
        return;
    }

    let dir = TempDir::new().unwrap();
    let spec_path = dir.path().join("bundle-spec.json");
    let output_dir = dir.path().join("bundle-output");
    fs::write(
        dir.path().join("tct_alpha.py"),
        r#"
def encode(json_str):
    return [0, 1]

def decode(tokens):
    tokens = list(tokens)
    if tokens != [0, 1]:
        raise ValueError(f"bad alpha payload: {tokens}")
    return '{"schema":"alpha"}', len(tokens), len(tokens)
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("tct_beta.py"),
        r#"
def encode(json_str):
    return [0, 2]

def decode(tokens):
    tokens = list(tokens)
    if tokens != [0, 2]:
        raise ValueError(f"bad beta payload: {tokens}")
    return '{"schema":"beta"}', len(tokens), len(tokens)
"#,
    )
    .unwrap();
    fs::write(
        &spec_path,
        serde_json::to_string_pretty(&json!({
            "package_name": "tct-bundle-demo",
            "version": "0.2.0",
            "schemas": [
                {
                    "schema_id": "alpha",
                    "module_name": "tct_alpha",
                    "schema_token": 3,
                    "raw_vocab_size": 3,
                    "raw_pad_token": 2,
                    "global_payload_start": 4,
                    "bpe_enabled": false
                },
                {
                    "schema_id": "beta",
                    "module_name": "tct_beta",
                    "schema_token": 4,
                    "raw_vocab_size": 4,
                    "raw_pad_token": 3,
                    "global_payload_start": 10,
                    "bpe_enabled": false
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-build-bundle-wrapper"))
        .args([
            "--spec",
            spec_path.to_str().unwrap(),
            "--output-dir",
            output_dir.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-build-bundle-wrapper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let pythonpath = format!("{}:{}", package_dir, dir.path().display());
    let script = r#"
import tct_bundle_demo

alpha = tct_bundle_demo.encode_sequence("alpha", "{}")
assert alpha == [1, 3, 4, 5, 2], alpha
schema_id, json_str, consumed, total = tct_bundle_demo.decode_sequence(alpha + [0, 0])
assert (schema_id, json_str, consumed, total) == ("alpha", '{"schema":"alpha"}', 5, 7)

beta = tct_bundle_demo.encode_sequence("beta", "{}")
assert beta == [1, 4, 10, 12, 2], beta
schema_id, json_str, consumed, total = tct_bundle_demo.decode_sequence(beta)
assert (schema_id, json_str, consumed, total) == ("beta", '{"schema":"beta"}', 5, 5)

try:
    tct_bundle_demo.decode_sequence([1, 3, 10, 2])
    raise AssertionError("wrong-schema payload should fail")
except tct_bundle_demo.SequenceBundleError:
    pass

try:
    tct_bundle_demo.decode_sequence([1, 3, 4, 2, 4])
    raise AssertionError("non-PAD after EOS should fail")
except tct_bundle_demo.SequenceBundleError:
    pass
"#;

    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .env("PYTHONPATH", pythonpath)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generated bundle wrapper failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn token_diff_reports_schema_layout_changes() {
    let dir = TempDir::new().unwrap();
    let old_schema_path = dir.path().join("old-schema.json");
    let new_schema_path = dir.path().join("new-schema.json");
    fs::write(
        &old_schema_path,
        serde_json::to_string(&json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &new_schema_path,
        serde_json::to_string(&json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "enabled": {"type": "boolean"}
            },
            "required": ["name"]
        }))
        .unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-token-diff"))
        .args([
            "--old-schema",
            old_schema_path.to_str().unwrap(),
            "--new-schema",
            new_schema_path.to_str().unwrap(),
            "--name",
            "Example",
            "--optimization",
            "O3",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-token-diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse token-diff JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(
        json["manifest"]["diff"]["compatibility"],
        "token_layout_unchanged"
    );
    assert_eq!(
        json["experiment_gate"]["evidence_scope"],
        "compiler_artifact_evidence_only"
    );
    assert_eq!(json["experiment_gate"]["downstream_claims_allowed"], false);
    let required_provenance = json["experiment_gate"]["required_downstream_provenance"]
        .as_array()
        .expect("token-diff report should list downstream provenance requirements");
    assert!(
        required_provenance
            .iter()
            .any(|value| value == "token_diff_report")
    );
    assert!(
        required_provenance
            .iter()
            .any(|value| value == "downstream_repo_commits")
    );
    assert_eq!(
        json["manifest"]["diff"]["stable_vocabulary"]["decision"],
        "breaking_change"
    );
    assert_eq!(
        json["manifest"]["diff"]["structural_compatibility"]["decision"],
        "breaking_change"
    );
    assert_eq!(
        json["manifest"]["diff"]["stable_vocabulary"]["generated_tokenizer_reuse"],
        "regenerate_required"
    );
    assert_eq!(
        json["manifest"]["diff"]["stable_vocabulary"]["downstream_bpe_or_embedding_reuse"],
        "regenerate_required"
    );
    assert_eq!(json["manifest"]["old"]["string_encoding"], "utf8");
    assert_eq!(json["manifest"]["new"]["string_encoding"], "utf8");
    assert!(
        json["manifest"]["old"]["compile_wall_time_us"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["manifest"]["old"]["generate_code_wall_time_us"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["manifest"]["old"]["generated_code_bytes"]
            .as_u64()
            .is_some_and(|value| value > 0)
    );
    let new_entries = json["manifest"]["new"]["structural_token_map"]["entries"]
        .as_array()
        .expect("new structural token map should expose entries");
    let enabled_entry = new_entries
        .iter()
        .find(|entry| entry["path"] == "#/properties/enabled")
        .expect("new structural token map should expose the added property");
    assert!(enabled_entry["generated_order"].as_u64().is_some());
    assert_eq!(enabled_entry["member_type_path"], "bool");
    assert_eq!(enabled_entry["encoding_decision"], "standard");
    assert_eq!(
        enabled_entry["emission"]["selector"]["kind"],
        "option_presence_flag"
    );
    assert_eq!(
        enabled_entry["emission"]["selector"]["fixed_token_count"],
        1
    );
    assert_eq!(
        enabled_entry["emission"]["value"]["emits_when_present"],
        true
    );
    assert_eq!(enabled_entry["emission"]["value"]["width"], "fixed");
    assert_eq!(enabled_entry["emission"]["value"]["fixed_token_count"], 1);
    assert_eq!(
        enabled_entry["emission"]["strict_stream_shape"],
        "presence_flag_then_value_if_present"
    );
    assert_eq!(enabled_entry["emission"]["catch_all_map"], false);
    assert_eq!(enabled_entry["emission"]["nested_object"], false);
    assert_eq!(
        json["manifest"]["old"]["max_base_vocab"],
        serde_json::Value::Null
    );
    assert_eq!(
        json["manifest"]["new"]["max_base_vocab"],
        serde_json::Value::Null
    );
    assert_eq!(json["manifest"]["diff"]["token_id_layout_changed"], false);
    assert_eq!(json["manifest"]["diff"]["generated_code_changed"], true);
    assert_sha256_field(&json["old_schema_file"]["sha256"]);
    assert_sha256_field(&json["new_schema_file"]["sha256"]);
    assert_sha256_field(&json["manifest"]["old"]["generated_code_sha256"]);
    assert_sha256_field(&json["manifest"]["new"]["generated_code_sha256"]);
}

#[test]
fn token_diff_promotes_appended_required_zero_token_const() {
    let dir = TempDir::new().unwrap();
    let old_schema_path = dir.path().join("old-schema.json");
    let new_schema_path = dir.path().join("new-schema.json");
    fs::write(
        &old_schema_path,
        serde_json::to_string(&json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"],
            "additionalProperties": false
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &new_schema_path,
        serde_json::to_string(&json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "kind": {"type": "string", "const": "Example"}
            },
            "required": ["name", "kind"],
            "additionalProperties": false
        }))
        .unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-token-diff"))
        .args([
            "--old-schema",
            old_schema_path.to_str().unwrap(),
            "--new-schema",
            new_schema_path.to_str().unwrap(),
            "--name",
            "Example",
            "--optimization",
            "O3",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-token-diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse token-diff JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(
        json["manifest"]["diff"]["compatibility"],
        "token_layout_unchanged"
    );
    assert_eq!(
        json["manifest"]["diff"]["stable_vocabulary"]["decision"],
        "additive_compatible"
    );
    assert_eq!(
        json["manifest"]["diff"]["structural_compatibility"]["decision"],
        "additive_compatible"
    );
    assert_eq!(
        json["manifest"]["diff"]["structural_compatibility"]["changed_paths"],
        json!(["#/properties/kind"])
    );
    let new_entries = json["manifest"]["new"]["structural_token_map"]["entries"]
        .as_array()
        .expect("new structural token map should expose entries");
    let kind_entry = new_entries
        .iter()
        .find(|entry| entry["path"] == "#/properties/kind")
        .expect("new structural token map should expose the const property");
    assert_eq!(kind_entry["role"], "zero_token_const");
    assert_eq!(kind_entry["required"], true);
    assert_eq!(kind_entry["generated_order"], 1);
}

#[test]
fn token_diff_reports_layout_affecting_config_changes() {
    let dir = TempDir::new().unwrap();
    let old_schema_path = dir.path().join("old-schema.json");
    let new_schema_path = dir.path().join("new-schema.json");
    fs::write(
        &old_schema_path,
        serde_json::to_string(&json!({"type": "string"})).unwrap(),
    )
    .unwrap();
    fs::write(
        &new_schema_path,
        serde_json::to_string(&json!({"type": "string"})).unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-token-diff"))
        .args([
            "--old-schema",
            old_schema_path.to_str().unwrap(),
            "--new-schema",
            new_schema_path.to_str().unwrap(),
            "--name",
            "Text",
            "--optimization",
            "O3",
            "--new-string-encoding",
            "ascii",
            "--new-max-base-vocab",
            "512",
            "--new-bit-packing",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-token-diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse token-diff JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(
        json["manifest"]["diff"]["compatibility"],
        "token_layout_changed"
    );
    assert_eq!(
        json["manifest"]["diff"]["stable_vocabulary"]["decision"],
        "breaking_change"
    );
    assert_eq!(json["manifest"]["old"]["string_encoding"], "utf8");
    assert_eq!(json["manifest"]["new"]["string_encoding"], "ascii");
    assert_eq!(
        json["manifest"]["old"]["max_base_vocab"],
        serde_json::Value::Null
    );
    assert_eq!(json["manifest"]["new"]["max_base_vocab"], 512);
    assert_eq!(
        json["manifest"]["old"]["optimization_pass_overrides"]["bit_packing"],
        false
    );
    assert_eq!(
        json["manifest"]["new"]["optimization_pass_overrides"]["bit_packing"],
        true
    );
    assert_eq!(json["manifest"]["old"]["encoding_base"], 256);
    assert_eq!(json["manifest"]["new"]["encoding_base"], 512);
    assert!(
        json["manifest"]["diff"]["changed_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("string_encoding"))
    );
    assert!(
        json["manifest"]["diff"]["changed_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("max_base_vocab"))
    );
    assert!(
        json["manifest"]["diff"]["changed_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("optimization_pass_overrides"))
    );
    assert_sha256_field(&json["manifest"]["old"]["generated_code_sha256"]);
    assert_sha256_field(&json["manifest"]["new"]["generated_code_sha256"]);
}

#[test]
fn token_diff_rejects_reserved_pattern_optimization_override() {
    let dir = TempDir::new().unwrap();
    let old_schema_path = dir.path().join("old-schema.json");
    let new_schema_path = dir.path().join("new-schema.json");
    let schema =
        serde_json::to_string(&json!({"type": "string", "pattern": "^[a-z]{3}$"})).unwrap();
    fs::write(&old_schema_path, &schema).unwrap();
    fs::write(&new_schema_path, &schema).unwrap();

    let output = Command::new(bin_path("tct-token-diff"))
        .args([
            "--old-schema",
            old_schema_path.to_str().unwrap(),
            "--new-schema",
            new_schema_path.to_str().unwrap(),
            "--name",
            "Text",
            "--new-pattern-optimization",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "reserved pattern override should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--new-pattern-optimization is reserved")
            && stderr.contains("patternProperties validation is schema-driven"),
        "token-diff should explain reserved pattern override: {stderr}"
    );
}

#[test]
fn token_diff_reports_additive_enum_compatibility() {
    let dir = TempDir::new().unwrap();
    let old_schema_path = dir.path().join("old-schema.json");
    let new_schema_path = dir.path().join("new-schema.json");
    fs::write(
        &old_schema_path,
        serde_json::to_string(&json!({"enum": ["alpha", "beta"]})).unwrap(),
    )
    .unwrap();
    fs::write(
        &new_schema_path,
        serde_json::to_string(&json!({"enum": ["alpha", "beta", "gamma"]})).unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-token-diff"))
        .args([
            "--old-schema",
            old_schema_path.to_str().unwrap(),
            "--new-schema",
            new_schema_path.to_str().unwrap(),
            "--name",
            "Choice",
            "--optimization",
            "O3",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-token-diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse token-diff JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(
        json["manifest"]["diff"]["compatibility"],
        "token_layout_unchanged"
    );
    assert_eq!(
        json["manifest"]["diff"]["structural_compatibility"]["decision"],
        "additive_compatible"
    );
    assert_eq!(
        json["manifest"]["diff"]["structural_compatibility"]["changed_paths"],
        json!(["#/enum/2"])
    );
    assert_eq!(
        json["manifest"]["diff"]["stable_vocabulary"]["decision"],
        "additive_compatible"
    );
    assert_eq!(
        json["manifest"]["diff"]["stable_vocabulary"]["generated_tokenizer_reuse"],
        "regenerate_required"
    );
    assert_eq!(
        json["manifest"]["diff"]["stable_vocabulary"]["downstream_bpe_or_embedding_reuse"],
        "review_required"
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["artifact_kind"],
        "tct_vocabulary_extension_handoff"
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["status"],
        "candidate_extension_artifact"
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["new_structural_paths"],
        json!(["#/enum/2"])
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["appended_token_ranges"],
        json!([])
    );
}

#[test]
fn token_diff_reports_bpe_artifact_evidence() {
    let dir = TempDir::new().unwrap();
    let old_schema_path = dir.path().join("old-schema.json");
    let new_schema_path = dir.path().join("new-schema.json");
    fs::write(
        &old_schema_path,
        serde_json::to_string(&json!({"type": "string"})).unwrap(),
    )
    .unwrap();
    fs::write(
        &new_schema_path,
        serde_json::to_string(&json!({"type": "string"})).unwrap(),
    )
    .unwrap();
    let old_bpe_path = bpe_merge_fixture(
        &dir,
        "old-bpe.json",
        vec![json!({"pair": [1, 2], "new_token": 257, "frequency": 10})],
    );
    let new_bpe_path = bpe_merge_fixture(
        &dir,
        "new-bpe.json",
        vec![
            json!({"pair": [1, 2], "new_token": 257, "frequency": 10}),
            json!({"pair": [257, 3], "new_token": 258, "frequency": 8}),
        ],
    );

    let output = Command::new(bin_path("tct-token-diff"))
        .args([
            "--old-schema",
            old_schema_path.to_str().unwrap(),
            "--new-schema",
            new_schema_path.to_str().unwrap(),
            "--name",
            "Text",
            "--optimization",
            "O3",
            "--old-bpe-merges",
            old_bpe_path.to_str().unwrap(),
            "--new-bpe-merges",
            new_bpe_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-token-diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse token-diff JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(
        json["manifest"]["diff"]["compatibility"],
        "token_layout_changed"
    );
    assert_eq!(
        json["manifest"]["diff"]["bpe_artifact"]["compatibility"],
        "prefix_extended"
    );
    assert_eq!(
        json["manifest"]["diff"]["bpe_artifact"]["old_is_prefix_of_new"],
        true
    );
    assert_eq!(
        json["manifest"]["diff"]["bpe_artifact"]["first_differing_merge_index"],
        1
    );
    assert_eq!(
        json["manifest"]["diff"]["bpe_artifact"]["merge_count_delta"],
        1
    );
    assert_eq!(
        json["manifest"]["old"]["bpe_artifact"]["base_vocab_size"],
        257
    );
    assert_eq!(
        json["manifest"]["old"]["bpe_artifact"]["first_new_token"],
        257
    );
    assert_eq!(
        json["manifest"]["new"]["bpe_artifact"]["last_new_token"],
        258
    );
    assert_eq!(
        json["manifest"]["new"]["bpe_artifact"]["matches_compiled_base_vocab"],
        true
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["status"],
        "candidate_extension_artifact"
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["bpe_prefix_extension"],
        true
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["bpe_artifact_compatibility"],
        "prefix_extended"
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["appended_token_ranges"][0]["start"],
        json["manifest"]["old"]["vocab_size"]
    );
    assert_eq!(
        json["manifest"]["vocabulary_extension_handoff"]["appended_token_ranges"][0]["end_exclusive"],
        json["manifest"]["new"]["vocab_size"]
    );
    assert_sha256_field(&json["manifest"]["vocabulary_extension_handoff"]["old_bpe_table_sha256"]);
    assert_sha256_field(
        &json["manifest"]["vocabulary_extension_handoff"]["new_bpe_merge_sequence_sha256"],
    );
    assert_sha256_field(&json["manifest"]["old"]["bpe_artifact"]["file"]["sha256"]);
    assert_sha256_field(&json["manifest"]["old"]["bpe_artifact"]["table_sha256"]);
    assert_sha256_field(&json["manifest"]["new"]["bpe_artifact"]["merge_sequence_sha256"]);
}

#[test]
fn token_diff_reports_partial_stale_bpe_artifact_evidence() {
    let dir = TempDir::new().unwrap();
    let old_schema_path = dir.path().join("old-schema.json");
    let new_schema_path = dir.path().join("new-schema.json");
    fs::write(
        &old_schema_path,
        serde_json::to_string(&json!({"type": "string"})).unwrap(),
    )
    .unwrap();
    fs::write(
        &new_schema_path,
        serde_json::to_string(&json!({"type": "string"})).unwrap(),
    )
    .unwrap();
    let stale_bpe_path = bpe_merge_fixture(
        &dir,
        "stale-bpe.json",
        vec![json!({"pair": [1, 2], "new_token": 257, "frequency": 10})],
    );

    let output = Command::new(bin_path("tct-token-diff"))
        .args([
            "--old-schema",
            old_schema_path.to_str().unwrap(),
            "--new-schema",
            new_schema_path.to_str().unwrap(),
            "--name",
            "Text",
            "--optimization",
            "O3",
            "--old-max-base-vocab",
            "512",
            "--old-bpe-merges",
            stale_bpe_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "stale BPE token-diff should exit nonzero"
    );
    assert!(
        output.stderr.is_empty(),
        "partial report should be emitted as JSON instead of stderr-only failure: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse partial token-diff JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(json["status"], "partial_error");
    assert_eq!(
        json["experiment_gate"]["evidence_scope"],
        "compiler_artifact_evidence_only"
    );
    assert_eq!(json["experiment_gate"]["downstream_claims_allowed"], false);
    assert_eq!(json["manifest"], serde_json::Value::Null);
    assert_eq!(json["partial"]["old"]["side"], "old");
    assert_eq!(json["partial"]["new"]["side"], "new");
    assert_eq!(json["partial"]["new"]["layout"]["base_vocab_size"], 257);
    assert_eq!(
        json["partial"]["old"]["layout_without_bpe"]["base_vocab_size"],
        512
    );
    assert_eq!(
        json["partial"]["old"]["bpe_artifact"]["base_vocab_size"],
        257
    );
    assert_eq!(
        json["partial"]["old"]["bpe_artifact"]["matches_compiled_base_vocab"],
        false
    );
    assert_eq!(
        json["partial"]["old"]["bpe_artifact"]["matches_compiled_bpe_start"],
        false
    );
    assert_eq!(
        json["partial"]["old"]["errors"][0]["kind"],
        "invalid_configuration"
    );
    assert_eq!(json["partial"]["old"]["errors"][0]["field"], "bpe_merges");
    assert!(
        json["partial"]["old"]["errors"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("current generated base vocabulary is 512")
    );
    assert_sha256_field(&json["partial"]["old"]["bpe_artifact"]["file"]["sha256"]);
    assert_sha256_field(&json["partial"]["old"]["bpe_artifact"]["table_sha256"]);
}

#[test]
fn measure_tokens_encoded_mode_reports_same_process_bpe_metrics() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    fs::write(&encoded_path, "[1,2,3]\n[1,2,1,2]\n").unwrap();
    let merge_path = simple_bpe_merge_fixture(&dir);

    let output = Command::new(bin_path("tct-measure-tokens"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--bpe-merges",
            merge_path.to_str().unwrap(),
            "--warmup-iterations",
            "1",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-measure-tokens encoded mode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse encoded-mode JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(json["tokens"], 4);
    assert_eq!(json["instances"], 2);
    assert_eq!(json["has_bpe"], true);
    assert_eq!(json["base_vocab_size"], 257);
    assert_eq!(json["generated_vocab_size"], 259);
    assert_eq!(json["pad_token"], 258);
    assert_eq!(json["same_process_bpe_metrics"]["available"], true);
    assert_eq!(json["same_process_bpe_metrics"]["input_tokens"], 7);
    assert_eq!(json["same_process_bpe_metrics"]["output_tokens"], 4);
    assert_eq!(json["same_process_bpe_metrics"]["compression_ratio"], 1.75);
    assert_eq!(
        json["same_process_bpe_metrics"]["decode_round_trip_ok"],
        true
    );
    assert_eq!(json["same_process_bpe_metrics"]["merges_count"], 1);
    assert_eq!(json["same_process_bpe_metrics"]["bpe_vocab_size"], 258);
    assert_eq!(json["provenance"]["base_vocab_source"], "bpe_merges");
    assert_eq!(
        json["provenance"]["encoded_input_file"]["path"],
        encoded_path.display().to_string()
    );
    assert_sha256_field(&json["provenance"]["encoded_input_file"]["sha256"]);
    assert_sha256_field(&json["provenance"]["bpe_merges_file"]["sha256"]);
}

#[test]
fn measure_tokens_schema_bpe_mode_reports_generated_bpe_metrics() {
    let dir = TempDir::new().unwrap();
    let (schema_path, _valid_path, _invalid_path, instance_pattern) = simple_schema_fixture(&dir);
    let merge_path = simple_bpe_merge_fixture(&dir);
    let generated_target_dir = cli_generated_target_dir();

    let output = Command::new(bin_path("tct-measure-tokens"))
        .env("CARGO_TARGET_DIR", &generated_target_dir)
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--instances",
            &instance_pattern,
            "--bpe-merges",
            merge_path.to_str().unwrap(),
            "--optimization",
            "Os",
            "--warmup-iterations",
            "1",
            "--track-allocations",
            "--generated-bpe-metrics",
            "--generated-encode-phase-metrics",
            "--generated-encode-hot-path-metrics",
            "--generated-encode-validation-phase-metrics",
            "--generated-decode-prefix-metrics",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-measure-tokens generated BPE metrics run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse generated-BPE JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(json["has_bpe"], true);
    assert_eq!(json["same_process_bpe_metrics"]["available"], false);
    assert_eq!(json["generated_batch_runtime_metrics"]["available"], true);
    assert!(
        json["generated_batch_runtime_metrics"]["generated_encode_child_wall_time_ms"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["timings"]["note"]
            .as_str()
            .unwrap()
            .contains("parent-observed full-contract envelopes")
    );
    assert!(
        json["generated_batch_runtime_metrics"]["generated_decode_parent_stdout_read_parse_ns"]
            .as_u64()
            .is_some()
    );
    assert_eq!(json["generated_bpe_metrics"]["available"], true);
    assert_eq!(json["generated_bpe_metrics"]["round_trip_ok"], true);
    assert!(
        json["generated_bpe_metrics"]["encode_wall_time_ms"]
            .as_u64()
            .is_some()
    );
    assert_eq!(json["generated_decode_prefix_metrics"]["available"], true);
    assert_eq!(
        json["generated_decode_prefix_metrics"]["warmup_iterations"],
        1
    );
    assert_eq!(json["generated_decode_prefix_metrics"]["records"], 1);
    assert_eq!(
        json["generated_decode_prefix_metrics"]["failed_prefixes"],
        0
    );
    assert_eq!(
        json["generated_decode_prefix_metrics"]["full_sequences_complete"],
        1
    );
    assert!(
        json["generated_decode_prefix_metrics"]["bpe_decode_ns"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_decode_prefix_metrics"]["decode_prefix_ns"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_bpe_metrics"]["decode_wall_time_ms"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_bpe_metrics"]["generated_bpe_encode_allocations"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(
        json["generated_bpe_metrics"]["generated_bpe_decode_allocations"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert_eq!(json["generated_encode_phase_metrics"]["available"], true);
    assert_eq!(json["generated_encode_phase_metrics"]["records"], 1);
    assert_eq!(json["generated_encode_phase_metrics"]["failures"], 0);
    assert!(
        json["generated_encode_phase_metrics"]["json_read_parse_ns"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_encode_phase_metrics"]["base_encode_ns"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_encode_phase_metrics"]["bpe_encode_ns"]
            .as_u64()
            .is_some()
    );
    assert_eq!(
        json["generated_encode_phase_metrics"]["token_sequences_match_primary"],
        true
    );
    assert_eq!(json["generated_encode_hot_path_metrics"]["available"], true);
    assert_eq!(json["generated_encode_hot_path_metrics"]["records"], 1);
    assert_eq!(json["generated_encode_hot_path_metrics"]["failures"], 0);
    assert_eq!(
        json["generated_encode_hot_path_metrics"]["token_count_matches_primary"],
        true
    );
    assert_eq!(
        json["generated_encode_hot_path_metrics"]["token_checksum_matches_primary"],
        true
    );
    assert!(
        json["generated_encode_hot_path_metrics"]["bpe_encode_ns"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_encode_hot_path_metrics"]["generated_encode_hot_path_allocations"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_encode_hot_path_metrics"]["generated_encode_hot_path_allocated_bytes"]
            .as_u64()
            .is_some()
    );
    assert_eq!(
        json["generated_encode_validation_phase_metrics"]["available"],
        true
    );
    assert_eq!(
        json["generated_encode_validation_phase_metrics"]["records"],
        1
    );
    assert_eq!(
        json["generated_encode_validation_phase_metrics"]["token_count_matches_primary"],
        true
    );
    assert_eq!(
        json["generated_encode_validation_phase_metrics"]["token_checksum_matches_primary"],
        true
    );
    assert!(
        json["generated_encode_validation_phase_metrics"]["file_read_ns"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_encode_validation_phase_metrics"]["json_value_parse_ns"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["generated_encode_validation_phase_metrics"]["serde_from_value_ns"]
            .as_u64()
            .is_some()
    );
}

#[test]
fn measure_tokens_schema_mode_reports_generated_decode_prefix_metrics() {
    let dir = TempDir::new().unwrap();
    let (schema_path, _valid_path, _invalid_path, instance_pattern) = simple_schema_fixture(&dir);
    let generated_target_dir = cli_generated_target_dir();

    let output = Command::new(bin_path("tct-measure-tokens"))
        .env("CARGO_TARGET_DIR", &generated_target_dir)
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--instances",
            &instance_pattern,
            "--optimization",
            "O3",
            "--warmup-iterations",
            "1",
            "--generated-decode-prefix-metrics",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-measure-tokens generated decode_prefix metrics run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse generated decode_prefix JSON output: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });

    assert_eq!(json["instances"], 1);
    assert_eq!(json["has_bpe"], false);
    assert_eq!(json["warmup_iterations"], 1);
    assert_eq!(json["generated_decode_prefix_metrics"]["available"], true);
    assert_eq!(
        json["generated_decode_prefix_metrics"]["warmup_iterations"],
        1
    );
    assert_eq!(json["generated_decode_prefix_metrics"]["records"], 1);
    assert_eq!(
        json["generated_decode_prefix_metrics"]["failed_prefixes"],
        0
    );
    assert_eq!(
        json["generated_decode_prefix_metrics"]["full_sequences_complete"],
        1
    );
    assert!(
        json["generated_decode_prefix_metrics"]["prefix_cuts"]
            .as_u64()
            .is_some_and(|value| value >= 2)
    );
    assert!(
        json["generated_decode_prefix_metrics"]["partial_prefixes"]
            .as_u64()
            .is_some_and(|value| value >= 1)
    );
    assert!(
        json["generated_decode_prefix_metrics"]["complete_prefixes"]
            .as_u64()
            .is_some_and(|value| value >= 1)
    );
    assert!(
        json["generated_decode_prefix_metrics"]["consumed_tokens"]
            .as_u64()
            .is_some_and(|value| value > 0)
    );
    assert!(
        json["generated_decode_prefix_metrics"]["decode_prefix_ns"]
            .as_u64()
            .is_some()
    );
    assert_eq!(
        json["generated_decode_prefix_metrics"]["bpe_decode_ns"],
        serde_json::Value::Null
    );
}

#[test]
fn prepare_data_rejects_missing_sources_or_encoded_input() {
    let dir = TempDir::new().unwrap();
    let output_dir = dir.path().join("prepared");

    let output = Command::new(bin_path("tct-prepare-data"))
        .args(["--output", output_dir.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Must specify either --sources or --encoded")
    );
}

#[test]
fn prepare_data_utf8_mode_writes_split_metadata_and_sources() {
    let dir = TempDir::new().unwrap();
    let input_dir = dir.path().join("input");
    let output_dir = dir.path().join("prepared");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("one.json"), br#"{"name":"one"}"#).unwrap();
    fs::write(input_dir.join("two.json"), br#"{"name":"two"}"#).unwrap();
    let pattern = input_dir.join("*.json").display().to_string();

    let output = Command::new(bin_path("tct-prepare-data"))
        .args([
            "--mode",
            "utf8",
            "--sources",
            &pattern,
            "--output",
            output_dir.to_str().unwrap(),
            "--train-ratio",
            "0.5",
            "--validate-ratio",
            "0.5",
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-prepare-data failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"train_sequences\":1"));
    assert!(stdout.contains("\"validate_sequences\":1"));

    let metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output_dir.join("metadata.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["mode"], "utf8");
    assert_eq!(metadata["total_files"], 2);
    assert_eq!(metadata["encoded_files"], 2);
    assert_eq!(metadata["base_vocab_size"], 256);
    assert_eq!(metadata["has_source_traceability"], true);
    assert_eq!(
        metadata["command"][0].as_str().unwrap(),
        bin_path("tct-prepare-data").display().to_string()
    );

    assert!(output_dir.join("train.jsonl").is_file());
    assert!(output_dir.join("validate.jsonl").is_file());
    assert!(output_dir.join("all.jsonl").is_file());
    assert!(output_dir.join("sources.jsonl").is_file());
    assert!(output_dir.join("stats.json").is_file());

    let first_source: serde_json::Value = serde_json::from_str(
        fs::read_to_string(output_dir.join("sources.jsonl"))
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert!(first_source["file"]["bytes"].as_u64().unwrap() > 0);
    assert_sha256_field(&first_source["file"]["sha256"]);
}

#[test]
fn prepare_train_measure_encoded_cli_workflow_preserves_provenance() {
    let dir = TempDir::new().unwrap();
    let input_dir = dir.path().join("input");
    let prepared_dir = dir.path().join("prepared");
    let all_jsonl_path = prepared_dir.join("all.jsonl");
    let metadata_path = prepared_dir.join("metadata.json");
    let merges_path = dir.path().join("trained-merges.json");
    let provenance_path = dir.path().join("trained-merges.provenance.json");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("one.txt"), b"aaaa").unwrap();
    fs::write(input_dir.join("two.txt"), b"aaab").unwrap();
    let pattern = input_dir.join("*.txt").display().to_string();

    let prepare_output = Command::new(bin_path("tct-prepare-data"))
        .args([
            "--mode",
            "utf8",
            "--sources",
            &pattern,
            "--output",
            prepared_dir.to_str().unwrap(),
            "--train-ratio",
            "1.0",
            "--validate-ratio",
            "0.0",
            "--quiet",
        ])
        .output()
        .unwrap();
    assert!(
        prepare_output.status.success(),
        "tct-prepare-data workflow step failed: {}",
        String::from_utf8_lossy(&prepare_output.stderr)
    );

    let train_output = Command::new(bin_path("tct-train-bpe"))
        .args([
            "--encoded",
            all_jsonl_path.to_str().unwrap(),
            "--target-vocab",
            "258",
            "--output",
            merges_path.to_str().unwrap(),
            "--name",
            "utf8-chain",
        ])
        .output()
        .unwrap();
    assert!(
        train_output.status.success(),
        "tct-train-bpe workflow step failed: {}",
        String::from_utf8_lossy(&train_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&train_output.stderr)
            .contains("Auto-detected base_vocab_size: 256")
    );

    let measure_output = Command::new(bin_path("tct-measure-tokens"))
        .args([
            "--encoded",
            all_jsonl_path.to_str().unwrap(),
            "--bpe-merges",
            merges_path.to_str().unwrap(),
            "--warmup-iterations",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        measure_output.status.success(),
        "tct-measure-tokens workflow step failed: {}",
        String::from_utf8_lossy(&measure_output.stderr)
    );

    let prepared_metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&metadata_path).unwrap()).unwrap();
    assert_eq!(prepared_metadata["mode"], "utf8");
    assert_eq!(prepared_metadata["base_vocab_size"], 256);

    let train_provenance: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&provenance_path).unwrap()).unwrap();
    assert_eq!(train_provenance["schema_name"], "utf8-chain");
    assert_eq!(train_provenance["tct_version"], env!("CARGO_PKG_VERSION"));
    assert_commit_or_null(&train_provenance["repo_commit"]);
    assert_bool_or_null(&train_provenance["repo_dirty"]);
    assert_eq!(
        train_provenance["base_vocab_source"],
        "prepare-data-metadata"
    );
    assert_eq!(
        train_provenance["prepare_data_metadata_file"]["path"],
        metadata_path.display().to_string()
    );
    assert_sha256_field(&train_provenance["prepare_data_metadata_file"]["sha256"]);
    assert_eq!(train_provenance["input_files"].as_array().unwrap().len(), 1);
    assert_eq!(
        train_provenance["input_files"][0]["path"],
        all_jsonl_path.display().to_string()
    );
    assert_sha256_field(&train_provenance["input_files"][0]["sha256"]);
    assert_sha256_field(&train_provenance["input_corpus_fingerprint"]);
    assert!(
        train_provenance["timings"]["training_wall_time_ns"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "training sidecar should record BPE trainer wall time"
    );
    assert!(
        train_provenance["timings"]["provenance_hashing_ns"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "training sidecar should attribute provenance hashing time"
    );

    let measure_json: serde_json::Value = serde_json::from_slice(&measure_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "failed to parse tct-measure-tokens workflow JSON output: {error}\n{}",
                String::from_utf8_lossy(&measure_output.stdout)
            )
        });
    assert_eq!(measure_json["instances"], 2);
    assert_eq!(measure_json["has_bpe"], true);
    assert_eq!(measure_json["base_vocab_size"], 256);
    assert_eq!(measure_json["token_length_stats"]["records"], 2);
    assert_eq!(
        measure_json["token_length_stats"]["total_tokens"],
        measure_json["tokens"]
    );
    assert!(
        measure_json["token_length_stats"]["p99_tokens"]
            .as_u64()
            .is_some()
    );
    assert_eq!(
        measure_json["token_length_stats"]["context_window_tokens"],
        2048
    );
    assert_eq!(
        measure_json["token_length_stats"]["context_window_pass_rate"],
        1.0
    );
    assert_eq!(
        measure_json["provenance"]["base_vocab_source"],
        "bpe_merges"
    );
    assert_eq!(
        measure_json["provenance"]["encoded_input_file"]["path"],
        all_jsonl_path.display().to_string()
    );
    assert_eq!(
        measure_json["provenance"]["bpe_merges_file"]["path"],
        merges_path.display().to_string()
    );
    assert_eq!(measure_json["same_process_bpe_metrics"]["available"], true);
    assert_eq!(
        measure_json["same_process_bpe_metrics"]["decode_round_trip_ok"],
        true
    );
    assert_sha256_field(&measure_json["provenance"]["encoded_input_file"]["sha256"]);
    assert_sha256_field(&measure_json["provenance"]["bpe_merges_file"]["sha256"]);
}

#[test]
fn prepare_data_encoded_mode_preserves_order_and_metadata() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    let output_dir = dir.path().join("prepared");
    fs::write(&encoded_path, "[1,2,3]\n[4,5]\n[6]\n").unwrap();

    let output = Command::new(bin_path("tct-prepare-data"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--train-ratio",
            "0.67",
            "--validate-ratio",
            "0.33",
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-prepare-data --encoded failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"train_sequences\":2"));
    assert!(stdout.contains("\"validate_sequences\":1"));
    assert!(stdout.contains("\"all_sequences\":3"));

    assert_eq!(
        read_jsonl_sequences(&output_dir.join("train.jsonl")),
        vec![vec![1, 2, 3], vec![4, 5]]
    );
    assert_eq!(
        read_jsonl_sequences(&output_dir.join("validate.jsonl")),
        vec![vec![6]]
    );
    assert_eq!(
        read_jsonl_sequences(&output_dir.join("all.jsonl")),
        vec![vec![1, 2, 3], vec![4, 5], vec![6]]
    );
    assert!(!output_dir.join("sources.jsonl").exists());

    let metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output_dir.join("metadata.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["mode"], "reshuffle");
    assert_eq!(metadata["schema"], serde_json::Value::Null);
    assert_eq!(
        metadata["sources"],
        json!([encoded_path.display().to_string()])
    );
    assert_eq!(metadata["total_files"], 3);
    assert_eq!(metadata["encoded_files"], 3);
    assert_eq!(metadata["base_vocab_size"], 7);
    assert_eq!(metadata["preserve_order"], true);
    assert_eq!(metadata["has_source_traceability"], false);
    assert_eq!(
        metadata["encoded_input_file"]["path"],
        encoded_path.display().to_string()
    );
    assert_sha256_field(&metadata["encoded_input_file"]["sha256"]);
}

#[test]
fn prepare_data_encoded_mode_applies_bpe_merges() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    let merges_path = dir.path().join("merges.json");
    let output_dir = dir.path().join("prepared-bpe");
    fs::write(&encoded_path, "[1,2,3]\n[1,2,1,2]\n").unwrap();
    fs::write(
        &merges_path,
        serde_json::to_string(&json!({
            "schema": "toy",
            "base_vocab_size": 257,
            "merges": [
                {"pair": [1, 2], "new_token": 257, "frequency": 3}
            ],
            "metadata": {
                "trained_on_instances": 2,
                "training_date": "2026-04-26",
                "max_merges": 1,
                "dict_size": null
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-prepare-data"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--bpe-merges",
            merges_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--train-ratio",
            "0.5",
            "--validate-ratio",
            "0.5",
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-prepare-data --encoded --bpe-merges failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        read_jsonl_sequences(&output_dir.join("all.jsonl")),
        vec![vec![257, 3], vec![257, 257]]
    );

    let metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output_dir.join("metadata.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["mode"], "encoded-bpe");
    assert_eq!(metadata["base_vocab_size"], 258);
    assert_eq!(metadata["bpe_merges"], merges_path.display().to_string());
    assert_eq!(
        metadata["bpe_merges_file"]["path"],
        merges_path.display().to_string()
    );
    assert_sha256_field(&metadata["bpe_merges_file"]["sha256"]);
}

#[test]
fn prepare_data_utf8_mode_applies_bpe_merges() {
    let dir = TempDir::new().unwrap();
    let input_dir = dir.path().join("input");
    let output_dir = dir.path().join("prepared-utf8-bpe");
    let merges_path = dir.path().join("utf8-merges.json");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("sample.txt"), b"aa").unwrap();
    fs::write(
        &merges_path,
        serde_json::to_string(&json!({
            "schema": "utf8",
            "base_vocab_size": 256,
            "merges": [
                {"pair": [97, 97], "new_token": 256, "frequency": 1}
            ],
            "metadata": {
                "trained_on_instances": 1,
                "training_date": "2026-04-26",
                "max_merges": 1,
                "dict_size": null
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let pattern = input_dir.join("*.txt").display().to_string();

    let output = Command::new(bin_path("tct-prepare-data"))
        .args([
            "--mode",
            "utf8",
            "--sources",
            &pattern,
            "--bpe-merges",
            merges_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--train-ratio",
            "1.0",
            "--validate-ratio",
            "0.0",
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-prepare-data utf8+BPE failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "{\"train_sequences\":1,\"validate_sequences\":0,\"all_sequences\":1,\"total_tokens\":1}"
    );
    assert_eq!(
        read_jsonl_sequences(&output_dir.join("all.jsonl")),
        vec![vec![256]]
    );

    let metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output_dir.join("metadata.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["mode"], "utf8");
    assert_eq!(metadata["base_vocab_size"], 257);
    assert_eq!(metadata["bpe_merges"], merges_path.display().to_string());
    assert_eq!(
        metadata["bpe_merges_file"]["path"],
        merges_path.display().to_string()
    );
    assert_sha256_field(&metadata["bpe_merges_file"]["sha256"]);
}

#[test]
fn prepare_data_encoded_rejects_malformed_jsonl_with_line_number() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    let output_dir = dir.path().join("prepared");
    fs::write(&encoded_path, "[1,2,3]\nnot-json\n").unwrap();

    let output = Command::new(bin_path("tct-prepare-data"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Failed to parse line 2"));
    assert!(!output_dir.exists());
}

#[test]
fn transliterate_workflows_outputs_ascii_json_and_report() {
    let dir = TempDir::new().unwrap();
    let input_dir = dir.path().join("workflows");
    let output_dir = dir.path().join("ascii");
    let report_path = dir.path().join("report.txt");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("workflow.json"),
        serde_json::to_string(&json!({
            "name": "déploy ✅",
            "jobs": {
                "build": {
                    "steps": [{"run": "echo \"héllo\""}]
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let output = Command::new(bin_path("tct-transliterate-workflows"))
        .args([
            "--input",
            input_dir.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--report",
            report_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-transliterate-workflows failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_json = fs::read_to_string(output_dir.join("workflow.json")).unwrap();
    assert!(output_json.is_ascii());
    assert!(output_json.contains("deploy check"));
    assert!(output_json.contains("hello"));
    assert!(report_path.is_file());
    assert!(
        fs::read_to_string(report_path)
            .unwrap()
            .contains("Total files processed:")
    );
}

#[test]
fn transliterate_workflows_continues_after_invalid_json() {
    let dir = TempDir::new().unwrap();
    let input_dir = dir.path().join("workflows");
    let output_dir = dir.path().join("ascii");
    let report_path = dir.path().join("report.txt");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("valid.json"),
        serde_json::to_string(&json!({"name": "héllo"})).unwrap(),
    )
    .unwrap();
    fs::write(input_dir.join("broken.json"), "{not-json").unwrap();

    let output = Command::new(bin_path("tct-transliterate-workflows"))
        .args([
            "--input",
            input_dir.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--report",
            report_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "transliteration should continue past malformed JSON: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("broken.json"));
    assert!(output_dir.join("valid.json").is_file());
    assert!(!output_dir.join("broken.json").exists());

    let output_json = fs::read_to_string(output_dir.join("valid.json")).unwrap();
    assert!(output_json.is_ascii());
    assert!(output_json.contains("hello"));
    assert!(
        fs::read_to_string(report_path)
            .unwrap()
            .contains("Total files processed:     1")
    );
}

#[test]
fn analyze_usage_reports_no_matching_instances_before_encoder_build() {
    let dir = TempDir::new().unwrap();
    let schema_path = dir.path().join("schema.json");
    fs::write(
        &schema_path,
        serde_json::to_string(&json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let missing_pattern = dir.path().join("missing/*.json").display().to_string();

    let output = Command::new(bin_path("tct-analyze-usage"))
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--instances",
            &missing_pattern,
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No instance files found"));
}

#[test]
fn schema_backed_cli_success_paths_use_generated_tokenizers() {
    let dir = TempDir::new().unwrap();
    let (schema_path, valid_path, invalid_path, instance_pattern) = simple_schema_fixture(&dir);
    let generated_target_dir = cli_generated_target_dir();

    let measure_output = Command::new(bin_path("tct-measure-tokens"))
        .env("CARGO_TARGET_DIR", &generated_target_dir)
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--instances",
            &instance_pattern,
            "--optimization",
            "Os",
            "--warmup-iterations",
            "1",
            "--track-allocations",
            "--generated-encode-phase-metrics",
            "--generated-encode-hot-path-metrics",
            "--generated-encode-validation-phase-metrics",
            "--cleanup-generated-helper",
        ])
        .output()
        .unwrap();
    assert!(
        measure_output.status.success(),
        "tct-measure-tokens schema-backed run failed: {}",
        String::from_utf8_lossy(&measure_output.stderr)
    );
    let measure_json: serde_json::Value = serde_json::from_slice(&measure_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "failed to parse tct-measure-tokens JSON output: {error}\n{}",
                String::from_utf8_lossy(&measure_output.stdout)
            )
        });
    assert_eq!(measure_json["instances"], 1);
    assert!(measure_json["tokens"].as_u64().unwrap() > 0);
    assert_eq!(measure_json["has_bpe"], false);
    assert_eq!(measure_json["warmup_iterations"], 1);
    assert_eq!(measure_json["base_vocab_size"], 257);
    assert_eq!(measure_json["generated_vocab_size"], 258);
    assert_eq!(measure_json["pad_token"], 257);
    assert_eq!(measure_json["token_length_stats"]["records"], 1);
    assert_eq!(
        measure_json["token_length_stats"]["total_tokens"],
        measure_json["tokens"]
    );
    assert_eq!(
        measure_json["token_length_stats"]["p95_tokens"],
        measure_json["tokens"]
    );
    assert_eq!(
        measure_json["token_length_stats"]["context_window_pass_rate"],
        1.0
    );
    assert!(
        measure_json["timings"]["generated_encode_wall_time_ms"]
            .as_u64()
            .is_some()
    );
    assert!(
        measure_json["timings"]["generated_decode_wall_time_ms"]
            .as_u64()
            .is_some()
    );
    assert!(
        measure_json["timings"]["note"]
            .as_str()
            .unwrap()
            .contains("generated_encode_hot_path_metrics")
    );
    assert!(
        measure_json["timings"]["generated_warmup_wall_time_ms"]
            .as_u64()
            .is_some()
    );
    assert_eq!(
        measure_json["generated_batch_runtime_metrics"]["available"],
        true
    );
    assert!(
        measure_json["generated_batch_runtime_metrics"]["generated_encode_child_wall_time_ms"]
            .as_u64()
            .is_some()
    );
    assert!(
        measure_json["generated_batch_runtime_metrics"]["generated_decode_child_wall_time_ms"]
            .as_u64()
            .is_some()
    );
    assert!(
        measure_json["generated_batch_runtime_metrics"]["generated_encode_parent_stdout_read_parse_ns"]
            .as_u64()
            .is_some()
    );
    assert!(
        measure_json["generated_batch_runtime_metrics"]["generated_decode_parent_stdout_json_parse_ns"]
            .as_u64()
            .is_some()
    );
    assert_eq!(measure_json["allocation_metrics"]["available"], true);
    assert!(
        measure_json["allocation_metrics"]["generated_encode_allocations"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(
        measure_json["allocation_metrics"]["generated_encode_allocated_bytes"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(
        measure_json["allocation_metrics"]["generated_decode_allocations"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(
        measure_json["allocation_metrics"]["generated_decode_allocated_bytes"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert_eq!(measure_json["provenance"]["base_vocab_source"], "grammar");
    assert_eq!(
        measure_json["generated_encode_phase_metrics"]["available"],
        true
    );
    assert_eq!(measure_json["generated_encode_phase_metrics"]["records"], 1);
    assert_eq!(
        measure_json["generated_encode_phase_metrics"]["failures"],
        0
    );
    assert_eq!(
        measure_json["generated_encode_phase_metrics"]["bpe_encode_ns"],
        serde_json::Value::Null
    );
    assert_eq!(
        measure_json["generated_encode_phase_metrics"]["token_sequences_match_primary"],
        true
    );
    assert_eq!(
        measure_json["generated_encode_phase_metrics"]["failed_paths_match_primary"],
        true
    );
    assert_eq!(
        measure_json["generated_encode_hot_path_metrics"]["available"],
        true
    );
    assert_eq!(
        measure_json["generated_encode_hot_path_metrics"]["records"],
        1
    );
    assert_eq!(
        measure_json["generated_encode_hot_path_metrics"]["bpe_encode_ns"],
        serde_json::Value::Null
    );
    assert_eq!(
        measure_json["generated_encode_hot_path_metrics"]["token_count_matches_primary"],
        true
    );
    assert_eq!(
        measure_json["generated_encode_hot_path_metrics"]["token_checksum_matches_primary"],
        true
    );
    assert!(
        measure_json["generated_encode_hot_path_metrics"]["generated_encode_hot_path_allocations"]
            .as_u64()
            .is_some()
    );
    assert!(
        measure_json["generated_encode_hot_path_metrics"]
            ["generated_encode_hot_path_allocated_bytes"]
            .as_u64()
            .is_some()
    );
    assert_eq!(
        measure_json["generated_encode_validation_phase_metrics"]["available"],
        true
    );
    assert_eq!(
        measure_json["generated_encode_validation_phase_metrics"]["records"],
        1
    );
    assert_eq!(
        measure_json["generated_encode_validation_phase_metrics"]["bpe_encode_ns"],
        serde_json::Value::Null
    );
    assert_eq!(
        measure_json["generated_encode_validation_phase_metrics"]["token_count_matches_primary"],
        true
    );
    assert_eq!(
        measure_json["generated_encode_validation_phase_metrics"]["token_checksum_matches_primary"],
        true
    );
    assert_eq!(measure_json["provenance"]["optimization_level"], "Os");
    assert_eq!(
        measure_json["provenance"]["input_files"][0]["path"],
        valid_path.display().to_string()
    );
    assert_sha256_field(&measure_json["provenance"]["schema_file"]["sha256"]);

    let prepared_dir = dir.path().join("prepared");
    let prepare_output = Command::new(bin_path("tct-prepare-data"))
        .env("CARGO_TARGET_DIR", &generated_target_dir)
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--sources",
            &instance_pattern,
            "--output",
            prepared_dir.to_str().unwrap(),
            "--optimization",
            "OVocab",
            "--train-ratio",
            "1.0",
            "--validate-ratio",
            "0.0",
            "--quiet",
        ])
        .output()
        .unwrap();
    assert!(
        prepare_output.status.success(),
        "tct-prepare-data schema-backed run failed: {}",
        String::from_utf8_lossy(&prepare_output.stderr)
    );
    let prepared_metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(prepared_dir.join("metadata.json")).unwrap())
            .unwrap();
    assert_eq!(prepared_metadata["mode"], "tct");
    assert_eq!(prepared_metadata["optimization_level"], "OVocab");
    assert_eq!(
        prepared_metadata["schema_file"]["path"],
        schema_path.display().to_string()
    );

    let schema_merges_path = dir.path().join("schema-merges.json");
    let schema_cache_path = dir.path().join("schema-cache.json");
    let train_output = Command::new(bin_path("tct-train-bpe"))
        .env("CARGO_TARGET_DIR", &generated_target_dir)
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--instances",
            &instance_pattern,
            "--target-vocab",
            "260",
            "--output",
            schema_merges_path.to_str().unwrap(),
            "--optimization",
            "OVocab",
            "--name",
            "simple",
            "--sample-size",
            "1",
            "--cache",
            schema_cache_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        train_output.status.success(),
        "tct-train-bpe schema-backed run failed: {}",
        String::from_utf8_lossy(&train_output.stderr)
    );
    let train_provenance_path = dir.path().join("schema-merges.provenance.json");
    let train_provenance: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(train_provenance_path).unwrap()).unwrap();
    assert_eq!(train_provenance["schema_name"], "simple");
    assert_eq!(train_provenance["tct_version"], env!("CARGO_PKG_VERSION"));
    assert_commit_or_null(&train_provenance["repo_commit"]);
    assert_bool_or_null(&train_provenance["repo_dirty"]);
    assert_eq!(train_provenance["optimization_level"], "OVocab");
    assert_eq!(
        train_provenance["schema_file"]["path"],
        schema_path.display().to_string()
    );
    let train_input_files = train_provenance["input_files"].as_array().unwrap();
    assert_eq!(train_input_files.len(), 2);
    assert!(train_input_files.iter().any(|file| {
        file["path"] == schema_path.display().to_string()
            && file["sha256"]
                .as_str()
                .is_some_and(|value| value.len() == 64)
    }));
    assert!(train_input_files.iter().any(|file| {
        file["path"] == valid_path.display().to_string()
            && file["sha256"]
                .as_str()
                .is_some_and(|value| value.len() == 64)
    }));
    assert_sha256_field(&train_provenance["input_corpus_fingerprint"]);
    assert!(
        train_provenance["timings"]["provenance_hashing_ns"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "schema-backed training sidecar should attribute provenance hashing time"
    );
    assert_eq!(
        train_provenance["token_corpus"]["source"],
        "instance_encoding"
    );
    assert_eq!(train_provenance["token_corpus"]["sequence_count"], 1);
    assert_sha256_field(&train_provenance["token_corpus"]["sequence_fingerprint"]);
    assert_eq!(
        train_provenance["token_corpus"]["sample_seed"],
        serde_json::Value::from(42)
    );
    assert_eq!(
        train_provenance["token_corpus"]["sample_size"],
        serde_json::Value::from(1)
    );
    assert_eq!(train_provenance["token_corpus"]["cache_written"], true);
    assert_eq!(train_provenance["token_corpus"]["cache_hit"], false);
    assert_eq!(
        train_provenance["token_corpus"]["cache_path"],
        schema_cache_path.display().to_string()
    );
    assert_eq!(
        train_provenance["cache_file"]["path"],
        schema_cache_path.display().to_string()
    );
    assert_eq!(
        train_provenance["token_corpus"]["parallel_early_stop"],
        true
    );
    assert_eq!(
        train_provenance["token_corpus"]["truncated_to_sample_size"],
        false
    );
    assert_eq!(
        train_provenance["token_corpus"]["selected_instance_files"][0]["path"],
        valid_path.display().to_string()
    );
    assert_eq!(
        train_provenance["token_corpus"]["accepted_instance_files"][0]["path"],
        valid_path.display().to_string()
    );
    assert_eq!(
        train_provenance["token_corpus"]["rejected_instance_files"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let cached_merges_path = dir.path().join("schema-merges-cached.json");
    let cached_train_output = Command::new(bin_path("tct-train-bpe"))
        .env("CARGO_TARGET_DIR", &generated_target_dir)
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--instances",
            &instance_pattern,
            "--target-vocab",
            "260",
            "--output",
            cached_merges_path.to_str().unwrap(),
            "--optimization",
            "OVocab",
            "--name",
            "simple",
            "--sample-size",
            "1",
            "--cache",
            schema_cache_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        cached_train_output.status.success(),
        "cached tct-train-bpe schema-backed run failed: {}",
        String::from_utf8_lossy(&cached_train_output.stderr)
    );
    let cached_train_provenance_path = dir.path().join("schema-merges-cached.provenance.json");
    let cached_train_provenance: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(cached_train_provenance_path).unwrap()).unwrap();
    assert_eq!(cached_train_provenance["token_corpus"]["source"], "cache");
    assert!(
        cached_train_provenance["timings"]["provenance_hashing_ns"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "cached training sidecar should attribute provenance hashing time"
    );
    assert_eq!(cached_train_provenance["token_corpus"]["cache_hit"], true);
    assert_eq!(
        cached_train_provenance["token_corpus"]["cache_written"],
        false
    );
    assert_eq!(
        cached_train_provenance["token_corpus"]["sequence_fingerprint"],
        train_provenance["token_corpus"]["sequence_fingerprint"]
    );
    assert_eq!(
        cached_train_provenance["token_corpus"]["accepted_instance_files"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let analyze_output = Command::new(bin_path("tct-analyze-usage"))
        .env("CARGO_TARGET_DIR", &generated_target_dir)
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--instances",
            &instance_pattern,
            "--format",
            "json",
            "--name",
            "simple",
        ])
        .output()
        .unwrap();
    assert!(
        analyze_output.status.success(),
        "tct-analyze-usage schema-backed run failed: {}",
        String::from_utf8_lossy(&analyze_output.stderr)
    );
    let analyze_json: serde_json::Value = serde_json::from_slice(&analyze_output.stdout)
        .unwrap_or_else(|error| {
            panic!(
                "failed to parse tct-analyze-usage JSON output: {error}\n{}",
                String::from_utf8_lossy(&analyze_output.stdout)
            )
        });
    assert_eq!(analyze_json["total_instances"], 1);
    assert!(analyze_json["total_tokens"].as_u64().unwrap() > 0);
    assert_eq!(analyze_json["base_vocab_size"], 257);
    assert_eq!(analyze_json["provenance"]["schema_name"], "simple");
    assert_eq!(analyze_json["provenance"]["base_vocab_source"], "grammar");
    assert_eq!(
        analyze_json["provenance"]["input_files"][0]["path"],
        valid_path.display().to_string()
    );

    let mut validate_child = Command::new(bin_path("tct-validate-server"))
        .env("CARGO_TARGET_DIR", &generated_target_dir)
        .args(["--schema", schema_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = validate_child.stdin.take().unwrap();
        writeln!(stdin, "{}", valid_path.display()).unwrap();
        writeln!(stdin, "{}", invalid_path.display()).unwrap();
        writeln!(stdin).unwrap();
    }
    let validate_output = validate_child.wait_with_output().unwrap();
    assert!(
        validate_output.status.success(),
        "tct-validate-server schema-backed run failed: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );
    let validate_stdout = String::from_utf8_lossy(&validate_output.stdout);
    assert!(
        validate_stdout.contains(&format!("OK {}", valid_path.display())),
        "validate-server did not accept valid fixture:\n{validate_stdout}"
    );
    assert!(
        validate_stdout.contains(&format!("ERROR {}", invalid_path.display())),
        "validate-server did not reject invalid fixture:\n{validate_stdout}"
    );
}

#[test]
fn train_bpe_requires_a_stopping_criterion() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    let output_path = dir.path().join("merges.json");
    fs::write(&encoded_path, "[1,2,3]\n").unwrap();

    let output = Command::new(bin_path("tct-train-bpe"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
            "--base-vocab",
            "257",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("stopping criterion"));
    assert!(stderr.contains("--target-vocab 2048"));
    assert!(stderr.contains("--checkpoints 512,1024,2048,4096"));
    assert!(!stderr.contains("--target-vocab 5000"));
}

#[test]
fn train_bpe_encoded_mode_writes_merge_table() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    let output_path = dir.path().join("toy-merges.json");
    fs::write(&encoded_path, "[1,2,1,2]\n[1,2,3]\n").unwrap();

    // Generated VOCAB_SIZE includes PAD + always-reserved BOS/EOS:
    // base 257 + 2 merges + 1 pad + 2 specials = 262.
    let output = Command::new(bin_path("tct-train-bpe"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--base-vocab",
            "257",
            "--target-vocab",
            "262",
            "--output",
            output_path.to_str().unwrap(),
            "--name",
            "toy",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-train-bpe --encoded failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let merge_table: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_path).unwrap()).unwrap();
    assert_eq!(merge_table["schema"], "toy");
    assert_eq!(merge_table["base_vocab_size"], 257);
    assert_eq!(merge_table["metadata"]["trained_on_instances"], 2);
    assert_eq!(
        merge_table["metadata"]["dict_size"],
        serde_json::Value::Null
    );

    let merges = merge_table["merges"].as_array().unwrap();
    assert_eq!(merges.len(), 2);
    assert_eq!(merges[0]["pair"], json!([1, 2]));
    assert_eq!(merges[0]["new_token"], 257);
    assert_eq!(merges[0]["frequency"], 3);

    let provenance_path = dir.path().join("toy-merges.provenance.json");
    let provenance: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(provenance_path).unwrap()).unwrap();
    assert_eq!(provenance["schema_name"], "toy");
    assert_eq!(provenance["tct_version"], env!("CARGO_PKG_VERSION"));
    assert_commit_or_null(&provenance["repo_commit"]);
    assert_bool_or_null(&provenance["repo_dirty"]);
    assert_eq!(
        provenance["encoded_input"],
        encoded_path.display().to_string()
    );
    assert_eq!(provenance["base_vocab_source"], "explicit");
    assert_eq!(provenance["target_vocab"], 262);
    assert_eq!(provenance["generated_merges"], 2);
    assert_eq!(
        provenance["encoded_input_file"]["path"],
        encoded_path.display().to_string()
    );
    assert_eq!(provenance["input_files"].as_array().unwrap().len(), 1);
    assert_eq!(
        provenance["input_files"][0]["path"],
        encoded_path.display().to_string()
    );
    assert_sha256_field(&provenance["input_files"][0]["sha256"]);
    assert_sha256_field(&provenance["input_corpus_fingerprint"]);
    assert_eq!(provenance["token_corpus"]["source"], "encoded_input");
    assert_eq!(provenance["token_corpus"]["sequence_count"], 2);
    assert_sha256_field(&provenance["token_corpus"]["sequence_fingerprint"]);
    assert_eq!(
        provenance["token_corpus"]["sample_seed"],
        serde_json::Value::Null
    );
    assert_eq!(provenance["token_corpus"]["cache_hit"], false);
    assert_eq!(provenance["token_corpus"]["cache_written"], false);
    assert_eq!(
        provenance["token_corpus"]["accepted_instance_files"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(provenance["output_files"].as_array().unwrap().len(), 1);
    assert_sha256_field(&provenance["encoded_input_file"]["sha256"]);
    assert_sha256_field(&provenance["output_files"][0]["sha256"]);
}

#[test]
fn train_bpe_records_clean_repo_state_before_writing_outputs() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    fs::write(&encoded_path, "[1,2,1,2]\n[1,2,3]\n").unwrap();

    for args in [
        vec!["init", "--quiet"],
        vec!["config", "user.email", "tct@example.invalid"],
        vec!["config", "user.name", "TCT Test"],
        vec!["add", "encoded.jsonl"],
        vec!["commit", "--quiet", "-m", "baseline"],
    ] {
        let output = Command::new("git")
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap_or_else(|error| panic!("failed to run git: {error}"));
        assert!(
            output.status.success(),
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new(bin_path("tct-train-bpe"))
        .current_dir(dir.path())
        .args([
            "--encoded",
            "encoded.jsonl",
            "--base-vocab",
            "257",
            "--target-vocab",
            "260",
            "--output",
            "toy-merges.json",
            "--name",
            "toy",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-train-bpe --encoded failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let provenance_path = dir.path().join("toy-merges.provenance.json");
    let provenance: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(provenance_path).unwrap()).unwrap();
    assert_eq!(
        provenance["repo_dirty"], false,
        "generated output files should not make the recorded source state dirty"
    );

    let status = Command::new("git")
        .current_dir(dir.path())
        .args(["status", "--porcelain"])
        .output()
        .unwrap();
    assert!(status.status.success());
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("?? toy-merges.json"));
    assert!(stdout.contains("?? toy-merges.provenance.json"));
}

#[test]
fn train_bpe_encoded_mode_honors_reproducible_training_date() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    let output_path = dir.path().join("toy-merges.json");
    fs::write(&encoded_path, "[1,2,1,2]\n[1,2,3]\n").unwrap();

    let output = Command::new(bin_path("tct-train-bpe"))
        .env("TCT_BPE_TRAINING_DATE", "2026-05-01")
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--base-vocab",
            "257",
            "--target-vocab",
            "260",
            "--output",
            output_path.to_str().unwrap(),
            "--name",
            "toy",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-train-bpe --encoded failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let merge_table: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_path).unwrap()).unwrap();
    assert_eq!(merge_table["metadata"]["training_date"], "2026-05-01");
}

#[test]
fn train_bpe_encoded_mode_autodetects_base_vocab_from_metadata() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("all.jsonl");
    let metadata_path = dir.path().join("metadata.json");
    let output_path = dir.path().join("toy-merges.json");
    fs::write(&encoded_path, "[1,2,1,2]\n[1,2,3]\n").unwrap();
    fs::write(
        &metadata_path,
        serde_json::to_string(&json!({"base_vocab_size": 257})).unwrap(),
    )
    .unwrap();

    // See train_bpe_encoded_mode_writes_merge_table: 257 base + 2 merges + PAD/BOS/EOS = 262.
    let output = Command::new(bin_path("tct-train-bpe"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--target-vocab",
            "262",
            "--output",
            output_path.to_str().unwrap(),
            "--name",
            "toy",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-train-bpe metadata autodetect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Auto-detected base_vocab_size: 257"));

    let merge_table: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_path).unwrap()).unwrap();
    assert_eq!(merge_table["schema"], "toy");
    assert_eq!(merge_table["base_vocab_size"], 257);
    assert_eq!(merge_table["metadata"]["trained_on_instances"], 2);
    assert_eq!(merge_table["merges"][0]["new_token"], 257);

    let provenance_path = dir.path().join("toy-merges.provenance.json");
    let provenance: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(provenance_path).unwrap()).unwrap();
    assert_eq!(provenance["base_vocab_source"], "prepare-data-metadata");
    assert_eq!(
        provenance["prepare_data_metadata_file"]["path"],
        metadata_path.display().to_string()
    );
    assert_sha256_field(&provenance["prepare_data_metadata_file"]["sha256"]);
    assert_eq!(provenance["input_files"].as_array().unwrap().len(), 1);
    assert_sha256_field(&provenance["input_corpus_fingerprint"]);
}

#[test]
fn train_bpe_encoded_checkpoints_write_expected_files() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("all.jsonl");
    let output_path = dir.path().join("toy.json");
    fs::write(&encoded_path, "[1,2,1,2]\n[1,2,3]\n").unwrap();

    let output = Command::new(bin_path("tct-train-bpe"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--base-vocab",
            "257",
            "--checkpoints",
            // Min VOCAB_SIZE with base 257 + PAD/BOS/EOS is 260; checkpoints
            // must sit above that to produce real merges (2 and 3 merges here).
            "262,263",
            "--output",
            output_path.to_str().unwrap(),
            "--name",
            "toy",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-train-bpe checkpoints failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for checkpoint in [262, 263] {
        let checkpoint_path = dir.path().join(format!("toy-{checkpoint}.json"));
        assert!(checkpoint_path.is_file());
        let merge_table: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(checkpoint_path).unwrap()).unwrap();
        assert_eq!(merge_table["schema"], "toy");
        assert_eq!(merge_table["base_vocab_size"], 257);
        let merges = merge_table["merges"].as_array().unwrap();
        assert_eq!(
            merge_table["metadata"]["max_merges"],
            serde_json::Value::from(merges.len())
        );
        assert!(
            !merges.is_empty(),
            "checkpoint {checkpoint} should include merges above the reserved minimum"
        );
        for (offset, merge) in merges.iter().enumerate() {
            assert_eq!(merge["new_token"], serde_json::Value::from(257 + offset));
        }
    }

    let provenance_path = dir.path().join("toy.provenance.json");
    let provenance: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(provenance_path).unwrap()).unwrap();
    assert_eq!(provenance["checkpoints"], json!([262, 263]));
    assert_eq!(provenance["output_files"].as_array().unwrap().len(), 2);
    assert_sha256_field(&provenance["output_files"][0]["sha256"]);
    assert_sha256_field(&provenance["output_files"][1]["sha256"]);
}

#[test]
fn train_bpe_encoded_rejects_malformed_jsonl_with_line_number() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    let output_path = dir.path().join("merges.json");
    fs::write(&encoded_path, "[1,2,3]\nnot-json\n").unwrap();

    let output = Command::new(bin_path("tct-train-bpe"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--base-vocab",
            "257",
            "--target-vocab",
            "260",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Failed to parse line 2"));
    assert!(!output_path.exists());
}

#[test]
fn train_bpe_encoded_rejects_tokens_outside_base_vocab() {
    let dir = TempDir::new().unwrap();
    let encoded_path = dir.path().join("encoded.jsonl");
    let output_path = dir.path().join("merges.json");
    let provenance_path = dir.path().join("merges.provenance.json");
    fs::write(&encoded_path, "[999,1000,999,1000]\n").unwrap();

    let output = Command::new(bin_path("tct-train-bpe"))
        .args([
            "--encoded",
            encoded_path.to_str().unwrap(),
            "--base-vocab",
            "257",
            "--target-vocab",
            "260",
            "--output",
            output_path.to_str().unwrap(),
            "--name",
            "toy",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("line 1"),
        "stderr should include encoded line context:\n{stderr}"
    );
    assert!(
        stderr.contains("token position 0"),
        "stderr should include token position:\n{stderr}"
    );
    assert!(
        stderr.contains("token id 999"),
        "stderr should include invalid token id:\n{stderr}"
    );
    assert!(
        stderr.contains("base vocabulary size 257"),
        "stderr should include base vocabulary:\n{stderr}"
    );
    assert!(!output_path.exists());
    assert!(!provenance_path.exists());
}

#[test]
fn train_bpe_cache_rejects_tokens_outside_base_vocab() {
    let dir = TempDir::new().unwrap();
    let (schema_path, _valid_path, _invalid_path, pattern) = simple_schema_fixture(&dir);
    let cache_path = dir.path().join("tokens-cache.json");
    let output_path = dir.path().join("merges.json");
    let provenance_path = dir.path().join("merges.provenance.json");
    fs::write(&cache_path, "[[999,1000,999,1000]]").unwrap();

    let output = Command::new(bin_path("tct-train-bpe"))
        .args([
            "--schema",
            schema_path.to_str().unwrap(),
            "--instances",
            &pattern,
            "--cache",
            cache_path.to_str().unwrap(),
            "--target-vocab",
            "260",
            "--output",
            output_path.to_str().unwrap(),
            "--name",
            "toy",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cache"),
        "stderr should include cache context:\n{stderr}"
    );
    assert!(
        stderr.contains("sequence 1"),
        "stderr should include sequence index:\n{stderr}"
    );
    assert!(
        stderr.contains("token id 999"),
        "stderr should include invalid token id:\n{stderr}"
    );
    assert!(
        stderr.contains("base vocabulary size"),
        "stderr should include base vocabulary:\n{stderr}"
    );
    assert!(!output_path.exists());
    assert!(!provenance_path.exists());
}
