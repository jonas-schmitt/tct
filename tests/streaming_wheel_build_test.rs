//! Test streaming wheel build

use serde_json::Value;
use std::fs;
use std::process::Command;
use tct::wheel_builder::WheelConfig;

#[test]
#[ignore = "requires maturin/Python wheel tooling; run scripts/testing/test-wheel-builds.sh"]
fn test_streaming_wheel_build() {
    let _ = fs::remove_dir_all("/tmp/streaming-wheel-test");

    let config = WheelConfig::new(
        "schemas/popular/kubernetes.json",
        "Kubernetes",
        "tct-kubernetes-streaming",
    )
    .with_version("1.0.0")
    .with_bpe_merges("schemas/bpe/kubernetes-tct-bpe-500.json")
    .with_streaming_encode(true)
    .with_output_dir("/tmp/streaming-wheel-test");

    let result = tct::wheel_builder::build_wheel(&config);

    match &result {
        Ok(path) => eprintln!("SUCCESS: {}", path.display()),
        Err(e) => eprintln!("ERROR: {}", e),
    }

    assert!(
        result.is_ok(),
        "Streaming wheel build should succeed: {:?}",
        result.err()
    );

    let wheel_path = result.unwrap();
    smoke_import_generated_wheel(&wheel_path);
}

fn smoke_import_generated_wheel(wheel_path: &std::path::Path) {
    let repo_root = std::env::current_dir().expect("repo root should be current directory");
    let instance_path =
        repo_root.join("tests/data/instances/kubernetes/synthetic/public_configmap.json");
    let venv_dir = std::path::Path::new("/tmp/streaming-wheel-test/generated-wheel-venv");
    let _ = fs::remove_dir_all(venv_dir);

    let venv_output = Command::new("python3")
        .args(["-m", "venv", venv_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke python3 -m venv");
    assert!(
        venv_output.status.success(),
        "failed to create generated wheel smoke venv:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&venv_output.stdout),
        String::from_utf8_lossy(&venv_output.stderr)
    );

    let python = venv_dir.join("bin/python");
    let install_output = Command::new(&python)
        .args([
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-deps",
            wheel_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to invoke venv pip install");
    assert!(
        install_output.status.success(),
        "failed to install generated wheel in smoke venv:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install_output.stdout),
        String::from_utf8_lossy(&install_output.stderr)
    );

    let smoke_script = r#"
import json
import os
import email.parser
import pathlib
import zipfile
import tct_kubernetes_streaming as tct

with zipfile.ZipFile(os.environ["TCT_WHEEL_SMOKE_WHEEL"]) as wheel:
    licenses = [name for name in wheel.namelist() if pathlib.PurePosixPath(name).name == "LICENSE"]
    assert licenses, "generated wheel must contain LICENSE"
    expected_license = pathlib.Path(os.environ["TCT_WHEEL_SMOKE_LICENSE"]).read_bytes()
    assert all(wheel.read(name) == expected_license for name in licenses), licenses
    notices = [name for name in wheel.namelist() if pathlib.PurePosixPath(name).name == "NOTICE"]
    assert notices, "generated wheel must contain NOTICE"
    expected_notice = pathlib.Path(os.environ["TCT_WHEEL_SMOKE_NOTICE"]).read_bytes()
    assert all(wheel.read(name) == expected_notice for name in notices), notices
    metadata_name = next(name for name in wheel.namelist() if name.endswith("/METADATA"))
    package_metadata = email.parser.BytesParser().parsebytes(wheel.read(metadata_name))
    assert package_metadata["License"] == "Apache-2.0", package_metadata

with open(os.environ["TCT_WHEEL_SMOKE_INSTANCE"], encoding="utf-8") as handle:
    source = handle.read()

source_value = json.loads(source)

tokens = tct.encode(source)
assert tokens, "raw encode returned no tokens"
assert all(0 <= token < tct.vocab_size() for token in tokens), (
    tokens,
    tct.vocab_size(),
)

decoded_json, consumed, total = tct.decode(tokens)
assert consumed == total, (consumed, total)
assert total >= len(tokens), (total, len(tokens))
assert json.loads(decoded_json) == source_value, decoded_json

sequence = tct.encode_sequence(source)
assert sequence[0] == tct.bos_token(), sequence
assert sequence[1] == tct.schema_token(), sequence
assert sequence[-1] == tct.eos_token(), sequence
assert all(0 <= token < tct.sequence_vocab_size() for token in sequence), (
    sequence,
    tct.sequence_vocab_size(),
)

decoded_sequence_json, sequence_consumed, sequence_total = tct.decode_sequence(sequence)
assert sequence_consumed == len(sequence), (sequence_consumed, len(sequence))
assert sequence_total == len(sequence), (sequence_total, len(sequence))
assert json.loads(decoded_sequence_json) == source_value, decoded_sequence_json

strict_roles, strict_complete = tct.strict_prefix_roles(tokens)
assert strict_complete is True
assert len(strict_roles) >= len(tokens), (len(strict_roles), len(tokens))
prefix_roles, prefix_complete = tct.strict_prefix_roles(tokens[: max(1, len(tokens) // 2)])
assert prefix_complete is False
assert prefix_roles
try:
    tct.strict_prefix_roles([tct.pad_token()])
except RuntimeError:
    pass
else:
    raise AssertionError("strict prefix roles accepted raw PAD")

metadata = json.loads(tct.sequence_metadata_json())
assert metadata["sequence_vocab_size"] == tct.sequence_vocab_size(), metadata
assert metadata["sequence_reserved_tokens"]["bos_token"] == tct.bos_token(), metadata
assert metadata["sequence_reserved_tokens"]["eos_token"] == tct.eos_token(), metadata
assert metadata["schema_token"] == tct.schema_token(), metadata

prefix_json, fields_decoded, complete = tct.decode_prefix(tokens[: max(1, len(tokens) // 2)])
assert isinstance(prefix_json, str)
assert isinstance(fields_decoded, int)
assert isinstance(complete, bool)
"#;
    let smoke_output = Command::new(&python)
        .arg("-c")
        .arg(smoke_script)
        .env("TCT_WHEEL_SMOKE_INSTANCE", &instance_path)
        .env("TCT_WHEEL_SMOKE_WHEEL", wheel_path)
        .env("TCT_WHEEL_SMOKE_LICENSE", repo_root.join("LICENSE"))
        .env("TCT_WHEEL_SMOKE_NOTICE", repo_root.join("NOTICE"))
        .output()
        .expect("failed to invoke generated wheel import smoke");
    assert!(
        smoke_output.status.success(),
        "generated wheel import smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&smoke_output.stdout),
        String::from_utf8_lossy(&smoke_output.stderr)
    );

    let source: Value = serde_json::from_str(
        &fs::read_to_string(&instance_path).expect("smoke instance should be readable"),
    )
    .expect("smoke instance should be valid JSON");
    assert!(
        source.is_object(),
        "smoke instance should stay representative"
    );
}
