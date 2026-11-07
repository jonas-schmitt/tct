//! Generated source must be a pure function of the schema and the options.
//!
//! Two hash-ordered sources used to reorder the generated items (and the fields of
//! compound discriminator keys) from one run to the next, so generated tokenizers
//! could not be compared, cached, or audited by hash. A `std` hash set draws its own
//! random seed per instance, so generating the same schema several times in one
//! process exposes that just as reliably as separate runs do.

use std::fs;

use tct::{OptimizationLevel, TctCompiler};

fn generate(schema_path: &str, optimization_level: OptimizationLevel) -> String {
    let text = fs::read_to_string(schema_path)
        .unwrap_or_else(|error| panic!("failed to read {schema_path}: {error}"));
    let schema = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{schema_path} is not JSON: {error}"));
    TctCompiler::new(schema, "Determinism")
        .expect("schema should be accepted")
        .with_optimization_level(optimization_level)
        .compile()
        .expect("schema should compile")
        .generate_code()
        .expect("code generation should succeed")
}

fn assert_deterministic(schema_path: &str, optimization_level: OptimizationLevel) {
    let first = generate(schema_path, optimization_level);
    for run in 1..6 {
        assert!(
            generate(schema_path, optimization_level) == first,
            "run {run} of {schema_path} at {optimization_level:?} generated different source \
             than run 0"
        );
    }
}

/// Before the fix this schema (hundreds of independent types) produced a different
/// item order on every run.
#[test]
fn test_cargo_schema_generates_identical_source_every_time() {
    assert_deterministic("schemas/popular/cargo.json", OptimizationLevel::O3);
}

/// A second schema shape: many `$ref` dependencies and `anyOf` unions.
#[test]
fn test_github_workflow_schema_generates_identical_source_every_time() {
    assert_deterministic(
        "schemas/popular/github-workflow.json",
        OptimizationLevel::O3,
    );
}

/// No single field identifies these variants (`kind` repeats, `apiVersion` repeats,
/// `group` is shared), but the pair `kind` + `apiVersion` does, as in Kubernetes. The
/// candidate fields used to sit in a randomly ordered hash map, so the generated
/// matching code was keyed "Cat|v1" in one run and "v1|Cat" in the next. The fields of
/// a compound key are now joined in alphabetical order.
#[test]
fn test_compound_discriminator_keys_list_their_fields_alphabetically() {
    let schema = serde_json::json!({
        "oneOf": [
            {"$ref": "#/definitions/CatV1"},
            {"$ref": "#/definitions/CatV2"},
            {"$ref": "#/definitions/DogV1"}
        ],
        "definitions": {
            "CatV1": {"type": "object", "properties": {
                "kind": {"enum": ["Cat"]}, "apiVersion": {"enum": ["v1"]},
                "group": {"enum": ["pets"]}}},
            "CatV2": {"type": "object", "properties": {
                "kind": {"enum": ["Cat"]}, "apiVersion": {"enum": ["v2"]},
                "group": {"enum": ["pets"]}}},
            "DogV1": {"type": "object", "properties": {
                "kind": {"enum": ["Dog"]}, "apiVersion": {"enum": ["v1"]},
                "group": {"enum": ["pets"]}}}
        }
    });

    for run in 0..40 {
        let code = TctCompiler::new(schema.clone(), "Animal")
            .expect("schema should be accepted")
            .compile()
            .expect("schema should compile")
            .generate_code()
            .expect("code generation should succeed");

        for key in ["\"v1|Cat\" =>", "\"v2|Cat\" =>", "\"v1|Dog\" =>"] {
            assert!(code.contains(key), "run {run}: missing match arm {key}");
        }
        for reversed in ["\"Cat|v1\"", "\"Cat|v2\"", "\"Dog|v1\""] {
            assert!(
                !code.contains(reversed),
                "run {run}: compound key {reversed} lists its fields out of order"
            );
        }
    }
}

/// The unoptimized profile takes different code paths for enums and integers.
#[test]
fn test_unoptimized_profile_generates_identical_source_every_time() {
    assert_deterministic("schemas/popular/codecov.json", OptimizationLevel::O0);
}
