//! Python DecodeState terminal-key cursor integration via an isolated wheel.
//!
//! Builds a streaming+decimal wheel for `schemas/synthetic/terminal_key_cursor.json`
//! under `target/terminal-key-cursor-wheel/`, installs it into a throwaway venv,
//! and proves `begin_decode_state().current_terminal_key()` /
//! `current_terminal_path()` match the Rust cursor contract (key before first
//! host-value byte and mid-value; None on complete/empty/dead).
//!
//! Requires maturin + Python packaging tooling. Does not touch any
//! installed wheel.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tct::wheel_builder::WheelConfig;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn isolated_output_dir() -> PathBuf {
    repo_root().join("target").join("terminal-key-cursor-wheel")
}

#[test]
#[ignore = "requires maturin/Python wheel tooling; run with --ignored"]
fn pyo3_decode_state_exposes_terminal_key_cursor() {
    let output_dir = isolated_output_dir();
    let _ = fs::remove_dir_all(&output_dir);
    fs::create_dir_all(&output_dir).expect("create isolated wheel output dir");

    let schema_path = repo_root().join("schemas/synthetic/terminal_key_cursor.json");
    assert!(
        schema_path.is_file(),
        "synthetic schema missing: {}",
        schema_path.display()
    );

    let config = WheelConfig::new(
        schema_path.to_str().expect("utf-8 schema path"),
        "TerminalKeyCursor",
        "tct-terminal-key-cursor",
    )
    .with_version("0.0.0")
    .with_description("Isolated terminal-key cursor DecodeState wheel fixture")
    .with_streaming_encode(true)
    .with_decimal_number_encoding()
    .with_quiet(true)
    .with_output_dir(output_dir.to_str().expect("utf-8 output dir"));

    let wheel_path = tct::wheel_builder::build_wheel(&config)
        .expect("isolated terminal-key cursor wheel build should succeed");
    assert!(
        wheel_path.is_file(),
        "wheel artifact missing: {}",
        wheel_path.display()
    );
    eprintln!("wheel artifact: {}", wheel_path.display());

    smoke_terminal_key_cursor_wheel(&wheel_path, &output_dir);
}

