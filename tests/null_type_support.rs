use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use quote::ToTokens;
use serde_json::{Value, json};
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use tct::optimization_pass::OptimizationLevel;

const TYPE_ARRAY_SCHEMA_PATH: &str = "schemas/synthetic/type_array_members.json";
const TYPE_ARRAY_VALID_DIR: &str = "tests/data/instances/type_array_members/synthetic";
const TYPE_ARRAY_INVALID_DIR: &str = "tests/data/instances/type_array_members/invalid";

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn generated_struct_exists(file: &syn::File, struct_name: &str) -> bool {
    file.items.iter().any(
        |item| matches!(item, syn::Item::Struct(item_struct) if item_struct.ident == struct_name),
    )
}

fn generated_struct<'a>(file: &'a syn::File, struct_name: &str) -> &'a syn::ItemStruct {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item_struct) if item_struct.ident == struct_name => Some(item_struct),
            _ => None,
        })
        .unwrap_or_else(|| panic!("generated code should define struct {struct_name}"))
}

fn generated_enum<'a>(file: &'a syn::File, enum_name: &str) -> &'a syn::ItemEnum {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident == enum_name => Some(item_enum),
            _ => None,
        })
        .unwrap_or_else(|| panic!("generated code should define enum {enum_name}"))
}

fn field_type<'a>(item_struct: &'a syn::ItemStruct, field_name: &str) -> &'a syn::Type {
    item_struct
        .fields
        .iter()
        .find(|field| {
            field
                .ident
                .as_ref()
                .is_some_and(|ident| ident == field_name)
        })
        .map(|field| &field.ty)
        .unwrap_or_else(|| {
            panic!(
                "struct {} should have field {field_name}",
                item_struct.ident
            )
        })
}

fn type_text(ty: &syn::Type) -> String {
    ty.to_token_stream().to_string().replace(' ', "")
}

/// Last path segment of a type and its generic type arguments.
fn last_segment(ty: &syn::Type) -> (String, Vec<&syn::Type>) {
    let syn::Type::Path(type_path) = ty else {
        panic!("expected a path type, got {}", type_text(ty));
    };
    let segment = type_path
        .path
        .segments
        .last()
        .unwrap_or_else(|| panic!("empty type path {}", type_text(ty)));
    let arguments = match &segment.arguments {
        syn::PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                syn::GenericArgument::Type(inner) => Some(inner),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    (segment.ident.to_string(), arguments)
}

/// Strips one wrapper (`OptionalField<T>`, `Option<T>`, `Vec<T>`) and returns
/// the inner type.
fn unwrap_one<'a>(ty: &'a syn::Type, wrapper: &str) -> &'a syn::Type {
    let (name, arguments) = last_segment(ty);
    assert_eq!(
        name,
        wrapper,
        "expected {wrapper}<...>, got {}",
        type_text(ty)
    );
    assert_eq!(arguments.len(), 1, "expected one type argument");
    arguments[0]
}

fn type_name(ty: &syn::Type) -> String {
    let (name, arguments) = last_segment(ty);
    assert!(
        arguments.is_empty(),
        "expected a plain type name, got {}",
        type_text(ty)
    );
    name
}

/// Variant payloads of a generated enum: `None` for a unit variant, otherwise
/// the payload type.
fn variant_payloads(item_enum: &syn::ItemEnum) -> Vec<Option<&syn::Type>> {
    item_enum
        .variants
        .iter()
        .map(|variant| match &variant.fields {
            syn::Fields::Unit => None,
            syn::Fields::Unnamed(fields) => {
                assert_eq!(fields.unnamed.len(), 1, "one payload per variant");
                Some(&fields.unnamed[0].ty)
            }
            syn::Fields::Named(_) => panic!("unexpected named variant fields"),
        })
        .collect()
}

fn unit_variant_names(item_enum: &syn::ItemEnum) -> Vec<String> {
    item_enum
        .variants
        .iter()
        .map(|variant| {
            assert!(
                matches!(variant.fields, syn::Fields::Unit),
                "{} should be a simple enum",
                item_enum.ident
            );
            variant.ident.to_string()
        })
        .collect()
}

fn assert_no_untyped_value(item_struct: &syn::ItemStruct) {
    for field in &item_struct.fields {
        let text = type_text(&field.ty);
        assert!(
            !text.contains("serde_json::Value"),
            "member {} of {} should be typed, got {text}",
            field
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            item_struct.ident
        );
    }
}

fn type_array_schema() -> Value {
    let text = fs::read_to_string(TYPE_ARRAY_SCHEMA_PATH)
        .unwrap_or_else(|error| panic!("failed to read {TYPE_ARRAY_SCHEMA_PATH}: {error}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse {TYPE_ARRAY_SCHEMA_PATH}: {error}"))
}

