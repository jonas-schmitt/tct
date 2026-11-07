//! UTF-8 String Encoding Tests
//!
//! Validates configurable StringEncoding mode (ASCII vs UTF-8).
//! - ASCII mode: bytes 0-127, END_STRING_MARKER = 128
//! - UTF-8 mode: bytes 0-255, END_STRING_MARKER = 256

use serde_json::json;
use std::fs;
use std::process::Command;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::{StringEncoding, TctCompiler};

fn generated_utf8_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

fn extract_constant_value(code: &str, constant_name: &str) -> u32 {
    let pattern = format!("pub const {}", constant_name);
    let line = code
        .lines()
        .find(|line| line.contains(&pattern))
        .unwrap_or_else(|| panic!("missing generated constant {}", constant_name));
    let value = line
        .split('=')
        .nth(1)
        .unwrap_or_else(|| panic!("generated constant {} has no value", constant_name))
        .trim()
        .trim_end_matches(';')
        .trim_end_matches("u32");
    value.parse::<u32>().unwrap_or_else(|error| {
        panic!(
            "failed to parse generated constant {} value {:?}: {}",
            constant_name, value, error
        )
    })
}

#[test]
fn test_string_encoding_constants() {
    // Verify END_STRING_MARKER values
    assert_eq!(StringEncoding::Ascii.end_marker(), 128);
    assert_eq!(StringEncoding::Utf8.end_marker(), 256);

    // Verify max byte values
    assert_eq!(StringEncoding::Ascii.max_byte_value(), 127);
    assert_eq!(StringEncoding::Utf8.max_byte_value(), 255);

    // Verify base vocabulary sizes
    assert_eq!(StringEncoding::Ascii.base_vocab_size(), 129);
    assert_eq!(StringEncoding::Utf8.base_vocab_size(), 257);
}

#[test]
fn test_ascii_mode_compilation() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "Person")
        .unwrap()
        .with_string_encoding(StringEncoding::Ascii)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Verify generated code contains ASCII constants
    assert!(code.contains("const END_STRING_MARKER: u32 = 128"));
    assert!(code.contains("const MAX_BYTE_VALUE: u32 = 127"));
    assert_eq!(grammar.base_vocab, 256);
    assert_eq!(extract_constant_value(&code, "BASE_VOCAB_SIZE"), 256);
    assert_eq!(extract_constant_value(&code, "MAX_BASE_VOCAB"), 256);
    assert_eq!(extract_constant_value(&code, "PAD_TOKEN"), 256);
    assert_eq!(extract_constant_value(&code, "VOCAB_SIZE"), 257);
}

#[test]
fn test_utf8_mode_compilation() {
    let schema = json!({
        "type": "object",
        "properties": {
            "text": {"type": "string"}
        },
        "required": ["text"]
    });

    let grammar = TctCompiler::new(schema, "Document")
        .unwrap()
        .with_string_encoding(StringEncoding::Utf8)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Verify generated code contains UTF-8 constants
    assert!(code.contains("const END_STRING_MARKER: u32 = 256"));
    assert!(code.contains("const MAX_BYTE_VALUE: u32 = 255"));
    assert_eq!(grammar.base_vocab, 257);
    assert_eq!(extract_constant_value(&code, "BASE_VOCAB_SIZE"), 257);
    assert_eq!(extract_constant_value(&code, "MAX_BASE_VOCAB"), 257);
    assert_eq!(extract_constant_value(&code, "PAD_TOKEN"), 257);
    assert_eq!(extract_constant_value(&code, "VOCAB_SIZE"), 258);
}

#[test]
fn test_default_is_utf8() {
    // Verify default behavior is UTF-8 mode
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {"type": "string"}
        },
        "required": ["value"]
    });

    let grammar = TctCompiler::new(schema, "Data")
        .unwrap()
        .compile() // No explicit string_encoding set
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Should default to UTF-8 (END_MARKER = 256)
    assert!(code.contains("const END_STRING_MARKER: u32 = 256"));
    assert!(code.contains("const MAX_BYTE_VALUE: u32 = 255"));
}

#[test]
fn test_ascii_mode_generated_encode_rejects_non_ascii() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "AsciiPerson")
        .unwrap()
        .with_string_encoding(StringEncoding::Ascii)
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    assert!(generated_code.contains("self.is_ascii()"));
    assert!(generated_code.contains("StringEncoding::Ascii cannot encode non-ASCII strings"));

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = GeneratedCargoToml::new("ascii-string-encoding-rejection")
        .with_rustc_hash()
        .with_tct_path(tct_path.display().to_string())
        .with_bin("ascii-string-encoding-rejection", "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated UTF-8 crate");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use serde_json::json;
