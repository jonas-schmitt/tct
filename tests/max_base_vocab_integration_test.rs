/// Integration tests for max_base_vocab configuration (P1-2)
///
/// Tests the complete pipeline: TctCompiler.with_max_base_vocab() → Grammar.encoding_base → generated code
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::{OptimizationLevel, TctCompiler};

fn generated_max_base_vocab_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

fn write_generated_max_base_vocab_cargo_toml(crate_dir: &Path, cargo_toml: GeneratedCargoToml) {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = cargo_toml
        .with_tct_path(tct_path.display().to_string())
        .render();
    fs::write(crate_dir.join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir)
        .expect("failed to copy workspace Cargo.lock into generated max-base-vocab crate");
}

#[test]
fn test_max_base_vocab_512() {
    // Create a simple integer schema
    let schema = json!({
        "type": "integer"
    });

    // Compile with max_base_vocab=512
    let grammar = TctCompiler::new(schema, "TestInt")
        .unwrap()
        .with_max_base_vocab(512)
        .unwrap()
        .compile()
        .unwrap();

    // Verify encoding_base is set correctly (512 is power-of-2, so should be 512)
    assert_eq!(grammar.encoding_base, 512, "encoding_base should be 512");
}

#[test]
fn test_max_base_vocab_1024() {
    let schema = json!({"type": "integer"});

    let grammar = TctCompiler::new(schema, "TestInt")
        .unwrap()
        .with_max_base_vocab(1024)
        .unwrap()
        .compile()
        .unwrap();

    assert_eq!(grammar.encoding_base, 1024, "encoding_base should be 1024");
}

#[test]
fn test_max_base_vocab_non_power_of_2() {
    // Test with 1000 (not power-of-2) → should use 512 (largest power-of-2 <= 1000)
    let schema = json!({"type": "integer"});

    let grammar = TctCompiler::new(schema, "TestInt")
        .unwrap()
        .with_max_base_vocab(1000)
        .unwrap()
        .compile()
        .unwrap();

    assert_eq!(
        grammar.encoding_base, 512,
        "encoding_base should be 512 (largest power-of-2 <= 1000)"
    );
}

#[test]
fn test_max_base_vocab_default() {
    // Without setting max_base_vocab, should default to 256
    let schema = json!({"type": "integer"});

    let grammar = TctCompiler::new(schema, "TestInt")
        .unwrap()
        .compile()
        .unwrap();

    assert_eq!(
        grammar.encoding_base, 256,
        "encoding_base should default to 256"
    );
}

#[test]
fn test_max_base_vocab_validation_too_small() {
    // max_base_vocab must be > 256
    let schema = json!({"type": "integer"});

    let result = TctCompiler::new(schema, "TestInt")
        .unwrap()
        .with_max_base_vocab(256);

    assert!(result.is_err(), "Should reject max_base_vocab=256");

    match result {
        Err(err) => {
            assert!(
                err.to_string().contains("max_base_vocab must be > 256"),
                "Error message should mention max_base_vocab > 256, got: {}",
                err
            );
        }
        Ok(_) => panic!("Expected error for max_base_vocab=256"),
    }
}

#[test]
fn test_max_base_vocab_minimum_257() {
    // 257 should be accepted (minimum valid value)
    let schema = json!({"type": "integer"});

    let grammar = TctCompiler::new(schema, "TestInt")
        .unwrap()
        .with_max_base_vocab(257)
        .unwrap()
        .compile()
        .unwrap();

    // 257 is not power-of-2, so should use 256
    assert_eq!(
        grammar.encoding_base, 256,
        "encoding_base should be 256 (largest power-of-2 <= 257)"
    );
}

// ============================================================================
// Phase 4: Comprehensive Round-Trip Tests
// ============================================================================

#[test]
fn test_complex_schema_with_max_base_vocab_512() {
    // Test with a more complex schema (object with integer fields)
    let schema = json!({
        "type": "object",
        "properties": {
            "count": {"type": "integer"},
            "value": {"type": "integer"},
            "name": {"type": "string"},
            "items": {
                "type": "array",
                "items": {"type": "integer"}
            }
        },
        "required": ["count", "value", "name", "items"]
    });

    let grammar = TctCompiler::new(schema, "TestObject")
        .unwrap()
        .with_max_base_vocab(512)
        .unwrap()
        .compile()
        .unwrap();

    // Verify encoding_base is threaded through
    assert_eq!(grammar.encoding_base, 512);

    // Verify code generation includes the encoding_base
    let code = grammar.generate_code().unwrap();
    let code_str = code.to_string();

    // The generated code should contain references to base-512 encoding for i32
    // (This is a basic sanity check - the real verification happens in subprocess tests)
    assert!(
        code_str.contains("encode_u32"),
        "Generated code should include encode_u32 calls"
    );
}

