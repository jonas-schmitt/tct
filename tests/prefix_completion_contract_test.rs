use serde_json::json;
use std::fs;
use std::process::Command;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::{OptimizationLevel, TctCompiler};

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn generated_file_has_public_struct(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Struct(item_struct) => {
            item_struct.ident == name && matches!(item_struct.vis, syn::Visibility::Public(_))
        }
        _ => false,
    })
}

fn generated_file_has_public_function_or_method(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Fn(function) => {
            function.sig.ident == name && matches!(function.vis, syn::Visibility::Public(_))
        }
        syn::Item::Impl(item_impl) => item_impl.items.iter().any(|impl_item| {
            matches!(
                impl_item,
                syn::ImplItem::Fn(function)
                    if function.sig.ident == name
                        && matches!(function.vis, syn::Visibility::Public(_))
            )
        }),
        _ => false,
    })
}

fn generated_contract_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

fn contract_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "kind": {"type": "string", "enum": ["Example"]},
            "name": {"type": "string"},
            "enabled": {"type": "boolean"}
        },
        "required": ["kind", "name"]
    })
}

fn generated_code(streaming: bool) -> String {
    let mut compiler = TctCompiler::new(contract_schema(), "PrefixContract")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O3);
    if streaming {
        compiler = compiler.with_streaming_encode();
    }

    compiler.compile().unwrap().generate_code().unwrap()
}

