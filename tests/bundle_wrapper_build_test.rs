//! Test assembled pure-Python bundle wrapper wheel builds.

use serde_json::json;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[ignore = "requires Python packaging tooling; run scripts/testing/test-wheel-builds.sh"]
fn test_bundle_wrapper_wheel_build() {
    let dir = TempDir::new().unwrap();
    let spec_path = dir.path().join("bundle-spec.json");
    let output_dir = dir.path().join("bundle-output");
    let wheel_dir = dir.path().join("dist");

    fs::write(
        dir.path().join("tct_alpha.py"),
        r#"
def encode(json_str):
    return [0, 2]

def decode(tokens):
    tokens = list(tokens)
    if tokens != [0, 2]:
        raise ValueError(f"bad alpha payload: {tokens}")
    return '{"schema":"alpha"}', len(tokens), len(tokens)
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("tct_beta.py"),
        r#"
def encode(json_str):
    return [0, 3]

def decode(tokens):
    tokens = list(tokens)
    if tokens != [0, 3]:
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
                    "schema_sentinel": "<|schema:alpha|>"
                },
                {
                    "schema_id": "beta",
                    "module_name": "tct_beta",
                    "schema_token": 4,
                    "raw_vocab_size": 5,
                    "raw_pad_token": 4,
                    "global_payload_start": 20,
                    "bpe_enabled": true,
                    "schema_sentinel": "<|schema:beta|>"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let wrapper_output = Command::new(env!("CARGO_BIN_EXE_tct-build-bundle-wrapper"))
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
        wrapper_output.status.success(),
        "bundle wrapper source generation failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&wrapper_output.stdout),
        String::from_utf8_lossy(&wrapper_output.stderr)
    );

    let package_dir = output_dir.join("tct-bundle-demo");
    fs::create_dir_all(&wheel_dir).unwrap();
    let wheel_output = Command::new("python3")
        .args([
            "-m",
            "pip",
            "wheel",
            "--no-deps",
            "--no-build-isolation",
            package_dir.to_str().unwrap(),
            "-w",
            wheel_dir.to_str().unwrap(),
        ])
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .output()
        .unwrap();
    assert!(
        wheel_output.status.success(),
        "pure-Python bundle wheel build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&wheel_output.stdout),
        String::from_utf8_lossy(&wheel_output.stderr)
    );

    let wheel_path = fs::read_dir(&wheel_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("whl"))
        .expect("bundle wrapper wheel should be built");

    let pythonpath = format!("{}:{}", wheel_path.display(), dir.path().display());
    let smoke_script = r#"
import json
import email.parser
import pathlib
import sys
import zipfile
import tct_bundle_demo

with zipfile.ZipFile(sys.argv[1]) as wheel:
    licenses = [name for name in wheel.namelist() if name.endswith("/LICENSE")]
    assert len(licenses) == 1, licenses
    assert wheel.read(licenses[0]) == pathlib.Path(sys.argv[2]).read_bytes()
    notices = [name for name in wheel.namelist() if name.endswith("/NOTICE")]
    assert len(notices) == 1, notices
    assert wheel.read(notices[0]) == pathlib.Path(sys.argv[3]).read_bytes()
    metadata_name = next(name for name in wheel.namelist() if name.endswith("/METADATA"))
    package_metadata = email.parser.BytesParser().parsebytes(wheel.read(metadata_name))
    assert package_metadata["License"] == "Apache-2.0", package_metadata

metadata = json.loads(tct_bundle_demo.sequence_bundle_metadata_json())
assert metadata["vocab_size"] == 24, metadata

alpha = tct_bundle_demo.encode_sequence("alpha", "{}")
assert alpha == [1, 3, 10, 12, 2], alpha
assert tct_bundle_demo.decode_sequence(alpha + [0]) == (
    "alpha",
    '{"schema":"alpha"}',
    5,
    6,
)

beta = tct_bundle_demo.encode_sequence("beta", "{}")
assert beta == [1, 4, 20, 23, 2], beta
assert tct_bundle_demo.decode_sequence(beta) == (
    "beta",
    '{"schema":"beta"}',
    5,
    5,
)
"#;
    let import_output = Command::new("python3")
        .arg("-c")
        .arg(smoke_script)
        .arg(&wheel_path)
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/LICENSE"))
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/NOTICE"))
        .env("PYTHONPATH", pythonpath)
        .output()
        .unwrap();
    assert!(
        import_output.status.success(),
        "built bundle wrapper wheel import smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&import_output.stdout),
        String::from_utf8_lossy(&import_output.stderr)
    );
}