#[test]
fn test_code_generation_with_different_bases() {
    let schema = json!({"type": "integer"});

    // Generate code with base-256 (default)
    let grammar_256 = TctCompiler::new(schema.clone(), "TestInt256")
        .unwrap()
        .compile()
        .unwrap();

    // Generate code with base-512
    let grammar_512 = TctCompiler::new(schema.clone(), "TestInt512")
        .unwrap()
        .with_max_base_vocab(512)
        .unwrap()
        .compile()
        .unwrap();

    // Generate code with base-1024
    let grammar_1024 = TctCompiler::new(schema, "TestInt1024")
        .unwrap()
        .with_max_base_vocab(1024)
        .unwrap()
        .compile()
        .unwrap();

    // Verify each has correct encoding_base
    assert_eq!(grammar_256.encoding_base, 256);
    assert_eq!(grammar_512.encoding_base, 512);
    assert_eq!(grammar_1024.encoding_base, 1024);

    // Verify all generate valid code
    assert!(grammar_256.generate_code().is_ok());
    assert!(grammar_512.generate_code().is_ok());
    assert!(grammar_1024.generate_code().is_ok());
}

#[test]
fn test_string_schema_unaffected_by_max_base_vocab() {
    // String encoding should not be affected by max_base_vocab
    // (strings use byte encoding with END_STRING_MARKER)
    let schema = json!({"type": "string"});

    let grammar_default = TctCompiler::new(schema.clone(), "TestString1")
        .unwrap()
        .compile()
        .unwrap();

    let grammar_512 = TctCompiler::new(schema, "TestString2")
        .unwrap()
        .with_max_base_vocab(512)
        .unwrap()
        .compile()
        .unwrap();

    // Both should have encoding_base set correctly
    assert_eq!(grammar_default.encoding_base, 256);
    assert_eq!(grammar_512.encoding_base, 512);

    // But string encoding is independent of encoding_base
    // (This is verified by the fact that both compile successfully)
    assert!(grammar_default.generate_code().is_ok());
    assert!(grammar_512.generate_code().is_ok());
}

#[test]
fn test_generated_round_trip_respects_max_base_vocab_257_and_512() {
    for (max_base_vocab, expected_encoding_base) in [(257, 256), (512, 512)] {
        run_generated_round_trip_with_max_base_vocab(max_base_vocab, expected_encoding_base);
    }
}

