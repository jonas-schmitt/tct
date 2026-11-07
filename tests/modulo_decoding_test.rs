//! Tests for modulo decoding functionality to ensure LLM robustness.
//!
//! This test suite validates that the tokenizer gracefully handles out-of-bounds
//! token IDs by mapping them to valid vocabulary options using modulo arithmetic.

use serde_json::json;
use std::fs;
use std::process::Command;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::*;

fn generated_modulo_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

#[test]
fn test_modulo_decoding_simple_enum_modulo_decoding() {
    let schema = json!({
        "title": "SimpleEnum",
        "type": "object",
        "properties": {
            "status": {
                "enum": ["pending", "complete", "cancelled"]
            }
        },
        "required": ["status"]
    });

    let result = TctCompiler::new(schema.clone(), "SimpleEnum")
        .unwrap()
        .compile();
    assert!(
        result.is_ok(),
        "Simple enum schema generation should succeed"
    );

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");

    run_generated_modulo_contract_crate(
        "simple-enum-modulo-contracts",
        &code,
        r#"
let original = SimpleEnum {
    status: SimpleEnumStatus::Cancelled,
    additional_properties: rustc_hash::FxHashMap::default(),
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
if tokens.first() != Some(&2u32) {
    return Err(format!("expected canonical Cancelled discriminator 2, got {tokens:?}"));
}
tokens[0] = 5u32;
tokens.push(9001u32);

let mut iter = tokens.into_iter();
let decoded = SimpleEnum::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded.status != SimpleEnumStatus::Cancelled {
    return Err(format!("generated enum did not recover 5 % 3 as Cancelled: {:?}", decoded.status));
}
if iter.next() != Some(9001u32) {
    return Err("generated enum decode consumed trailing sentinel".to_string());
}

let mut strict_status_iter = vec![5u32].into_iter();
if let Ok(value) = SimpleEnumStatus::decode_strict(&mut strict_status_iter) {
    return Err(format!("strict enum decode accepted malformed discriminator as {value:?}"));
}

match SimpleEnum::decode_strict_full(vec![5u32]) {
    Ok(value) => {
        return Err(format!("strict generated struct decode accepted malformed enum field as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 5
            || context.type_name != "SimpleEnum"
            || context.field_name != Some("status")
            || context.expected != "enum variant index in range"
        {
            return Err(format!("strict generated struct enum field reported wrong context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict generated struct enum field reported wrong error: {error}"));
    }
}

let recovered_full = SimpleEnumStatus::decode_full(vec![5u32])
    .map_err(|error| format!("generated enum decode_full rejected recovered discriminator: {error}"))?;
if recovered_full != SimpleEnumStatus::Cancelled {
    return Err(format!("generated enum decode_full did not recover 5 % 3 as Cancelled: {recovered_full:?}"));
}

let recovery_full_error = match SimpleEnumStatus::decode_full(vec![5u32, 9001u32]) {
    Ok(value) => {
        return Err(format!("generated enum decode_full accepted trailing sentinel after recovery as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !recovery_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("generated enum decode_full reported wrong trailing-token error: {recovery_full_error}"));
}

let strict_full_error = match SimpleEnumStatus::decode_strict_full(vec![2u32, 9001u32]) {
    Ok(value) => {
        return Err(format!("strict full enum decode accepted trailing sentinel as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !strict_full_error.contains("decode_strict_full left 1 trailing token") {
    return Err(format!("strict full enum decode reported wrong trailing-token error: {strict_full_error}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_modulo_decoding_bool_modulo_decoding() {
    // Test that bool types use modulo 2 logic
    use tct::tokenizer::JsonTokenEncoder;

    // Test normal cases
    let mut iter = vec![0u32].into_iter();
    let result = bool::decode(&mut iter);
    assert!(!result.unwrap());

    let mut iter = vec![1u32].into_iter();
    let result = bool::decode(&mut iter);
    assert!(result.unwrap());

    // Test modulo cases - out of bounds tokens should wrap
    let mut iter = vec![2u32].into_iter(); // 2 % 2 = 0 -> false
    let result = bool::decode(&mut iter);
    assert!(!result.unwrap());

    let mut iter = vec![3u32].into_iter(); // 3 % 2 = 1 -> true
    let result = bool::decode(&mut iter);
    assert!(result.unwrap());

    let mut iter = vec![100u32].into_iter(); // 100 % 2 = 0 -> false
    let result = bool::decode(&mut iter);
    assert!(!result.unwrap());

    let mut iter = vec![101u32].into_iter(); // 101 % 2 = 1 -> true
    let result = bool::decode(&mut iter);
    assert!(result.unwrap());
}

#[test]
fn test_modulo_decoding_option_modulo_decoding() {
    // Test that Option<T> types use modulo 2 logic
    use tct::tokenizer::JsonTokenEncoder;

    // Test normal cases
    let mut iter = vec![0u32].into_iter();
    let result = Option::<i64>::decode(&mut iter);
    assert_eq!(result.unwrap(), None);

    // i64 uses 8-byte encoding (little-endian), so 42i64 = [42, 0, 0, 0, 0, 0, 0, 0]
    let mut iter = vec![1u32, 42, 0, 0, 0, 0, 0, 0, 0].into_iter(); // 1 = Some, then 8 bytes for i64
    let result = Option::<i64>::decode(&mut iter);
    assert_eq!(result.unwrap(), Some(42i64));

    // Test modulo cases
    let mut iter = vec![2u32].into_iter(); // 2 % 2 = 0 -> None
    let result = Option::<i64>::decode(&mut iter);
    assert_eq!(result.unwrap(), None);

    // 123i64 = [123, 0, 0, 0, 0, 0, 0, 0]
    let mut iter = vec![3u32, 123, 0, 0, 0, 0, 0, 0, 0].into_iter(); // 3 % 2 = 1 -> Some, 8 bytes for i64
    let result = Option::<i64>::decode(&mut iter);
    assert_eq!(result.unwrap(), Some(123i64));
}

#[test]
fn test_modulo_decoding_large_enum_modulo_decoding() {
    let schema = json!({
        "title": "LargeEnum",
        "type": "object",
        "properties": {
            "choice": {
                "enum": ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"] // 10 options
            }
        },
        "required": ["choice"]
    });

    let result = TctCompiler::new(schema.clone(), "LargeEnum")
        .unwrap()
        .compile();
    assert!(
        result.is_ok(),
        "Large enum schema generation should succeed"
    );

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");

    run_generated_modulo_contract_crate(
        "large-enum-modulo-contracts",
        &code,
        r#"
let original = LargeEnum {
    choice: LargeEnumChoice::J,
    additional_properties: rustc_hash::FxHashMap::default(),
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
let token_count = original.token_count();
if token_count != tokens.len() {
    return Err(format!("generated large enum token_count {token_count} did not match encoded length {}", tokens.len()));
}
if let Some(max_token) = tokens.iter().max() {
    if *max_token >= VOCAB_SIZE {
        return Err(format!("generated large enum emitted token {max_token} outside VOCAB_SIZE {VOCAB_SIZE}: {tokens:?}"));
    }
}
if tokens.first() != Some(&9u32) {
    return Err(format!("expected canonical J discriminator 9, got {tokens:?}"));
}
tokens[0] = 19u32;
tokens.push(9001u32);

let mut iter = tokens.into_iter();
let decoded = LargeEnum::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded.choice != LargeEnumChoice::J {
    return Err(format!("generated enum did not recover 19 % 10 as J: {:?}", decoded.choice));
}
if iter.next() != Some(9001u32) {
    return Err("generated large enum decode consumed trailing sentinel".to_string());
}

let mut recovered_full_tokens = Vec::new();
original.encode(&mut recovered_full_tokens);
recovered_full_tokens[0] = 19u32;
let recovered_full = LargeEnum::decode_full(recovered_full_tokens.clone())
    .map_err(|error| format!("generated large enum decode_full rejected recovered discriminator: {error}"))?;
if recovered_full.choice != LargeEnumChoice::J {
    return Err(format!("generated large enum decode_full did not recover 19 % 10 as J: {:?}", recovered_full.choice));
}

let mut recovered_full_with_trailing = recovered_full_tokens.clone();
recovered_full_with_trailing.push(9001u32);
let recovery_full_error = match LargeEnum::decode_full(recovered_full_with_trailing) {
    Ok(value) => {
        return Err(format!("generated large enum decode_full accepted trailing sentinel after recovery as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !recovery_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("generated large enum decode_full reported wrong trailing-token error: {recovery_full_error}"));
}

match LargeEnum::decode_strict_full(recovered_full_tokens.clone()) {
    Ok(value) => {
        return Err(format!("strict large enum struct decode accepted recovered discriminator as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 19
            || context.type_name != "LargeEnum"
            || context.field_name != Some("choice")
            || context.expected != "enum variant index in range"
        {
            return Err(format!("strict large enum struct decode reported wrong choice context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict large enum struct decode reported wrong discriminator error: {error}"));
    }
}

match LargeEnumChoice::decode_strict_full(vec![19u32]) {
    Ok(value) => {
        return Err(format!("strict large enum choice decode accepted recovered discriminator as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 19
            || context.type_name != "LargeEnumChoice"
            || context.field_name != Some("variant_index")
            || context.expected != "enum variant index in range"
        {
            return Err(format!("strict large enum choice decode reported wrong context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict large enum choice decode reported wrong discriminator error: {error}"));
    }
}

Ok(())
"#,
    );
}

#[test]
fn test_modulo_decoding_oneof_modulo_decoding() {
    let schema = json!({
        "title": "OneOfTest",
        "oneOf": [
            {"type": "string"},
            {"type": "number"},
            {"type": "boolean"}
        ]
    });

    let result = TctCompiler::new(schema.clone(), "OneOfTest")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "OneOf schema generation should succeed");

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");

    run_generated_modulo_contract_crate(
        "oneof-modulo-contracts",
        &code,
        r#"
let original = OneOfTest::Variant2(true);
let mut tokens = Vec::new();
original.encode(&mut tokens);
if tokens.first() != Some(&2u32) {
    return Err(format!("expected canonical boolean variant discriminator 2, got {tokens:?}"));
}
tokens[0] = 5u32;
tokens.push(9001u32);

let mut iter = tokens.into_iter();
let decoded = OneOfTest::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded != OneOfTest::Variant2(true) {
    return Err(format!("generated oneOf did not recover 5 % 3 as boolean true: {:?}", decoded));
}
if iter.next() != Some(9001u32) {
    return Err("generated oneOf decode consumed trailing sentinel".to_string());
}

let mut strict_oneof_iter = vec![5u32, 1u32].into_iter();
if let Ok(value) = OneOfTest::decode_strict(&mut strict_oneof_iter) {
    return Err(format!("strict oneOf decode accepted malformed discriminator as {value:?}"));
}

let recovered_full = OneOfTest::decode_full(vec![5u32, 1u32])
    .map_err(|error| format!("generated oneOf decode_full rejected recovered discriminator: {error}"))?;
if recovered_full != OneOfTest::Variant2(true) {
    return Err(format!("generated oneOf decode_full did not recover 5 % 3 as boolean true: {recovered_full:?}"));
}

let recovery_full_error = match OneOfTest::decode_full(vec![5u32, 1u32, 9001u32]) {
    Ok(value) => {
        return Err(format!("generated oneOf decode_full accepted trailing sentinel after recovery as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !recovery_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("generated oneOf decode_full reported wrong trailing-token error: {recovery_full_error}"));
}

match OneOfTest::decode_strict_full(vec![5u32, 1u32]) {
    Ok(value) => {
        return Err(format!("strict oneOf decode accepted recovered discriminator as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 5
            || context.type_name != "OneOfTest"
            || context.field_name != Some("discriminator")
            || context.expected != "enum discriminator in range"
        {
            return Err(format!("strict oneOf decode reported wrong discriminator context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict oneOf decode reported wrong discriminator error: {error}"));
    }
}

Ok(())
"#,
    );
}

#[test]
fn test_modulo_decoding_extreme_token_values() {
    // Test with very large token values that would normally cause InvalidToken errors
    let schema = json!({
        "title": "ExtremeTest",
        "type": "object",
        "properties": {
            "flag": {"enum": ["yes", "no"]}, // 2 options
            "grade": {"enum": ["A", "B", "C", "D", "F"]} // 5 options
        },
        "required": ["flag", "grade"]
    });

    let result = TctCompiler::new(schema.clone(), "ExtremeTest")
        .unwrap()
        .compile();
    assert!(
        result.is_ok(),
        "Extreme test schema generation should succeed"
    );

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");

    run_generated_modulo_contract_crate(
        "extreme-enum-modulo-contracts",
        &code,
        r#"
let original = ExtremeTest {
    flag: ExtremeTestFlag::No,
    grade: ExtremeTestGrade::F,
    additional_properties: rustc_hash::FxHashMap::default(),
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
let token_count = original.token_count();
if token_count != tokens.len() {
    return Err(format!("generated extreme enum token_count {token_count} did not match encoded length {}", tokens.len()));
}
if let Some(max_token) = tokens.iter().max() {
    if *max_token >= VOCAB_SIZE {
        return Err(format!("generated extreme enum emitted token {max_token} outside VOCAB_SIZE {VOCAB_SIZE}: {tokens:?}"));
    }
}
if tokens.get(0) != Some(&1u32) || tokens.get(1) != Some(&4u32) {
    return Err(format!("expected canonical flag/grade discriminators [1, 4], got {tokens:?}"));
}
tokens[0] = 101u32;
tokens[1] = 104u32;
tokens.push(9001u32);

let mut iter = tokens.into_iter();
let decoded = ExtremeTest::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded.grade != ExtremeTestGrade::F {
    return Err(format!("generated grade enum did not recover 104 % 5 as F: {:?}", decoded.grade));
}
if decoded.flag != ExtremeTestFlag::No {
    return Err(format!("generated flag enum did not recover 101 % 2 as No: {:?}", decoded.flag));
}
if iter.next() != Some(9001u32) {
    return Err("generated extreme enum decode consumed trailing sentinel".to_string());
}

let mut recovered_full_tokens = Vec::new();
original.encode(&mut recovered_full_tokens);
recovered_full_tokens[0] = 101u32;
recovered_full_tokens[1] = 104u32;
let recovered_full = ExtremeTest::decode_full(recovered_full_tokens.clone())
    .map_err(|error| format!("generated extreme enum decode_full rejected recovered discriminators: {error}"))?;
if recovered_full.grade != ExtremeTestGrade::F {
    return Err(format!("generated extreme enum decode_full grade did not recover 104 % 5 as F: {:?}", recovered_full.grade));
}
if recovered_full.flag != ExtremeTestFlag::No {
    return Err(format!("generated extreme enum decode_full flag did not recover 101 % 2 as No: {:?}", recovered_full.flag));
}

let mut recovered_full_with_trailing = recovered_full_tokens.clone();
recovered_full_with_trailing.push(9001u32);
let recovery_full_error = match ExtremeTest::decode_full(recovered_full_with_trailing) {
    Ok(value) => {
        return Err(format!("generated extreme enum decode_full accepted trailing sentinel after recovery as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !recovery_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("generated extreme enum decode_full reported wrong trailing-token error: {recovery_full_error}"));
}

let mut strict_flag_tokens = recovered_full_tokens.clone();
strict_flag_tokens[1] = 4u32;
match ExtremeTest::decode_strict_full(strict_flag_tokens) {
    Ok(value) => {
        return Err(format!("strict extreme enum decode accepted recovered flag discriminator as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 101
            || context.type_name != "ExtremeTest"
            || context.field_name != Some("flag")
            || context.expected != "enum variant index in range"
        {
            return Err(format!("strict extreme enum decode reported wrong flag context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict extreme enum decode reported wrong flag error: {error}"));
    }
}

let mut strict_grade_tokens = recovered_full_tokens;
strict_grade_tokens[0] = 1u32;
match ExtremeTest::decode_strict_full(strict_grade_tokens) {
    Ok(value) => {
        return Err(format!("strict extreme enum decode accepted recovered grade discriminator as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 104
            || context.type_name != "ExtremeTest"
            || context.field_name != Some("grade")
            || context.expected != "enum variant index in range"
        {
            return Err(format!("strict extreme enum decode reported wrong grade context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict extreme enum decode reported wrong grade error: {error}"));
    }
}

Ok(())
"#,
    );
}

#[test]
fn test_modulo_decoding_nested_structure_modulo() {
    let schema = json!({
        "title": "NestedTest",
        "type": "object",
        "properties": {
            "config": {
                "type": "object",
                "properties": {
                    "mode": {"enum": ["dev", "prod"]},
                    "level": {"enum": ["low", "medium", "high"]}
                },
                "required": ["mode", "level"]
            },
            "enabled": {"type": "boolean"}
        },
        "required": ["config", "enabled"]
    });

    let result = TctCompiler::new(schema.clone(), "NestedTest")
        .unwrap()
        .compile();
    assert!(
        result.is_ok(),
        "Nested test schema generation should succeed"
    );

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");

    run_generated_modulo_contract_crate(
        "nested-enum-modulo-contracts",
        &code,
        r#"
let original = NestedTest {
    config: NestedTestConfig {
        mode: NestedTestConfigMode::Prod,
        level: NestedTestConfigLevel::High,
        additional_properties: rustc_hash::FxHashMap::default(),
    },
    enabled: true,
    additional_properties: rustc_hash::FxHashMap::default(),
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
if tokens.get(0) != Some(&1u32) || tokens.get(1) != Some(&2u32) {
    return Err(format!("expected canonical mode/level discriminators [1, 2], got {tokens:?}"));
}
tokens[0] = 3u32;
tokens[1] = 5u32;
tokens.push(9001u32);

let mut iter = tokens.into_iter();
let decoded = NestedTest::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded.config.level != NestedTestConfigLevel::High {
    return Err(format!("generated nested level enum did not recover 5 % 3 as High: {:?}", decoded.config.level));
}
if decoded.config.mode != NestedTestConfigMode::Prod {
    return Err(format!("generated nested mode enum did not recover 3 % 2 as Prod: {:?}", decoded.config.mode));
}
if iter.next() != Some(9001u32) {
    return Err("generated nested enum decode consumed trailing sentinel".to_string());
}

let mut recovered_full_tokens = Vec::new();
original.encode(&mut recovered_full_tokens);
recovered_full_tokens[0] = 3u32;
recovered_full_tokens[1] = 5u32;
let recovered_full = NestedTest::decode_full(recovered_full_tokens.clone())
    .map_err(|error| format!("generated nested enum decode_full rejected recovered discriminators: {error}"))?;
if recovered_full.config.level != NestedTestConfigLevel::High {
    return Err(format!("generated nested decode_full level did not recover 5 % 3 as High: {:?}", recovered_full.config.level));
}
if recovered_full.config.mode != NestedTestConfigMode::Prod {
    return Err(format!("generated nested decode_full mode did not recover 3 % 2 as Prod: {:?}", recovered_full.config.mode));
}

let mut recovered_full_with_trailing = recovered_full_tokens;
recovered_full_with_trailing.push(9001u32);
let recovery_full_error = match NestedTest::decode_full(recovered_full_with_trailing) {
    Ok(value) => {
        return Err(format!("generated nested enum decode_full accepted trailing sentinel after recovery as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !recovery_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("generated nested enum decode_full reported wrong trailing-token error: {recovery_full_error}"));
}

let mut strict_config_tokens = Vec::new();
original.config.encode(&mut strict_config_tokens);
strict_config_tokens[0] = 3u32;
match NestedTestConfig::decode_strict_full(strict_config_tokens) {
    Ok(value) => {
        return Err(format!("strict nested config decode accepted recovered mode discriminator as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 3
            || context.type_name != "NestedTestConfig"
            || context.field_name != Some("mode")
            || context.expected != "enum variant index in range"
        {
            return Err(format!("strict nested config decode reported wrong mode context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict nested config decode reported wrong mode error: {error}"));
    }
}

let mut strict_root_tokens = Vec::new();
original.encode(&mut strict_root_tokens);
strict_root_tokens[0] = 3u32;
match NestedTest::decode_strict_full(strict_root_tokens) {
    Ok(value) => {
        return Err(format!("strict nested root decode accepted recovered mode discriminator as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 3
            || context.type_name != "NestedTest"
            || context.field_name != Some("config")
            || context.expected != "enum variant index in range"
        {
            return Err(format!("strict nested root decode reported wrong config context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict nested root decode reported wrong mode error: {error}"));
    }
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_optional_presence_uses_modulo_recovery() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {"type": "integer"}
        },
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "OptionalPresenceRecovery")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile()
        .expect("optional presence recovery schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("optional presence recovery schema should generate code");

    run_generated_modulo_contract_crate(
        "optional-presence-recovery-contracts",
        &generated_code,
        r#"
let mut generic_some_tokens = vec![3u32];
42i64.encode(&mut generic_some_tokens);
generic_some_tokens.push(9001);
let mut iter = generic_some_tokens.into_iter();
let decoded_option = Option::<i64>::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded_option != Some(42) {
    return Err(format!("generated Option<T> did not recover 3 % 2 as Some: {decoded_option:?}"));
}
if iter.next() != Some(9001) {
    return Err("generated Option<T> consumed the sentinel after Some payload".to_string());
}

let mut generic_none_tokens = vec![2u32, 77];
let mut iter = generic_none_tokens.into_iter();
let decoded_option = Option::<i64>::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded_option != None {
    return Err(format!("generated Option<T> did not recover 2 % 2 as None: {decoded_option:?}"));
}
if iter.next() != Some(77) {
    return Err("generated Option<T> consumed payload tokens for a recovered None".to_string());
}

let mut struct_some_tokens = vec![3u32];
crate::tokenizer::JsonInteger::from(42i64).encode(&mut struct_some_tokens);
struct_some_tokens.push(9001);
let mut iter = struct_some_tokens.into_iter();
let decoded = OptionalPresenceRecovery::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded.value != Some(crate::tokenizer::JsonInteger::from(42i64)) {
    return Err(format!("generated struct did not recover 3 % 2 as Some: {:?}", decoded.value));
}
if iter.next() != Some(9001) {
    return Err("generated struct consumed the sentinel after Some payload".to_string());
}

let struct_none_tokens = vec![2u32, 77];
let mut iter = struct_none_tokens.into_iter();
let decoded = OptionalPresenceRecovery::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded.value != None {
    return Err(format!("generated struct did not recover 2 % 2 as None: {:?}", decoded.value));
}
if iter.next() != Some(77) {
    return Err("generated struct consumed payload tokens for a recovered None".to_string());
}

let mut struct_some_full_tokens = vec![3u32];
crate::tokenizer::JsonInteger::from(42i64).encode(&mut struct_some_full_tokens);
let decoded = OptionalPresenceRecovery::decode_full(struct_some_full_tokens)
    .map_err(|error| format!("generated struct decode_full rejected recovered Some payload: {error}"))?;
if decoded.value != Some(crate::tokenizer::JsonInteger::from(42i64)) {
    return Err(format!("generated struct decode_full did not recover 3 % 2 as Some: {:?}", decoded.value));
}

let struct_none_full_error = match OptionalPresenceRecovery::decode_full(vec![2u32, 77u32]) {
    Ok(value) => {
        return Err(format!("generated struct decode_full accepted trailing payload after recovered None as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !struct_none_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("generated struct decode_full reported wrong recovered-None trailing-token error: {struct_none_full_error}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_optional_presence_strict_rejects_recovered_state() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {"type": "integer"}
        },
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "OptionalPresenceStrict")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile()
        .expect("optional presence strict schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("optional presence strict schema should generate code");

    run_generated_modulo_contract_crate(
        "optional-presence-strict-contracts",
        &generated_code,
        r#"
let mut recovered_presence_tokens = vec![3u32];
crate::tokenizer::JsonInteger::from(42i64).encode(&mut recovered_presence_tokens);
recovered_presence_tokens.push(9001u32);

let mut recovery_iter = recovered_presence_tokens.clone().into_iter();
let recovered = OptionalPresenceStrict::decode(&mut recovery_iter)
    .map_err(|error| format!("generated optional presence recovery rejected malformed state: {error}"))?;
if recovered.value != Some(crate::tokenizer::JsonInteger::from(42i64)) {
    return Err(format!("generated optional presence recovery did not normalize 3 % 2 as Some(42): {:?}", recovered.value));
}
if recovery_iter.next() != Some(9001u32) {
    return Err("generated optional presence recovery consumed trailing sentinel after Some payload".to_string());
}

let mut strict_error_tokens = vec![3u32];
crate::tokenizer::JsonInteger::from(42i64).encode(&mut strict_error_tokens);
match OptionalPresenceStrict::decode_strict_full(strict_error_tokens) {
    Ok(value) => {
        return Err(format!("strict optional presence decode accepted recovered state as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 3
            || context.type_name != "OptionalPresenceStrict"
            || context.field_name != Some("value")
            || context.expected != "presence flag 0 or 1"
        {
            return Err(format!("strict optional presence decode reported wrong state context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict optional presence decode reported wrong recovered-state error: {error}"));
    }
}

let mut strict_some_with_surplus = vec![1u32];
crate::tokenizer::JsonInteger::from(42i64).encode(&mut strict_some_with_surplus);
strict_some_with_surplus.push(9001u32);
let strict_full_error = match OptionalPresenceStrict::decode_strict_full(strict_some_with_surplus) {
    Ok(value) => {
        return Err(format!("strict optional presence decode_full accepted trailing sentinel as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !strict_full_error.contains("decode_strict_full left 1 trailing token") {
    return Err(format!("strict optional presence decode_strict_full reported wrong trailing-token error: {strict_full_error}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_nullable_optional_field_uses_modulo_recovery() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {"type": ["integer", "null"]}
        },
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "NullableOptionalRecovery")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile()
        .expect("nullable optional recovery schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("nullable optional recovery schema should generate code");

    run_generated_modulo_contract_crate(
        "nullable-optional-field-recovery-contracts",
        &generated_code,
        r#"
let mut present_tokens = vec![5u32];
serde_json::json!(42).encode(&mut present_tokens);
present_tokens.push(9001u32);
let mut iter = present_tokens.into_iter();
let decoded = NullableOptionalRecovery::decode(&mut iter).map_err(|error| error.to_string())?;
match decoded.value {
    crate::tokenizer::OptionalField::Present(value) if value == serde_json::json!(42) => {}
    other => return Err(format!("generated nullable field did not recover 5 % 3 as Present(42): {other:?}")),
}
if iter.next() != Some(9001u32) {
    return Err("generated nullable field consumed the sentinel after recovered Present payload".to_string());
}

let mut present_full_tokens = vec![5u32];
serde_json::json!(42).encode(&mut present_full_tokens);
let decoded = NullableOptionalRecovery::decode_full(present_full_tokens)
    .map_err(|error| format!("generated nullable field decode_full rejected recovered Present payload: {error}"))?;
match decoded.value {
    crate::tokenizer::OptionalField::Present(value) if value == serde_json::json!(42) => {}
    other => return Err(format!("generated nullable field decode_full did not recover 5 % 3 as Present(42): {other:?}")),
}

let decoded = NullableOptionalRecovery::decode_full(vec![4u32])
    .map_err(|error| format!("generated nullable field decode_full rejected recovered Null: {error}"))?;
if !decoded.value.is_null() {
    return Err(format!("generated nullable field decode_full did not recover 4 % 3 as Null: {:?}", decoded.value));
}

let absent_full_error = match NullableOptionalRecovery::decode_full(vec![3u32, 77u32]) {
    Ok(value) => {
        return Err(format!("generated nullable field decode_full accepted trailing payload after recovered Absent as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !absent_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("generated nullable field decode_full reported wrong recovered-Absent trailing-token error: {absent_full_error}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_nullable_optional_field_strict_rejects_recovered_state() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {"type": ["integer", "null"]}
        },
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "NullableOptionalStrict")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile()
        .expect("nullable optional strict schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("nullable optional strict schema should generate code");

    run_generated_modulo_contract_crate(
        "nullable-optional-field-strict-contracts",
        &generated_code,
        r#"
let mut recovered_state_tokens = vec![5u32];
serde_json::json!(42).encode(&mut recovered_state_tokens);
recovered_state_tokens.push(9001u32);

let mut recovery_iter = recovered_state_tokens.clone().into_iter();
let recovered = NullableOptionalStrict::decode(&mut recovery_iter)
    .map_err(|error| format!("generated nullable field recovery rejected malformed state: {error}"))?;
match recovered.value {
    crate::tokenizer::OptionalField::Present(value) if value == serde_json::json!(42) => {}
    other => return Err(format!("generated nullable field recovery did not normalize 5 % 3 as Present(42): {other:?}")),
}
if recovery_iter.next() != Some(9001u32) {
    return Err("generated nullable field recovery consumed trailing sentinel after Present payload".to_string());
}

let mut strict_error_tokens = vec![5u32];
serde_json::json!(42).encode(&mut strict_error_tokens);
match NullableOptionalStrict::decode_strict_full(strict_error_tokens) {
    Ok(value) => {
        return Err(format!("strict nullable field decode accepted recovered state as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 5
            || context.type_name != "NullableOptionalStrict"
            || context.field_name != Some("value")
            || context.expected != "state flag 0, 1, or 2"
        {
            return Err(format!("strict nullable field decode reported wrong state context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict nullable field decode reported wrong recovered-state error: {error}"));
    }
}

let mut strict_present_with_surplus = vec![2u32];
serde_json::json!(42).encode(&mut strict_present_with_surplus);
strict_present_with_surplus.push(9001u32);
let strict_full_error = match NullableOptionalStrict::decode_strict_full(strict_present_with_surplus) {
    Ok(value) => {
        return Err(format!("strict nullable field decode_full accepted trailing sentinel as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !strict_full_error.contains("decode_strict_full left 1 trailing token") {
    return Err(format!("strict nullable field decode_strict_full reported wrong trailing-token error: {strict_full_error}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_bitpacked_optional_presence_strict_rejects_malformed_masks_and_payloads() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "alpha": {"type": "string"},
            "beta": {"type": "integer"},
            "gamma": {"type": "boolean"},
            "delta": {"enum": ["red", "blue"]}
        },
        "required": ["name"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "BitPackedPresenceStrict")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O3)
        .compile()
        .expect("bit-packed presence strict schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("bit-packed presence strict schema should generate code");

    run_generated_modulo_contract_crate(
        "bitpacked-presence-strict-contracts",
        &generated_code,
        r#"
let mut recovery_tokens = vec![0u32, 0, 0, 16];
"ok".to_string().encode(&mut recovery_tokens);
recovery_tokens.push(9001u32);

let mut iter = recovery_tokens.clone().into_iter();
let decoded = BitPackedPresenceStrict::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded.name != "ok" {
    return Err(format!("generated bit-packed struct decoded wrong required field: {:?}", decoded.name));
}
if decoded.alpha.is_some() || decoded.beta.is_some() || decoded.gamma.is_some() || decoded.delta.is_some() {
    return Err(format!("generated bit-packed recovery did not ignore unused presence bits: {:?}", decoded));
}
if iter.next() != Some(9001u32) {
    return Err("generated bit-packed recovery consumed trailing sentinel after unused presence bits".to_string());
}

let recovery_full_error = match BitPackedPresenceStrict::decode_full(recovery_tokens) {
    Ok(value) => {
        return Err(format!("generated bit-packed decode_full accepted trailing sentinel after unused-bit recovery as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !recovery_full_error.contains("decode_full left 1 trailing token") {
    return Err(format!("generated bit-packed decode_full reported wrong trailing-token error: {recovery_full_error}"));
}

let mut strict_tokens = vec![0u32, 0, 0, 16];
"ok".to_string().encode(&mut strict_tokens);
match BitPackedPresenceStrict::decode_strict(&mut strict_tokens.into_iter()) {
    Ok(value) => {
        return Err(format!("strict bit-packed decode accepted unused presence bits as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 16
            || context.type_name != "BitPackedPresenceStrict"
            || context.field_name != Some("presence_mask_0")
            || context.expected != "presence mask bits for optional fields"
        {
            return Err(format!("strict bit-packed decode reported wrong unused-bit context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict bit-packed decode reported wrong unused-bit error: {error}"));
    }
}

let mut codec_digit_recovery_tokens = vec![0u32, 0, 0, 260];
"ok".to_string().encode(&mut codec_digit_recovery_tokens);
codec_digit_recovery_tokens.push(1u32);

let mut codec_digit_recovery_iter = codec_digit_recovery_tokens.clone().into_iter();
let codec_digit_recovered = BitPackedPresenceStrict::decode(&mut codec_digit_recovery_iter)
    .map_err(|error| format!("generated bit-packed codec-digit recovery failed: {error}"))?;
if codec_digit_recovered.name != "ok" || codec_digit_recovered.gamma != Some(true) {
    return Err(format!("generated bit-packed codec-digit recovery decoded wrong value: {codec_digit_recovered:?}"));
}

match BitPackedPresenceStrict::decode_strict_full(codec_digit_recovery_tokens) {
    Ok(value) => {
        return Err(format!("strict bit-packed decode accepted recovered presence-mask digit as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 260
            || context.type_name != "BitPackedPresenceStrict"
            || context.field_name != Some("presence_mask_0")
            || context.expected != "presence mask codec digit below base"
        {
            return Err(format!("strict bit-packed codec-digit decode reported wrong context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict bit-packed codec-digit decode reported wrong error: {error:?}"));
    }
}

let mut payload_recovery_tokens = vec![0u32, 0, 0, 4];
"ok".to_string().encode(&mut payload_recovery_tokens);
payload_recovery_tokens.push(3u32);

let mut payload_recovery_iter = payload_recovery_tokens.clone().into_iter();
let payload_recovered = BitPackedPresenceStrict::decode(&mut payload_recovery_iter)
    .map_err(|error| format!("generated bit-packed payload recovery failed: {error}"))?;
if payload_recovered.name != "ok" || payload_recovered.gamma != Some(true) {
    return Err(format!("generated bit-packed payload recovery decoded wrong value: {payload_recovered:?}"));
}

match BitPackedPresenceStrict::decode_strict_full(payload_recovery_tokens) {
    Ok(value) => {
        return Err(format!("strict bit-packed decode accepted recovered optional payload as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 3
            || context.type_name != "BitPackedPresenceStrict"
            || context.field_name != Some("gamma")
            || context.expected != "boolean token 0 or 1"
        {
            return Err(format!("strict bit-packed payload decode reported wrong context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict bit-packed payload decode reported wrong error: {error:?}"));
    }
}

let mut strict_full_trailing_tokens = vec![0u32, 0, 0, 4];
"ok".to_string().encode(&mut strict_full_trailing_tokens);
true.encode(&mut strict_full_trailing_tokens);
strict_full_trailing_tokens.push(9001u32);
let strict_full_error = match BitPackedPresenceStrict::decode_strict_full(strict_full_trailing_tokens) {
    Ok(value) => {
        return Err(format!("strict bit-packed decode_strict_full accepted trailing sentinel as {value:?}"));
    }
    Err(error) => error.to_string(),
};
if !strict_full_error.contains("decode_strict_full left 1 trailing token") {
    return Err(format!("strict bit-packed decode_strict_full reported wrong trailing-token error: {strict_full_error}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_bounded_integer_strict_rejects_recovered_tokens() {
    let small_schema = json!({
        "type": "object",
        "properties": {
            "score": {"type": "integer", "minimum": 0, "maximum": 299}
        },
        "required": ["score"],
        "additionalProperties": false
    });

    let small_grammar = TctCompiler::new(small_schema, "StrictSmallBounded")
        .unwrap()
        .with_max_base_vocab(257)
        .unwrap()
        .compile()
        .expect("small bounded-integer strict schema should compile");
    let small_generated_code = small_grammar
        .generate_code()
        .expect("small bounded-integer strict schema should generate code");

    run_generated_modulo_contract_crate(
        "small-bounded-integer-strict-contracts",
        &small_generated_code,
        r#"
let original = StrictSmallBounded {
    score: StrictSmallBoundedScore(299),
};
let mut canonical = Vec::new();
original.encode(&mut canonical);
if canonical != vec![1, 1, 43] {
    return Err(format!("expected base-256 bounded score token shape [1, 1, 43], got {canonical:?}"));
}

let recovered_tokens = vec![1u32, 1u32, 299u32];
let mut recovery_iter = recovered_tokens.clone().into_iter();
let recovered = StrictSmallBounded::decode(&mut recovery_iter)
    .map_err(|error| format!("bounded integer recovery decode failed: {error}"))?;
if recovered.score.0 != 299 {
    return Err(format!("bounded integer recovery did not normalize to 299: {:?}", recovered.score));
}

match StrictSmallBounded::decode_strict_full(recovered_tokens.clone()) {
    Ok(value) => {
        return Err(format!("strict bounded integer struct accepted recovered offset digit as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 299
            || context.type_name != "StrictSmallBounded"
            || context.field_name != Some("score")
            || context.expected != "multi-token digit below GENERATED_ENCODING_BASE"
        {
            return Err(format!("strict bounded integer struct reported wrong offset digit context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict bounded integer struct reported wrong offset digit error: {error:?}"));
    }
}

match StrictSmallBoundedScore::decode_strict_full(recovered_tokens) {
    Ok(value) => {
        return Err(format!("strict bounded integer newtype accepted recovered offset digit as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 299
            || context.type_name != "StrictSmallBoundedScore"
            || context.field_name != Some("digit")
            || context.expected != "multi-token digit below GENERATED_ENCODING_BASE"
        {
            return Err(format!("strict bounded integer newtype reported wrong offset digit context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict bounded integer newtype reported wrong offset digit error: {error:?}"));
    }
}

Ok(())
"#,
    );

    let offset_schema = json!({
        "type": "object",
        "properties": {
            "score": {"type": "integer", "minimum": 0, "maximum": 262143}
        },
        "required": ["score"],
        "additionalProperties": false
    });

    let offset_grammar = TctCompiler::new(offset_schema, "StrictWideBounded")
        .unwrap()
        .with_max_base_vocab(512)
        .unwrap()
        .compile()
        .expect("wide bounded-integer strict schema should compile");
    let offset_generated_code = offset_grammar
        .generate_code()
        .expect("wide bounded-integer strict schema should generate code");

    run_generated_modulo_contract_crate(
        "wide-bounded-integer-strict-contracts",
        &offset_generated_code,
        r#"
let original = StrictWideBounded {
    score: StrictWideBoundedScore(262143),
};
let mut canonical = Vec::new();
original.encode(&mut canonical);
if canonical != vec![1, 511, 511] {
    return Err(format!("expected base-512 bounded score token shape [1, 511, 511], got {canonical:?}"));
}

let recovered_tokens = vec![1u32, 512u32, 511u32];
let mut recovery_iter = recovered_tokens.clone().into_iter();
let recovered = StrictWideBounded::decode(&mut recovery_iter)
    .map_err(|error| format!("bounded integer offset recovery decode failed: {error}"))?;
if recovered.score.0 != 511 {
    return Err(format!("bounded integer offset recovery did not normalize malformed digit to 511: {:?}", recovered.score));
}

match StrictWideBounded::decode_strict_full(recovered_tokens.clone()) {
    Ok(value) => {
        return Err(format!("strict bounded integer struct accepted recovered offset digit as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 512
            || context.type_name != "StrictWideBounded"
            || context.field_name != Some("score")
            || context.expected != "multi-token digit below GENERATED_ENCODING_BASE"
        {
            return Err(format!("strict bounded integer struct reported wrong offset digit context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict bounded integer struct reported wrong offset digit error: {error:?}"));
    }
}

match StrictWideBoundedScore::decode_strict_full(recovered_tokens) {
    Ok(value) => {
        return Err(format!("strict bounded integer newtype accepted recovered offset digit as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 512
            || context.type_name != "StrictWideBoundedScore"
            || context.field_name != Some("digit")
            || context.expected != "multi-token digit below GENERATED_ENCODING_BASE"
        {
            return Err(format!("strict bounded integer newtype reported wrong offset digit context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict bounded integer newtype reported wrong offset digit error: {error:?}"));
    }
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_unbounded_integer_strict_rejects_recovered_tokens() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {"type": "integer"}
        },
        "required": ["value"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "StrictUnboundedInteger")
        .unwrap()
        .compile()
        .expect("unbounded integer strict schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("unbounded integer strict schema should generate code");

    run_generated_modulo_contract_crate(
        "unbounded-integer-strict-contracts",
        &generated_code,
        r#"
let original = StrictUnboundedInteger {
    value: crate::tokenizer::JsonInteger::from(42i64),
};
let mut canonical = Vec::new();
original.encode(&mut canonical);
if canonical != vec![0, 42, 0, 0, 0, 0, 0, 0, 0] {
    return Err(format!("expected signed JsonInteger token shape, got {canonical:?}"));
}

let mut recovered_marker_tokens = canonical.clone();
recovered_marker_tokens[0] = 3;
let mut recovery_iter = recovered_marker_tokens.clone().into_iter();
let recovered_marker = StrictUnboundedInteger::decode(&mut recovery_iter)
    .map_err(|error| format!("unbounded integer marker recovery decode failed: {error}"))?;
let recovered_marker_json = serde_json::to_value(recovered_marker)
    .map_err(|error| format!("marker recovery serialization failed: {error}"))?;
if recovered_marker_json != serde_json::json!({"value": 42}) {
    return Err(format!("unbounded integer marker recovery decoded {recovered_marker_json}"));
}

match StrictUnboundedInteger::decode_strict_full(recovered_marker_tokens) {
    Ok(value) => {
        return Err(format!("strict unbounded integer struct accepted recovered marker as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 3
            || context.type_name != "StrictUnboundedInteger"
            || context.field_name != Some("value")
            || context.expected != "integer signed/unsigned marker 0 or 1"
        {
            return Err(format!("strict unbounded integer marker reported wrong context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict unbounded integer marker reported wrong error: {error:?}"));
    }
}

let mut recovered_byte_tokens = canonical;
recovered_byte_tokens[1] = 300;
let mut recovery_iter = recovered_byte_tokens.clone().into_iter();
let recovered_byte = StrictUnboundedInteger::decode(&mut recovery_iter)
    .map_err(|error| format!("unbounded integer byte recovery decode failed: {error}"))?;
let recovered_byte_json = serde_json::to_value(recovered_byte)
    .map_err(|error| format!("byte recovery serialization failed: {error}"))?;
if recovered_byte_json != serde_json::json!({"value": 44}) {
    return Err(format!("unbounded integer byte recovery decoded {recovered_byte_json}"));
}

match StrictUnboundedInteger::decode_strict_full(recovered_byte_tokens) {
    Ok(value) => {
        return Err(format!("strict unbounded integer struct accepted recovered byte as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 300
            || context.type_name != "StrictUnboundedInteger"
            || context.field_name != Some("value")
            || context.expected != "byte token 0..=255"
        {
            return Err(format!("strict unbounded integer byte reported wrong context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict unbounded integer byte reported wrong error: {error:?}"));
    }
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_dynamic_json_strict_decode_rejects_invalid_tags() {
    let schema = json!({
        "type": "object",
        "additionalProperties": true
    });

    let grammar = TctCompiler::new(schema, "DynamicJsonStrictTags")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile()
        .expect("dynamic JSON schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("dynamic JSON schema should generate code");

    run_generated_modulo_contract_crate(
        "dynamic-json-strict-tags-contracts",
        &generated_code,
        r#"
let value_recovery_tokens = vec![7u32, 1u32];
let mut iter = value_recovery_tokens.clone().into_iter();
let recovered = serde_json::Value::decode(&mut iter).map_err(|error| error.to_string())?;
if recovered != serde_json::Value::Bool(true) {
    return Err(format!("generated serde_json::Value did not recover 7 % 6 as bool true: {recovered:?}"));
}
if iter.next().is_some() {
    return Err("generated serde_json::Value recovery left unexpected tokens".to_string());
}

match serde_json::Value::decode_strict(&mut value_recovery_tokens.into_iter()) {
    Ok(value) => {
        return Err(format!("strict generated serde_json::Value accepted malformed type tag as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 7
            || context.type_name != "serde_json::Value"
            || context.field_name != Some("type_tag")
            || context.expected != "value type tag 0..=5"
        {
            return Err(format!("strict generated serde_json::Value reported wrong tag error: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict generated serde_json::Value reported wrong tag error: {error}"));
    }
}

let mut number_recovery_tokens = vec![5u32];
42u64.encode(&mut number_recovery_tokens);
let mut iter = number_recovery_tokens.clone().into_iter();
let recovered = serde_json::Number::decode(&mut iter).map_err(|error| error.to_string())?;
if recovered != serde_json::Number::from(42u64) {
    return Err(format!("generated serde_json::Number did not recover 5 % 3 as u64 42: {recovered:?}"));
}
if iter.next().is_some() {
    return Err("generated serde_json::Number recovery left unexpected tokens".to_string());
}

match serde_json::Number::decode_strict(&mut number_recovery_tokens.into_iter()) {
    Ok(value) => {
        return Err(format!("strict generated serde_json::Number accepted malformed marker as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 5
            || context.type_name != "serde_json::Number"
            || context.field_name != Some("marker")
            || context.expected != "number marker 0, 1, or 2"
        {
            return Err(format!("strict generated serde_json::Number reported wrong marker error: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict generated serde_json::Number reported wrong marker error: {error}"));
    }
}

let mut nested_number_tokens = vec![2u32, 5u32];
42u64.encode(&mut nested_number_tokens);
match serde_json::Value::decode_strict(&mut nested_number_tokens.into_iter()) {
    Ok(value) => {
        return Err(format!("strict generated serde_json::Value accepted malformed nested number marker as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 5
            || context.type_name != "serde_json::Number"
            || context.field_name != Some("marker")
            || context.expected != "number marker 0, 1, or 2"
        {
            return Err(format!("strict generated serde_json::Value did not propagate number marker error: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict generated serde_json::Value did not propagate number marker error: {error}"));
    }
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_dynamic_json_nested_recovery_error_preserves_field_context_and_surplus() {
    let schema = json!({
        "type": "object",
        "properties": {
            "payload": {}
        },
        "required": ["payload"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "DynamicJsonNestedError")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile()
        .expect("dynamic JSON field schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("dynamic JSON field schema should generate code");

    run_generated_modulo_contract_crate(
        "dynamic-json-nested-error-contracts",
        &generated_code,
        r#"
let original = DynamicJsonNestedError {
    payload: serde_json::json!({"outer": [{"nested": "ok"}]}),
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
let payload_start = tokens
    .windows(3)
    .position(|window| window == [b'o' as u32, b'k' as u32, 256u32])
    .ok_or_else(|| format!("encoded dynamic payload did not contain the expected string token window: {tokens:?}"))?;
tokens[payload_start] = 300u32;
tokens.push(9001u32);

let mut iter = tokens.into_iter();
match DynamicJsonNestedError::decode(&mut iter) {
    Ok(value) => {
        return Err(format!("generated dynamic JSON decode accepted invalid nested string byte as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 300
            || context.type_name != "DynamicJsonNestedError"
            || context.field_name != Some("payload")
            || context.expected != "valid token for field"
        {
            return Err(format!("generated dynamic JSON decode reported wrong field context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("generated dynamic JSON decode reported wrong nested error: {error:?}"));
    }
}

let remaining = iter.collect::<Vec<_>>();
if remaining.first() != Some(&(b'k' as u32)) {
    return Err(format!("generated dynamic JSON decode consumed too far after nested error: {remaining:?}"));
}
if remaining.last() != Some(&9001u32) {
    return Err(format!("generated dynamic JSON decode consumed trailing sentinel after nested error: {remaining:?}"));
}

let strict_original = DynamicJsonNestedError {
    payload: serde_json::json!(true),
};
let mut strict_tokens = Vec::new();
strict_original.encode(&mut strict_tokens);
let tag = strict_tokens.first().copied()
    .ok_or_else(|| "expected encoded dynamic JSON payload to start with a type tag".to_string())?;
if tag != 1u32 {
    return Err(format!("expected bool payload type tag 1, got {tag} in {strict_tokens:?}"));
}
strict_tokens[0] = 7u32;

let mut recovery_iter = strict_tokens.clone().into_iter();
let recovered = DynamicJsonNestedError::decode(&mut recovery_iter)
    .map_err(|error| format!("generated dynamic JSON recovery rejected malformed type tag: {error}"))?;
if recovered.payload != serde_json::json!(true) {
    return Err(format!("generated dynamic JSON recovery did not normalize 7 % 6 as bool true: {:?}", recovered.payload));
}
if recovery_iter.next().is_some() {
    return Err("generated dynamic JSON recovery left unexpected tokens after bool payload".to_string());
}

match DynamicJsonNestedError::decode_strict_full(strict_tokens) {
    Ok(value) => {
        return Err(format!("strict generated dynamic JSON field accepted malformed type tag as {value:?}"));
    }
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 7
            || context.type_name != "DynamicJsonNestedError"
            || context.field_name != Some("payload")
            || context.expected != "value type tag 0..=5"
        {
            return Err(format!("strict generated dynamic JSON field reported wrong tag context: {token:?} {context:?}"));
        }
    }
    Err(error) => {
        return Err(format!("strict generated dynamic JSON field reported wrong tag error: {error:?}"));
    }
}

Ok(())
"#,
    );
}

fn run_generated_modulo_contract_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated modulo temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated modulo src dir");

    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_rustc_hash()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs");
    if generated_code.contains("jsonschema ::") || generated_code.contains("jsonschema::") {
        cargo_toml = cargo_toml.with_jsonschema();
    }
    let cargo_toml = cargo_toml.render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated modulo Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated modulo crate");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
{body}
}}
"#,
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated modulo main.rs");

    let output = generated_modulo_cargo_command()
        .arg("run")
        .arg("--quiet")
        .env(
            "CARGO_TARGET_DIR",
            std::env::current_dir()
                .unwrap()
                .join("target/tct-modulo-contracts"),
        )
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated modulo contract crate");

    assert!(
        output.status.success(),
        "generated modulo contract failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
