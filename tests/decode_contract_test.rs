use std::fs;
use std::path::PathBuf;
use std::process::Command;

use rustc_hash::FxHashMap;
use serde_json::json;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::tokenizer::{DEFAULT_MAX_COLLECTION_LENGTH, DecodeError, JsonTokenEncoder};

fn generated_contract_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command.env("CARGO_TARGET_DIR", generated_contract_target_dir(&tct_path));
    command
}

fn generated_contract_target_dir(repo_root: &std::path::Path) -> PathBuf {
    let target_dir = std::env::var_os("TCT_GENERATED_CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            repo_root
                .join("target")
                .join("tct-generated-cargo")
                .join("decode-contract")
        });
    fs::create_dir_all(&target_dir).unwrap_or_else(|error| {
        panic!(
            "failed to create generated decode-contract Cargo target dir {}: {}",
            target_dir.display(),
            error
        )
    });
    target_dir
}

#[test]
fn test_complete_decode_rejects_invalid_string_token() {
    let mut token_iter = vec![300u32].into_iter();
    let error = String::decode(&mut token_iter).unwrap_err();
    assert_eq!(error, DecodeError::InvalidToken(300));
}

#[test]
fn test_string_decode_eof_is_strict_but_streaming_returns_partial() {
    let mut strict_tokens = vec![b'a' as u32, b'b' as u32].into_iter();
    let error = String::decode(&mut strict_tokens).unwrap_err();
    assert!(matches!(error, DecodeError::UnexpectedEndOfStream { .. }));

    let mut streaming_tokens = vec![b'a' as u32, b'b' as u32].into_iter();
    let (value, complete, bytes_decoded) =
        tct::tokenizer::decode_string_streaming(&mut streaming_tokens);
    assert_eq!(value, "ab");
    assert!(!complete);
    assert_eq!(bytes_decoded, 2);
}

#[test]
fn test_bounded_choice_decode_recovers_with_modulo_current_contract() {
    let mut token_iter = vec![3u32].into_iter();
    assert!(bool::decode(&mut token_iter).unwrap());

    let mut token_iter = vec![2u32].into_iter();
    assert_eq!(Option::<i64>::decode(&mut token_iter).unwrap(), None);
}

#[test]
fn test_strict_numeric_byte_decoders_reject_recovered_bytes() {
    let recovered_i64 = i64::decode(&mut vec![300u32, 0, 0, 0, 0, 0, 0, 0].into_iter()).unwrap();
    assert_eq!(recovered_i64, 44);

    let error = i64::decode_strict(&mut vec![300u32, 0, 0, 0, 0, 0, 0, 0].into_iter()).unwrap_err();
    assert_eq!(
        error,
        DecodeError::InvalidTokenInContext {
            token: 300,
            context: tct::tokenizer::EOFContext::new("i64", Some("byte"), "byte token 0..=255"),
        }
    );

    let mut number_tokens = vec![0u32, 300, 0, 0, 0, 0, 0, 0, 0].into_iter();
    assert_eq!(
        serde_json::Number::decode(&mut number_tokens).unwrap(),
        serde_json::Number::from(44)
    );

    let error =
        serde_json::Number::decode_strict(&mut vec![0u32, 300, 0, 0, 0, 0, 0, 0, 0].into_iter())
            .unwrap_err();
    assert_eq!(
        error,
        DecodeError::InvalidTokenInContext {
            token: 300,
            context: tct::tokenizer::EOFContext::new("i64", Some("byte"), "byte token 0..=255"),
        }
    );
}

#[test]
fn test_json_integer_strict_rejects_recovered_marker_and_payload_byte() {
    let mut recovery_tokens = vec![3u32, 42, 0, 0, 0, 0, 0, 0, 0].into_iter();
    assert_eq!(
        tct::tokenizer::JsonInteger::decode(&mut recovery_tokens)
            .unwrap()
            .to_number(),
        serde_json::Number::from(42u64)
    );

    let marker_error = tct::tokenizer::JsonInteger::decode_strict(
        &mut vec![3u32, 42, 0, 0, 0, 0, 0, 0, 0].into_iter(),
    )
    .unwrap_err();
    assert_eq!(
        marker_error,
        DecodeError::InvalidTokenInContext {
            token: 3,
            context: tct::tokenizer::EOFContext::new(
                "JsonInteger",
                Some("marker"),
                "integer signed/unsigned marker 0 or 1"
            ),
        }
    );

    let byte_error = tct::tokenizer::JsonInteger::decode_strict(
        &mut vec![0u32, 300, 0, 0, 0, 0, 0, 0, 0].into_iter(),
    )
    .unwrap_err();
    assert_eq!(
        byte_error,
        DecodeError::InvalidTokenInContext {
            token: 300,
            context: tct::tokenizer::EOFContext::new("i64", Some("byte"), "byte token 0..=255"),
        }
    );
}