fn run_generated_contract(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated prefix-completion crate");

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
        .unwrap_or_else(|error| panic!("failed to run generated crate {crate_name}: {error}"));

    assert!(
        output.status.success(),
        "generated crate {} failed\nstdout:\n{}\nstderr:\n{}",
        crate_name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn decode_prefix_contract_is_streaming_gated() {
    let non_streaming = generated_code(false);
    let non_streaming_file = parse_generated_code(&non_streaming);
    assert!(
        !generated_file_has_public_function_or_method(&non_streaming_file, "decode_prefix"),
        "decode_prefix methods should only be generated when streaming is enabled"
    );

    let streaming = generated_code(true);
    let streaming_file = parse_generated_code(&streaming);
    assert!(
        generated_file_has_public_function_or_method(&streaming_file, "decode_prefix"),
        "streaming generation should expose decode_prefix methods"
    );
    assert!(
        generated_file_has_public_struct(&streaming_file, "PrefixDecodeResult"),
        "streaming support should include PrefixDecodeResult"
    );
    assert!(streaming.contains("instance"));
    assert!(streaming.contains("fields_decoded"));
    assert!(streaming.contains("complete"));
    assert!(streaming.contains("partial_field"));

    run_generated_contract(
        "prefix-metadata-contract",
        &streaming,
        r#"
let metadata = sequence::metadata();
if !metadata.decode_prefix_supported {
    return Err(format!(
        "streaming-generated metadata should report decode_prefix support: {:?}",
        metadata
    ));
}
Ok(())
"#,
    );
}

#[test]
fn gap_tracking_api_is_not_suffix_constrained_infill() {
    let streaming = generated_code(true);
    let streaming_file = parse_generated_code(&streaming);

    assert!(
        generated_file_has_public_struct(&streaming_file, "GapInfo"),
        "generated code should keep existing gap metadata available"
    );
    assert!(
        generated_file_has_public_struct(&streaming_file, "PartialEncoding"),
        "generated code should keep existing partial encoding metadata available"
    );
    assert!(
        generated_file_has_public_function_or_method(&streaming_file, "encode_partial"),
        "generated structs should expose default-filled partial encoding"
    );
    assert!(
        generated_file_has_public_function_or_method(&streaming_file, "decode_to_first_gap"),
        "generated structs should expose first-gap decoding"
    );
    for unsupported_api in ["decode_gap", "decode_suffix", "decode_infill"] {
        assert!(
            !streaming.contains(unsupported_api),
            "{unsupported_api} would imply suffix-constrained gapped infill, which is not implemented"
        );
    }
}

#[test]
fn gap_tracking_runtime_is_complete_defaulted_sequence_metadata() {
    let streaming = generated_code(true);
    run_generated_contract(
        "gap-tracking-runtime-contract",
        &streaming,
        r#"
let instance = PrefixContract {
    kind: PrefixContractKind::Example,
    name: "alpha".to_string(),
    enabled: None,
    additional_properties: rustc_hash::FxHashMap::default(),
};
let presence = vec![true, true, false, true];
let partial = instance.encode_partial(&presence);

if partial.tokens.is_empty() {
    return Err("partial encoding should still produce a complete token sequence".to_string());
}
if partial.gaps.len() != 1 {
    return Err(format!("expected only enabled to be tracked as a gap: {:?}", partial.gaps));
}
let gap = &partial.gaps[0];
if gap.field_index != 2 || gap.field_name != "enabled" {
    return Err(format!("unexpected gap identity: {:?}", gap));
}
if gap.token_range.start >= gap.token_range.end || gap.token_range.end > partial.tokens.len() {
    return Err(format!(
        "gap range should identify encoded default tokens inside the complete sequence: {:?}, len={}",
        gap,
        partial.tokens.len()
    ));
}

let decoded = PrefixContract::decode_partial(&partial.tokens)
    .map_err(|error| format!("decode_partial should consume the complete defaulted sequence: {error:?}"))?;
if decoded.name != "alpha" || decoded.enabled.is_some() {
    return Err(format!("decode_partial changed semantic data: {:?}", decoded));
}

let prefix = PrefixContract::decode_to_first_gap(&partial)
    .map_err(|error| format!("decode_to_first_gap failed: {error:?}"))?;
if prefix.stopped_at_field != gap.field_index {
    return Err(format!("decode_to_first_gap stopped at wrong field: {:?}", prefix));
}
if prefix.tokens_consumed != partial.tokens.len() || prefix.total_tokens != partial.tokens.len() {
    return Err(format!(
        "decode_to_first_gap should report the complete default-filled stream, got {:?}",
        prefix
    ));
}
if prefix.instance.name != "alpha" || prefix.instance.enabled.is_some() {
    return Err(format!("decode_to_first_gap changed semantic data: {:?}", prefix.instance));
}

Ok(())
"#,
    );
}

#[test]
fn decode_prefix_runtime_reports_truncation_without_gap_claims() {
    let streaming = generated_code(true);
    run_generated_contract(
        "prefix-completion-runtime-contract",
        &streaming,
        r#"
let instance = PrefixContract {
    kind: PrefixContractKind::Example,
    name: "alpha".to_string(),
    enabled: Some(true),
    additional_properties: rustc_hash::FxHashMap::default(),
};
let mut tokens = Vec::new();
instance.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let full = PrefixContract::decode_prefix(&mut full_iter);
if !full.complete || full.fields_decoded != 4 || full.partial_field.is_some() {
    return Err(format!("unexpected full prefix result: {:?}", full));
}
if full.instance.name != "alpha" || full.instance.enabled != Some(true) {
    return Err(format!("full prefix changed semantic data: {:?}", full.instance));
}

let end_marker_index = tokens
    .iter()
    .position(|token| *token == 256)
    .ok_or_else(|| "encoded name should contain END_STRING_MARKER".to_string())?;
let mut partial_iter = tokens[..end_marker_index].iter().copied();
let partial = PrefixContract::decode_prefix(&mut partial_iter);
if partial.complete {
    return Err(format!("truncated string prefix reported complete: {:?}", partial));
}
if partial.fields_decoded != 2 {
    return Err(format!("expected kind plus partial name progress to be reported, got {:?}", partial));
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| format!("truncated string should report partial_field: {:?}", partial))?;
if partial_field.field_name != "name"
    || partial_field.field_type != "String"
    || partial_field.items_decoded != 5
    || partial_field.expected_length.is_some()
{
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial.instance.name != "alpha" || partial.instance.enabled.is_some() {
    return Err(format!("partial instance should keep partial name and default later fields: {:?}", partial.instance));
}

Ok(())
"#,
    );
}