fn type_array_code(level: OptimizationLevel) -> String {
    TctCompiler::new(type_array_schema(), "TypeArrayMembers")
        .expect("type-array schema is accepted")
        .with_optimization_level(level)
        .compile()
        .expect("type-array schema compiles")
        .generate_code()
        .expect("type-array schema generates code")
}

#[test]
fn test_null_type_simple_schema_generates_unit_type() {
    let schema = json!({
        "type": "null"
    });

    let result = TctCompiler::new(schema, "SimpleNull").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for simple null type: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");

    // Should generate a struct with unit type field
    assert!(
        code.contains("()"),
        "Generated code should contain unit type for null"
    );
}

#[test]
fn test_null_type_in_union_generates_enum_with_variants() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {
                "type": ["string", "null"]
            }
        }
    });

    let result = TctCompiler::new(schema, "MultiTypeWithNull").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for multi-type with null: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");
    let file = parse_generated_code(&code);

    // The member is a generated enum with a String variant and a unit variant
    // for null, not an untyped serde_json::Value.
    let root = generated_struct(&file, "MultiTypeWithNull");
    let value_type = field_type(root, "value");
    assert!(
        !type_text(value_type).contains("serde_json::Value"),
        "member value should be typed, got {}",
        type_text(value_type)
    );
    let enum_name = type_name(unwrap_one(value_type, "OptionalField"));
    let payloads: Vec<Option<String>> = variant_payloads(generated_enum(&file, &enum_name))
        .into_iter()
        .map(|payload| payload.map(type_text))
        .collect();
    assert_eq!(
        payloads,
        vec![Some("String".to_string()), None],
        "enum {enum_name} should have a String variant and a null variant"
    );
}

#[test]
fn test_type_array_members_compile_to_typed_enums() {
    let code = type_array_code(OptimizationLevel::O3);
    let file = parse_generated_code(&code);
    let root = generated_struct(&file, "TypeArrayMembers");
    assert_no_untyped_value(root);

    // Required [integer, null]: the enum itself (null is a value, absence is an
    // error), with the bounded integer in one variant and a unit null variant.
    let required = field_type(root, "required_count");
    let required_payloads = variant_payloads(generated_enum(&file, &type_name(required)));
    assert_eq!(required_payloads.len(), 2);
    let bounded = required_payloads[0].expect("integer variant carries a value");
    assert!(
        generated_struct_exists(&file, &type_name(bounded)),
        "integer variant should use the generated bounded-integer type, got {}",
        type_text(bounded)
    );
    assert!(required_payloads[1].is_none(), "null variant is a unit");

    // Optional nullable members keep the three states absent/null/present.
    let optional = field_type(root, "optional_count");
    let optional_enum = generated_enum(&file, &type_name(unwrap_one(optional, "OptionalField")));
    assert_eq!(variant_payloads(optional_enum).len(), 2);

    // [object, null] with properties: the object variant is a generated struct
    // with the declared members.
    let options = field_type(root, "options");
    let options_enum = generated_enum(&file, &type_name(unwrap_one(options, "OptionalField")));
    let options_payloads = variant_payloads(options_enum);
    assert_eq!(options_payloads.len(), 2);
    let options_struct = generated_struct(
        &file,
        &type_name(options_payloads[0].expect("object variant")),
    );
    field_type(options_struct, "strict");
    field_type(options_struct, "target");
    assert!(options_payloads[1].is_none(), "null variant is a unit");

    // [string, boolean] is not nullable: an optional member without null.
    let flag = field_type(root, "flag_or_name");
    let flag_enum = generated_enum(&file, &type_name(unwrap_one(flag, "Option")));
    let flag_payloads: Vec<String> = variant_payloads(flag_enum)
        .into_iter()
        .map(|payload| type_text(payload.expect("data variant")))
        .collect();
    assert_eq!(flag_payloads, vec!["String", "bool"]);

    // An enum that contains null: a string enum of the string values plus a
    // null variant.
    let level = field_type(root, "level");
    let level_enum = generated_enum(&file, &type_name(unwrap_one(level, "OptionalField")));
    let level_payloads = variant_payloads(level_enum);
    assert_eq!(level_payloads.len(), 2);
    let level_values = generated_enum(
        &file,
        &type_name(level_payloads[0].expect("string variant")),
    );
    assert_eq!(unit_variant_names(level_values), vec!["Low", "High"]);
    assert!(level_payloads[1].is_none(), "null variant is a unit");

    // Strings and booleans in one enum: each type keeps its own values.
    let source_maps = field_type(root, "source_maps");
    let source_maps_enum = generated_enum(&file, &type_name(unwrap_one(source_maps, "Option")));
    let source_maps_payloads = variant_payloads(source_maps_enum);
    assert_eq!(source_maps_payloads.len(), 2);
    let source_map_values = generated_enum(
        &file,
        &type_name(source_maps_payloads[0].expect("string variant")),
    );
    assert_eq!(
        unit_variant_names(source_map_values),
        vec!["Inline", "Both"]
    );
    assert_eq!(
        type_text(source_maps_payloads[1].expect("boolean variant")),
        "bool"
    );

    // A one-element type array is the plain type.
    assert_eq!(
        type_text(unwrap_one(field_type(root, "single"), "Option")),
        "String"
    );

    // Array items and map values with type arrays get their own enums, also
    // when several of them share a parent type.
    let tags_item = type_name(unwrap_one(
        unwrap_one(field_type(root, "tags"), "Option"),
        "Vec",
    ));
    let scores_item = type_name(unwrap_one(
        unwrap_one(field_type(root, "scores"), "Option"),
        "Vec",
    ));
    let env_map = unwrap_one(field_type(root, "env"), "Option");
    let (env_map_name, env_map_arguments) = last_segment(env_map);
    assert_eq!(env_map_name, "FxHashMap");
    let env_value = type_name(env_map_arguments[1]);
    assert_ne!(tags_item, scores_item);
    assert_ne!(tags_item, env_value);
    assert_ne!(scores_item, env_value);
    let tags_payloads: Vec<Option<String>> = variant_payloads(generated_enum(&file, &tags_item))
        .into_iter()
        .map(|payload| payload.map(type_text))
        .collect();
    assert_eq!(tags_payloads, vec![Some("String".to_string()), None]);
    let env_payloads: Vec<String> = variant_payloads(generated_enum(&file, &env_value))
        .into_iter()
        .map(|payload| type_text(payload.expect("data variant")))
        .collect();
    assert_eq!(env_payloads, vec!["String", "serde_json::Number", "bool"]);
}

