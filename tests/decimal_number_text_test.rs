//! Decimal number text in generated tokenizers (`--decimal-numbers`).
//!
//! An f64 is written as plain decimal text when `1e-6 <= |x| < 1e21` (and for
//! zero), and in Rust's shortest exponent form outside that range, so very
//! large and very small values do not become long runs of zeros. For every
//! value the test checks the exact text, the end marker, bit-exact decode
//! (recovery and strict), `token_count() == encode().len()`, and a schema
//! round trip through a `number` member and an untyped member. Streams
//! written in plain decimal form by earlier builds must still decode.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

/// (value, expected text) pairs; the text is what the encoder must emit.
fn cases() -> Vec<(f64, &'static str)> {
    vec![
        (5e-324, "5e-324"),
        (f64::MAX, "1.7976931348623157e308"),
        (1e21, "1e21"),
        (1e-7, "1e-7"),
        (5.4e-11, "5.4e-11"),
        (1000.0, "1000"),
        (0.001, "0.001"),
        (-0.0, "-0"),
        (123.456, "123.456"),
        // Edges of the plain-text range and negative values.
        (0.0, "0"),
        (1e-6, "0.000001"),
        (999_999_999_999_999_900_000.0, "999999999999999900000"),
        (-2.5e-8, "-2.5e-8"),
        (-1e300, "-1e300"),
        (6.02e23, "6.02e23"),
    ]
}

/// (old plain-decimal text, value) pairs that earlier builds emitted.
fn legacy_texts() -> Vec<(String, f64)> {
    vec![
        ("1000000000000000000000".to_string(), 1e21),
        ("0.0000001".to_string(), 1e-7),
        ("0.000000000054".to_string(), 5.4e-11),
        (format!("0.{}5", "0".repeat(323)), 5e-324),
    ]
}

fn decimal_text_body() -> String {
    let cases: Vec<String> = cases()
        .into_iter()
        .map(|(value, text)| format!("({:#x}u64, {text:?})", value.to_bits()))
        .collect();
    let legacy: Vec<String> = legacy_texts()
        .into_iter()
        .map(|(text, value)| format!("({text:?}, {:#x}u64)", value.to_bits()))
        .collect();
    format!(
        r##"
let cases: &[(u64, &str)] = &[{cases}];
let legacy: &[(&str, u64)] = &[{legacy}];
let end_marker: u32 = crate::tokenizer::MAX_BASE_VOCAB - 1;

for &(bits, text) in cases {{
    let value = f64::from_bits(bits);
    let mut tokens = Vec::new();
    JsonTokenEncoder::encode(&value, &mut tokens);
    let mut expected: Vec<u32> = text.bytes().map(u32::from).collect();
    expected.push(end_marker);
    if tokens != expected {{
        let written: String = tokens
            .iter()
            .filter(|token| **token < 256)
            .map(|token| *token as u8 as char)
            .collect();
        return Err(format!("{{value:e}}: wrote {{written:?}} ({{}} tokens), expected {{text:?}}", tokens.len()));
    }}
    let count = JsonTokenEncoder::token_count(&value);
    if count != tokens.len() {{
        return Err(format!("{{text}}: token_count {{count}} != stream length {{}}", tokens.len()));
    }}
    let decoded = <f64 as JsonTokenEncoder>::decode(&mut tokens.clone().into_iter())
        .map_err(|error| format!("{{text}}: decode failed: {{error}}"))?;
    let strict = <f64 as JsonTokenEncoder>::decode_strict(&mut tokens.clone().into_iter())
        .map_err(|error| format!("{{text}}: strict decode failed: {{error}}"))?;
    for (label, got) in [("decode", decoded), ("decode_strict", strict)] {{
        if got.to_bits() != bits {{
            return Err(format!("{{text}}: {{label}} gave bits {{:#x}}, expected {{bits:#x}}", got.to_bits()));
        }}
    }}

    // Schema round trip: a `number` member and an untyped member.
    let instance = serde_json::json!({{ "x": value, "v": value }});
    let typed: DecimalText = serde_json::from_value(instance)
        .map_err(|error| format!("{{text}}: instance did not deserialize: {{error}}"))?;
    let mut stream = Vec::new();
    typed.encode(&mut stream);
    if typed.token_count() != stream.len() {{
        return Err(format!("{{text}}: instance token_count {{}} != stream length {{}}", typed.token_count(), stream.len()));
    }}
    let occurrences = stream.windows(expected.len()).filter(|window| *window == expected.as_slice()).count();
    if occurrences != 2 {{
        return Err(format!("{{text}}: expected the text twice in the instance stream, found {{occurrences}}"));
    }}
    for (label, decoded) in [
        ("decode_full", DecimalText::decode_full(stream.clone())),
        ("decode_strict_full", DecimalText::decode_strict_full(stream.clone())),
    ] {{
        let decoded = decoded.map_err(|error| format!("{{text}}: {{label}} failed: {{error}}"))?;
        let json_value = serde_json::to_value(&decoded)
            .map_err(|error| format!("{{text}}: decoded value did not serialize: {{error}}"))?;
        for member in ["x", "v"] {{
            let got = json_value[member]
                .as_f64()
                .ok_or_else(|| format!("{{text}}: {{label}} member {{member}} is not a number: {{json_value}}"))?;
            if got.to_bits() != bits {{
                return Err(format!("{{text}}: {{label}} member {{member}} gave bits {{:#x}}, expected {{bits:#x}}", got.to_bits()));
            }}
        }}
        let mut reencoded = Vec::new();
        decoded.encode(&mut reencoded);
        if reencoded != stream {{
            return Err(format!("{{text}}: {{label}}: encode(decode(t)) != t"));
        }}
    }}
}}

// Plain-decimal streams written by earlier builds still decode bit-exactly.
for &(text, bits) in legacy {{
    let mut tokens: Vec<u32> = text.bytes().map(u32::from).collect();
    tokens.push(end_marker);
    for strict in [false, true] {{
        let mut iter = tokens.clone().into_iter();
        let got = if strict {{
            <f64 as JsonTokenEncoder>::decode_strict(&mut iter)
        }} else {{
            <f64 as JsonTokenEncoder>::decode(&mut iter)
        }}
        .map_err(|error| format!("legacy {{text}}: decode (strict={{strict}}) failed: {{error}}"))?;
        if got.to_bits() != bits {{
            return Err(format!("legacy {{text}}: decoded bits {{:#x}}, expected {{bits:#x}}", got.to_bits()));
        }}
    }}
}}
Ok(())
"##,
        cases = cases.join(", "),
        legacy = legacy.join(", "),
    )
}