#[test]
fn test_generated_bounded_integer_uses_effective_base_512_for_offset_encoding() {
    let schema = json!({
        "type": "object",
        "properties": {
            "score": {"type": "integer", "minimum": 0, "maximum": 262143}
        },
        "required": ["score"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "BoundedScore")
        .unwrap()
        .with_max_base_vocab(512)
        .unwrap()
        .compile()
        .unwrap();
    assert_eq!(grammar.encoding_base, 512);

    let generated_code = grammar.generate_code().unwrap();
    assert_eq!(
        extract_generated_constant_value(&generated_code, "BASE_VOCAB_SIZE"),
        512
    );
    assert_eq!(
        extract_generated_constant_value(&generated_code, "VOCAB_SIZE"),
        513
    );
    assert_eq!(
        extract_generated_constant_value(&generated_code, "PAD_TOKEN"),
        512
    );

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_max_base_vocab_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("bounded-integer-base-512-round-trip")
            .with_preserve_order()
            .with_rustc_hash()
            .with_bin("bounded-integer-base-512-round-trip", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use serde_json::{{json, Value}};
use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn decoded_with_generic(tokens: Vec<u32>) -> Result<u32, String> {{
    let mut token_iter = tokens.into_iter();
    crate::tokenizer::variable_base::decode_u32(
        &mut token_iter,
        "BoundedScore",
        crate::tokenizer::variable_base::GENERATED_ENCODING_BASE,
    )
    .map_err(|error| format!("{{:?}}", error))
}}

fn decoded_with_fixed(tokens: Vec<u32>) -> Result<u32, String> {{
    let mut token_iter = tokens.into_iter();
    crate::tokenizer::variable_base::decode_u32_fixed_base(&mut token_iter, "BoundedScore")
        .map_err(|error| format!("{{:?}}", error))
}}

fn assert_fixed_decode_matches_generic(tokens: Vec<u32>) -> Result<(), String> {{
    let generic = decoded_with_generic(tokens.clone());
    let fixed = decoded_with_fixed(tokens.clone());
    if generic != fixed {{
        return Err(format!(
            "fixed-base decode result {{:?}} differed from generic {{:?}} for tokens {{:?}}",
            fixed, generic, tokens
        ));
    }}
    Ok(())
}}

fn main() -> Result<(), String> {{
    for tokens in [
        vec![],
        vec![0],
        vec![0, 70000],
        vec![4, 999],
        vec![2, 511, 512, 513],
        vec![3, 70000, 70000, 70000, 70000],
    ] {{
        assert_fixed_decode_matches_generic(tokens)?;
    }}

    let json_value = json!({{"score": 262143}});
    let typed_instance: BoundedScore = serde_json::from_value(json_value.clone())
        .map_err(|error| format!("generated type parse failed: {{}}", error))?;

    let mut tokens = Vec::new();
    typed_instance.encode(&mut tokens);
    if BASE_VOCAB_SIZE != 512 || PAD_TOKEN != 512 || VOCAB_SIZE != 513 {{
        return Err(format!(
            "expected base-512 layout BASE_VOCAB_SIZE=512 PAD_TOKEN=512 VOCAB_SIZE=513, got {{}}/{{}}/{{}}",
            BASE_VOCAB_SIZE,
            PAD_TOKEN,
            VOCAB_SIZE
        ));
    }}
    let expected_tokens = vec![1, 511, 511];
    if tokens != expected_tokens {{
        return Err(format!(
            "expected base-512 bounded integer token shape {{:?}}, got {{:?}}",
            expected_tokens, tokens
        ));
    }}
    if !tokens.iter().any(|token| *token >= 257) {{
        return Err(format!("expected at least one widened-base token in {{:?}}", tokens));
    }}
    if tokens.contains(&PAD_TOKEN) {{
        return Err(format!("bounded integer payload emitted reserved PAD_TOKEN {{}} in {{:?}}", PAD_TOKEN, tokens));
    }}
    if let Some(max_token) = tokens.iter().max() {{
        if *max_token >= PAD_TOKEN {{
            return Err(format!("structural payload token {{}} collided with PAD_TOKEN {{}} in {{:?}}", max_token, PAD_TOKEN, tokens));
        }}
    }}

    let mut token_iter = tokens.into_iter();
    let decoded_instance = BoundedScore::decode(&mut token_iter)
        .map_err(|error| format!("decode failed: {{}}", error))?;
    if let Some(remaining_token) = token_iter.next() {{
        return Err(format!("Unconsumed token remains: {{}}", remaining_token));
    }}

    let decoded_json = serde_json::to_value(decoded_instance)
        .map_err(|error| format!("decoded JSON serialization failed: {{}}", error))?;
    if decoded_json != json_value {{
        return Err(format!("decoded {{}} but expected {{}}", decoded_json, json_value));
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_max_base_vocab_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated bounded integer base-512 crate");

    assert!(
        output.status.success(),
        "generated bounded integer base-512 round trip failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_o0_large_enum_vocab_covers_standard_variant_tokens() {
    let variants: Vec<_> = (0..300)
        .map(|index| json!(format!("variant_{}", index)))
        .collect();
    let schema = json!({ "enum": variants });

    let grammar = TctCompiler::new(schema, "LargeEnumO0")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    assert!(
        extract_generated_constant_value(&generated_code, "BASE_VOCAB_SIZE") >= 300,
        "O0 standard enum variant tokens must fit below BASE_VOCAB_SIZE"
    );

    run_generated_vocab_layout_crate(
        "large-enum-o0-vocab-layout",
        &generated_code,
        r#"
let input = serde_json::json!("variant_299");
let typed_instance: LargeEnumO0 = serde_json::from_value(input.clone())
    .map_err(|error| format!("generated enum parse failed: {}", error))?;
let mut tokens = Vec::new();
typed_instance.encode(&mut tokens);
if tokens != vec![299] {
    return Err(format!("expected standard enum token 299, got {:?}", tokens));
}
let token_count = typed_instance.token_count();
if token_count != tokens.len() {
    return Err(format!("generated enum token_count {token_count} did not match encoded length {}", tokens.len()));
}
if BASE_VOCAB_SIZE <= 299 {
    return Err(format!("BASE_VOCAB_SIZE {BASE_VOCAB_SIZE} does not cover emitted variant token 299"));
}
if VOCAB_SIZE <= BASE_VOCAB_SIZE {
    return Err(format!("VOCAB_SIZE {VOCAB_SIZE} must leave room above BASE_VOCAB_SIZE {BASE_VOCAB_SIZE}"));
}
assert_tokens_fit_vocab(&tokens)?;
let mut token_iter = tokens.clone().into_iter();
let decoded_instance = LargeEnumO0::decode(&mut token_iter)
    .map_err(|error| format!("decode failed: {}", error))?;
if let Some(remaining_token) = token_iter.next() {
    return Err(format!("Unconsumed token remains: {}", remaining_token));
}
let decoded_json = serde_json::to_value(decoded_instance)
    .map_err(|error| format!("decoded JSON serialization failed: {}", error))?;
if decoded_json != input {
    return Err(format!("decoded {} but expected {}", decoded_json, input));
}

let strict_decoded = LargeEnumO0::decode_strict_full(tokens.clone())
    .map_err(|error| format!("strict decode rejected canonical token: {error}"))?;
let strict_json = serde_json::to_value(strict_decoded)
    .map_err(|error| format!("strict decoded JSON serialization failed: {}", error))?;
if strict_json != input {
    return Err(format!("strict decoded {} but expected {}", strict_json, input));
}

let recovered = LargeEnumO0::decode_full(vec![599u32])
    .map_err(|error| format!("decode_full rejected recovered variant token: {error}"))?;
let recovered_json = serde_json::to_value(recovered)
    .map_err(|error| format!("recovered JSON serialization failed: {}", error))?;
if recovered_json != input {
    return Err(format!("decode_full recovered {} but expected {}", recovered_json, input));
}

let recovery_full_error = match LargeEnumO0::decode_full(vec![599u32, 9001u32]) {
    Ok(value) => {
        return Err(format!("decode_full accepted trailing sentinel after recovered token as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !recovery_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("decode_full reported wrong recovered-token trailing error: {recovery_full_error}"));
}

match LargeEnumO0::decode_strict_full(vec![599u32]) {
    Ok(value) => {
        return Err(format!("strict decode accepted recovered variant token as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 599
            || context.type_name != "LargeEnumO0"
            || context.field_name != Some("variant_index")
            || context.expected != "enum variant index in range"
        {
            return Err(format!("strict decode reported wrong malformed-token context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict decode reported wrong malformed-token error: {error}"));
    }
}
"#,
    );
}

#[test]
fn test_root_array_length_tokens_fit_vocab() {
    let schema = json!({
        "type": "array",
        "items": { "type": "integer" }
    });

    let grammar = TctCompiler::new(schema, "IntArray")
        .unwrap()
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    assert_eq!(
        extract_generated_constant_value(&generated_code, "BASE_VOCAB_SIZE"),
        grammar.base_vocab
    );

    run_generated_vocab_layout_crate(
        "root-array-length-vocab-layout",
        &generated_code,
        r#"
let values: Vec<i64> = (0..300).collect();
let input = serde_json::json!(values);
let typed_instance: IntArray = serde_json::from_value(input.clone())
    .map_err(|error| format!("generated array parse failed: {}", error))?;
let mut tokens = Vec::new();
typed_instance.encode(&mut tokens);
assert_tokens_fit_vocab(&tokens)?;
let mut token_iter = tokens.into_iter();
let decoded_instance = IntArray::decode(&mut token_iter)
    .map_err(|error| format!("decode failed: {}", error))?;
if let Some(remaining_token) = token_iter.next() {
    return Err(format!("Unconsumed token remains: {}", remaining_token));
}
let decoded_json = serde_json::to_value(decoded_instance)
    .map_err(|error| format!("decoded JSON serialization failed: {}", error))?;
if decoded_json != input {
    return Err(format!("decoded {} but expected {}", decoded_json, input));
}
"#,
    );
}

fn run_generated_round_trip_with_max_base_vocab(max_base_vocab: u32, expected_encoding_base: u32) {
    let schema = json!({
        "type": "object",
        "properties": {
            "count": {"type": "integer"},
            "value": {"type": "integer"},
            "name": {"type": "string"}
        },
        "required": ["count", "value", "name"]
    });

    let grammar = TctCompiler::new(schema, "TestObject")
        .unwrap()
        .with_max_base_vocab(max_base_vocab)
        .unwrap()
        .compile()
        .unwrap();
    assert_eq!(grammar.encoding_base, expected_encoding_base);

    let generated_code = grammar.generate_code().unwrap();
    let expected_base_vocab_size = expected_encoding_base.max(257);
    assert_eq!(
        extract_generated_constant_value(&generated_code, "BASE_VOCAB_SIZE"),
        expected_base_vocab_size,
        "generated BASE_VOCAB_SIZE should include the effective encoding base"
    );
    assert_eq!(
        extract_generated_constant_value(&generated_code, "MAX_BASE_VOCAB"),
        expected_base_vocab_size,
        "generated tokenizer MAX_BASE_VOCAB should match Grammar.base_vocab"
    );
    assert_eq!(
        extract_generated_constant_value(&generated_code, "VOCAB_SIZE"),
        expected_base_vocab_size + 1,
        "generated VOCAB_SIZE should leave room for PAD_TOKEN after the base vocabulary"
    );

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_max_base_vocab_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new(format!("max-base-vocab-{}-round-trip", max_base_vocab))
            .with_preserve_order()
            .with_rustc_hash()
            .with_regex()
            .with_lazy_static()
            .with_jsonschema()
            .with_bin("max-base-vocab-round-trip", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use serde_json::{{json, Value}};
use crate::tokenizer::JsonTokenEncoder;

{generated_code}

const EXPECTED_ENCODING_BASE: u32 = {expected_encoding_base};

fn run_case(json_value: Value) -> Result<(), String> {{
    let typed_instance: TestObject = serde_json::from_value(json_value.clone())
        .map_err(|error| format!("generated type parse failed: {{}}", error))?;

    let mut tokens = Vec::new();
    typed_instance.encode(&mut tokens);
    let max_token = tokens.iter().copied().max().unwrap_or(0);
    if max_token >= VOCAB_SIZE {{
        return Err(format!(
            "Vocabulary constraint violation: token {{}} >= VOCAB_SIZE {{}}",
            max_token, VOCAB_SIZE
        ));
    }}
    if EXPECTED_ENCODING_BASE > 256 && !tokens.iter().any(|token| *token >= 257) {{
        return Err(format!(
            "Expected a token from the widened base range for encoding base {{}}",
            EXPECTED_ENCODING_BASE
        ));
    }}

    let mut token_iter = tokens.into_iter();
    let decoded_instance = TestObject::decode(&mut token_iter)
        .map_err(|error| format!("decode failed: {{}}", error))?;
    if let Some(remaining_token) = token_iter.next() {{
        return Err(format!("Unconsumed token remains: {{}}", remaining_token));
    }}

    let decoded_json = serde_json::to_value(decoded_instance)
        .map_err(|error| format!("decoded JSON serialization failed: {{}}", error))?;
    if decoded_json != json_value {{
        return Err(format!(
            "Original and decoded JSON do not match\nOriginal: {{}}\nDecoded: {{}}",
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| json_value.to_string()),
            serde_json::to_string_pretty(&decoded_json).unwrap_or_else(|_| decoded_json.to_string())
        ));
    }}

    Ok(())
}}

fn main() -> Result<(), String> {{
    let items: Vec<Value> = (0..511).map(|idx| json!(idx)).collect();
    run_case(json!({{"count": 511, "value": 123456, "name": "max-base-vocab", "items": items}}))
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_max_base_vocab_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated max_base_vocab crate");

    assert!(
        output.status.success(),
        "generated max_base_vocab={} round trip failed\nstdout:\n{}\nstderr:\n{}",
        max_base_vocab,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_generated_vocab_layout_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_max_base_vocab_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new(crate_name)
            .with_preserve_order()
            .with_rustc_hash()
            .with_bin(crate_name, "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn assert_tokens_fit_vocab(tokens: &[u32]) -> Result<(), String> {{
    if let Some(max_token) = tokens.iter().max() {{
        if *max_token >= VOCAB_SIZE {{
            return Err(format!("token {{}} outside VOCAB_SIZE {{}} in {{:?}}", max_token, VOCAB_SIZE, tokens));
        }}
    }}
    Ok(())
}}

fn main() -> Result<(), String> {{
{body}
    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_max_base_vocab_cargo_command()
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

fn extract_generated_constant_value(code: &str, constant_name: &str) -> u32 {
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
