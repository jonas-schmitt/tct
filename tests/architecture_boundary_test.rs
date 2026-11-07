use std::fs;
use std::path::{Path, PathBuf};

const BOUNDARY_FILES: &[&str] = &[
    "src/generator/additional_properties_analyzer.rs",
    "src/generator/encoding_decision.rs",
    "src/generator/external_schemas.rs",
    "src/generator/optimization_phase.rs",
    "src/generator/passes.rs",
    "src/generator/schema_discovery.rs",
    "src/generator/type_analysis.rs",
    "src/generator/type_analyzer.rs",
];

const BOUNDARY_DIRS: &[&str] = &["src/generator/schema_walker", "src/generator/type_analyzer"];

const JSON_SCHEMA_COMPILER_DIRS: &[&str] = &["src/compiler", "src/generator"];

const FORBIDDEN_CODEGEN_MARKERS: &[(&str, &str)] = &[
    (
        "proc_macro2::TokenStream",
        "generated TokenStream data belongs in generation, not discovery/analysis/optimization",
    ),
    (
        "TokenStream",
        "generated token streams are a generation-layer concern",
    ),
    ("quote::quote", "quote usage belongs in generation"),
    ("quote!", "quote usage belongs in generation"),
    ("parse_quote!", "quote parsing belongs in generation"),
    ("syn::", "Rust syntax AST handling belongs in generation"),
];

const FORBIDDEN_GENERATED_SOURCE_CLASSIFIERS: &[(&str, &str)] = &[
    (
        "starts_with(\"pub struct",
        "generated source text must not classify discovered types",
    ),
    (
        "starts_with(\"pub enum",
        "generated source text must not classify discovered types",
    ),
    (
        "starts_with(\"pub type",
        "generated source text must not classify discovered types",
    ),
    (
        "contains(\"pub struct",
        "generated source text must not classify discovered types",
    ),
    (
        "contains(\"pub enum",
        "generated source text must not classify discovered types",
    ),
    (
        "contains(\"pub type",
        "generated source text must not classify discovered types",
    ),
    (
        "contains(\"impl crate::tokenizer::JsonTokenEncoder",
        "generated source text must not classify token encoder support",
    ),
    (
        "definition_tokens.to_string()",
        "generated token text must not feed discovery/analysis/optimization decisions",
    ),
    (
        "implementation_tokens.to_string()",
        "generated token text must not feed discovery/analysis/optimization decisions",
    ),
];

const FORBIDDEN_COMPILER_SOURCE_CLASSIFIERS: &[(&str, &str)] = &[
    (
        "root.definition.to_string()",
        "compiler conversion must route root definitions by metadata, not rendered source",
    ),
    (
        "generated_enum.definition.to_string()",
        "compiler conversion must route enum definitions by metadata, not rendered source",
    ),
    (
        "starts_with(\"pub struct",
        "compiler conversion must not classify generated definitions by source text",
    ),
    (
        "starts_with(\"pub enum",
        "compiler conversion must not classify generated definitions by source text",
    ),
    (
        "contains(\"pub struct",
        "compiler conversion must not classify generated definitions by source text",
    ),
    (
        "contains(\"pub enum",
        "compiler conversion must not classify generated definitions by source text",
    ),
];

const FORBIDDEN_OPTIMIZATION_DISCOVERY_MUTATIONS: &[(&str, &str)] = &[
    (
        "refined_discovery",
        "optimization must emit explicit metadata instead of refined discovery graphs",
    ),
    (
        "TctResult<(EncodingStrategy, SchemaDiscovery)>",
        "optimization must not return replacement SchemaDiscovery output",
    ),
    (
        "TctResult < (EncodingStrategy, SchemaDiscovery) >",
        "optimization must not return replacement SchemaDiscovery output",
    ),
];

const FORBIDDEN_NON_JSON_FRONTEND_MARKERS: &[(&str, &str)] = &[
    (
        "go_frontend",
        "Go/non-JSON frontend code must stay outside the JSON Schema compiler path",
    ),
    (
        "GoIr",
        "GoIR-specific behavior must stay outside the JSON Schema compiler path",
    ),
    (
        "goir",
        "GoIR-specific behavior must stay outside the JSON Schema compiler path",
    ),
    (
        "schemas/non_json",
        "non-JSON schema fixtures must not be hard-coded into compiler or generator code",
    ),
    (
        "experimental-tokenization",
        "experimental frontend artifacts must stay outside production compiler and generator code",
    ),
];

