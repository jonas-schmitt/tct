//! BPE compression integration tests.
//!
//! These tests exercise the supported post-TCT compression path. Legacy
//! dictionary-trained BPE artifacts are rejected rather than interpreted with a
//! different token layout.

use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::{StringEncoding, TctCompiler};

fn parse_generated_code(code: &str) -> syn::File {
    match syn::parse_file(code) {
        Ok(file) => file,
        Err(error) => panic!("generated BPE code should parse as Rust: {error}"),
    }
}

fn generated_module<'a>(file: &'a syn::File, name: &str) -> Option<&'a syn::ItemMod> {
    file.items.iter().find_map(|item| match item {
        syn::Item::Mod(item_mod) if item_mod.ident == name => Some(item_mod),
        _ => None,
    })
}

fn generated_module_items<'a>(file: &'a syn::File, name: &str) -> &'a [syn::Item] {
    generated_module(file, name)
        .and_then(|item_mod| item_mod.content.as_ref())
        .map(|(_, items)| items.as_slice())
        .unwrap_or_else(|| panic!("expected inline generated module {name}"))
}

fn generated_function<'a>(items: &'a [syn::Item], name: &str) -> &'a syn::ItemFn {
    items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(item_fn) if item_fn.sig.ident == name => Some(item_fn),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected generated function {name}"))
}

fn generated_struct<'a>(items: &'a [syn::Item], name: &str) -> &'a syn::ItemStruct {
    items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item_struct) if item_struct.ident == name => Some(item_struct),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected generated struct {name}"))
}

fn generated_enum<'a>(items: &'a [syn::Item], name: &str) -> &'a syn::ItemEnum {
    items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident == name => Some(item_enum),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected generated enum {name}"))
}

fn generated_const<'a>(items: &'a [syn::Item], name: &str) -> &'a syn::ItemConst {
    items
        .iter()
        .find_map(|item| match item {
            syn::Item::Const(item_const) if item_const.ident == name => Some(item_const),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected generated constant {name}"))
}

fn generated_const_literal_u32(items: &[syn::Item], name: &str) -> u32 {
    let item_const = generated_const(items, name);
    match item_const.expr.as_ref() {
        syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Int(lit_int) => lit_int
                .base10_parse()
                .unwrap_or_else(|error| panic!("invalid u32 literal for {name}: {error}")),
            _ => panic!("expected integer literal for generated constant {name}"),
        },
        _ => panic!("expected literal expression for generated constant {name}"),
    }
}

fn generated_const_literal_usize(items: &[syn::Item], name: &str) -> usize {
    let item_const = generated_const(items, name);
    match item_const.expr.as_ref() {
        syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Int(lit_int) => lit_int
                .base10_parse()
                .unwrap_or_else(|error| panic!("invalid usize literal for {name}: {error}")),
            _ => panic!("expected integer literal for generated constant {name}"),
        },
        _ => panic!("expected literal expression for generated constant {name}"),
    }
}

fn generated_function_signature_contains(item_fn: &syn::ItemFn, expected: &str) -> bool {
    use quote::ToTokens;

    item_fn.sig.to_token_stream().to_string().contains(expected)
}

fn generated_bpe_cargo_command() -> Command {
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

fn write_bpe_merge_file(base_vocab_size: u32, merges: Vec<BpeMerge>) -> tempfile::NamedTempFile {
    write_bpe_merge_file_with_dict_size(base_vocab_size, merges, None)
}

fn write_bpe_merge_file_with_dict_size(
    base_vocab_size: u32,
    merges: Vec<BpeMerge>,
    dict_size: Option<usize>,
) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().expect("failed to create temp BPE file");
    let max_merges = merges.len();
    let table = BpeMergeTable {
        schema: "test".to_string(),
        base_vocab_size,
        merges,
        metadata: BpeMetadata {
            trained_on_instances: 1,
            training_date: "2026-01-01".to_string(),
            max_merges,
            dict_size,
        },
    };
    let json = serde_json::to_string_pretty(&table).expect("failed to serialize BPE table");
    fs::write(file.path(), json).expect("failed to write temp BPE file");
    file
}

fn single_ab_merge(base_vocab_size: u32) -> Vec<BpeMerge> {
    vec![BpeMerge {
        pair: [97, 98],
        new_token: base_vocab_size,
        frequency: 10,
    }]
}

fn representative_bpe_merges(base_vocab_size: u32) -> Vec<BpeMerge> {
    vec![
        BpeMerge {
            pair: [97, 98],
            new_token: base_vocab_size,
            frequency: 50,
        },
        BpeMerge {
            pair: [base_vocab_size, 99],
            new_token: base_vocab_size + 1,
            frequency: 40,
        },
        BpeMerge {
            pair: [120, 120],
            new_token: base_vocab_size + 2,
            frequency: 30,
        },
        BpeMerge {
            pair: [base_vocab_size + 2, 120],
            new_token: base_vocab_size + 3,
            frequency: 20,
        },
        BpeMerge {
            pair: [42, 43],
            new_token: base_vocab_size + 4,
            frequency: 10,
        },
    ]
}

