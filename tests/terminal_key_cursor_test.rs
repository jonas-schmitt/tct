//! Decoder-cursor terminal object-property key introspection.
//!
//! Locks the generic Rust API that reports the active terminal schema
//! object-property key/path when a raw token prefix is positioned at a
//! host-value leaf (immediately before the first value byte, or mid-value
//! before VALUE_END). Proves the API works where `decode_prefix` /
//! `render_prefix` still hide the first optional numeric field.
//!
//! Does not consult gold JSON; keys come from the decoder path stack only.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

fn repo_schema(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn compile_decimal_streaming_tokenizer(schema_path: &Path, type_name: &str) -> String {
    let schema_content = fs::read_to_string(schema_path).expect("read schema");
    let schema: serde_json::Value =
        serde_json::from_str(&schema_content).expect("parse schema json");
    let grammar = TctCompiler::new(schema, type_name)
        .expect("compiler")
        .with_streaming_encode()
        .with_decimal_number_encoding()
        .compile()
        .expect("compile");
    grammar.generate_code().expect("generate code")
}

fn generated_contract_cargo_command() -> Command {
    let tct_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    let target_dir = tct_path
        .join("target")
        .join("tct-generated-cargo")
        .join("terminal-key-cursor");
    fs::create_dir_all(&target_dir).expect("create generated cargo target dir");
    command.env("CARGO_TARGET_DIR", target_dir);
    command
}

fn run_generated_contract(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("tempdir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("src dir");

    let tct_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs");
    if generated_code.contains("jsonschema ::") || generated_code.contains("jsonschema::") {
        cargo_toml = cargo_toml.with_jsonschema();
    }
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml.render()).expect("write Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path()).expect("copy lock");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn encode_instance(value: &serde_json::Value) -> Result<Vec<u32>, String> {{
    let typed: TerminalKeyCursor = serde_json::from_value(value.clone())
        .map_err(|e| format!("parse {{value}}: {{e}}"))?;
    let mut tokens = Vec::new();
    typed.encode(&mut tokens);
    Ok(tokens)
}}

fn terminal_key_path(tokens: &[u32]) -> Option<(String, String)> {{
    tokenizer::terminal_key_export::current_terminal_key_path::<TerminalKeyCursor>(tokens)
}}

fn render_prefix_json(tokens: &[u32]) -> String {{
    let mut iter = tokens.iter().copied();
    let result = TerminalKeyCursor::decode_prefix(&mut iter);
    serde_json::to_string(&result.instance).unwrap_or_default()
}}

fn roles_at(tokens: &[u32]) -> Result<(Vec<u8>, bool), String> {{
    tokenizer::role_export::strict_prefix_roles::<TerminalKeyCursor>(tokens)
        .map_err(|e| e.to_string())
}}

fn main() -> Result<(), String> {{
{body}
    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("write main.rs");

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
fn generated_code_includes_terminal_key_export() {
    let schema = repo_schema("schemas/synthetic/terminal_key_cursor.json");
    let generated = compile_decimal_streaming_tokenizer(&schema, "TerminalKeyCursor");
    assert!(
        generated.contains("pub mod terminal_key_export"),
        "generated tokenizer must export terminal_key_export"
    );
    assert!(
        generated.contains("current_terminal_key_path"),
        "generated tokenizer must expose current_terminal_key_path"
    );
    assert!(
        generated.contains("with_object_property"),
        "generated field decode must push object property path"
    );
    assert!(
        generated.contains("begin_host_value"),
        "host-value leaves must open the terminal-key cursor"
    );
}

#[test]
fn first_optional_numeric_field_key_before_render_visibility() {
    // Reproduces a tool-call-schema failure: optional argument fields where the
    // first host-value (`limit`) is accepted by strict roles, but decode_prefix
    // still renders arguments as {{}} until a later transition.
    let schema = repo_schema("schemas/synthetic/terminal_key_cursor.json");
    let generated = compile_decimal_streaming_tokenizer(&schema, "TerminalKeyCursor");

    let instance = json!({
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

    let body = format!(
        r#"
    let instance: serde_json::Value = serde_json::from_str({instance_json:?})
        .map_err(|e| e.to_string())?;
    let tokens = encode_instance(&instance)?;
    assert!(!tokens.is_empty());

    // Locate the first host-value span of the second call (limit=1).
    // Decimal host values are ROLE_VALUE bytes terminated by ROLE_VALUE_END.
    let (roles, complete) = roles_at(&tokens)?;
    assert!(complete, "full stream must be complete");
    assert_eq!(roles.len(), tokens.len());

    let mut value_end_positions = Vec::new();
    for (idx, role) in roles.iter().enumerate() {{
        if *role == tokenizer::role_export::ROLE_VALUE_END {{
            value_end_positions.push(idx);
        }}
    }}
    // tool_beta has two host values (limit, skip); tool_alpha has no host value
    // for boolean count_only (structure). Expect at least two VALUE_END marks.
    assert!(
        value_end_positions.len() >= 2,
        "expected >=2 host values, got {{value_end_positions:?}} roles={{roles:?}}"
    );

    // Prefix ending immediately before the first VALUE_BYTE of the first
    // host-value span for limit (second call).
    let first_value_end = value_end_positions[0];
    // Walk back over contiguous VALUE bytes to the first value byte of this span.
    let mut first_value_byte = first_value_end;
    while first_value_byte > 0
        && roles[first_value_byte - 1] == tokenizer::role_export::ROLE_VALUE
    {{
        first_value_byte -= 1;
    }}
    // Position immediately before host value: structure through presence, no value bytes yet.
    let before_value = &tokens[..first_value_byte];
    let mid_value = &tokens[..=first_value_byte]; // one value byte consumed
    let after_value_end = &tokens[..=first_value_end];

    let rendered_before = render_prefix_json(before_value);
    // The historical bug: render still shows empty arguments for the active call.
    // We do not require that to change; we require the cursor API to report limit.
    let key_before = terminal_key_path(before_value);
    assert_eq!(
        key_before.as_ref().map(|(k, _)| k.as_str()),
        Some("limit"),
        "before host value: key must be limit; rendered={{rendered_before}} roles_prefix={{:?}}",
        &roles[..first_value_byte.min(roles.len())]
    );
    let path_before = key_before.as_ref().map(|(_, p)| p.as_str());
    assert!(
        path_before.map(|p| p.ends_with("arguments.limit") || p.contains("limit")).unwrap_or(false),
        "path should include arguments.limit, got {{path_before:?}}"
    );

    let key_mid = terminal_key_path(mid_value);
    assert_eq!(
        key_mid.as_ref().map(|(k, _)| k.as_str()),
        Some("limit"),
        "mid host value must still report limit"
    );

    // After VALUE_END the cursor is no longer inside that host value
    // (next token is structure for the following optional field).
    let key_after = terminal_key_path(after_value_end);
    assert_ne!(
        key_after.as_ref().map(|(k, _)| k.as_str()),
        Some("limit"),
        "after completing limit value, cursor must leave that host leaf; got {{key_after:?}}"
    );

    // Full stream: complete decode → None.
    assert_eq!(terminal_key_path(&tokens), None);

    // Empty prefix: None.
    assert_eq!(terminal_key_path(&[]), None);

    // Dead prefix (invalid token after a valid host-open prefix): fail closed.
    let mut dead = before_value.to_vec();
    dead.push(u32::MAX);
    assert_eq!(
        terminal_key_path(&dead),
        None,
        "dead prefix must not invent a terminal key"
    );

    // Structural wait: strip the final presence bit that opens `limit` so the
    // cursor sits on presence/structure, not a host-value leaf.
    if first_value_byte > 0 {{
        let structural = &tokens[..first_value_byte.saturating_sub(1)];
        assert_eq!(
            terminal_key_path(structural),
            None,
            "presence/structure wait must return None (fail-closed)"
        );
    }}
    let _ = rendered_before;
"#,
        instance_json = instance.to_string()
    );

    run_generated_contract("terminal_key_first_limit", &generated, &body);
}

#[test]
fn nested_array_and_multi_call_terminal_keys() {
    let schema = repo_schema("schemas/synthetic/terminal_key_cursor.json");
    let generated = compile_decimal_streaming_tokenizer(&schema, "TerminalKeyCursor");

    let instance = json!({
        "calls": [
            {
                "name": "tool_gamma",
                "arguments": {
                    "nested": { "depth": 2, "label": "ab" },
                    "tags": ["x", "yz"]
                }
            },
            {
                "name": "tool_beta",
                "arguments": { "title": "hello" }
            }
        ]
    });

    let body = format!(
        r#"
    let instance: serde_json::Value = serde_json::from_str({instance_json:?})
        .map_err(|e| e.to_string())?;
    let tokens = encode_instance(&instance)?;
    let (roles, complete) = roles_at(&tokens)?;
    assert!(complete);
    assert_eq!(roles.len(), tokens.len());

    let mut seen_keys: Vec<String> = Vec::new();
    let mut seen_paths: Vec<String> = Vec::new();
    for len in 0..=tokens.len() {{
        let prefix = &tokens[..len];
        if let Some((key, path)) = terminal_key_path(prefix) {{
            // Only record transitions into a new host-value leaf (prefix ends
            // with structure immediately before first VALUE of a span, or mid).
            if roles.get(len.saturating_sub(1)) != Some(&tokenizer::role_export::ROLE_VALUE_END) {{
                if seen_keys.last().map(|k| k.as_str()) != Some(key.as_str())
                    || seen_paths.last().map(|p| p.as_str()) != Some(path.as_str())
                {{
                    // Prefer recording at the open of a host value (no trailing VALUE yet)
                    // or first mid-value of a new leaf.
                    if len == 0
                        || roles.get(len.saturating_sub(1)) != Some(&tokenizer::role_export::ROLE_VALUE)
                        || seen_paths.last().map(|p| p.as_str()) != Some(path.as_str())
                    {{
                        if seen_paths.last().map(|p| p.as_str()) != Some(path.as_str()) {{
                            seen_keys.push(key);
                            seen_paths.push(path);
                        }}
                    }}
                }}
            }}
        }}
    }}

    // Expected host-value terminal keys in stream order:
    // depth, label, tags[0], tags[1], title
    assert!(
        seen_keys.iter().any(|k| k == "depth"),
        "missing nested depth key; seen={{seen_keys:?}} paths={{seen_paths:?}}"
    );
    assert!(
        seen_keys.iter().any(|k| k == "label"),
        "missing nested label key; seen={{seen_keys:?}} paths={{seen_paths:?}}"
    );
    assert!(
        seen_keys.iter().any(|k| k == "tags"),
        "missing tags array host-value key; seen={{seen_keys:?}} paths={{seen_paths:?}}"
    );
    assert!(
        seen_keys.iter().any(|k| k == "title"),
        "missing second-call title key; seen={{seen_keys:?}} paths={{seen_paths:?}}"
    );
    assert!(
        seen_paths.iter().any(|p| p.contains("nested.depth")),
        "path must include nested.depth; paths={{seen_paths:?}}"
    );
    assert!(
        seen_paths.iter().any(|p| p.contains("tags.0") || p.contains("tags.1")),
        "path must include array index for tags; paths={{seen_paths:?}}"
    );

    // Round-trip invariant: encode/decode still preserves semantic JSON.
    let mut iter = tokens.iter().copied();
    let decoded = TerminalKeyCursor::decode(&mut iter).map_err(|e| e.to_string())?;
    let decoded_json = serde_json::to_value(&decoded).map_err(|e| e.to_string())?;
    assert_eq!(decoded_json, instance);

    // Role stream unchanged in shape: every ROLE_VALUE_END still present.
    let value_ends = roles.iter().filter(|r| **r == tokenizer::role_export::ROLE_VALUE_END).count();
    assert!(value_ends >= 4, "expected host-value spans for depth/label/tags/title");
"#,
        instance_json = instance.to_string()
    );

    run_generated_contract("terminal_key_nested_multi", &generated, &body);
}
