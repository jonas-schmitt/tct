//! In-repo DecodeState prefix-equivalence oracle and root-token restriction.
//!
//! DecodeState in the wheel is a thin PyO3 handle over full-prefix `decode`
//! plus optional first-token restriction. This suite locks the pure-Rust
//! classification contract on a small multi-branch synthetic schema without
//! maturin or external gold data.

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

fn compile_streaming_tokenizer(schema_path: &Path, type_name: &str) -> String {
    let schema_content = fs::read_to_string(schema_path).expect("read schema");
    let schema: serde_json::Value =
        serde_json::from_str(&schema_content).expect("parse schema json");
    let grammar = TctCompiler::new(schema, type_name)
        .expect("compiler")
        .with_streaming_encode()
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
        .join("decode-state-equivalence");
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

fn classify_decode_tokens(tokens: &[u32]) -> String {{
    if tokens.is_empty() {{
        return "prefix_ok".to_string();
    }}
    let total = tokens.len();
    let mut iter = tokens.iter().copied().peekable();
    match DecodeStateMultiBranch::decode(&mut iter) {{
        Ok(_) => {{
            let remaining = iter.count();
            if remaining == 0 {{
                "complete".to_string()
            }} else {{
                "dead".to_string()
            }}
        }}
        Err(error) => {{
            let message = error.to_string();
            if message.contains("Unexpected end of token stream") {{
                let _ = total;
                "prefix_ok".to_string()
            }} else {{
                "dead".to_string()
            }}
        }}
    }}
}}

fn root_token_allowed(tokens: &[u32], allowed: Option<&[u32]>) -> bool {{
    match (allowed, tokens.first()) {{
        (None, _) | (_, None) => true,
        (Some(allowed), Some(first)) => allowed.contains(first),
    }}
}}

fn classify_with_restriction(tokens: &[u32], allowed: Option<&[u32]>) -> String {{
    if !root_token_allowed(tokens, allowed) {{
        return "dead".to_string();
    }}
    classify_decode_tokens(tokens)
}}

/// Incremental append chain must match single-shot full-prefix classify.
fn incremental_status(tokens: &[u32], allowed: Option<&[u32]>) -> String {{
    let mut prefix = Vec::new();
    let mut last = classify_with_restriction(&prefix, allowed);
    for &token in tokens {{
        prefix.push(token);
        last = classify_with_restriction(&prefix, allowed);
    }}
    last
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
fn prefix_equivalence_and_root_restriction_on_multi_branch_schema() {
    let schema = repo_schema("schemas/synthetic/decode_state_multi_branch.json");
    let generated = compile_streaming_tokenizer(&schema, "DecodeStateMultiBranch");

    let instances = vec![
        json!({"kind": "alpha", "name": "one"}),
        json!({"kind": "alpha", "name": "longer-name"}),
        json!({"kind": "beta", "count": 0}),
        json!({"kind": "beta", "count": 42}),
        json!({"kind": "gamma", "flag": true}),
        json!({"kind": "gamma", "flag": false}),
    ];

    let body = format!(
        r#"
    let instances: Vec<serde_json::Value> = serde_json::from_str({instances_json:?})
        .map_err(|e| e.to_string())?;

    let mut encoded: Vec<Vec<u32>> = Vec::new();
    for instance in &instances {{
        let typed: DecodeStateMultiBranch = serde_json::from_value(instance.clone())
            .map_err(|e| format!("parse instance {{instance}}: {{e}}"))?;
        let mut tokens = Vec::new();
        typed.encode(&mut tokens);
        assert!(!tokens.is_empty(), "encoded stream must be non-empty for {{instance}}");
        encoded.push(tokens);
    }}

    // Every prefix of every valid stream: incremental append ≡ full-prefix classify.
    for (stream_idx, tokens) in encoded.iter().enumerate() {{
        for len in 0..=tokens.len() {{
            let prefix = &tokens[..len];
            let full = classify_decode_tokens(prefix);
            let mut acc = Vec::new();
            let mut incremental = classify_decode_tokens(&acc);
            for &token in prefix {{
                acc.push(token);
                incremental = classify_decode_tokens(&acc);
            }}
            assert_eq!(
                incremental, full,
                "stream {{stream_idx}} prefix_len={{len}}: incremental {{incremental}} != full {{full}}"
            );
            if len == 0 {{
                assert_eq!(full, "prefix_ok");
            }}
            if len == tokens.len() {{
                assert_eq!(full, "complete", "full stream {{stream_idx}} must complete");
            }}
        }}
    }}

    // Cross-stream wrong first token should be dead (or at least not complete under
    // another stream's continuation) when restriction pins one root token.
    let root_tokens: Vec<u32> = encoded.iter().map(|t| t[0]).collect();
    assert!(
        root_tokens.iter().collect::<std::collections::BTreeSet<_>>().len() >= 2,
        "fixture must produce at least two distinct root tokens; got {{root_tokens:?}}"
    );

    let allowed_alpha = vec![encoded[0][0]];
    // Alpha streams under alpha restriction stay non-dead and complete at full length.
    for (stream_idx, tokens) in encoded.iter().enumerate().take(2) {{
        assert_eq!(tokens[0], allowed_alpha[0], "first two instances are alpha");
        for len in 0..=tokens.len() {{
            let prefix = &tokens[..len];
            let restricted = classify_with_restriction(prefix, Some(&allowed_alpha));
            let unrestricted = classify_decode_tokens(prefix);
            assert_eq!(
                restricted, unrestricted,
                "alpha stream {{stream_idx}} prefix {{len}} should match unrestricted under its own root"
            );
            let incr = incremental_status(prefix, Some(&allowed_alpha));
            assert_eq!(incr, restricted);
        }}
        assert_eq!(
            classify_with_restriction(tokens, Some(&allowed_alpha)),
            "complete"
        );
    }}

    // Beta/gamma streams die as soon as the first token is committed under alpha restriction.
    for (stream_idx, tokens) in encoded.iter().enumerate().skip(2) {{
        assert_ne!(tokens[0], allowed_alpha[0]);
        assert_eq!(
            classify_with_restriction(&tokens[..0], Some(&allowed_alpha)),
            "prefix_ok",
            "empty still ok"
        );
        assert_eq!(
            classify_with_restriction(&tokens[..1], Some(&allowed_alpha)),
            "dead",
            "stream {{stream_idx}} first token must be rejected"
        );
        assert_eq!(
            incremental_status(&tokens[..1], Some(&allowed_alpha)),
            "dead"
        );
        // Full stream also dead under wrong restriction (not complete).
        assert_eq!(
            classify_with_restriction(tokens, Some(&allowed_alpha)),
            "dead"
        );
    }}

    // Empty allowed set: any non-empty prefix is dead.
    let empty_allowed: &[u32] = &[];
    assert_eq!(
        classify_with_restriction(&encoded[0][..0], Some(empty_allowed)),
        "prefix_ok"
    );
    assert_eq!(
        classify_with_restriction(&encoded[0][..1], Some(empty_allowed)),
        "dead"
    );

    // clone isolation analogue: mutating a copy of the prefix vec does not change a prior classify.
    let mut clone = encoded[0].clone();
    let before = classify_decode_tokens(&clone);
    clone.push(999_999);
    let after_orig = classify_decode_tokens(&encoded[0]);
    assert_eq!(before, after_orig);
    assert_eq!(classify_decode_tokens(&clone), "dead");
"#,
        instances_json = serde_json::to_string(&instances).unwrap()
    );

    run_generated_contract("decode_state_equivalence_multi_branch", &generated, &body);
}