fn decode_chain_bpe_merges(base_vocab_size: u32, depth: usize) -> (Vec<BpeMerge>, Vec<u32>, u32) {
    assert!(depth > 0);
    let mut merges = Vec::with_capacity(depth);
    let mut expected = vec![1, 2];

    merges.push(BpeMerge {
        pair: [1, 2],
        new_token: base_vocab_size,
        frequency: depth as u64,
    });

    for index in 1..depth {
        let next_base = 3 + (index as u32 % 70);
        expected.push(next_base);
        merges.push(BpeMerge {
            pair: [base_vocab_size + index as u32 - 1, next_base],
            new_token: base_vocab_size + index as u32,
            frequency: (depth - index) as u64,
        });
    }

    let last_token = base_vocab_size + depth as u32 - 1;
    (merges, expected, last_token)
}

#[test]
fn test_bpe_code_generation_with_merge_file() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(257, single_ab_merge(257));

    let compiler = TctCompiler::new(schema, "TestData")
        .expect("Failed to create compiler")
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE merge file");

    let grammar = compiler.compile().expect("Compilation failed");
    let code = grammar.generate_code().expect("Code generation failed");
    let parsed = parse_generated_code(&code);
    let bpe_items = generated_module_items(&parsed, "bpe");

    generated_function(bpe_items, "encode");
    generated_function(bpe_items, "encode_owned");
    generated_function(bpe_items, "decode");
    generated_const(bpe_items, "BASE_VOCAB_SIZE");
    assert!(code.contains("const MERGE_PAIRS"));
    assert!(code.contains("const MERGE_TOKENS"));
    assert!(code.contains("const MERGE_PAIR_INDEX"));
    assert!(code.contains("const DECODE_STEP_BUDGETS"));
    assert!(!code.contains("PAIR_MAP_CACHE"));
    assert!(!code.contains("DECODE_STEP_BUDGETS_CACHE"));
    assert!(!code.contains("OnceLock"));
    assert!(!code.contains("build_decode_step_budgets"));
    assert!(code.contains("let mut expansion_stack = Vec::new();"));
    assert!(!code.contains("expand_token(left"));
    assert!(!code.contains("expand_token(right"));
}

#[test]
fn test_bpe_merge_file_validation() {
    let schema = json!({"type": "string"});

    let result = TctCompiler::new(schema, "Test")
        .unwrap()
        .with_bpe_merges("nonexistent.json");

    assert!(result.is_err(), "Should fail with non-existent merge file");
}

#[test]
fn test_bpe_optional_feature() {
    let schema = json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer"}
        }
    });

    let compiler = TctCompiler::new(schema, "Simple").expect("Failed to create compiler");

    let grammar = compiler.compile().expect("Compilation failed");
    let code = grammar.generate_code().expect("Code generation failed");
    let parsed = parse_generated_code(&code);

    assert!(
        generated_module(&parsed, "bpe").is_none(),
        "Generated code should not contain BPE module when BPE is not enabled"
    );
}

#[test]
fn test_bpe_merge_table_embedded_correctly() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(
        257,
        vec![
            BpeMerge {
                pair: [97, 98],
                new_token: 257,
                frequency: 10,
            },
            BpeMerge {
                pair: [257, 99],
                new_token: 258,
                frequency: 5,
            },
        ],
    );

    let compiler = TctCompiler::new(schema, "StringBpe")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE");

    let grammar = compiler.compile().expect("Compilation failed");
    let code = grammar.generate_code().expect("Code generation failed");
    let parsed = parse_generated_code(&code);
    let bpe_items = generated_module_items(&parsed, "bpe");

    generated_const(bpe_items, "BASE_VOCAB_SIZE");
    assert_eq!(generated_const_literal_usize(bpe_items, "MAX_MERGES"), 2);
    let compact_code: String = code.split_whitespace().collect();
    let budget_start = code
        .find("const DECODE_STEP_BUDGETS")
        .expect("missing decode budget constant");
    let budget_snippet = &code[budget_start..(budget_start + 160).min(code.len())];
    assert!(
        compact_code.contains("constDECODE_STEP_BUDGETS:[usize;2usize]=[3usize,5usize];"),
        "unexpected decode budget constant: {}",
        budget_snippet
    );
}

#[test]
fn test_bpe_start_uses_base_vocab() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(257, single_ab_merge(257));

    let compiler = TctCompiler::new(schema, "BaseBpe")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE");

    let grammar = compiler.compile().expect("Compilation should succeed");
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");

    assert_eq!(
        extract_generated_constant_value(&code, "BASE_VOCAB_SIZE"),
        257
    );
    assert_eq!(extract_generated_constant_value(&code, "BPE_START"), 257);
    assert_eq!(
        extract_generated_constant_value(&code, "VOCAB_SIZE"),
        259,
        "VOCAB_SIZE should be base_vocab + bpe_merges + PAD"
    );
}