fn decimal_text_schema() -> serde_json::Value {
    json!({
        "title": "DecimalText",
        "type": "object",
        "properties": {
            "x": { "type": "number" },
            "v": {}
        },
        "required": ["x", "v"],
        "additionalProperties": false
    })
}

#[test]
fn test_expected_texts_parse_back_to_the_same_value() {
    for (value, text) in cases() {
        let parsed: f64 = text.parse().expect("expected text parses");
        assert_eq!(parsed.to_bits(), value.to_bits(), "{text}");
    }
    for (text, value) in legacy_texts() {
        let parsed: f64 = text.parse().expect("legacy text parses");
        assert_eq!(parsed.to_bits(), value.to_bits(), "{text}");
    }
}

#[test]
fn test_generated_decimal_f64_text_uses_exponent_outside_plain_range() {
    let generated = TctCompiler::new(decimal_text_schema(), "DecimalText")
        .expect("schema is accepted")
        .with_decimal_number_encoding()
        .compile()
        .expect("schema compiles")
        .generate_code()
        .expect("code generates");
    run_generated_decimal_text_crate("generated-decimal-text", &generated, &decimal_text_body());
}

fn generated_cargo_command(repo_root: &Path) -> Command {
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, repo_root);
    command.env("CARGO_TARGET_DIR", generated_target_dir(repo_root));
    command
}

fn generated_target_dir(repo_root: &Path) -> PathBuf {
    let target_dir = std::env::var_os("TCT_GENERATED_CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            repo_root
                .join("target")
                .join("tct-generated-cargo")
                .join("decimal-number-text")
        });
    fs::create_dir_all(&target_dir).unwrap_or_else(|error| {
        panic!(
            "failed to create generated decimal-number-text Cargo target dir {}: {}",
            target_dir.display(),
            error
        )
    });
    target_dir
}

fn run_generated_decimal_text_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let repo_root = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_rustc_hash()
        .with_regex()
        .with_chrono()
        .with_jsonschema()
        .with_tct_path(repo_root.display().to_string())
        .with_bin(crate_name, "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&repo_root, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated decimal-number-text crate");

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

    let output = generated_cargo_command(&repo_root)
        .arg("run")
        .arg("--quiet")
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
