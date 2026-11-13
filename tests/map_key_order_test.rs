//! Canonical map key order in generated tokenizers.
//!
//! Map entries (typed `additionalProperties` maps, `patternProperties`
//! storage, root catch-all maps, and objects inside untyped values) must be
//! emitted in ascending key order (UTF-8 byte order), so that one JSON value
//! has exactly one token stream whatever order its keys had in the source.
//! Decode stays order-tolerant: a stream with entries in another order still
//! decodes to the same value.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value, json};
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

/// Key permutations applied to every map in the instance.
#[derive(Clone, Copy)]
enum SourceOrder {
    Ascending,
    Descending,
    Shuffled,
}

fn reorder<T: Clone>(items: &[T], order: SourceOrder) -> Vec<T> {
    match order {
        SourceOrder::Ascending => items.to_vec(),
        SourceOrder::Descending => items.iter().rev().cloned().collect(),
        SourceOrder::Shuffled => {
            // 7 is coprime to every length used below, so this is a permutation.
            let n = items.len();
            (0..n).map(|i| items[(i * 7 + 3) % n].clone()).collect()
        }
    }
}

fn object(entries: Vec<(String, Value)>, order: SourceOrder) -> Value {
    let mut sorted = entries;
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut map = Map::new();
    for (key, value) in reorder(&sorted, order) {
        map.insert(key, value);
    }
    Value::Object(map)
}

fn count_keys() -> Vec<String> {
    (0..16).map(|i| format!("c{i:02}")).collect()
}

fn flag_keys() -> Vec<String> {
    (b'a'..=b'p').map(|c| format!("x-{}", c as char)).collect()
}

fn meta_keys() -> Vec<String> {
    (0..10).map(|i| format!("m{i:02}")).collect()
}

fn nested_keys() -> Vec<String> {
    (0..13).map(|i| format!("n{i:02}")).collect()
}

/// Root catch-all keys; byte order puts upper case before lower case and the
/// two-byte `é` after every ASCII key.
fn extra_keys() -> Vec<String> {
    let mut keys: Vec<String> = ["Beta", "Zulu", "alpha", "zeta", "\u{e9}clair"]
        .iter()
        .map(|key| key.to_string())
        .collect();
    keys.extend((0..8).map(|i| format!("r{i:02}")));
    keys
}

/// The same JSON value with every map's keys in the given source order.
fn instance(order: SourceOrder) -> Value {
    let counts = object(
        count_keys()
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key, json!((i * 37) % 101)))
            .collect(),
        order,
    );
    let flags = object(
        flag_keys()
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key, json!(i % 3 == 0)))
            .collect(),
        order,
    );
    let nested = object(
        nested_keys()
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key, json!(i * 11)))
            .collect(),
        order,
    );
    let mut meta_entries: Vec<(String, Value)> = meta_keys()
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, json!(format!("v{i}"))))
        .collect();
    meta_entries.push(("nested".to_string(), nested));
    let meta = object(meta_entries, order);

    let mut root_entries: Vec<(String, Value)> = vec![
        ("name".to_string(), json!("demo")),
        ("counts".to_string(), counts),
        ("flags".to_string(), flags),
        ("meta".to_string(), meta),
    ];
    root_entries.extend(
        extra_keys()
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key, json!(i as u64 * 1000))),
    );
    object(root_entries, order)
}

fn map_order_schema() -> Value {
    json!({
        "title": "MapOrder",
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "counts": {
                "type": "object",
                "additionalProperties": { "type": "integer", "minimum": 0, "maximum": 100 }
            },
            "flags": {
                "type": "object",
                "patternProperties": { "^x-": { "type": "boolean" } },
                "additionalProperties": false
            },
            "meta": {}
        },
        "required": ["name", "counts", "flags", "meta"]
    })
}

fn rust_string_array(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|item| format!("{item:?}")).collect();
    format!("&[{}]", quoted.join(", "))
}

