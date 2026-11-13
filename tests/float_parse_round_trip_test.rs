//! Correctly rounded float parsing in generated crates.
//!
//! A generated crate parses JSON text with `serde_json`. Without serde_json's
//! `float_roundtrip` feature its float parser may be off by one unit in the last
//! place, so the value is already wrong before it is encoded and
//! JSON -> tokens -> JSON changes it (`1.23e25` came back as
//! `1.2299999999999998e25`). The generated Cargo template enables the feature.
//!
//! The test builds a generated crate from the template alone (it does not link
//! the `tct` crate, whose own serde_json features would otherwise be unified
//! into the build) and checks, for each literal, that the value parsed from JSON
//! text into a `number` member and into an untyped member encodes and decodes
//! (recovery and strict) to the bits of Rust's correctly rounded
//! `str::parse::<f64>`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

/// JSON number literals. The 17-significant-digit decimals and `1.23e25` were
/// rounded to a neighbouring double by serde_json's default parser; the
/// others are controls.
const LITERALS: &[&str] = &[
    "1.23e25",
    "186039.52559446109",
    "990949.67815712491",
    "7847.7274263869464",
    "116831.73625255851",
    "393899.45669967563",
    "689945.92855533228",
    "0.1",
    "123.456",
    "-2.5e-300",
];

fn float_parse_body() -> String {
    let cases: Vec<String> = LITERALS
        .iter()
        .map(|literal| {
            let value: f64 = literal.parse().expect("literal parses");
            format!("({literal:?}, {:#x}u64)", value.to_bits())
        })
        .collect();
    format!(
        r##"
let cases: &[(&str, u64)] = &[{cases}];
let mut failures = Vec::new();
for &(literal, bits) in cases {{
    let expected = f64::from_bits(bits);
    let text = format!("{{{{\"x\": {{literal}}, \"v\": {{literal}}}}}}");
    let instance: FloatParse = serde_json::from_str(&text)
        .map_err(|error| format!("{{literal}}: instance did not deserialize: {{error}}"))?;
    let mut stream = Vec::new();
    instance.encode(&mut stream);
    for (label, decoded) in [
        ("decode_full", FloatParse::decode_full(stream.clone())),
        ("decode_strict_full", FloatParse::decode_strict_full(stream.clone())),
    ] {{
        let decoded = decoded.map_err(|error| format!("{{literal}}: {{label}} failed: {{error}}"))?;
        let json_value = serde_json::to_value(&decoded)
            .map_err(|error| format!("{{literal}}: decoded value did not serialize: {{error}}"))?;
        for member in ["x", "v"] {{
            let got = json_value[member]
                .as_f64()
                .ok_or_else(|| format!("{{literal}}: {{label}} member {{member}} is not a number: {{json_value}}"))?;
            if got.to_bits() != bits {{
                failures.push(format!(
                    "{{literal}}: {{label}} member {{member}} decoded to {{got:e}} (bits {{:#x}}), expected {{expected:e}} (bits {{bits:#x}})",
                    got.to_bits()
                ));
            }}
        }}
    }}
}}
if failures.is_empty() {{
    Ok(())
}} else {{
    Err(failures.join("\n"))
}}
"##,
        cases = cases.join(", "),
    )
}

fn float_parse_schema() -> serde_json::Value {
    json!({
        "title": "FloatParse",
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
fn test_expected_bits_come_from_correctly_rounded_parse() {
    // The test's reference is Rust's correctly rounded parser; pin a few
    // values so the reference itself is visible.
    assert_eq!("1.23e25".parse::<f64>().unwrap(), 1.23e25);
    assert_eq!(
        "186039.52559446109".parse::<f64>().unwrap(),
        186_039.525_594_461_1
    );
}

#[test]
fn test_generated_crate_parses_json_floats_correctly_rounded() {
    let generated = TctCompiler::new(float_parse_schema(), "FloatParse")
        .expect("schema is accepted")
        .compile()
        .expect("schema compiles")
        .generate_code()
        .expect("code generates");
    run_generated_float_parse_crate("generated-float-parse", &generated, &float_parse_body());
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
                .join("float-parse")
        });
    fs::create_dir_all(&target_dir).unwrap_or_else(|error| {
        panic!(
            "failed to create generated float-parse Cargo target dir {}: {}",
            target_dir.display(),
            error
        )
    });
    target_dir
}

fn run_generated_float_parse_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let repo_root = std::env::current_dir().expect("failed to resolve repo root");
    // No `with_tct_path`: the generated code is self-contained, and linking the
    // tct crate would unify its serde_json features into this build.
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_rustc_hash()
        .with_regex()
        .with_lazy_static()
        .with_chrono()
        .with_base64()
        .with_jsonschema()
        .with_bin(crate_name, "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&repo_root, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated float-parse crate");

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
