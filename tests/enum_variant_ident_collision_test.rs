//! Regression: string enum wire values that sanitize to the same PascalCase
//! ident must still compile and round-trip (BUG-1 / BUG-4).

use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

fn generated_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

fn write_generated_cargo_toml(crate_dir: &Path, cargo_toml: GeneratedCargoToml) {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = cargo_toml
        .with_tct_path(tct_path.display().to_string())
        .render();
    fs::write(crate_dir.join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir)
        .expect("failed to copy workspace Cargo.lock into generated crate");
}

#[test]
fn colliding_date_format_enum_variants_compile_and_round_trip() {
    let schema = json!({
        "title": "DateFormat",
        "type": "string",
        "enum": ["YYYY-MM-DD", "YYYY/MM/DD"]
    });

    let grammar = TctCompiler::new(schema, "DateFormat")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();

    // Sanity: generated source names both variants distinctly with wire renames.
    assert!(
        generated_code.contains("YyyyMmDd") && generated_code.contains("YyyyMmDd2"),
        "expected disambiguated variant idents:\n{generated_code}"
    );
    assert!(
        generated_code.contains(r#"rename = "YYYY-MM-DD""#)
            && generated_code.contains(r#"rename = "YYYY/MM/DD""#),
        "expected serde renames for both wire values:\n{generated_code}"
    );

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("enum-ident-collision-round-trip")
            .with_preserve_order()
            .with_rustc_hash()
            .with_bin("enum-ident-collision-round-trip", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use serde_json::{{json, Value}};
use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn round_trip(value: Value) -> Result<(), String> {{
    let typed: DateFormat = serde_json::from_value(value.clone())
        .map_err(|error| format!("deserialize {{}} failed: {{}}", value, error))?;
    let mut tokens = Vec::new();
    typed.encode(&mut tokens);
    if let Some(max_token) = tokens.iter().max() {{
        if *max_token >= VOCAB_SIZE {{
            return Err(format!("token {{}} outside VOCAB_SIZE {{}}", max_token, VOCAB_SIZE));
        }}
    }}
    let mut token_iter = tokens.into_iter();
    let decoded = DateFormat::decode(&mut token_iter)
        .map_err(|error| format!("decode failed for {{}}: {{}}", value, error))?;
    if token_iter.next().is_some() {{
        return Err(format!("unconsumed tokens for {{}}", value));
    }}
    let decoded_json = serde_json::to_value(decoded)
        .map_err(|error| format!("serialize decoded failed: {{}}", error))?;
    if decoded_json != value {{
        return Err(format!("round-trip mismatch: got {{}} expected {{}}", decoded_json, value));
    }}
    Ok(())
}}

fn main() -> Result<(), String> {{
    round_trip(json!("YYYY-MM-DD"))?;
    round_trip(json!("YYYY/MM/DD"))?;
    // Distinct wire values must remain distinct after encode (different tokens).
    let a: DateFormat = serde_json::from_value(json!("YYYY-MM-DD")).map_err(|e| e.to_string())?;
    let b: DateFormat = serde_json::from_value(json!("YYYY/MM/DD")).map_err(|e| e.to_string())?;
    let mut ta = Vec::new();
    let mut tb = Vec::new();
    a.encode(&mut ta);
    b.encode(&mut tb);
    if ta == tb {{
        return Err("colliding enum variants produced identical token sequences".to_string());
    }}
    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated enum collision crate");

    assert!(
        output.status.success(),
        "colliding date-format enum round trip failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
