//! Regression coverage for schema names that collide with generated locals and
//! for closed, zero-argument object schemas.

use serde_json::json;
use std::fs;
use std::process::Command;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

#[test]
fn generated_tokens_field_and_empty_object_compile_and_round_trip() {
    let schema = json!({
        "type": "object",
        "properties": {
            "tokens": {"type": "integer"},
            "arguments": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        "required": ["tokens", "arguments"],
        "additionalProperties": false
    });
    let generated_code = TctCompiler::new(schema, "CollisionAndEmptyObject")
        .expect("schema should be supported")
        .with_streaming_encode()
        .compile()
        .expect("schema should compile")
        .generate_code()
        .expect("code generation should succeed");

    let crate_dir = tempfile::tempdir().expect("generated crate tempdir should be created");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("generated crate src should be created");

    let tct_path = std::env::current_dir().expect("TCT repo root should resolve");
    let cargo_toml = GeneratedCargoToml::new("generated-local-identifier-regression")
        .with_preserve_order()
        .with_rustc_hash()
        .with_tct_path(tct_path.display().to_string())
        .with_bin("generated-local-identifier-regression", "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("generated Cargo.toml should be written");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("workspace Cargo.lock should be copied");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
    let expected = serde_json::json!({{"tokens": 7, "arguments": {{}}}});
    let typed: CollisionAndEmptyObject = serde_json::from_value(expected.clone())
        .map_err(|error| format!("valid input was rejected: {{error}}"))?;
    let mut token_ids = Vec::new();
    typed.encode(&mut token_ids);
    let mut token_iter = token_ids.into_iter();
    let decoded = CollisionAndEmptyObject::decode(&mut token_iter)
        .map_err(|error| format!("decode failed: {{error}}"))?;
    if token_iter.next().is_some() {{
        return Err("decode left an unconsumed token".to_string());
    }}
    let actual = serde_json::to_value(decoded)
        .map_err(|error| format!("decoded value did not serialize: {{error}}"))?;
    if actual != expected {{
        return Err(format!("round trip mismatch: expected {{expected}}, got {{actual}}"));
    }}
    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("generated main.rs should be written");

    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    let output = command
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("generated crate should run");
    assert!(
        output.status.success(),
        "generated crate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