fn smoke_terminal_key_cursor_wheel(wheel_path: &Path, output_dir: &Path) {
    let venv_dir = output_dir.join("generated-wheel-venv");
    let _ = fs::remove_dir_all(&venv_dir);

    let venv_output = Command::new("python3")
        .args(["-m", "venv", venv_dir.to_str().unwrap()])
        .output()
        .expect("failed to invoke python3 -m venv");
    assert!(
        venv_output.status.success(),
        "failed to create isolated wheel venv:\nstdout:\n{}\nstderr:\n{}",
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
        "failed to install isolated wheel:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install_output.stdout),
        String::from_utf8_lossy(&install_output.stderr)
    );

    // Multi-call instance: second call exposes optional decimal host leaves
    // (`limit`, then `skip`). Roles locate the first host-value span without
    // consulting gold JSON for key identity — the DecodeState API must report
    // the schema property from decoder cursor state alone.
    let instance = serde_json::json!({
        "calls": [
            {
                "name": "tool_alpha",
                "arguments": { "count_only": true }
            },
            {
                "name": "tool_beta",
                "arguments": { "limit": 1, "skip": 0 }
            }
        ]
    });
    let instance_path = output_dir.join("fixture_instance.json");
    fs::write(
        &instance_path,
        serde_json::to_string_pretty(&instance).expect("serialize fixture"),
    )
    .expect("write fixture instance");

    let smoke_script = r#"
import json
import os
import tct_terminal_key_cursor as tct

with open(os.environ["TCT_TERMINAL_KEY_INSTANCE"], encoding="utf-8") as handle:
    source = handle.read()
source_value = json.loads(source)

# Encode/decode round-trip still works for the full instance.
tokens = tct.encode(source)
assert tokens, "encode returned no tokens"
assert all(0 <= token < tct.vocab_size() for token in tokens), (tokens, tct.vocab_size())
decoded_json, consumed, total = tct.decode(tokens)
assert consumed == total, (consumed, total)
assert json.loads(decoded_json) == source_value, decoded_json

# Locate first host-value span via strict roles (ROLE_VALUE / ROLE_VALUE_END).
roles, complete = tct.strict_prefix_roles(tokens)
assert complete is True
assert len(roles) == len(tokens), (len(roles), len(tokens))

# role_export constants are not re-exported as Python names; match generated
# role_export (STRUCTURE=0, VALUE=1, VALUE_END=2, DISCRIMINATOR=3).
ROLE_VALUE = 1
ROLE_VALUE_END = 2
value_end_positions = [i for i, role in enumerate(roles) if role == ROLE_VALUE_END]
assert len(value_end_positions) >= 2, (value_end_positions, roles)

first_value_end = value_end_positions[0]
first_value_byte = first_value_end
while first_value_byte > 0 and roles[first_value_byte - 1] == ROLE_VALUE:
    first_value_byte -= 1

before = tokens[:first_value_byte]
mid = tokens[: first_value_byte + 1]
after_end = tokens[: first_value_end + 1]

# Full-prefix DecodeState: feed tokens then query cursor methods.
def state_for(prefix):
    state = tct.begin_decode_state()
    for token in prefix:
        status = state.try_append(token)
        assert status in ("prefix_ok", "complete", "dead"), status
    return state

state_before = state_for(before)
key_before = state_before.current_terminal_key()
path_before = state_before.current_terminal_path()
assert key_before == "limit", (key_before, path_before, before)
assert path_before is not None and "limit" in path_before, path_before
assert "arguments" in path_before or path_before.endswith("limit"), path_before

state_mid = state_for(mid)
assert state_mid.current_terminal_key() == "limit", state_mid.current_terminal_key()
assert state_mid.current_terminal_path() is not None
assert "limit" in state_mid.current_terminal_path()

# After VALUE_END the cursor must leave the limit host leaf.
state_after = state_for(after_end)
assert state_after.current_terminal_key() != "limit", state_after.current_terminal_key()

# Complete stream and empty prefix → None.
state_full = state_for(tokens)
assert state_full.status() == "complete"
assert state_full.current_terminal_key() is None
assert state_full.current_terminal_path() is None

empty = tct.begin_decode_state()
assert empty.current_terminal_key() is None
assert empty.current_terminal_path() is None

# Fail-closed on a corrupted host-value byte: strict terminal-key replay must
# not invent a key. (Recovery classify may still report prefix_ok for some
# high tokens, so do not require status=="dead" here.)
corrupt = state_for(before)
corrupt.try_append(2**32 - 1)
assert corrupt.current_terminal_key() is None
assert corrupt.current_terminal_path() is None

# Explicit dead via empty root-token restriction (any first token is dead).
restricted = tct.begin_decode_state_restricted([])
assert restricted.try_append(tokens[0]) == "dead"
assert restricted.current_terminal_key() is None
assert restricted.current_terminal_path() is None

print("terminal_key_cursor_wheel_ok")
print("wheel_path=" + os.environ.get("TCT_TERMINAL_KEY_WHEEL", ""))
"#;

    let smoke_output = Command::new(&python)
        .arg("-c")
        .arg(smoke_script)
        .env("TCT_TERMINAL_KEY_INSTANCE", &instance_path)
        .env("TCT_TERMINAL_KEY_WHEEL", wheel_path)
        .output()
        .expect("failed to invoke terminal-key wheel Python smoke");
    assert!(
        smoke_output.status.success(),
        "terminal-key wheel Python smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&smoke_output.stdout),
        String::from_utf8_lossy(&smoke_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&smoke_output.stdout);
    assert!(
        stdout.contains("terminal_key_cursor_wheel_ok"),
        "missing success marker in smoke stdout:\n{stdout}"
    );

    // Fixture remains a plain object (sanity for future edits).
    let source: Value = serde_json::from_str(
        &fs::read_to_string(&instance_path).expect("fixture should be readable"),
    )
    .expect("fixture should be valid JSON");
    assert!(source.is_object());
}