#[test]
fn test_ascii_bpe_round_trip_uses_effective_base_vocab_start() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(256, single_ab_merge(256));

    let grammar = TctCompiler::new(schema, "AsciiBpeRoundTrip")
        .unwrap()
        .with_string_encoding(StringEncoding::Ascii)
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE")
        .compile()
        .expect("ASCII BPE compilation should succeed");

    assert_eq!(grammar.base_vocab, 256);
    let generated_code = grammar.generate_code().unwrap();
    assert_eq!(
        extract_generated_constant_value(&generated_code, "BASE_VOCAB_SIZE"),
        256
    );
    assert_eq!(
        extract_generated_constant_value(&generated_code, "BPE_START"),
        256
    );
    assert_eq!(
        extract_generated_constant_value(&generated_code, "PAD_TOKEN"),
        257
    );
    assert_eq!(
        extract_generated_constant_value(&generated_code, "VOCAB_SIZE"),
        258
    );

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("ascii-bpe-round-trip")
            .with_rustc_hash()
            .with_bin("ascii-bpe-round-trip", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
    if BASE_VOCAB_SIZE != 256 {{
        return Err(format!("BASE_VOCAB_SIZE mismatch: {{}}", BASE_VOCAB_SIZE));
    }}
    if PAD_TOKEN != 257 {{
        return Err(format!("PAD_TOKEN mismatch: {{}}", PAD_TOKEN));
    }}
    if VOCAB_SIZE != 258 {{
        return Err(format!("VOCAB_SIZE mismatch: {{}}", VOCAB_SIZE));
    }}

    let input = String::from("ab");
    let tokens = codec::encode(&input);
    if !tokens.contains(&256) {{
        return Err(format!("expected ASCII BPE token 256 in {{:?}}", tokens));
    }}
    if let Some(max_token) = tokens.iter().max() {{
        if *max_token >= VOCAB_SIZE {{
            return Err(format!("token {{}} outside VOCAB_SIZE {{}}", max_token, VOCAB_SIZE));
        }}
    }}

    let (decoded, consumed, total) = codec::decode::<String>(tokens.clone())
        .map_err(|error| format!("decode failed: {{}}", error))?;
    if consumed != total {{
        return Err(format!("decode left {{}} surplus base tokens", total - consumed));
    }}
    if decoded != input {{
        return Err(format!("decoded {{:?}} but expected {{:?}}", decoded, input));
    }}

    let (strict_decoded, strict_consumed, strict_total) = codec::decode_strict::<String>(tokens.clone())
        .map_err(|error| format!("strict decode failed: {{}}", error))?;
    if strict_consumed != strict_total {{
        return Err(format!("strict decode left {{}} surplus base tokens", strict_total - strict_consumed));
    }}
    if strict_decoded != input {{
        return Err(format!("strict decoded {{:?}} but expected {{:?}}", strict_decoded, input));
    }}

    let recovered_pad = bpe::decode(&[PAD_TOKEN], VOCAB_SIZE);
    if recovered_pad != vec![PAD_TOKEN % BASE_VOCAB_SIZE] {{
        return Err(format!("recovery decode did not normalize PAD_TOKEN: {{:?}}", recovered_pad));
    }}

    match bpe::decode_strict(&[PAD_TOKEN], VOCAB_SIZE) {{
        Ok(value) => {{
            return Err(format!("strict BPE decode accepted PAD_TOKEN as {{:?}}", value));
        }}
        Err(crate::tokenizer::DecodeError::InvalidTokenInContext {{ token, context }}) => {{
            if token != PAD_TOKEN
                || context.type_name != "BpeDecoder"
                || context.field_name != Some("token")
                || context.expected != "base token or known BPE merge token"
            {{
                return Err(format!("strict BPE decode reported wrong PAD_TOKEN error: {{token:?}} {{context:?}}"));
            }}
        }}
        Err(error) => {{
            return Err(format!("strict BPE decode reported wrong PAD_TOKEN error: {{error}}"));
        }}
    }}

    match bpe::decode_strict(&[VOCAB_SIZE], VOCAB_SIZE) {{
        Ok(value) => {{
            return Err(format!("strict BPE decode accepted out-of-range token as {{:?}}", value));
        }}
        Err(crate::tokenizer::DecodeError::InvalidTokenInContext {{ token, context }}) => {{
            if token != VOCAB_SIZE
                || context.type_name != "BpeDecoder"
                || context.field_name != Some("token")
                || context.expected != "token below BPE vocabulary size"
            {{
                return Err(format!("strict BPE decode reported wrong out-of-range error: {{token:?}} {{context:?}}"));
            }}
        }}
        Err(error) => {{
            return Err(format!("strict BPE decode reported wrong out-of-range error: {{error}}"));
        }}
    }}

    match codec::decode_strict::<String>(vec![PAD_TOKEN]) {{
        Ok((value, consumed, total)) => {{
            return Err(format!("strict codec decode accepted PAD_TOKEN as {{value:?}} consumed={{consumed}} total={{total}}"));
        }}
        Err(crate::tokenizer::DecodeError::InvalidTokenInContext {{ token, context }}) => {{
            if token != PAD_TOKEN
                || context.type_name != "BpeDecoder"
                || context.field_name != Some("token")
                || context.expected != "base token or known BPE merge token"
            {{
                return Err(format!("strict codec decode reported wrong PAD_TOKEN error: {{token:?}} {{context:?}}"));
            }}
        }}
        Err(error) => {{
            return Err(format!("strict codec decode reported wrong PAD_TOKEN error: {{error}}"));
        }}
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_bpe_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated ASCII BPE crate");

    assert!(
        output.status.success(),
        "generated ASCII BPE round trip failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bpe_start_uses_widened_base_vocab() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(512, single_ab_merge(512));

    let compiler = TctCompiler::new(schema, "WideBaseBpe")
        .unwrap()
        .with_max_base_vocab(512)
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE");

    let grammar = compiler.compile().expect("Compilation should succeed");
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");

    assert_eq!(
        extract_generated_constant_value(&code, "BASE_VOCAB_SIZE"),
        512
    );
    assert_eq!(extract_generated_constant_value(&code, "BPE_START"), 512);
    assert_eq!(
        extract_generated_constant_value(&code, "VOCAB_SIZE"),
        514,
        "VOCAB_SIZE should track the widened generated base vocabulary"
    );
}

#[test]
fn test_ascii_bpe_rejects_stale_129_base_vocab_layout() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(129, single_ab_merge(129));

    let result = TctCompiler::new(schema, "AsciiStaleBaseBpe")
        .unwrap()
        .with_string_encoding(StringEncoding::Ascii)
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load stale BPE")
        .compile();

    let error = match result {
        Ok(_) => panic!("stale ASCII BPE layout should be rejected"),
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(
        message.contains("base_vocab_size 129")
            && message.contains("current generated base vocabulary is 256")
            && message.contains("Retrain BPE"),
        "unexpected stale ASCII BPE layout error: {}",
        message
    );
}

#[test]
fn test_bpe_rejects_stale_base_vocab_layout() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(257, single_ab_merge(257));

    let result = TctCompiler::new(schema, "StaleBaseBpe")
        .unwrap()
        .with_max_base_vocab(512)
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load stale BPE")
        .compile();

    let error = match result {
        Ok(_) => panic!("stale BPE layout should be rejected"),
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(
        message.contains("base_vocab_size 257")
            && message.contains("current generated base vocabulary is 512"),
        "unexpected stale BPE layout error: {}",
        message
    );
}

#[test]
fn test_bpe_rejects_legacy_dictionary_metadata() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file_with_dict_size(257, single_ab_merge(257), Some(1000));

    let result = TctCompiler::new(schema, "LegacyDictBpe")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("legacy BPE file path should load before compile-time validation")
        .compile();

    let error = match result {
        Ok(_) => panic!("dictionary-trained BPE should be rejected"),
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(
        message.contains("legacy dict_size 1000"),
        "unexpected legacy dictionary BPE error: {}",
        message
    );
}

#[test]
fn test_generated_bpe_round_trip_uses_base_vocab_start() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(257, single_ab_merge(257));

    let grammar = TctCompiler::new(schema, "BpeRoundTrip")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE")
        .compile()
        .expect("Compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("bpe-round-trip")
            .with_rustc_hash()
            .with_bin("bpe-round-trip", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
    if BASE_VOCAB_SIZE != 257 {{
        return Err(format!("BASE_VOCAB_SIZE mismatch: {{}}", BASE_VOCAB_SIZE));
    }}
    if VOCAB_SIZE != 259 {{
        return Err(format!("VOCAB_SIZE mismatch: {{}}", VOCAB_SIZE));
    }}

    let input = String::from("ab");
    let tokens = codec::encode(&input);
    if !tokens.contains(&257) {{
        return Err(format!("expected BPE token 257 in {{:?}}", tokens));
    }}
    if let Some(max_token) = tokens.iter().max() {{
        if *max_token >= VOCAB_SIZE {{
            return Err(format!("token {{}} outside VOCAB_SIZE {{}}", max_token, VOCAB_SIZE));
        }}
    }}

    let (decoded, consumed, total) = codec::decode::<String>(tokens.clone())
        .map_err(|error| format!("decode failed: {{}}", error))?;
    if consumed != total {{
        return Err(format!("decode left {{}} surplus base tokens", total - consumed));
    }}
    if decoded != input {{
        return Err(format!("decoded {{:?}} but expected {{:?}}", decoded, input));
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_bpe_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated BPE crate");

    assert!(
        output.status.success(),
        "generated BPE round trip failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_generated_bpe_encode_matches_runtime_and_trainer_simulation() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(257, representative_bpe_merges(257));

    let grammar = TctCompiler::new(schema, "BpeEquivalence")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE")
        .compile()
        .expect("Compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("bpe-equivalence")
            .with_rustc_hash()
            .with_bin("bpe-equivalence", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use tct::bpe::encoder::{{BpeDecoder, SequentialBpeEncoder}};
use tct::bpe::{{BpeMerge, BpeMergeTable, BpeMetadata}};

{generated_code}

fn trainer_encode(
    mut tokens: Vec<u32>,
    table: &BpeMergeTable,
    max_vocab_size: u32,
) -> Vec<u32> {{
    for merge in table.get_merges(max_vocab_size) {{
        tct::bpe::trainer::apply_merge_to_sequence(
            &mut tokens,
            (merge.pair[0], merge.pair[1]),
            merge.new_token,
        );
    }}
    tokens
}}

fn main() -> Result<(), String> {{
    let table = BpeMergeTable {{
        schema: "test".to_string(),
        base_vocab_size: 257,
        merges: vec![
            BpeMerge {{ pair: [97, 98], new_token: 257, frequency: 50 }},
            BpeMerge {{ pair: [257, 99], new_token: 258, frequency: 40 }},
            BpeMerge {{ pair: [120, 120], new_token: 259, frequency: 30 }},
            BpeMerge {{ pair: [259, 120], new_token: 260, frequency: 20 }},
            BpeMerge {{ pair: [42, 43], new_token: 261, frequency: 10 }},
        ],
        metadata: BpeMetadata {{
            trained_on_instances: 1,
            training_date: "2026-01-01".to_string(),
            max_merges: 5,
            dict_size: None,
        }},
    }};

    let bpe_start = table.bpe_start();
    let max_vocab_sizes = [bpe_start, bpe_start + 1, bpe_start + 2, VOCAB_SIZE - 1, VOCAB_SIZE];
    let lookup_cases = vec![
        ((97, 98), bpe_start, None),
        ((97, 98), bpe_start + 1, Some(257)),
        ((120, 120), 259, None),
        ((120, 120), 260, Some(259)),
        ((42, 43), VOCAB_SIZE - 1, Some(261)),
        ((7, 7), VOCAB_SIZE, None),
    ];
    let cascading_post_merge_fixture = vec![97, 98, 99, 120, 120, 120, 42, 43];
    let sequences = vec![
        cascading_post_merge_fixture.clone(),
        vec![97, 98, 99, 120, 120, 120, 42],
        vec![120, 120, 120, 120],
        vec![1, 2, 3, 4],
        vec![97, 98, 97, 98],
        vec![97, 98, 99, 97, 98, 99],
        vec![42, 43, 42],
    ];

    for (pair, max_vocab_size, expected) in lookup_cases {{
        let actual = bpe::lookup_merge(pair.0, pair.1, max_vocab_size);
        if actual != expected {{
            return Err(format!(
                "lookup_merge mismatch for {{:?}} at vocab {{}}: actual={{:?}} expected={{:?}}",
                pair, max_vocab_size, actual, expected
            ));
        }}
    }}

    for max_vocab_size in max_vocab_sizes {{
        let runtime_encoder = SequentialBpeEncoder::new(&table, max_vocab_size);
        let runtime_decoder = BpeDecoder::new(&table, max_vocab_size);
        for sequence in &sequences {{
            let generated = bpe::encode(sequence, max_vocab_size);
            let generated_owned = bpe::encode_owned(sequence.clone(), max_vocab_size);
            let runtime = runtime_encoder.encode(sequence);
            let trainer = trainer_encode(sequence.clone(), &table, max_vocab_size);

            if generated_owned != generated {{
                return Err(format!(
                    "generated borrowed/owned mismatch for vocab {{}} and {{:?}}: borrowed={{:?}} owned={{:?}}",
                    max_vocab_size, sequence, generated, generated_owned
                ));
            }}

            if generated != runtime {{
                return Err(format!(
                    "generated/runtime mismatch for vocab {{}} and {{:?}}: generated={{:?}} runtime={{:?}}",
                    max_vocab_size, sequence, generated, runtime
                ));
            }}

            if generated != trainer {{
                return Err(format!(
                    "generated/trainer mismatch for vocab {{}} and {{:?}}: generated={{:?}} trainer={{:?}}",
                    max_vocab_size, sequence, generated, trainer
                ));
            }}

            let decoded = bpe::decode(&generated, max_vocab_size);
            if decoded.as_slice() != sequence.as_slice() {{
                return Err(format!(
                    "decode mismatch for vocab {{}} and {{:?}}: generated={{:?}} decoded={{:?}}",
                    max_vocab_size, sequence, generated, decoded
                ));
            }}
        }}

        if max_vocab_size == VOCAB_SIZE {{
            let generated_cascade =
                bpe::encode_owned(cascading_post_merge_fixture.clone(), max_vocab_size);
            let expected_cascade = vec![258, 260, 261];
            if generated_cascade != expected_cascade {{
                return Err(format!(
                    "generated owned cascading post-merge fixture mismatch: generated={{:?}} expected={{:?}}",
                    generated_cascade, expected_cascade
                ));
            }}
        }}

        if max_vocab_size > bpe_start {{
            let decode_cases = vec![
                vec![bpe_start - 1],
                vec![bpe_start],
                vec![PAD_TOKEN],
                vec![VOCAB_SIZE],
                vec![u32::MAX],
                vec![999],
                vec![bpe_start, PAD_TOKEN, VOCAB_SIZE],
            ];

            for tokens in &decode_cases {{
                let generated = bpe::decode(tokens, max_vocab_size);
                let runtime = runtime_decoder.decode(tokens);
                if generated != runtime {{
                    return Err(format!(
                        "generated/runtime decode mismatch for vocab {{}} and {{:?}}: generated={{:?}} runtime={{:?}}",
                        max_vocab_size, tokens, generated, runtime
                    ));
                }}
            }}
        }}
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_bpe_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated BPE equivalence crate");

    assert!(
        output.status.success(),
        "generated BPE equivalence failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_generated_bpe_decode_deep_chain_matches_runtime() {
    let schema = json!({"type": "string"});
    let (merges, expected, last_token) = decode_chain_bpe_merges(257, 512);
    let table = BpeMergeTable {
        schema: "test".to_string(),
        base_vocab_size: 257,
        metadata: BpeMetadata {
            trained_on_instances: 1,
            training_date: "2026-01-01".to_string(),
            max_merges: merges.len(),
            dict_size: None,
        },
        merges: merges.clone(),
    };
    let table_json = serde_json::to_string(&table).expect("failed to serialize chain BPE table");
    let table_json_literal =
        serde_json::to_string(&table_json).expect("failed to quote chain BPE table JSON");
    let expected_literal = format!(
        "vec![{}]",
        expected
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    let bpe_file = write_bpe_merge_file(257, merges);

    let grammar = TctCompiler::new(schema, "BpeDecodeChain")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE")
        .compile()
        .expect("Compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("bpe-decode-chain")
            .with_rustc_hash()
            .with_bin("bpe-decode-chain", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use tct::bpe::BpeMergeTable;
use tct::bpe::encoder::BpeDecoder;

{generated_code}

fn main() -> Result<(), String> {{
    let table: BpeMergeTable = serde_json::from_str({table_json_literal})
        .map_err(|error| format!("table parse failed: {{}}", error))?;
    let runtime_decoder = BpeDecoder::new(&table, VOCAB_SIZE);
    let expected = {expected_literal};
    let generated = bpe::decode(&[{last_token}], VOCAB_SIZE);
    let runtime = runtime_decoder.decode(&[{last_token}]);

    if generated != expected {{
        return Err(format!(
            "generated decode mismatch: expected {{:?}} got {{:?}}",
            expected, generated
        ));
    }}
    if runtime != generated {{
        return Err(format!(
            "runtime/generated decode mismatch: runtime={{:?}} generated={{:?}}",
            runtime, generated
        ));
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_bpe_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated BPE decode-chain crate");

    assert!(
        output.status.success(),
        "generated BPE decode-chain failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_generated_bpe_matches_runtime_and_trainer_for_real_merge_table() {
    let schema = json!({"type": "string"});
    let bpe_path = "schemas/bpe/kubernetes-tct-bpe-500.json";
    let table = BpeMergeTable::from_file(bpe_path).expect("real Kubernetes BPE table should load");
    let table_json = serde_json::to_string(&table).expect("failed to serialize real BPE table");
    let table_json_literal =
        serde_json::to_string(&table_json).expect("failed to quote real BPE table JSON");

    let grammar = TctCompiler::new(schema, "RealBpeEquivalence")
        .unwrap()
        .with_bpe_merges(bpe_path)
        .expect("Failed to load real BPE")
        .compile()
        .expect("Compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("real-bpe-equivalence")
            .with_rustc_hash()
            .with_bin("real-bpe-equivalence", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use tct::bpe::encoder::SequentialBpeEncoder;
use tct::bpe::BpeMergeTable;

{generated_code}

fn trainer_encode(
    mut tokens: Vec<u32>,
    table: &BpeMergeTable,
    max_vocab_size: u32,
) -> Vec<u32> {{
    for merge in table.get_merges(max_vocab_size) {{
        tct::bpe::trainer::apply_merge_to_sequence(
            &mut tokens,
            (merge.pair[0], merge.pair[1]),
            merge.new_token,
        );
    }}
    tokens
}}

fn expand_real_merge_token(token: u32, table: &BpeMergeTable, out: &mut Vec<u32>) {{
    let bpe_start = table.bpe_start();
    if token < bpe_start {{
        out.push(token);
        return;
    }}

    let merge_index = (token - bpe_start) as usize;
    if let Some(merge) = table.merges.get(merge_index) {{
        expand_real_merge_token(merge.pair[0], table, out);
        expand_real_merge_token(merge.pair[1], table, out);
    }} else {{
        out.push(token % bpe_start);
    }}
}}

fn real_merge_sequences(table: &BpeMergeTable) -> Vec<Vec<u32>> {{
    let mut selected_indices: Vec<usize> = (0..16.min(table.merges.len())).collect();
    if !table.merges.is_empty() {{
        selected_indices.push(table.merges.len() / 2);
        selected_indices.push(table.merges.len() - 1);
    }}
    selected_indices.sort_unstable();
    selected_indices.dedup();

    let mut sequences = Vec::new();
    for index in selected_indices {{
        let mut sequence = Vec::new();
        expand_real_merge_token(table.merges[index].new_token, table, &mut sequence);
        if !sequences.contains(&sequence) {{
            sequences.push(sequence);
        }}
    }}

    if table.merges.len() >= 2 {{
        let mut combined = Vec::new();
        expand_real_merge_token(table.merges[0].new_token, table, &mut combined);
        expand_real_merge_token(table.merges[1].new_token, table, &mut combined);
        sequences.push(combined);
    }}

    sequences.push(vec![1, 2, 3, 4, 5]);
    sequences
}}

fn main() -> Result<(), String> {{
    let table: BpeMergeTable = serde_json::from_str({table_json_literal})
        .map_err(|error| format!("failed to parse embedded BPE table: {{}}", error))?;
    let bpe_start = table.bpe_start();
    let max_vocab_sizes = [
        bpe_start,
        bpe_start + 1,
        bpe_start + 16,
        bpe_start + 64,
        VOCAB_SIZE - 1,
        VOCAB_SIZE,
    ];
    let sequences = real_merge_sequences(&table);

    for max_vocab_size in max_vocab_sizes {{
        let runtime_encoder = SequentialBpeEncoder::new(&table, max_vocab_size);
        for sequence in &sequences {{
            let generated = bpe::encode(sequence, max_vocab_size);
            let runtime = runtime_encoder.encode(sequence);
            let trainer = trainer_encode(sequence.clone(), &table, max_vocab_size);

            if generated != runtime {{
                return Err(format!(
                    "generated/runtime mismatch for vocab {{}} and {{:?}}: generated={{:?}} runtime={{:?}}",
                    max_vocab_size, sequence, generated, runtime
                ));
            }}

            if generated != trainer {{
                return Err(format!(
                    "generated/trainer mismatch for vocab {{}} and {{:?}}: generated={{:?}} trainer={{:?}}",
                    max_vocab_size, sequence, generated, trainer
                ));
            }}

            let decoded = bpe::decode(&generated, max_vocab_size);
            if decoded.as_slice() != sequence.as_slice() {{
                return Err(format!(
                    "decode mismatch for vocab {{}} and {{:?}}: generated={{:?}} decoded={{:?}}",
                    max_vocab_size, sequence, generated, decoded
                ));
            }}
        }}
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_bpe_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run real BPE equivalence crate");

    assert!(
        output.status.success(),
        "real BPE equivalence failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_kubernetes_bpe_code_generation() {
    let schema_path = std::path::Path::new("schemas/popular/kubernetes.json");
    let schema_content =
        std::fs::read_to_string(schema_path).expect("Failed to read Kubernetes schema");
    let schema: serde_json::Value =
        serde_json::from_str(&schema_content).expect("Failed to parse Kubernetes schema");

    let compiler = TctCompiler::new(schema, "Kubernetes")
        .expect("Failed to create compiler")
        .with_bpe_merges("schemas/bpe/kubernetes-tct-bpe-500.json")
        .expect("Failed to load Kubernetes BPE merge file");

    let grammar = compiler.compile().expect("Compilation failed");
    let code = grammar.generate_code().expect("Code generation failed");
    let parsed = parse_generated_code(&code);
    let bpe_items = generated_module_items(&parsed, "bpe");

    let source: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("schemas/bpe/kubernetes-tct-bpe-500.json")
            .expect("Kubernetes BPE artifact should exist"),
    )
    .expect("Kubernetes BPE artifact should parse");
    let expected_merges = source["merges"]
        .as_array()
        .expect("BPE artifact should contain ordered merges")
        .len();
    assert!(
        expected_merges > 0,
        "fixture should exercise BPE code generation"
    );
    assert_eq!(
        generated_const_literal_usize(bpe_items, "MAX_MERGES"),
        expected_merges,
        "generated BPE must retain every merge in the distributed artifact"
    );
    assert_eq!(
        generated_const_literal_u32(bpe_items, "BASE_VOCAB_SIZE"),
        257
    );
}

#[test]
fn test_bpe_priority_ordered_encoding() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(257, single_ab_merge(257));

    let compiler = TctCompiler::new(schema, "PriorityTest")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE");

    let grammar = compiler.compile().expect("Compilation failed");
    let code = grammar.generate_code().expect("Code generation failed");

    assert!(
        code.contains("build_initial_pair_positions"),
        "BPE encode should build a sorted initial active-pair position list"
    );
    assert!(
        !code.contains("fn apply_merge_pass("),
        "BPE encode should not use the old pass-wide merge helper"
    );
    assert!(
        code.contains("for merge_idx in 0..max_merges"),
        "BPE encode should iterate through merges in priority order"
    );
    assert!(
        code.contains("index_state.future_positions.is_empty()"),
        "BPE encode should stop after initial and future position candidates are exhausted"
    );
}

#[test]
fn test_bpe_encoding_matches_training_simulation_structure() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(257, single_ab_merge(257));

    let compiler = TctCompiler::new(schema, "SimTest")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("Failed to load BPE");

    let grammar = compiler.compile().expect("Compilation failed");
    let code = grammar.generate_code().expect("Code generation failed");
    let parsed = parse_generated_code(&code);
    let bpe_items = generated_module_items(&parsed, "bpe");
    let encode = generated_function(bpe_items, "encode");
    let encode_owned = generated_function(bpe_items, "encode_owned");

    assert!(generated_function_signature_contains(
        encode,
        "tokens : & [u32] , max_vocab_size : u32"
    ));
    assert!(generated_function_signature_contains(
        encode,
        "-> Vec < u32 >"
    ));
    assert!(generated_function_signature_contains(
        encode_owned,
        "mut current : Vec < u32 > , max_vocab_size : u32"
    ));
    assert!(generated_function_signature_contains(
        encode_owned,
        "-> Vec < u32 >"
    ));
    assert!(code.contains("const NO_LINK: u32 = u32::MAX;"));
    assert!(code.contains("let mut next: Vec<u32>"));
    assert!(code.contains("let mut prev: Vec<u32>"));
    assert!(!code.contains("Vec<Option<usize>>"));
    assert!(code.contains("let pair = MERGE_PAIRS[merge_idx]"));
    assert!(code.contains("let new_token = MERGE_TOKENS[merge_idx]"));
    assert!(code.contains("const MERGE_PAIR_INDEX"));
    assert!(code.contains("binary_search_by_key"));
    assert!(code.contains("merge_pair_lookup_key"));
    assert!(!code.contains("PAIR_MAP_CACHE"));
    assert!(!code.contains("build_pair_map"));
    assert!(code.contains("let initial_pair_positions = build_initial_pair_positions("));
    assert!(code.contains("if initial_pair_positions.is_empty()"));
    assert!(code.contains("return current;"));
    assert!(code.contains(") -> Vec<u64>"));
    assert!(code.contains("pack_initial_pair_position(merge_idx, left)"));
    assert!(code.contains("candidate_merge_idx(initial_pair_positions[initial_cursor])"));
    assert!(code.contains("candidate_left_position(candidate)"));
    assert!(code.contains("encode_owned_with_full_merge_scan(current, max_merges)"));
    let position_bucket = generated_enum(bpe_items, "PositionBucket");
    assert!(
        position_bucket
            .variants
            .iter()
            .any(|variant| variant.ident == "One")
    );
    assert!(
        position_bucket
            .variants
            .iter()
            .any(|variant| variant.ident == "Many")
    );
    assert!(code.contains("One(u32)"));
    assert!(code.contains("Many(Vec<u32>)"));
    assert!(code.contains("let mut future_positions: FxHashMap<u32, PositionBucket>"));
    assert!(code.contains("let mut position_pool: Vec<Vec<u32>> = Vec::new();"));
    generated_struct(bpe_items, "PairIndexState");
    assert!(code.contains("let mut index_state = PairIndexState"));
    assert!(code.contains("apply_merge_positions("));
    assert!(code.contains("positions.sort_unstable();"));
    assert!(code.contains("positions.dedup();"));
    assert!(code.contains("unwrap_or_default();"));
    assert!(code.contains("PositionBucket::One(left_position)"));
    assert!(code.contains("PositionBucket::Many(positions)"));
    assert!(code.contains("push_current_pair("));
    assert!(code.contains("if merge_idx <= current_merge_idx"));
    assert!(code.contains("compact_linked_tokens(current, &next)"));
    assert!(code.contains("fn compact_linked_tokens(mut tokens: Vec<u32>, next: &[u32])"));
    assert!(!code.contains("collect_linked_tokens"));
}

fn extract_generated_constant_value(code: &str, constant_name: &str) -> u32 {
    let patterns = [
        format!("pub const {}: u32", constant_name),
        format!("pub const {} : u32", constant_name),
        format!("const {}: u32", constant_name),
        format!("const {} : u32", constant_name),
    ];
    let line = code
        .lines()
        .find(|line| patterns.iter().any(|pattern| line.contains(pattern)))
        .unwrap_or_else(|| panic!("missing generated constant {}", constant_name));
    let value = line
        .split('=')
        .nth(1)
        .unwrap_or_else(|| panic!("generated constant {} has no value", constant_name))
        .trim()
        .trim_end_matches(';')
        .trim()
        .trim_end_matches("u32")
        .trim();
    value.parse::<u32>().unwrap_or_else(|error| {
        panic!(
            "failed to parse generated constant {} value {:?}: {}",
            constant_name, value, error
        )
    })
}