fn map_order_body() -> String {
    let texts: Vec<String> = [
        SourceOrder::Ascending,
        SourceOrder::Descending,
        SourceOrder::Shuffled,
    ]
    .into_iter()
    .map(|order| serde_json::to_string(&instance(order)).expect("instance serializes"))
    .collect();
    let mut nested_and_meta = meta_keys();
    nested_and_meta.push("nested".to_string());
    format!(
        r##"
let texts: &[&str] = {texts};
let key_groups: &[(&str, &[&str])] = &[
    ("typed additionalProperties map", {counts}),
    ("patternProperties map", {flags}),
    ("untyped object", {meta}),
    ("nested untyped object", {nested}),
    ("root catch-all map", {extras}),
];

fn find(haystack: &[u32], needle: &[u32]) -> Option<usize> {{
    haystack.windows(needle.len()).position(|window| window == needle)
}}

fn key_position(stream: &[u32], key: &str) -> Result<usize, String> {{
    let mut key_tokens = Vec::new();
    key.to_string().encode(&mut key_tokens);
    find(stream, &key_tokens).ok_or_else(|| format!("key {{key:?}} not found in stream"))
}}

let mut streams: Vec<Vec<u32>> = Vec::new();
for text in texts {{
    let value: MapOrder = serde_json::from_str(text)
        .map_err(|error| format!("instance did not deserialize: {{error}}"))?;
    let mut tokens = Vec::new();
    value.encode(&mut tokens);
    if value.token_count() != tokens.len() {{
        return Err(format!("token_count {{}} != stream length {{}}", value.token_count(), tokens.len()));
    }}

    let decoded = MapOrder::decode_full(tokens.clone())
        .map_err(|error| format!("decode_full failed: {{error}}"))?;
    let expected: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("instance is not JSON: {{error}}"))?;
    let actual = serde_json::to_value(&decoded)
        .map_err(|error| format!("decoded value did not serialize: {{error}}"))?;
    if actual != expected {{
        return Err(format!("decoded value differs: {{actual}} != {{expected}}"));
    }}

    let mut reencoded = Vec::new();
    decoded.encode(&mut reencoded);
    if reencoded != tokens {{
        return Err("encode(decode(t)) != t".to_string());
    }}
    streams.push(tokens);
}}

for (index, stream) in streams.iter().enumerate().skip(1) {{
    if stream != &streams[0] {{
        return Err(format!("source order {{index}} produced a different token stream than source order 0"));
    }}
}}

let canonical = &streams[0];
for (label, keys) in key_groups {{
    let mut sorted: Vec<&str> = keys.to_vec();
    sorted.sort();
    let mut previous: Option<(usize, &str)> = None;
    for key in sorted {{
        let position = key_position(canonical, key)?;
        if let Some((previous_position, previous_key)) = previous {{
            if position <= previous_position {{
                return Err(format!(
                    "{{label}}: key {{key:?}} at {{position}} is not after {{previous_key:?}} at {{previous_position}}"
                ));
            }}
        }}
        previous = Some((position, key));
    }}
}}

// Decode is order-tolerant: swap the first two patternProperties entries
// (key tokens plus one bool token each) and decode the non-canonical stream.
let first = key_position(canonical, "x-a")?;
let second = key_position(canonical, "x-b")?;
let third = key_position(canonical, "x-c")?;
let mut swapped = canonical[..first].to_vec();
swapped.extend_from_slice(&canonical[second..third]);
swapped.extend_from_slice(&canonical[first..second]);
swapped.extend_from_slice(&canonical[third..]);
if &swapped == canonical {{
    return Err("swapping two entries did not change the stream".to_string());
}}
let from_swapped = MapOrder::decode_full(swapped.clone())
    .map_err(|error| format!("non-canonical stream did not decode: {{error}}"))?;
let from_swapped_strict = MapOrder::decode_strict_full(swapped)
    .map_err(|error| format!("non-canonical stream did not strict-decode: {{error}}"))?;
let canonical_value = serde_json::to_value(MapOrder::decode_full(canonical.clone())
    .map_err(|error| format!("canonical stream did not decode: {{error}}"))?)
    .map_err(|error| format!("decoded value did not serialize: {{error}}"))?;
for decoded in [from_swapped, from_swapped_strict] {{
    let value = serde_json::to_value(&decoded)
        .map_err(|error| format!("decoded value did not serialize: {{error}}"))?;
    if value != canonical_value {{
        return Err(format!("non-canonical stream decoded to {{value}}, expected {{canonical_value}}"));
    }}
    let mut reencoded = Vec::new();
    decoded.encode(&mut reencoded);
    if &reencoded != canonical {{
        return Err("re-encoding a non-canonical stream did not give the canonical stream".to_string());
    }}
}}
Ok(())
"##,
        texts = rust_string_array(&texts),
        counts = rust_string_array(&count_keys()),
        flags = rust_string_array(&flag_keys()),
        meta = rust_string_array(&nested_and_meta),
        nested = rust_string_array(&nested_keys()),
        extras = rust_string_array(&extra_keys()),
    )
}

fn generated_map_order_code() -> String {
    TctCompiler::new(map_order_schema(), "MapOrder")
        .expect("schema is accepted")
        .compile()
        .expect("schema compiles")
        .generate_code()
        .expect("code generates")
}

#[test]
fn test_instance_fixtures_differ_only_in_key_order() {
    let ascending = serde_json::to_string(&instance(SourceOrder::Ascending)).unwrap();
    let descending = serde_json::to_string(&instance(SourceOrder::Descending)).unwrap();
    let shuffled = serde_json::to_string(&instance(SourceOrder::Shuffled)).unwrap();
    assert_ne!(ascending, descending);
    assert_ne!(ascending, shuffled);
    assert_ne!(descending, shuffled);
    assert_eq!(
        instance(SourceOrder::Ascending),
        instance(SourceOrder::Descending)
    );
    assert_eq!(
        instance(SourceOrder::Ascending),
        instance(SourceOrder::Shuffled)
    );
}

/// Untyped values keep source key order in `serde_json::Map` when the
/// generated crate enables `preserve_order`; the encoder must sort anyway.
#[test]
fn test_generated_map_entries_use_ascending_key_order_with_preserve_order() {
    run_generated_map_order_crate(
        "generated-map-key-order-preserve",
        &generated_map_order_code(),
        &map_order_body(),
        true,
    );
}

/// Without `preserve_order`, untyped objects are already sorted on input, but
/// typed and pattern maps are hash maps and must still be emitted sorted.
#[test]
fn test_generated_map_entries_use_ascending_key_order_without_preserve_order() {
    run_generated_map_order_crate(
        "generated-map-key-order-btree",
        &generated_map_order_code(),
        &map_order_body(),
        false,
    );
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
                .join("map-key-order")
        });
    fs::create_dir_all(&target_dir).unwrap_or_else(|error| {
        panic!(
            "failed to create generated map-key-order Cargo target dir {}: {}",
            target_dir.display(),
            error
        )
    });
    target_dir
}

fn run_generated_map_order_crate(
    crate_name: &str,
    generated_code: &str,
    body: &str,
    preserve_order: bool,
) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let repo_root = std::env::current_dir().expect("failed to resolve repo root");
    let mut cargo_toml = GeneratedCargoToml::new(crate_name);
    if preserve_order {
        cargo_toml = cargo_toml.with_preserve_order();
    }
    let cargo_toml = cargo_toml
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
        .expect("failed to copy workspace Cargo.lock into generated map-key-order crate");

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