#[test]
fn test_type_array_enum_without_value_of_listed_types_is_rejected() {
    let schema = json!({
        "title": "NoAdmissibleValue",
        "type": "object",
        "properties": {
            "mode": {"type": ["string", "boolean"], "enum": [1, 2]}
        }
    });

    let error = TctCompiler::new(schema, "NoAdmissibleValue")
        .and_then(|compiler| compiler.compile())
        .err()
        .expect("a type array whose enum admits none of its types should not compile");
    let message = error.to_string();
    assert!(
        message.contains("enum") && message.contains("mode"),
        "error should name the enum and the member: {message}"
    );
}

#[test]
fn test_type_array_without_json_type_name_is_rejected() {
    for (label, type_array) in [
        ("unknown name", json!(["text", "null"])),
        ("empty array", json!([])),
        ("no string entry", json!([5])),
    ] {
        let schema = json!({
            "title": "BadTypeArray",
            "type": "object",
            "properties": {
                "value": {"type": type_array}
            }
        });

        let error = TctCompiler::new(schema, "BadTypeArray")
            .and_then(|compiler| compiler.compile())
            .err()
            .unwrap_or_else(|| panic!("{label}: the type array should not compile"));
        let message = error.to_string();
        assert!(
            message.contains("`type` array") && message.contains("value"),
            "{label}: error should name the type array and the member: {message}"
        );
    }
}

#[test]
fn test_type_array_skips_non_string_entries() {
    // Non-string entries are skipped, as type-array discovery always did
    // (see the schema walker's `test_discover_type_array_with_invalid_element`).
    let schema = json!({
        "title": "NonStringTypeEntry",
        "type": "object",
        "properties": {
            "value": {"type": ["string", 5]}
        }
    });

    let code = TctCompiler::new(schema, "NonStringTypeEntry")
        .and_then(|compiler| compiler.compile())
        .and_then(|grammar| grammar.generate_code())
        .expect("the string entry of the type array should compile");
    let file = parse_generated_code(&code);
    let root = generated_struct(&file, "NonStringTypeEntry");
    assert_eq!(
        type_text(unwrap_one(field_type(root, "value"), "Option")),
        "String"
    );
}

#[test]
fn test_type_array_members_round_trip_and_reject_invalid_instances() {
    let valid = read_instances(TYPE_ARRAY_VALID_DIR);
    let invalid = read_instances(TYPE_ARRAY_INVALID_DIR);
    assert!(valid.len() >= 4, "expected valid type-array instances");
    assert!(invalid.len() >= 10, "expected invalid type-array instances");

    let code = type_array_code(OptimizationLevel::O3);
    run_generated_type_array_crate(
        "generated-type-array-members",
        &code,
        &type_array_body(&valid, &invalid),
    );
}