use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
    let input: AsciiPerson = serde_json::from_value(json!({{"name": "café"}}))
        .map_err(|error| format!("generated type parse failed: {{}}", error))?;
    let result = std::panic::catch_unwind(|| {{
        let mut tokens = Vec::new();
        input.encode(&mut tokens);
    }});

    if result.is_ok() {{
        return Err("ASCII encoding accepted non-ASCII input".to_string());
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_utf8_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated ASCII rejection crate");

    assert!(
        output.status.success(),
        "generated ASCII rejection check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_utf8_mode_error_messages() {
    let schema = json!({
        "type": "object",
        "properties": {
            "message": {"type": "string"}
        },
        "required": ["message"]
    });

    let grammar = TctCompiler::new(schema, "Message")
        .unwrap()
        .with_string_encoding(StringEncoding::Utf8)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Verify error messages reference UTF-8 mode
    assert!(code.contains("UTF-8 byte or END_STRING_MARKER (256)"));
}

#[test]
fn test_ascii_mode_error_messages() {
    let schema = json!({
        "type": "object",
        "properties": {
            "text": {"type": "string"}
        },
        "required": ["text"]
    });

    let grammar = TctCompiler::new(schema, "Text")
        .unwrap()
        .with_string_encoding(StringEncoding::Ascii)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Verify error messages reference ASCII mode
    assert!(code.contains("ASCII byte or END_STRING_MARKER (128)"));
}

#[test]
fn test_compilation_constraints_default() {
    use tct::generator::encoding_decision::CompilationConstraints;

    let constraints = CompilationConstraints::default();
    assert_eq!(constraints.string_encoding, StringEncoding::Utf8);
}

#[test]
fn test_compilation_constraints_with_utf8() {
    use tct::generator::encoding_decision::CompilationConstraints;

    let constraints = CompilationConstraints::default().with_string_encoding(StringEncoding::Utf8);

    assert_eq!(constraints.string_encoding, StringEncoding::Utf8);
}

#[test]
fn test_vocab_size_includes_string_bytes_ascii() {
    // Schema with only string field (no enums) - ASCII mode
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "Person")
        .unwrap()
        .with_string_encoding(StringEncoding::Ascii)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // ASCII strings need 129 tokens, but the variable-base digit alphabet keeps
    // the generated base vocabulary at the default floor of 256.
    assert_eq!(grammar.base_vocab, 256);
    assert_eq!(extract_constant_value(&code, "BASE_VOCAB_SIZE"), 256);
    assert_eq!(extract_constant_value(&code, "VOCAB_SIZE"), 257);
}

#[test]
fn test_vocab_size_includes_string_bytes_utf8() {
    // Schema with only string field (no enums) - UTF-8 mode
    let schema = json!({
        "type": "object",
        "properties": {
            "text": {"type": "string"}
        },
        "required": ["text"]
    });

    let grammar = TctCompiler::new(schema, "Document")
        .unwrap()
        .with_string_encoding(StringEncoding::Utf8)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // UTF-8 uses one more base token than the default variable-base floor.
    assert_eq!(grammar.base_vocab, 257);
    assert_eq!(extract_constant_value(&code, "BASE_VOCAB_SIZE"), 257);
    assert_eq!(extract_constant_value(&code, "VOCAB_SIZE"), 258);
}

#[test]
fn test_vocab_size_with_enum_ascii() {
    // Schema with enum (structural tokens) - ASCII mode
    let schema = json!({
        "type": "object",
        "properties": {
            "status": {"enum": ["active", "pending", "completed"]},
            "name": {"type": "string"}
        },
        "required": ["status", "name"]
    });

    let grammar = TctCompiler::new(schema, "Task")
        .unwrap()
        .with_string_encoding(StringEncoding::Ascii)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Enum tokens overlap with string/digit base tokens; the default
    // variable-base floor still determines the base vocabulary.
    assert_eq!(grammar.base_vocab, 256);
    assert_eq!(extract_constant_value(&code, "BASE_VOCAB_SIZE"), 256);
    assert_eq!(extract_constant_value(&code, "VOCAB_SIZE"), 257);
}

#[test]
fn test_vocab_size_with_large_enum_utf8() {
    // Schema with large enum that exceeds string bytes - UTF-8 mode
    // Create enum with 300 variants (exceeds UTF-8's 257 tokens)
    let mut enum_values: Vec<serde_json::Value> = Vec::new();
    for i in 0..300 {
        enum_values.push(json!(format!("value_{}", i)));
    }

    let schema = json!({
        "type": "object",
        "properties": {
            "code": {"enum": enum_values},
            "text": {"type": "string"}
        },
        "required": ["code", "text"]
    });

    let grammar = TctCompiler::new(schema, "Data")
        .unwrap()
        .with_string_encoding(StringEncoding::Utf8)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // With 300 enum variants, structural vocab exceeds UTF-8 and the
    // variable-base floor. VOCAB_SIZE still includes PAD.
    assert_eq!(grammar.base_vocab, 300);
    assert_eq!(extract_constant_value(&code, "BASE_VOCAB_SIZE"), 300);
    assert_eq!(extract_constant_value(&code, "VOCAB_SIZE"), 301);
}