#[test]
fn test_variable_base_strict_decode_rejects_recovered_width_and_digits() {
    let mut recovered_width_tokens = vec![4u32, 42u32].into_iter();
    assert_eq!(i32::decode(&mut recovered_width_tokens).unwrap(), 42);

    let width_error = i32::decode_strict(&mut vec![4u32, 42u32].into_iter()).unwrap_err();
    assert_eq!(
        width_error,
        DecodeError::InvalidTokenInContext {
            token: 4,
            context: tct::tokenizer::EOFContext::new("i32", Some("width"), "width indicator 0..=3"),
        }
    );

    let mut recovered_digit_tokens = vec![0u32, 300u32].into_iter();
    assert_eq!(i32::decode(&mut recovered_digit_tokens).unwrap(), 44);

    let digit_error = i32::decode_strict(&mut vec![0u32, 300u32].into_iter()).unwrap_err();
    assert_eq!(
        digit_error,
        DecodeError::InvalidTokenInContext {
            token: 300,
            context: tct::tokenizer::EOFContext::new(
                "i32",
                Some("digit"),
                "multi-token digit below base"
            ),
        }
    );
}

#[test]
fn test_collection_strict_decode_rejects_recovered_length_tokens() {
    let mut recovered_vec_tokens = vec![0u32, 257u32, 1u32].into_iter();
    assert_eq!(
        Vec::<bool>::decode(&mut recovered_vec_tokens).unwrap(),
        vec![true]
    );

    let vec_error =
        Vec::<bool>::decode_strict(&mut vec![0u32, 257u32, 1u32].into_iter()).unwrap_err();
    assert_eq!(
        vec_error,
        DecodeError::InvalidTokenInContext {
            token: 257,
            context: tct::tokenizer::EOFContext::new(
                "Vec<T>",
                Some("digit"),
                "multi-token digit below base"
            ),
        }
    );

    let mut recovered_map_tokens = Vec::new();
    tct::base256::encode_u32(1, &mut recovered_map_tokens, tct::base256::DEFAULT_BASE);
    recovered_map_tokens[1] = 257;
    "x".to_string().encode(&mut recovered_map_tokens);
    true.encode(&mut recovered_map_tokens);

    let mut recovery_iter = recovered_map_tokens.clone().into_iter();
    let recovered = FxHashMap::<String, bool>::decode(&mut recovery_iter).unwrap();
    assert_eq!(recovered.get("x"), Some(&true));

    let mut strict_iter = recovered_map_tokens.into_iter();
    let map_error = FxHashMap::<String, bool>::decode_strict(&mut strict_iter).unwrap_err();
    assert_eq!(
        map_error,
        DecodeError::InvalidTokenInContext {
            token: 257,
            context: tct::tokenizer::EOFContext::new(
                "FxHashMap<K,V>",
                Some("digit"),
                "multi-token digit below base"
            ),
        }
    );
}

#[test]
fn test_decode_full_rejects_trailing_tokens_after_recovery_decode() {
    assert!(bool::decode_full(vec![3u32]).unwrap());

    let error = bool::decode_full(vec![3u32, 99u32]).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("decode_full left 1 trailing token"),
        "expected trailing-token error, got: {}",
        message
    );
}