#[test]
fn test_null_type_in_object_property_generates_struct_field() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "value": {"type": "null"},
            "count": {"type": "integer"}
        }
    });

    let result = TctCompiler::new(schema, "ObjectWithNull").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for object with null field: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");
    let generated_file = parse_generated_code(&code);

    // Should generate a struct with various field types including unit type
    assert!(
        generated_struct_exists(&generated_file, "ObjectWithNull"),
        "Should generate main struct"
    );
    assert!(code.contains("String"), "Should contain string field type");
    assert!(
        code.contains("JsonInteger"),
        "Should contain integer field type"
    );
}

#[test]
fn test_null_type_in_oneof_schema_generates_enum_variant() {
    let schema = json!({
        "oneOf": [
            {"type": "string"},
            {"type": "integer"},
            {"type": "null"}
        ]
    });

    let result = TctCompiler::new(schema, "OneOfWithNull").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for oneOf with null: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");

    // Should generate an enum with all three variants
    assert!(code.contains("enum"), "Should generate enum for oneOf");
}

#[test]
fn test_null_type_as_array_items_generates_vec_unit() {
    let schema = json!({
        "type": "array",
        "items": {
            "type": "null"
        }
    });

    let result = TctCompiler::new(schema, "ArrayWithNullItems").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for array with null items: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");

    // Should generate Vec<()> for array of null values
    assert!(code.contains("Vec"), "Should generate Vec type for array");
}

/// `(file stem, JSON text)` for every instance in a directory, sorted by name.
fn read_instances(dir: &str) -> Vec<(String, String)> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to list {dir}: {error}"))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_else(|| panic!("instance file name {}", path.display()))
                .to_string();
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            serde_json::from_str::<Value>(&text)
                .unwrap_or_else(|error| panic!("{} is not JSON: {error}", path.display()));
            (name, text)
        })
        .collect()
}

fn rust_instance_table(instances: &[(String, String)]) -> String {
    instances
        .iter()
        .map(|(name, text)| format!("({name:?}, {text:?})"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn type_array_body(valid: &[(String, String)], invalid: &[(String, String)]) -> String {
    format!(
        r##"
let valid: &[(&str, &str)] = &[{valid}];
let invalid: &[(&str, &str)] = &[{invalid}];
let mut failures = Vec::new();
for &(name, text) in valid {{
    let expected: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("{{name}}: not JSON: {{error}}"))?;
    let instance: TypeArrayMembers = match serde_json::from_str(text) {{
        Ok(instance) => instance,
        Err(error) => {{
            failures.push(format!("{{name}}: valid instance rejected: {{error}}"));
            continue;
        }}
    }};
    let mut stream = Vec::new();
    instance.encode(&mut stream);
    if instance.token_count() != stream.len() {{
        failures.push(format!(
            "{{name}}: token_count {{}} != stream length {{}}",
            instance.token_count(),
            stream.len()
        ));
    }}
    for (label, decoded) in [
        ("decode_full", TypeArrayMembers::decode_full(stream.clone())),
        ("decode_strict_full", TypeArrayMembers::decode_strict_full(stream.clone())),
    ] {{
        let decoded = match decoded {{
            Ok(decoded) => decoded,
            Err(error) => {{
                failures.push(format!("{{name}}: {{label}} failed: {{error}}"));
                continue;
            }}
        }};
        let json_value = serde_json::to_value(&decoded)
            .map_err(|error| format!("{{name}}: decoded value did not serialize: {{error}}"))?;
        if json_value != expected {{
            failures.push(format!("{{name}}: {{label}} gave {{json_value}}, expected {{expected}}"));
        }}
        let mut reencoded = Vec::new();
        decoded.encode(&mut reencoded);
        if reencoded != stream {{
            failures.push(format!("{{name}}: {{label}} re-encodes to a different stream"));
        }}
    }}
}}
for &(name, text) in invalid {{
    if serde_json::from_str::<TypeArrayMembers>(text).is_ok() {{
        failures.push(format!("{{name}}: schema-invalid instance was accepted"));
    }}
}}
if failures.is_empty() {{
    Ok(())
}} else {{
    Err(failures.join("\n"))
}}
"##,
        valid = rust_instance_table(valid),
        invalid = rust_instance_table(invalid),
    )
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
                .join("type-array-members")
        });
    fs::create_dir_all(&target_dir).unwrap_or_else(|error| {
        panic!(
            "failed to create generated type-array Cargo target dir {}: {}",
            target_dir.display(),
            error
        )
    });
    target_dir
}

fn run_generated_type_array_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let repo_root = std::env::current_dir().expect("failed to resolve repo root");
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
        .expect("failed to copy workspace Cargo.lock into generated type-array crate");

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