#[test]
fn discovery_analysis_and_optimization_do_not_depend_on_generated_code() {
    let boundary_sources = boundary_sources();

    let mut violations = Vec::new();
    for path in boundary_sources {
        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", path.display());
        });
        let code = strip_line_comments(&source);

        for (pattern, reason) in FORBIDDEN_CODEGEN_MARKERS
            .iter()
            .chain(FORBIDDEN_GENERATED_SOURCE_CLASSIFIERS.iter())
        {
            if code.contains(pattern) {
                violations.push(format!("{} contains `{pattern}`: {reason}", path.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "architecture boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn optimization_pass_does_not_replace_discovery_output() {
    let source = fs::read_to_string("src/generator/optimization_phase.rs")
        .expect("failed to read src/generator/optimization_phase.rs");
    let production_source = source
        .split_once("#[cfg(test)]")
        .map_or(source.as_str(), |(production, _tests)| production);
    let code = strip_line_comments(production_source);

    let mut violations = Vec::new();
    for (pattern, reason) in FORBIDDEN_OPTIMIZATION_DISCOVERY_MUTATIONS {
        if code.contains(pattern) {
            violations.push(format!(
                "src/generator/optimization_phase.rs contains `{pattern}`: {reason}"
            ));
        }
    }

    let passes_source = fs::read_to_string("src/generator/passes.rs")
        .expect("failed to read src/generator/passes.rs");
    let optimization_wrapper = passes_source
        .split("impl CompilationPass for OptimizationPassWrapper")
        .nth(1)
        .and_then(|section| {
            section
                .split("/// Orchestrates multiple compilation passes")
                .next()
        })
        .expect("failed to isolate OptimizationPassWrapper implementation");
    if strip_line_comments(optimization_wrapper).contains("context.schema_discovery =") {
        violations.push(
            "OptimizationPassWrapper must not write context.schema_discovery; generation should consume optimization metadata"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "optimization discovery mutation violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn compiler_conversion_does_not_classify_generated_source_text() {
    let source =
        fs::read_to_string("src/compiler/mod.rs").expect("failed to read src/compiler/mod.rs");
    let production_source = source
        .split_once("#[cfg(test)]")
        .map_or(source.as_str(), |(production, _tests)| production);
    let code = strip_line_comments(production_source);

    let mut violations = Vec::new();
    for (pattern, reason) in FORBIDDEN_COMPILER_SOURCE_CLASSIFIERS {
        if code.contains(pattern) {
            violations.push(format!(
                "src/compiler/mod.rs contains `{pattern}`: {reason}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "compiler generated-source classifier violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn json_schema_compiler_path_does_not_depend_on_non_json_frontends() {
    let mut sources = Vec::new();
    for dir in JSON_SCHEMA_COMPILER_DIRS {
        collect_rs_files(Path::new(dir), &mut sources);
    }
    sources.sort();

    let mut violations = Vec::new();
    for path in sources {
        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", path.display());
        });
        let code = strip_line_comments(&source);

        for (pattern, reason) in FORBIDDEN_NON_JSON_FRONTEND_MARKERS {
            if code.contains(pattern) {
                violations.push(format!("{} contains `{pattern}`: {reason}", path.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "non-JSON frontend boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn legacy_optimization_pass_api_is_classified_as_compatibility_only() {
    let module_source = fs::read_to_string("src/optimization_pass/mod.rs")
        .expect("failed to read src/optimization_pass/mod.rs");
    let core_source = fs::read_to_string("src/optimization_pass/core.rs")
        .expect("failed to read src/optimization_pass/core.rs");
    let config_source = fs::read_to_string("src/optimization_pass/config.rs")
        .expect("failed to read src/optimization_pass/config.rs");
    let passes_source = fs::read_to_string("src/optimization_pass/passes.rs")
        .expect("failed to read src/optimization_pass/passes.rs");
    let compiler_source =
        fs::read_to_string("src/compiler/mod.rs").expect("failed to read src/compiler/mod.rs");
    let cli_source = fs::read_to_string("src/main.rs").expect("failed to read src/main.rs");
    let architecture =
        fs::read_to_string("ARCHITECTURE.md").expect("failed to read ARCHITECTURE.md");

    assert!(
        module_source.contains("Compatibility-only optimization configuration"),
        "src/optimization_pass/mod.rs must classify the legacy API as compatibility-only"
    );
    assert!(
        module_source.contains("#[doc(hidden)]\npub mod core;")
            && module_source.contains("#[doc(hidden)]\npub mod passes;"),
        "legacy pass-construction modules should stay hidden from generated docs"
    );
    assert!(
        core_source.contains("Compatibility-only trait for historic optimization passes"),
        "src/optimization_pass/core.rs must not present TokenStream pass hooks as the active extension model"
    );
    assert!(
        core_source.contains("#[deprecated(")
            && core_source.contains("Compatibility-only TokenStream hook")
            && core_source.contains("EncodingStrategy decisions"),
        "legacy OptimizationPass must stay explicitly deprecated so new callers do not use it as an extension point"
    );
    assert!(
        passes_source.contains("Legacy built-in optimization pass implementations"),
        "src/optimization_pass/passes.rs must classify concrete legacy passes as compatibility-only"
    );
    assert!(
        config_source.contains("not arbitrary")
            && config_source.contains("custom passes that return rendered `TokenStream` fragments"),
        "src/optimization_pass/config.rs must document the compatibility boundary for custom pass lists"
    );
    assert!(
        compiler_source.contains("Add a legacy named built-in optimization pass override")
            && compiler_source.contains("without exposing the legacy `OptimizationPass`"),
        "TctCompiler::with_optimization_pass docs must describe named built-ins, not custom pass loading"
    );
    assert!(
        cli_source.contains("Enable a named built-in optimization pass")
            && cli_source
                .contains("does not load custom TokenStream-returning optimization passes"),
        "CLI --enable-pass help must describe named built-ins, not custom pass loading"
    );
    assert!(
        architecture.contains("The public `tct::optimization_pass` module is compatibility-only"),
        "ARCHITECTURE.md must keep the public legacy optimization API aligned with the four-pass model"
    );
}

fn boundary_sources() -> Vec<PathBuf> {
    let mut sources = BOUNDARY_FILES.iter().map(PathBuf::from).collect::<Vec<_>>();
    for dir in BOUNDARY_DIRS {
        collect_rs_files(Path::new(dir), &mut sources);
    }
    sources.sort();
    sources
}

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _comment)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_rs_files(dir: &Path, sources: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", dir.display());
    }) {
        let path = entry
            .unwrap_or_else(|error| panic!("failed to read entry in {}: {error}", dir.display()))
            .path();

        if path.is_dir() {
            collect_rs_files(&path, sources);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            sources.push(path);
        }
    }
}