#[test]
fn test_generated_zero_token_decode_full_rejects_surplus_tokens() {
    let root_const = TctCompiler::new(json!({"const": null}), "Nothing")
        .unwrap()
        .compile()
        .expect("root null const should compile");
    let root_const_code = root_const
        .generate_code()
        .expect("root null const should generate code");
    run_generated_decode_contract_crate(
        "zero-token-root-decode-full",
        &root_const_code,
        r#"
let decoded = Nothing::decode_full(Vec::<u32>::new())
    .map_err(|error| format!("zero-token root decode_full rejected an empty stream: {error}"))?;
let decoded_json = serde_json::to_value(decoded)
    .map_err(|error| format!("zero-token root serialization failed: {error}"))?;
if decoded_json != serde_json::json!(null) {
    return Err(format!("zero-token root decoded {decoded_json}, expected null"));
}

match Nothing::decode_full(vec![17u32, 23u32]) {
    Ok(value) => Err(format!("zero-token root decode_full accepted surplus tokens as {value:?}")),
    Err(crate::tokenizer::DecodeError::InvalidData(message)) => {
        if !message.contains("decode_full left 2 trailing tokens") {
            return Err(format!("expected two trailing zero-token root tokens, got: {message}"));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected zero-token root InvalidData, got: {error:?}")),
}
"#,
    );

    let const_struct_schema = json!({
        "type": "object",
        "properties": {
            "version": {"type": "integer", "const": 1},
            "flag": {"type": "boolean", "const": false}
        },
        "required": ["version", "flag"],
        "additionalProperties": false
    });
    let const_struct = TctCompiler::new(const_struct_schema, "ZeroTokenStruct")
        .unwrap()
        .compile()
        .expect("zero-token struct should compile");
    let const_struct_code = const_struct
        .generate_code()
        .expect("zero-token struct should generate code");
    run_generated_decode_contract_crate(
        "zero-token-struct-decode-full",
        &const_struct_code,
        r#"
let decoded = ZeroTokenStruct::decode_full(Vec::<u32>::new())
    .map_err(|error| format!("zero-token struct decode_full rejected an empty stream: {error}"))?;
let decoded_json = serde_json::to_value(decoded)
    .map_err(|error| format!("zero-token struct serialization failed: {error}"))?;
if decoded_json != serde_json::json!({"version": 1, "flag": false}) {
    return Err(format!("zero-token struct decoded {decoded_json}, expected version/flag consts"));
}

match ZeroTokenStruct::decode_full(vec![99u32]) {
    Ok(value) => Err(format!("zero-token struct decode_full accepted surplus tokens as {value:?}")),
    Err(crate::tokenizer::DecodeError::InvalidData(message)) => {
        if !message.contains("decode_full left 1 trailing token") {
            return Err(format!("expected one trailing zero-token struct token, got: {message}"));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected zero-token struct InvalidData, got: {error:?}")),
}
"#,
    );
}

#[test]
fn test_fallback_vec_strict_decode_enforces_default_collection_cap() {
    let mut tokens = Vec::new();
    tct::base256::encode_u32(
        (DEFAULT_MAX_COLLECTION_LENGTH + 1) as u32,
        &mut tokens,
        tct::base256::DEFAULT_BASE,
    );

    let mut token_iter = tokens.into_iter();
    let error = Vec::<i32>::decode(&mut token_iter).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("Vec length 2049 exceeds maximum 2048"),
        "expected fallback Vec cap error, got: {}",
        message
    );
}

#[test]
fn test_fallback_hashmap_strict_decode_enforces_default_collection_cap() {
    let mut tokens = Vec::new();
    tct::base256::encode_u32(
        (DEFAULT_MAX_COLLECTION_LENGTH + 1) as u32,
        &mut tokens,
        tct::base256::DEFAULT_BASE,
    );

    let mut token_iter = tokens.into_iter();
    let error = FxHashMap::<String, i32>::decode(&mut token_iter).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("HashMap length 2049 exceeds maximum 2048"),
        "expected fallback HashMap cap error, got: {}",
        message
    );
}

#[test]
fn test_root_array_strict_decode_enforces_collection_cap() {
    let schema = json!({
        "type": "array",
        "items": { "type": "integer" }
    });

    let grammar = TctCompiler::new(schema, "SmallArray")
        .unwrap()
        .with_max_collection_length(3)
        .compile()
        .unwrap();
    assert_eq!(grammar.max_collection_length, 3);

    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "root-array-strict-decode-cap",
        &generated_code,
        r#"
let mut tokens = vec![0u32, 4u32].into_iter();
match SmallArray::decode(&mut tokens) {
    Ok(value) => Err(format!("strict decode accepted oversized array: {:?}", value)),
    Err(error) => {
        let message = error.to_string();
        if !message.contains("length 4 exceeds maximum 3") {
            return Err(format!("expected collection cap error, got: {}", message));
        }
        Ok(())
    }
}
"#,
    );
}

#[test]
fn test_generated_array_field_cap_reports_context_without_consuming_surplus() {
    let schema = json!({
        "title": "CollectionCapObject",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "items": {
                "type": "array",
                "items": { "type": "integer" }
            }
        },
        "required": ["items"]
    });

    let grammar = TctCompiler::new(schema, "CollectionCapObject")
        .unwrap()
        .with_max_collection_length(2)
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-array-field-cap-context",
        &generated_code,
        r#"
let mut tokens = vec![0u32, 3u32, 424_242u32].into_iter();
match CollectionCapObject::decode(&mut tokens) {
    Ok(value) => Err(format!("generated array field decode accepted oversized array: {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidData(message)) => {
        if !message.contains("CollectionCapObject.items") {
            return Err(format!("expected schema field context for oversized array, got: {message}"));
        }
        if !message.contains("Vec length 3 exceeds maximum 2") {
            return Err(format!("expected collection cap detail, got: {message}"));
        }
        if tokens.next() != Some(424_242u32) {
            return Err("generated array field cap error consumed trailing sentinel".to_string());
        }
        Ok(())
    }
    Err(error) => Err(format!("expected generated array field InvalidData, got: {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_additional_properties_cap_reports_context_without_consuming_surplus() {
    let schema = json!({
        "title": "AdditionalCapObject",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "name": { "type": "string" }
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "AdditionalCapObject")
        .unwrap()
        .with_max_collection_length(2)
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-additional-properties-cap-context",
        &generated_code,
        r#"
let mut tokens = Vec::new();
"ok".to_string().encode(&mut tokens);
tokens.extend([0u32, 3u32, 515_151u32]);
let mut tokens = tokens.into_iter();
match AdditionalCapObject::decode(&mut tokens) {
    Ok(value) => Err(format!("generated additional_properties decode accepted oversized map: {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidData(message)) => {
        if !message.contains("AdditionalCapObject.additional_properties") {
            return Err(format!("expected schema field context for oversized additional_properties map, got: {message}"));
        }
        if !message.contains("HashMap length 3 exceeds maximum 2") {
            return Err(format!("expected map cap detail, got: {message}"));
        }
        if tokens.next() != Some(515_151u32) {
            return Err("generated additional_properties cap error consumed trailing sentinel".to_string());
        }
        Ok(())
    }
    Err(error) => Err(format!("expected generated additional_properties InvalidData, got: {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_strict_array_length_rejects_recovered_variable_base_tokens() {
    let schema = json!({
        "title": "StrictBoolArray",
        "type": "array",
        "items": { "type": "boolean" }
    });

    let grammar = TctCompiler::new(schema, "StrictBoolArray")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-strict-array-length-variable-base",
        &generated_code,
        r#"
let mut recovered_width_tokens = vec![4u32, 0u32].into_iter();
let recovered_width = StrictBoolArray::decode(&mut recovered_width_tokens)
    .map_err(|error| format!("recovery array decode rejected recovered width: {error}"))?;
let recovered_width_json = serde_json::to_value(&recovered_width)
    .map_err(|error| format!("failed to serialize recovered width array: {error}"))?;
if recovered_width_json != serde_json::json!([]) {
    return Err(format!("expected recovered width to decode an empty array, got {recovered_width_json}"));
}

match StrictBoolArray::decode_strict_full(vec![4u32, 0u32]) {
    Ok(value) => Err(format!("strict array decode accepted recovered width as {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 4u32 {
            return Err(format!("expected invalid width token 4, got {token}"));
        }
        if context.type_name != "StrictBoolArray" || context.field_name != Some("width") {
            return Err(format!("expected StrictBoolArray.width context, got {:?}", context));
        }
        if context.expected != "width indicator 0..=3" {
            return Err(format!("expected strict width detail, got {:?}", context));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected strict array invalid width token, got {error:?}")),
}?;

let mut recovered_digit_tokens = vec![0u32, 257u32, 1u32].into_iter();
let recovered_digit = StrictBoolArray::decode(&mut recovered_digit_tokens)
    .map_err(|error| format!("recovery array decode rejected recovered digit: {error}"))?;
let recovered_digit_json = serde_json::to_value(&recovered_digit)
    .map_err(|error| format!("failed to serialize recovered digit array: {error}"))?;
if recovered_digit_json != serde_json::json!([true]) {
    return Err(format!("expected recovered digit to decode [true], got {recovered_digit_json}"));
}

match StrictBoolArray::decode_strict_full(vec![0u32, 257u32, 1u32]) {
    Ok(value) => Err(format!("strict array decode accepted recovered digit as {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 257u32 {
            return Err(format!("expected invalid digit token 257, got {token}"));
        }
        if context.type_name != "StrictBoolArray" || context.field_name != Some("digit") {
            return Err(format!("expected StrictBoolArray.digit context, got {:?}", context));
        }
        if context.expected != "multi-token digit below GENERATED_ENCODING_BASE" {
            return Err(format!("expected strict digit detail, got {:?}", context));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected strict array invalid digit token, got {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_strict_array_field_length_rejects_recovered_digit() {
    let schema = json!({
        "title": "StrictArrayFieldObject",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "items": {
                "type": "array",
                "items": { "type": "boolean" }
            }
        },
        "required": ["items"]
    });

    let grammar = TctCompiler::new(schema, "StrictArrayFieldObject")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-strict-array-field-length",
        &generated_code,
        r#"
let tokens = vec![0u32, 257u32, 1u32];

let mut recovery_iter = tokens.clone().into_iter();
let recovered = StrictArrayFieldObject::decode(&mut recovery_iter)
    .map_err(|error| format!("recovery array-field decode rejected recovered digit: {error}"))?;
if recovered.items != vec![true] {
    return Err(format!("expected recovered array field [true], got {:?}", recovered.items));
}

match StrictArrayFieldObject::decode_strict_full(tokens) {
    Ok(value) => Err(format!("strict array field decode accepted recovered length digit as {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 257u32 {
            return Err(format!("expected invalid digit token 257, got {token}"));
        }
        if context.type_name != "StrictArrayFieldObject" || context.field_name != Some("items") {
            return Err(format!("expected StrictArrayFieldObject.items context, got {:?}", context));
        }
        if context.expected != "multi-token digit below GENERATED_ENCODING_BASE" {
            return Err(format!("expected strict digit detail, got {:?}", context));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected contextual strict array-field invalid token, got {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_strict_additional_properties_length_rejects_recovered_digit() {
    let schema = json!({
        "title": "StrictAdditionalMap",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "name": { "type": "string" }
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "StrictAdditionalMap")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-strict-additional-map-length",
        &generated_code,
        r#"
let mut tokens = Vec::new();
"ok".to_string().encode(&mut tokens);
tokens.extend([0u32, 257u32]);
"x".to_string().encode(&mut tokens);
serde_json::json!(true).encode(&mut tokens);

let mut recovery_iter = tokens.clone().into_iter();
let recovered = StrictAdditionalMap::decode(&mut recovery_iter)
    .map_err(|error| format!("recovery map decode rejected recovered digit: {error}"))?;
if recovered.additional_properties.get("x") != Some(&serde_json::json!(true)) {
    return Err(format!("expected recovered additional property x=true, got {:?}", recovered.additional_properties));
}

match StrictAdditionalMap::decode_strict_full(tokens) {
    Ok(value) => Err(format!("strict additional_properties decode accepted recovered length digit as {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 257u32 {
            return Err(format!("expected invalid digit token 257, got {token}"));
        }
        if context.type_name != "StrictAdditionalMap" || context.field_name != Some("additional_properties") {
            return Err(format!("expected StrictAdditionalMap.additional_properties context, got {:?}", context));
        }
        if context.expected != "multi-token digit below GENERATED_ENCODING_BASE" {
            return Err(format!("expected strict digit detail, got {:?}", context));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected contextual strict additional_properties invalid token, got {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_strict_pattern_properties_length_rejects_recovered_digit() {
    let schema = json!({
        "title": "StrictPatternMap",
        "type": "object",
        "additionalProperties": false,
        "patternProperties": {
            "^x_": { "type": "boolean" }
        }
    });

    let grammar = TctCompiler::new(schema, "StrictPatternMap")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-strict-pattern-map-length",
        &generated_code,
        r#"
let mut tokens = Vec::new();
tokens.extend([0u32, 257u32]);
"x_flag".to_string().encode(&mut tokens);
true.encode(&mut tokens);

let mut recovery_iter = tokens.clone().into_iter();
let recovered = StrictPatternMap::decode(&mut recovery_iter)
    .map_err(|error| format!("recovery patternProperties decode rejected recovered digit: {error}"))?;
if recovered.pattern_0.get("x_flag") != Some(&true) {
    return Err(format!("expected recovered pattern property x_flag=true, got {:?}", recovered.pattern_0));
}

match StrictPatternMap::decode_strict_full(tokens) {
    Ok(value) => Err(format!("strict patternProperties decode accepted recovered length digit as {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 257u32 {
            return Err(format!("expected invalid digit token 257, got {token}"));
        }
        if context.type_name != "StrictPatternMap" || context.field_name != Some("pattern_0") {
            return Err(format!("expected StrictPatternMap.pattern_0 context, got {:?}", context));
        }
        if context.expected != "multi-token digit below GENERATED_ENCODING_BASE" {
            return Err(format!("expected strict digit detail, got {:?}", context));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected contextual strict patternProperties invalid token, got {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_strict_root_integer_wrapper_rejects_recovered_marker() {
    let schema = json!({
        "title": "StrictRootInteger",
        "type": "integer"
    });

    let grammar = TctCompiler::new(schema, "StrictRootInteger")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-strict-root-integer-wrapper",
        &generated_code,
        r#"
let recovered_tokens = vec![3u32, 42u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32];
let mut recovery_iter = recovered_tokens.clone().into_iter();
let recovered = StrictRootInteger::decode(&mut recovery_iter)
    .map_err(|error| format!("recovery root integer decode rejected recovered marker: {error}"))?;
let recovered_json = serde_json::to_value(&recovered)
    .map_err(|error| format!("failed to serialize recovered root integer: {error}"))?;
if recovered_json != serde_json::json!(42u64) {
    return Err(format!("expected recovered root integer 42, got {recovered_json}"));
}

match StrictRootInteger::decode_strict_full(recovered_tokens) {
    Ok(value) => Err(format!("strict root integer wrapper accepted recovered marker as {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 3u32 {
            return Err(format!("expected invalid marker token 3, got {token}"));
        }
        if context.type_name != "JsonInteger" || context.field_name != Some("marker") {
            return Err(format!("expected JsonInteger.marker context, got {:?}", context));
        }
        if context.expected != "integer signed/unsigned marker 0 or 1" {
            return Err(format!("expected strict integer marker detail, got {:?}", context));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected strict root integer invalid marker, got {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_strict_format_string_wrapper_rejects_recovered_payload_digit() {
    let schema = json!({
        "title": "StrictInt32String",
        "type": "string",
        "format": "int32"
    });

    let grammar = TctCompiler::new(schema, "StrictInt32String")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-strict-format-string-wrapper",
        &generated_code,
        r#"
let recovered_tokens = vec![0u32, 0u32, 300u32];
let mut recovery_iter = recovered_tokens.clone().into_iter();
let recovered = StrictInt32String::decode(&mut recovery_iter)
    .map_err(|error| format!("recovery format string decode rejected recovered digit: {error}"))?;
let recovered_json = serde_json::to_value(&recovered)
    .map_err(|error| format!("failed to serialize recovered format string: {error}"))?;
if recovered_json != serde_json::json!("44") {
    return Err(format!("expected recovered format string \"44\", got {recovered_json}"));
}

match StrictInt32String::decode_strict_full(recovered_tokens) {
    Ok(value) => Err(format!("strict format string wrapper accepted recovered digit as {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 300u32 {
            return Err(format!("expected invalid digit token 300, got {token}"));
        }
        if context.type_name != "i32" || context.field_name != Some("digit") {
            return Err(format!("expected i32.digit context, got {:?}", context));
        }
        if context.expected != "multi-token digit below GENERATED_ENCODING_BASE" {
            return Err(format!("expected strict digit detail, got {:?}", context));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected strict format string invalid digit, got {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_validators_never_fetch_external_refs() {
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    // A loopback listener stands in for a remote schema host. Schema-aware
    // anyOf/oneOf deserialization compiles each branch with `jsonschema`, whose
    // default retriever would GET the referenced URL; generated tokenizers must
    // stay offline ("TCT does not fetch schemas while compiling or while
    // generated tokenizers run").
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind loopback listener");
    listener
        .set_nonblocking(true)
        .expect("failed to make the loopback listener non-blocking");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let watcher = {
        let hits = Arc::clone(&hits);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        hits.fetch_add(1, Ordering::SeqCst);
                        drop(stream);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        })
    };

    let schema = json!({
        "title": "RemoteRef",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "v": {
                "anyOf": [
                    { "$ref": format!("http://127.0.0.1:{port}/external-schema.json") },
                    { "type": "integer" }
                ]
            }
        }
    });
    let grammar = TctCompiler::new(schema, "RemoteRef")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-validators-offline",
        &generated_code,
        r##"
let value: RemoteRef = serde_json::from_str(r#"{"v": 5}"#)
    .map_err(|error| format!("failed to deserialize RemoteRef: {error}"))?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let decoded = RemoteRef::decode_full(tokens)
    .map_err(|error| format!("decode failed: {error:?}"))?;
let round_tripped = serde_json::to_value(&decoded)
    .map_err(|error| format!("failed to serialize decoded value: {error}"))?;
if round_tripped != serde_json::json!({"v": 5}) {
    return Err(format!("expected {{\"v\":5}}, got {round_tripped}"));
}
Ok(())
"##,
    );

    stop.store(true, Ordering::SeqCst);
    watcher.join().expect("loopback watcher thread panicked");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "the generated tokenizer connected to the external $ref host; generated validators must not fetch schemas"
    );
}

#[test]
fn test_generated_code_round_trips_hostile_property_names_and_enum_values() {
    // Property names and enum values are arbitrary JSON strings. They must never
    // panic the compiler (control characters, symbols, and emoji used to), break
    // out of the emitted string literals or identifiers, or change on the way
    // through a generated tokenizer. Colliding names ("a-b" and "a_b" sanitize to
    // the same identifier) must stay distinct fields.
    let property_names = [
        "a\"b",
        "x\\y",
        "line\nbreak",
        "tab\there",
        "cr\rhere",
        "/* comment */",
        "*/ fn evil() {} /*",
        "pub fn evil() {}",
        "r#\"raw\"#",
        "}}{{",
        "self",
        "crate",
        "type",
        "9starts_with_digit",
        "ünï✓",
        "🙂",
        "a\u{2028}b",
        "a\u{a0}b",
        "nul\u{0}x",
        "ß",
        "İ",
        "a-b",
        "a_b",
        "aB",
        "_",
        "",
    ];
    let enum_values = [
        "a\"b",
        "x\\y",
        "line\nbreak",
        "/* c */",
        "}}",
        "'; drop table",
        "{{ }}",
        "tab\tend",
        "\u{2028}",
        "🙂",
        "",
        "r#\"x\"#",
    ];

    let mut properties = serde_json::Map::new();
    for name in property_names {
        properties.insert(name.to_string(), json!({ "type": "string" }));
    }
    properties.insert("mode".to_string(), json!({ "enum": enum_values }));
    let schema = json!({
        "title": "Hostile",
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
    });

    let grammar = TctCompiler::new(schema, "Hostile")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();

    // `{:?}` renders each string as a valid, escaped Rust literal.
    let literal_list = |values: &[&str]| {
        values
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let body = format!(
        r#"
let names: &[&str] = &[{names}];
let modes: &[&str] = &[{modes}];
let mut base = serde_json::Map::new();
for (index, name) in names.iter().enumerate() {{
    base.insert(name.to_string(), serde_json::json!(format!("value-{{index}}")));
}}
for mode in modes {{
    let mut instance = base.clone();
    instance.insert("mode".to_string(), serde_json::json!(mode));
    let expected = serde_json::Value::Object(instance);
    let original: Hostile = serde_json::from_value(expected.clone())
        .map_err(|error| format!("{{mode:?}}: failed to deserialize: {{error}}"))?;
    let mut tokens = Vec::new();
    original.encode(&mut tokens);
    for (kind, decoded) in [
        ("recovery", Hostile::decode_full(tokens.clone())),
        ("strict", Hostile::decode_strict_full(tokens.clone())),
    ] {{
        let decoded = decoded.map_err(|error| format!("{{mode:?}}: {{kind}} decode failed: {{error:?}}"))?;
        let actual = serde_json::to_value(&decoded)
            .map_err(|error| format!("{{mode:?}}: failed to serialize: {{error}}"))?;
        if actual != expected {{
            return Err(format!("{{mode:?}}: {{kind}} decode changed the value:\n  expected {{expected}}\n  actual   {{actual}}"));
        }}
    }}
}}
Ok(())
"#,
        names = literal_list(&property_names),
        modes = literal_list(&enum_values),
    );
    run_generated_decode_contract_crate("generated-hostile-names", &generated_code, &body);
}

#[test]
fn test_generated_date_time_format_round_trips_fractional_seconds() {
    // The O3 date-time fast path stores only a whole-second Unix timestamp, so
    // it may only be used for values with no sub-second part. Everything else
    // must take the string fallback and round trip exactly.
    let schema = json!({
        "title": "Stamp",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "at": { "type": "string", "format": "date-time" }
        },
        "required": ["at"]
    });

    let grammar = TctCompiler::new(schema, "Stamp")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-date-time-fractional-seconds",
        &generated_code,
        r#"
for text in [
    "2024-01-01T00:00:00+00:00",
    "2024-01-01T00:00:00.500+00:00",
    "2024-01-01T00:00:00.123456+00:00",
    "2024-01-01T00:00:00.123456789+00:00",
    "2016-12-31T23:59:60+00:00",
    "2024-01-01T00:00:00Z",
] {
    let expected = serde_json::json!({ "at": text });
    let original: Stamp = serde_json::from_value(expected.clone())
        .map_err(|error| format!("{text}: failed to build Stamp: {error}"))?;
    let mut tokens = Vec::new();
    original.encode(&mut tokens);
    if tokens.len() != original.token_count() {
        return Err(format!(
            "{text}: token_count {} does not match the {} encoded tokens",
            original.token_count(),
            tokens.len()
        ));
    }
    for (mode, decoded) in [
        ("recovery", Stamp::decode_full(tokens.clone())),
        ("strict", Stamp::decode_strict_full(tokens.clone())),
    ] {
        let decoded = decoded.map_err(|error| format!("{text}: {mode} decode failed: {error:?}"))?;
        let actual = serde_json::to_value(&decoded)
            .map_err(|error| format!("{text}: failed to serialize decoded Stamp: {error}"))?;
        if actual != expected {
            return Err(format!("{text}: {mode} decode returned {actual}, expected {expected}"));
        }
    }
}
Ok(())
"#,
    );
}

#[test]
fn test_generated_struct_decode_reports_surplus_tokens_without_consuming_them() {
    let schema = json!({
        "title": "SurplusObject",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "status": {
                "enum": ["ready", "blocked"]
            },
            "flag": {
                "type": "boolean"
            }
        },
        "required": ["status", "flag"]
    });

    let grammar = TctCompiler::new(schema, "SurplusObject")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-struct-surplus-accounting",
        &generated_code,
        r#"
let original = SurplusObject {
    status: SurplusObjectStatus::Ready,
    flag: true,
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
tokens.push(424_242u32);
tokens.push(99u32);

let total = tokens.len();
let mut iter = crate::tokenizer::BufferedTokenIterator::from_vec(tokens.clone());
let decoded = SurplusObject::decode(&mut iter).map_err(|error| error.to_string())?;
if decoded.status != SurplusObjectStatus::Ready || !decoded.flag {
    return Err(format!("generated decode changed payload while checking surplus accounting: {:?}", decoded));
}
if iter.remaining() != 2 {
    return Err(format!("expected two surplus tokens to remain, got {}", iter.remaining()));
}
let consumed = total - iter.remaining();
if consumed + 2 != total {
    return Err(format!("consumed-token accounting drifted: consumed={consumed}, total={total}"));
}
if iter.next() != Some(424_242u32) || iter.next() != Some(99u32) {
    return Err("generated decode consumed or reordered trailing sentinels".to_string());
}

match SurplusObject::decode_full(tokens) {
    Ok(value) => return Err(format!("generated decode_full accepted surplus tokens: {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidData(message)) => {
        if !message.contains("decode_full left 2 trailing tokens") {
            return Err(format!("expected generated decode_full trailing-token error, got: {message}"));
        }
    }
    Err(error) => return Err(format!("expected generated decode_full InvalidData, got: {error:?}")),
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_required_field_eof_reports_schema_field_context() {
    let schema = json!({
        "title": "TruncatedRequired",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {
                "type": "string"
            }
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "TruncatedRequired")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-required-field-eof-context",
        &generated_code,
        r#"
let original = TruncatedRequired {
    name: "ok".to_string(),
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
let removed = tokens.pop().ok_or_else(|| "expected encoded string to contain an end marker".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove string end marker, got {removed}"));
}

let mut iter = tokens.into_iter();
match TruncatedRequired::decode(&mut iter) {
    Ok(value) => Err(format!("generated decode accepted a truncated required field: {:?}", value)),
    Err(error) => {
        let message = error.to_string();
        if !message.contains("TruncatedRequired.name") {
            return Err(format!("expected schema field context for truncated field, got: {message}"));
        }
        if !message.contains("byte or END_STRING_MARKER") {
            return Err(format!("expected primitive EOF expectation to be preserved, got: {message}"));
        }
        Ok(())
    }
}
"#,
    );
}

#[test]
fn test_generated_required_field_invalid_token_reports_schema_field_context() {
    let schema = json!({
        "title": "MalformedRequired",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {
                "type": "string"
            }
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "MalformedRequired")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-required-field-invalid-token-context",
        &generated_code,
        r#"
let original = MalformedRequired {
    name: "ok".to_string(),
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
let first = tokens.first_mut().ok_or_else(|| "expected encoded string to contain a byte".to_string())?;
*first = 300u32;

let mut iter = tokens.into_iter();
match MalformedRequired::decode(&mut iter) {
    Ok(value) => Err(format!("generated decode accepted an invalid required-field token: {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token, context }) => {
        if token != 300u32 {
            return Err(format!("expected invalid token 300, got {token}"));
        }
        if context.type_name != "MalformedRequired" || context.field_name != Some("name") {
            return Err(format!("expected MalformedRequired.name context, got {:?}", context));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected contextual invalid token error, got: {error:?}")),
}
"#,
    );
}

#[test]
fn test_generated_required_field_invalid_utf8_reports_schema_field_context() {
    let schema = json!({
        "title": "MalformedUtf8",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {
                "type": "string"
            }
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "MalformedUtf8")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    run_generated_decode_contract_crate(
        "generated-required-field-invalid-utf8-context",
        &generated_code,
        r#"
let original = MalformedUtf8 {
    name: "ok".to_string(),
};
let mut tokens = Vec::new();
original.encode(&mut tokens);
tokens.clear();
tokens.push(0xC3u32);
tokens.push(256u32);

let mut iter = tokens.into_iter();
match MalformedUtf8::decode(&mut iter) {
    Ok(value) => Err(format!("generated decode accepted invalid UTF-8: {:?}", value)),
    Err(crate::tokenizer::DecodeError::InvalidData(message)) => {
        if !message.contains("MalformedUtf8.name") {
            return Err(format!("expected field context for invalid UTF-8, got: {message}"));
        }
        if !message.contains("Invalid UTF-8") {
            return Err(format!("expected UTF-8 detail, got: {message}"));
        }
        Ok(())
    }
    Err(error) => Err(format!("expected contextual invalid-data error, got: {error:?}")),
}
"#,
    );
}

fn run_generated_decode_contract_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_regex()
        .with_chrono()
        .with_jsonschema()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated decode contract crate");

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

    let output = generated_contract_cargo_command()
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

/// Users compile generated code without the lint caps that this repository's own
/// builds apply (`GENERATED_CARGO_RUSTFLAGS`), so a warning here is a warning in
/// every user's build. The schema covers the constructs that used to warn: constant
/// (single-variant) enums, a discriminated union with partial encoding, and an
/// `allOf` whose member needs a JSON Schema validator but no object-field checks.
#[test]
fn test_generated_code_compiles_without_rustc_warnings() {
    let schema = json!({
        "title": "WarningFree",
        "type": "object",
        "required": ["apiVersion", "item", "limits"],
        "properties": {
            "apiVersion": { "enum": ["v1"] },
            "item": { "oneOf": [
                { "$ref": "#/definitions/Cat" },
                { "$ref": "#/definitions/Dog" }
            ] },
            "limits": { "allOf": [
                { "type": "object", "properties": {
                    "cpu": { "type": "string" },
                    "memory": { "type": "string" }
                } },
                { "anyOf": [ { "required": ["cpu"] }, { "required": ["memory"] } ] }
            ] }
        },
        "definitions": {
            "Cat": {
                "type": "object",
                "required": ["kind", "lives"],
                "properties": {
                    "kind": { "enum": ["cat"] },
                    "lives": { "type": "integer", "minimum": 1, "maximum": 9 }
                }
            },
            "Dog": {
                "type": "object",
                "required": ["kind", "good"],
                "properties": { "kind": { "enum": ["dog"] }, "good": { "type": "boolean" } }
            }
        }
    });
    let generated_code = TctCompiler::new(schema, "WarningFree")
        .expect("schema should be accepted")
        .with_optimization_level(tct::OptimizationLevel::O3)
        .compile()
        .expect("schema should compile")
        .generate_code()
        .expect("code generation should succeed");

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = GeneratedCargoToml::new("warning_free_codegen")
        .with_rustc_hash()
        .with_regex()
        .with_lazy_static()
        .with_jsonschema()
        .with_chrono()
        .with_base64()
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into the generated crate");
    // The generated file is the crate root, exactly as the README tells users to use it.
    fs::write(src_dir.join("lib.rs"), generated_code).expect("failed to write generated lib.rs");

    // Replace the lint cap with a deny: registry dependencies stay capped by Cargo
    // itself, so only the generated crate can fail.
    let output = generated_contract_cargo_command()
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env("RUSTFLAGS", "-D warnings")
        .arg("check")
        .arg("--lib")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run cargo check on the generated crate");

    assert!(
        output.status.success(),
        "generated code emits rustc warnings\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Real schemas often name definitions like Rust prelude items (`string`, `default`,
/// `result`, `option`). The generated code uses those items unqualified, so a type
/// with the same name would shadow them and break the whole crate; such definitions
/// get a `Type` suffix instead, and the renamed types are part of the generated API.
#[test]
fn test_generated_code_compiles_when_definitions_are_named_like_prelude_items() {
    let schema = json!({
        "title": "PreludeNames",
        "type": "object",
        "required": ["name", "mode", "outcome"],
        "properties": {
            "name": { "$ref": "#/definitions/string" },
            "mode": { "$ref": "#/definitions/default" },
            "outcome": { "$ref": "#/definitions/result" },
            "tags": { "type": "array", "items": { "$ref": "#/definitions/option" } }
        },
        "definitions": {
            "string": { "type": "string", "minLength": 1 },
            "default": { "enum": ["fast", "slow"] },
            "result": {
                "type": "object",
                "required": ["ok"],
                "properties": { "ok": { "type": "boolean" } }
            },
            "option": {
                "type": "object",
                "properties": { "count": { "type": "integer", "minimum": 0, "maximum": 9 } }
            }
        }
    });
    let generated_code = TctCompiler::new(schema, "PreludeNames")
        .expect("schema should be accepted")
        .compile()
        .expect("schema should compile")
        .generate_code()
        .expect("code generation should succeed");

    run_generated_decode_contract_crate(
        "prelude_named_definitions",
        &generated_code,
        r##"
    let json = r#"{"name":"bob","mode":"fast","outcome":{"ok":true},"tags":[{"count":3},{}]}"#;
    let value: PreludeNames = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let mut tokens = Vec::new();
    value.encode(&mut tokens);
    let decoded = PreludeNames::decode_full(tokens.iter().copied())
        .map_err(|error| format!("{error:?}"))?;
    let expected: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let actual = serde_json::to_value(&decoded).map_err(|error| error.to_string())?;
    if actual != expected {
        return Err(format!("round trip changed the value: {actual}"));
    }
    // The renamed definition types are part of the generated API.
    let _mode = DefaultType::Fast;
    let _outcome: Option<ResultType> = None;
    let _option: Option<OptionType> = None;
    let _name: Option<StringType> = None;
    Ok(())
"##,
    );
}
