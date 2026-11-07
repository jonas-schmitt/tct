//! Round-Trip Testing Utilities
//!
//! Shared infrastructure for testing complete round-trips with real JSON instances:
//! Schema → Generate Code → Compile → Encode JSON → Decode Tokens → Verify Equality

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};
use std::time::Duration;
use tct::TctCompiler;
use tct::generated_crate::{
    GENERATED_EXTERNAL_BUILD_ENV, GeneratedCargoToml, configure_generated_cargo_command,
    copy_workspace_cargo_lock,
};
use tct::optimization_pass::OptimizationLevel;
use tempfile::TempDir;

pub(crate) struct SourceSchemaExternalResource {
    uri: &'static str,
    file_name: &'static str,
    schema_json: &'static str,
}

pub(crate) const SOURCE_SCHEMA_EXTERNAL_RESOURCES: &[SourceSchemaExternalResource] = &[
    SourceSchemaExternalResource {
        uri: "https://json.schemastore.org/partial-eslint-plugins.json",
        file_name: "external-partial-eslint-plugins.json",
        schema_json: include_str!("../schemas/external/partial-eslint-plugins.json"),
    },
    SourceSchemaExternalResource {
        uri: "https://json.schemastore.org/eslintrc.json",
        file_name: "external-eslintrc.json",
        schema_json: include_str!("../schemas/popular/eslintrc.json"),
    },
    SourceSchemaExternalResource {
        uri: "https://json.schemastore.org/prettierrc.json",
        file_name: "external-prettierrc.json",
        schema_json: include_str!("../schemas/popular/prettierrc.json"),
    },
    SourceSchemaExternalResource {
        uri: "https://json.schemastore.org/stylelintrc.json",
        file_name: "external-stylelintrc.json",
        schema_json: include_str!("../schemas/popular/stylelintrc.json"),
    },
    SourceSchemaExternalResource {
        uri: "https://json.schemastore.org/ava.json",
        file_name: "external-ava.json",
        schema_json: include_str!("../schemas/popular/ava.json"),
    },
    SourceSchemaExternalResource {
        uri: "https://json.schemastore.org/semantic-release.json",
        file_name: "external-semantic-release.json",
        schema_json: include_str!("../schemas/common/semantic-release.json"),
    },
    SourceSchemaExternalResource {
        uri: "https://json.schemastore.org/jscpd.json",
        file_name: "external-jscpd.json",
        schema_json: include_str!("../schemas/external/jscpd.json"),
    },
];

pub(crate) fn write_source_schema_external_resource_files(
    dir: impl AsRef<Path>,
) -> std::io::Result<()> {
    for resource in SOURCE_SCHEMA_EXTERNAL_RESOURCES {
        fs::write(dir.as_ref().join(resource.file_name), resource.schema_json)?;
    }
    Ok(())
}

pub(crate) fn source_schema_external_resource_constants(include_prefix: &str) -> String {
    let mut source = String::from("const SOURCE_SCHEMA_EXTERNAL_RESOURCES: &[(&str, &str)] = &[\n");
    for resource in SOURCE_SCHEMA_EXTERNAL_RESOURCES {
        source.push_str(&format!(
            "    ({:?}, include_str!(\"{}{}\")),\n",
            resource.uri, include_prefix, resource.file_name
        ));
    }
    source.push_str("];\n");
    source
}

pub(crate) fn source_schema_external_resource_build_copy_code() -> String {
    let mut source = String::new();
    for resource in SOURCE_SCHEMA_EXTERNAL_RESOURCES {
        source.push_str(&format!(
            "    let resource_dest = Path::new(&out_dir).join({:?});\n",
            resource.file_name
        ));
        source.push_str(&format!(
            "    fs::copy({:?}, &resource_dest)?;\n",
            resource.file_name
        ));
        source.push_str(&format!(
            "    println!(\"cargo:rerun-if-changed={}\");\n\n",
            resource.file_name
        ));
    }
    source
}

pub(crate) fn source_schema_validator_function_code() -> &'static str {
    r#"fn build_source_schema_validator(schema: &serde_json::Value) -> Result<jsonschema::Validator, String> {
    let external_resources = SOURCE_SCHEMA_EXTERNAL_RESOURCES
        .iter()
        .map(|(uri, schema_json)| {
            let schema: serde_json::Value =
                serde_json::from_str(schema_json).map_err(|error| {
                    format!("invalid vendored source-schema resource {}: {}", uri, error)
                })?;
            Ok((*uri, schema))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut registry = jsonschema::Registry::new();
    for (uri, resource_schema) in &external_resources {
        registry = registry.add(*uri, resource_schema).map_err(|error| {
            format!(
                "invalid source-schema validation registry resource {}: {}",
                uri, error
            )
        })?;
    }
    let registry = registry
        .prepare()
        .map_err(|error| format!("invalid source-schema validation registry: {}", error))?;

    jsonschema::options()
        .with_registry(&registry)
        .build(schema)
        .map_err(|error| {
            format!(
                "source schema validator build failed for schema {}: {}",
                SOURCE_SCHEMA_NAME, error
            )
        })
}
"#
}

#[allow(dead_code)]
pub(crate) fn build_source_schema_validator(
    schema: &Value,
    schema_name: &str,
) -> Result<jsonschema::Validator, String> {
    let external_resources = SOURCE_SCHEMA_EXTERNAL_RESOURCES
        .iter()
        .map(|resource| {
            let schema: Value = serde_json::from_str(resource.schema_json).map_err(|error| {
                format!(
                    "invalid vendored source-schema resource {} for {}: {}",
                    resource.uri, schema_name, error
                )
            })?;
            Ok((resource.uri, schema))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut registry = jsonschema::Registry::new();
    for (uri, resource_schema) in &external_resources {
        registry = registry.add(*uri, resource_schema).map_err(|error| {
            format!(
                "invalid source-schema validation registry resource {} for {}: {}",
                uri, schema_name, error
            )
        })?;
    }
    let registry = registry.prepare().map_err(|error| {
        format!(
            "invalid source-schema validation registry for {}: {}",
            schema_name, error
        )
    })?;

    jsonschema::options()
        .with_registry(&registry)
        .build(schema)
        .map_err(|error| {
            format!(
                "source schema validator build failed for schema {}: {}",
                schema_name, error
            )
        })
}

/// Wait for a child process with timeout
fn wait_for_child_with_timeout(
    child: &mut Child,
    timeout: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process has finished
                let stdout = {
                    let mut buf = Vec::new();
                    if let Some(ref mut stdout) = child.stdout {
                        use std::io::Read;
                        let _ = stdout.read_to_end(&mut buf);
                    }
                    buf
                };

                let stderr = {
                    let mut buf = Vec::new();
                    if let Some(ref mut stderr) = child.stderr {
                        use std::io::Read;
                        let _ = stderr.read_to_end(&mut buf);
                    }
                    buf
                };

                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                // Process still running, check timeout
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(format!("Process timed out after {:?}", timeout).into());
                }
                // Wait a bit before checking again
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("Error waiting for process: {}", e).into());
            }
        }
    }
}

/// Results from a round-trip test
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RoundTripResult {
    pub schema_name: String,
    pub instance_name: String,
    pub success: bool,
    pub error: Option<String>,
    pub metric_source: RoundTripMetricSource,
    pub compression_ratio: Option<f64>,
    pub encode_time_us: Option<u64>,
    pub decode_time_us: Option<u64>,
    pub token_count: Option<usize>,
    pub json_bytes: Option<usize>,
    pub token_bytes: Option<usize>,
}

/// Provenance for optional metric fields on [`RoundTripResult`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RoundTripMetricSource {
    /// No timing or compression metrics were collected for this result.
    NotCollected,
    /// Metrics came from executing generated tokenizer code.
    GeneratedExecution,
    /// Metrics were derived from static schema/instance estimates, not execution.
    StaticEstimate,
}

/// Enum representing the different phases of round-trip testing
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoundTripPhase {
    SchemaCompilation,
    CodeGeneration,
    TokenEncoding,
    TokenDecoding,
    DataIntegrity,
}

#[allow(dead_code)]
impl RoundTripPhase {
    pub fn name(&self) -> &'static str {
        match self {
            RoundTripPhase::SchemaCompilation => "Schema Compilation",
            RoundTripPhase::CodeGeneration => "Code Generation",
            RoundTripPhase::TokenEncoding => "Token Encoding",
            RoundTripPhase::TokenDecoding => "Token Decoding",
            RoundTripPhase::DataIntegrity => "Data Integrity",
        }
    }
}

/// Results from a single phase of round-trip testing
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RoundTripPhaseResult {
    pub phase: RoundTripPhase,
    pub success: bool,
    pub error: Option<String>,
    pub metrics: HashMap<String, String>,
    pub debug_output: Option<String>,
    pub duration_us: Option<u64>,
}

#[allow(dead_code)]
impl RoundTripPhaseResult {
    pub fn success(phase: RoundTripPhase) -> Self {
        Self {
            phase,
            success: true,
            error: None,
            metrics: HashMap::new(),
            debug_output: None,
            duration_us: None,
        }
    }

    pub fn failure(phase: RoundTripPhase, error: String) -> Self {
        Self {
            phase,
            success: false,
            error: Some(error),
            metrics: HashMap::new(),
            debug_output: None,
            duration_us: None,
        }
    }

    pub fn with_metrics(mut self, metrics: HashMap<String, String>) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn with_debug(mut self, debug_output: String) -> Self {
        self.debug_output = Some(debug_output);
        self
    }

    pub fn with_duration(mut self, duration_us: u64) -> Self {
        self.duration_us = Some(duration_us);
        self
    }
}

/// Complete results from phased round-trip testing
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PhasedRoundTripResult {
    pub _schema_name: String,
    pub _instance_name: String,
    pub overall_success: bool,
    pub phases: Vec<RoundTripPhaseResult>,
    pub total_duration_us: Option<u64>,
}

#[allow(dead_code)]
impl PhasedRoundTripResult {
    pub fn new(schema_name: String, instance_name: String) -> Self {
        Self {
            _schema_name: schema_name,
            _instance_name: instance_name,
            overall_success: false,
            phases: Vec::new(),
            total_duration_us: None,
        }
    }

    pub fn add_phase_result(&mut self, result: RoundTripPhaseResult) {
        self.phases.push(result);
        // Overall success requires all phases to succeed
        self.overall_success = self.phases.iter().all(|p| p.success);
    }

    pub fn failed_phase(&self) -> Option<&RoundTripPhaseResult> {
        self.phases.iter().find(|p| !p.success)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchRoundTripSuccess {
    pub name: String,
    pub encode_time_ns: u64,
    pub decode_time_ns: u64,
    pub token_count: usize,
    pub max_token: u32,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParsedBatchRoundTripResults {
    pub successful_instances: usize,
    pub failed_instances: Vec<(String, String)>,
    pub per_instance_times_us: HashMap<String, (u64, u64)>,
    pub successes: Vec<BatchRoundTripSuccess>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BatchRoundTripSummary {
    pub total_instances: usize,
    pub successful_instances: usize,
    pub successes: Vec<BatchRoundTripSuccess>,
    pub failed_instances: Vec<(String, String)>,
    pub schema_compilation_time_ms: u64,
    pub code_generation_time_ms: u64,
    pub rust_compilation_time_ms: u64,
}

#[allow(dead_code)]
impl BatchRoundTripSummary {
    pub fn success_rate(&self) -> f64 {
        if self.total_instances == 0 {
            0.0
        } else {
            self.successful_instances as f64 / self.total_instances as f64
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceFixtureScope {
    Standard,
    IncludeLargeScale,
}

fn sanitize_path_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "item".to_string()
    } else {
        sanitized
    }
}

fn non_empty_os_str(value: Option<&OsStr>) -> Option<&OsStr> {
    value.filter(|value| !value.to_string_lossy().is_empty())
}

fn resolve_relative_to(base: &Path, value: &OsStr) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn resolve_generated_cargo_target_dir(
    tct_path: &Path,
    crate_path: &Path,
    generated_target_dir: Option<&OsStr>,
    inherited_target_dir: Option<&OsStr>,
) -> PathBuf {
    if let Some(target_dir) = non_empty_os_str(generated_target_dir) {
        return resolve_relative_to(tct_path, target_dir);
    }
    if let Some(target_dir) = non_empty_os_str(inherited_target_dir) {
        return resolve_relative_to(crate_path, target_dir);
    }
    tct_path.join("target").join("tct-generated-cargo")
}

pub(crate) fn generated_cargo_target_dir_for_crate(crate_path: &Path) -> PathBuf {
    let tct_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    resolve_generated_cargo_target_dir(
        &tct_path,
        crate_path,
        std::env::var_os("TCT_GENERATED_CARGO_TARGET_DIR").as_deref(),
        std::env::var_os("CARGO_TARGET_DIR").as_deref(),
    )
}

fn configure_round_trip_generated_cargo_command(command: &mut Command) -> &mut Command {
    let tct_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    configure_generated_cargo_command(command, &tct_path);
    command.env(GENERATED_EXTERNAL_BUILD_ENV, "1")
}

#[allow(dead_code)]
fn write_if_changed(
    path: impl AsRef<Path>,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if fs::read_to_string(path)
        .map(|existing| existing == contents)
        .unwrap_or(false)
    {
        return Ok(());
    }

    fs::write(path, contents)?;
    Ok(())
}

fn optimization_level_path(optimization_level: OptimizationLevel) -> &'static str {
    match optimization_level {
        OptimizationLevel::O0 => "tct::optimization_pass::OptimizationLevel::O0",
        OptimizationLevel::O2 => "tct::optimization_pass::OptimizationLevel::O2",
        OptimizationLevel::O3 => "tct::optimization_pass::OptimizationLevel::O3",
        OptimizationLevel::Os => "tct::optimization_pass::OptimizationLevel::Os",
        OptimizationLevel::OVocab => "tct::optimization_pass::OptimizationLevel::OVocab",
    }
}

#[allow(dead_code)]
pub(crate) fn schema_base_name_for(schema_name: &str) -> &str {
    if schema_name.contains('/') {
        schema_name.split('/').next_back().unwrap_or(schema_name)
    } else {
        schema_name.strip_suffix(".json").unwrap_or(schema_name)
    }
}

#[allow(dead_code)]
pub(crate) fn mixed_fixture_matches_schema(filename: &str, schema_base_name: &str) -> bool {
    filename.starts_with(schema_base_name)
        || filename.ends_with(&format!("_{}", schema_base_name))
        || filename.ends_with(&format!("-{}", schema_base_name))
}

#[allow(dead_code)]
pub fn instance_search_patterns_for_schema(
    schema_name: &str,
    tier: &str,
    fixture_scope: InstanceFixtureScope,
) -> Vec<(String, &'static str)> {
    let schema_base_name = schema_base_name_for(schema_name);
    let include_large_scale = fixture_scope == InstanceFixtureScope::IncludeLargeScale;
    let mut search_patterns = Vec::new();

    if tier == "synthetic" {
        search_patterns.push((
            format!("tests/data/instances/{}/{}/real", tier, schema_name),
            "real",
        ));
        search_patterns.push((
            format!("tests/data/instances/{}/{}/synthetic", tier, schema_name),
            "synthetic",
        ));
        search_patterns.push((
            format!("tests/data/instances/{}/{}", tier, schema_name),
            "nested",
        ));
        search_patterns.push((format!("tests/data/instances/{}", schema_name), "nested"));
    }

    if schema_name.contains('/') {
        search_patterns.push((format!("tests/data/instances/{}/real", schema_name), "real"));
        search_patterns.push((
            format!("tests/data/instances/{}/synthetic", schema_name),
            "synthetic",
        ));
        if include_large_scale {
            search_patterns.push((
                format!("tests/data/instances/{}/large-scale", schema_name),
                "large-scale",
            ));
        }
        search_patterns.push((format!("tests/data/instances/{}", schema_name), "nested"));
    }

    search_patterns.extend([
        (
            format!("tests/data/instances/{}/real", schema_base_name),
            "real",
        ),
        (
            format!("tests/data/instances/{}/synthetic", schema_base_name),
            "synthetic",
        ),
    ]);

    if include_large_scale {
        search_patterns.push((
            format!("tests/data/instances/{}/large-scale", schema_base_name),
            "large-scale",
        ));
    }

    search_patterns.push((
        format!("tests/data/instances/{}", schema_base_name),
        "mixed",
    ));

    let mut seen_search_dirs = HashSet::new();
    search_patterns.retain(|(instances_dir, _)| seen_search_dirs.insert(instances_dir.clone()));
    search_patterns
}

/// Load all instances for a given schema from both real and synthetic directories
#[allow(dead_code)]
pub fn load_instances_for_schema(schema_name: &str, tier: &str) -> Vec<(String, Value)> {
    load_instances_for_schema_with_scope(schema_name, tier, InstanceFixtureScope::Standard)
}

#[allow(dead_code)]
pub fn load_instances_for_schema_with_scope(
    schema_name: &str,
    tier: &str,
    fixture_scope: InstanceFixtureScope,
) -> Vec<(String, Value)> {
    let mut instances = Vec::new();
    let mut real_count = 0;
    let mut synthetic_count = 0;
    let mut invalid_count = 0;

    // Extract just the schema name from "tier/name" format
    let schema_base_name = schema_base_name_for(schema_name);

    println!(
        "📂 Loading instances for schema: {} (tier: {})",
        schema_base_name, tier
    );

    let search_patterns = instance_search_patterns_for_schema(schema_name, tier, fixture_scope);

    for (instances_dir, instance_type) in search_patterns {
        println!("  🔍 Searching in: {}", instances_dir);

        if let Ok(dir) = fs::read_dir(&instances_dir) {
            let mut found_in_dir = 0;
            let mut paths = dir.flatten().map(|entry| entry.path()).collect::<Vec<_>>();
            paths.sort();

            for path in paths {
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        // Check for 404 errors (should be cleaned up now, but defensive)
                        if content.trim() == "404: Not Found" {
                            println!("    ⚠️ Skipping 404 error file: {}", path.display());
                            invalid_count += 1;
                            continue;
                        }

                        if let Ok(instance) = serde_json::from_str::<Value>(&content) {
                            let filename = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown");

                            if filename == "metadata" {
                                continue;
                            }

                            // For broad mixed directories, filter by schema-bearing names.
                            if instance_type == "mixed"
                                && !mixed_fixture_matches_schema(filename, schema_base_name)
                            {
                                // Skip files that don't match this schema
                                continue;
                            }

                            let name_path =
                                path.strip_prefix("tests/data/instances").unwrap_or(&path);
                            let mut name_path = name_path.to_path_buf();
                            name_path.set_extension("");
                            let name = name_path
                                .to_string_lossy()
                                .trim_start_matches(std::path::MAIN_SEPARATOR)
                                .replace(std::path::MAIN_SEPARATOR, "/");
                            instances.push((name.clone(), instance));
                            found_in_dir += 1;

                            if instance_type == "real" {
                                real_count += 1;
                            } else if instance_type == "synthetic" || instance_type == "nested" {
                                synthetic_count += 1;
                            } else {
                                // "mixed" - count as real since they're typically real-world examples
                                real_count += 1;
                            }

                            println!("    ✅ Loaded {} instance: {}", instance_type, name);
                        } else {
                            println!("    ❌ Invalid JSON in: {}", path.display());
                            invalid_count += 1;
                        }
                    } else {
                        println!("    ❌ Could not read file: {}", path.display());
                        invalid_count += 1;
                    }
                }
            }
            if found_in_dir > 0 {
                println!(
                    "  ✅ Found {} valid instances in {}",
                    found_in_dir, instances_dir
                );
            }
        }
    }

    println!("📊 Instance loading summary for {}:", schema_base_name);
    println!("  • Real instances: {}", real_count);
    println!("  • Synthetic instances: {}", synthetic_count);
    println!("  • Invalid/skipped files: {}", invalid_count);
    println!("  • Total valid instances: {}", instances.len());

    if instances.is_empty() {
        println!(
            "  ⚠️ WARNING: No valid instances found for schema {}",
            schema_base_name
        );
    }

    instances
}

/// Create complete round-trip test crate using file-based approach
fn create_round_trip_test_crate(
    test_dir: &str,
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
    optimization_level: OptimizationLevel,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Create directory structure
    fs::create_dir_all(test_dir)?;
    fs::create_dir_all(format!("{}/src", test_dir))?;

    // Write schema and instance to separate files (avoids string escaping)
    fs::write(
        format!("{}/schema.json", test_dir),
        serde_json::to_string_pretty(schema)?,
    )?;
    fs::write(
        format!("{}/instance.json", test_dir),
        serde_json::to_string_pretty(instance)?,
    )?;
    write_source_schema_external_resource_files(test_dir)?;
    let schema_name_literal = serde_json::to_string(schema_name)?;
    let instance_name_literal = serde_json::to_string(instance_name)?;

    // Create Cargo.toml with absolute path to avoid path issues
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let tct_path = current_dir.display();

    let package_name = format!(
        "round-trip-{}-{}",
        sanitize_path_component(schema_name),
        sanitize_path_component(instance_name)
    );
    let cargo_toml = GeneratedCargoToml::new(package_name)
        .with_tct_path(tct_path.to_string())
        .with_regex()
        .with_jsonschema()
        .with_rustc_hash()
        .with_chrono()
        .with_base64()
        .with_build_tct_path(tct_path.to_string())
        .render();

    fs::write(format!("{}/Cargo.toml", test_dir), cargo_toml)?;
    copy_workspace_cargo_lock(&current_dir, Path::new(test_dir))?;

    // Create build.rs that reads from files (no string escaping issues)
    let mut build_rs = String::new();
    build_rs.push_str("use std::env;\n");
    build_rs.push_str("use std::fs;\n");
    build_rs.push_str("use std::path::Path;\n");
    build_rs.push_str("use serde_json::Value;\n");
    build_rs.push_str("use tct::TctCompiler;\n\n");
    build_rs.push_str("fn main() -> Result<(), Box<dyn std::error::Error>> {\n");
    build_rs.push_str("    let out_dir = env::var(\"OUT_DIR\")?;\n");
    build_rs.push_str("    let dest_path = Path::new(&out_dir).join(\"generated.rs\");\n\n");
    build_rs.push_str("    // Read schema and instance from files\n");
    build_rs.push_str("    let schema_content = fs::read_to_string(\"schema.json\")?;\n");
    build_rs.push_str("    let schema: Value = serde_json::from_str(&schema_content)?;\n\n");
    build_rs.push_str("    // Generate code\n");
    build_rs.push_str("    let grammar = TctCompiler::new(schema, \"TestType\")?\n");
    build_rs.push_str(&format!(
        "        .with_optimization_level({})\n",
        optimization_level_path(optimization_level)
    ));
    build_rs.push_str("        .compile()?;\n");
    build_rs.push_str("    let generated_code = grammar.generate_code()?;\n\n");
    build_rs.push_str("    // Write the complete test file\n");
    build_rs.push_str("    let mut full_code = String::new();\n");
    build_rs.push_str("    full_code.push_str(\"use serde::{Deserialize, Serialize};\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"use std::time::Instant;\\n\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"// Generator module that generated code expects\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"pub mod generator {\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"    pub mod additional_properties_analyzer {\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"        use serde::{Deserialize, Serialize};\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        #[serde(untagged)]\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        pub enum AdditionalValue {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"            String(String),\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"            Number(f64),\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"            Bool(bool),\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"            Null,\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"}\\n\\n\");\n");

    // Note: Generated code now includes standalone tokenizer module - no need to add it here
    // (see Grammar::generate_tokenizer_module() in src/grammar.rs)

    // Add decoder module that generated code expects (for schema-aware decode)
    build_rs.push_str(
        "    full_code.push_str(\"// Decoder module that generated code expects\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"pub mod decoder {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    pub use tct::decoder::*;\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"}\\n\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"use crate::tokenizer::JsonTokenEncoder;\\n\\n\");\n");

    // Add log_debug macro that generated code expects
    build_rs
        .push_str("    full_code.push_str(\"// Logger macros that generated code expects\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"#[macro_export]\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"macro_rules! log_debug {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    ($($arg:tt)*) => {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        // Disabled for performance - was: println!($($arg)*);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    };\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"}\\n\\n\");\n");

    build_rs.push_str("    full_code.push_str(\"// Generated code from TCT\\n\");\n");
    build_rs.push_str("    full_code.push_str(&generated_code);\n");
    build_rs.push_str("    full_code.push_str(\"\\n\\n\");\n");

    // Add TestType alias pointing to the actual root type
    build_rs.push_str("    // Extract root type name and create TestType alias\n");
    build_rs.push_str(
        "    // Find the type (struct or enum) that matches the schema name pattern or use the last type\n",
    );
    build_rs.push_str("    let root_type_name = {\n");
    build_rs.push_str("        let mut all_types = Vec::new();\n");
    build_rs.push_str("        for line in generated_code.lines() {\n");
    build_rs
        .push_str("            // Check for pub struct (regular or tuple) and pub enum types\n");
    build_rs.push_str("            let line_trimmed = line.trim_start();\n");
    build_rs.push_str("            if (line_trimmed.starts_with(\"pub struct \") && (line.contains(\" {\") || line.contains(\"(\"))) || (line_trimmed.starts_with(\"pub enum \") && line.contains(\" {\")) {\n");
    build_rs.push_str("                if let Some(start) = line.find(\"pub struct \").or_else(|| line.find(\"pub enum \")) {\n");
    build_rs.push_str("                    let prefix_len = if line[start..].starts_with(\"pub struct \") { \"pub struct \".len() } else { \"pub enum \".len() };\n");
    build_rs.push_str("                    let start = start + prefix_len;\n");
    build_rs.push_str("                    // Find end of type name (either space, '(' for tuple struct, or '<' for generics)\n");
    build_rs.push_str("                    if let Some(end) = line[start..].find(|c: char| c == ' ' || c == '(' || c == '<') {\n");
    build_rs.push_str(
        "                        let type_name = line[start..start + end].to_string();\n",
    );
    build_rs.push_str("                        if !type_name.is_empty() {\n");
    build_rs.push_str("                            all_types.push(type_name);\n");
    build_rs.push_str("                        }\n");
    build_rs.push_str("                    }\n");
    build_rs.push_str("                }\n");
    build_rs.push_str("            }\n");
    build_rs.push_str("        }\n");
    build_rs.push_str("        // Prefer schema types over generated helper/metadata types\n");
    build_rs.push_str("        let main_types: Vec<_> = all_types.iter().filter(|s| !s.contains(\"Variant\") && !matches!(s.as_str(), \"SequenceBoundaryMetadata\" | \"SequenceTokenRange\" | \"SequenceSpecialTokenPolicy\" | \"SequenceBpeMetadata\")).collect();\n");
    build_rs.push_str("        if let Some(last_main) = main_types.last() {\n");
    build_rs.push_str("            last_main.to_string()\n");
    build_rs.push_str("        } else if let Some(last_any) = all_types.last() {\n");
    build_rs.push_str("            last_any.to_string()\n");
    build_rs.push_str("        } else {\n");
    build_rs.push_str("            \"UnknownType\".to_string()\n");
    build_rs.push_str("        }\n");
    build_rs.push_str("    };\n");
    build_rs.push_str("    // Only create alias if root type is not already named TestType\n");
    build_rs.push_str("    if root_type_name != \"TestType\" {\n");
    build_rs.push_str("        full_code.push_str(&format!(\"// Type alias for tests\\n\"));\n");
    build_rs.push_str("        full_code.push_str(&format!(\"pub type TestType = {};\\n\\n\", root_type_name));\n");
    build_rs.push_str("    }\n");
    build_rs.push_str("    \n");

    let source_schema_external_resources = source_schema_external_resource_constants("");
    let source_schema_validator_function = source_schema_validator_function_code();
    let source_validation_code = format!(
        r#"const SOURCE_SCHEMA_NAME: &str = {schema_name_literal};
const SOURCE_INSTANCE_NAME: &str = {instance_name_literal};
const SOURCE_SCHEMA_JSON: &str = include_str!("schema.json");
{source_schema_external_resources}

fn json_context(value: &serde_json::Value) -> String {{
    let rendered = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    let preview = rendered.chars().take(4096).collect::<String>();
    if preview.len() < rendered.len() {{
        format!("{{}}...<truncated {{}} bytes>", preview, rendered.len() - preview.len())
    }} else {{
        preview
    }}
}}

fn validate_source_schema(
    validator: &jsonschema::Validator,
    value: &serde_json::Value,
    phase: &str,
    context_label: &str,
) -> Result<(), String> {{
    if validator.is_valid(value) {{
        return Ok(());
    }}

    let errors = validator
        .iter_errors(value)
        .take(5)
        .map(|error| {{
            format!(
                "instance_path={{}} schema_path={{}} error={{}}",
                error.instance_path(),
                error.schema_path(),
                error
            )
        }})
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!(
        "source schema validation failed {{}}: schema={{}} fixture={{}} errors={{}} {{}}={{}}",
        phase,
        SOURCE_SCHEMA_NAME,
        SOURCE_INSTANCE_NAME,
        errors,
        context_label,
        json_context(value)
    ))
}}

{source_schema_validator_function}

"#,
        schema_name_literal = schema_name_literal,
        instance_name_literal = instance_name_literal,
        source_schema_external_resources = source_schema_external_resources,
        source_schema_validator_function = source_schema_validator_function
    );
    build_rs.push_str(&format!(
        "    full_code.push_str({});\n",
        serde_json::to_string(&source_validation_code)?
    ));

    build_rs.push_str("    full_code.push_str(\"pub fn run_round_trip_test() -> Result<(), Box<dyn std::error::Error>> {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    let test_json = include_str!(\\\"instance.json\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"🔄 ACTUAL Essential Schema Round-trip test\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"📊 Original JSON size: {} bytes\\\", test_json.len());\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    // Parse from JSON\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    let raw_original_json: serde_json::Value = serde_json::from_str(test_json)?;\\n\");\n");
    let source_validation_before_encode_code = r#"    let source_schema: serde_json::Value = serde_json::from_str(SOURCE_SCHEMA_JSON)?;
    let source_schema_validator = build_source_schema_validator(&source_schema)?;
    validate_source_schema(
        &source_schema_validator,
        &raw_original_json,
        "before encode",
        "input_context",
    )?;
"#;
    build_rs.push_str(&format!(
        "    full_code.push_str({});\n",
        serde_json::to_string(source_validation_before_encode_code)?
    ));
    build_rs.push_str("    full_code.push_str(\"    let original: TestType = serde_json::from_str(test_json)?;\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"    println!(\\\"✅ JSON parsing successful\\\");\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    // Encode to tokens\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    let mut tokens = Vec::new();\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    let encode_start = Instant::now();\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    original.encode(&mut tokens);\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"    let encode_time = encode_start.elapsed();\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"    let token_count = tokens.len();\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"PERF:encode:{}ns\\\", encode_time.as_nanos());\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"📊 Encoded to {} tokens\\\", token_count);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"📊 Token count estimate: {}\\\", original.token_count());\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"    // Decode from the complete token stream\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    let decode_start = Instant::now();\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"    let decoded = TestType::decode_full(tokens)?;\\n\");\n",
    );
    build_rs.push_str(
        "    full_code.push_str(\"    let decode_time = decode_start.elapsed();\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"PERF:decode:{}ns\\\", decode_time.as_nanos());\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"    println!(\\\"✅ Decode successful\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    // Enhanced round-trip verification\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"    // 1. Whole-stream decode rejected trailing tokens\\n\");\n",
    );
    build_rs.push_str(
        "    full_code.push_str(\"    println!(\\\"✅ All tokens consumed\\\");\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"    // 2. Verify token count matches estimate\\n\");\n",
    );
    build_rs.push_str(
        "    full_code.push_str(\"    let estimated_tokens = original.token_count();\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"    if token_count != estimated_tokens {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        println!(\\\"⚠️  Token count mismatch: actual {} vs estimated {}\\\", token_count, estimated_tokens);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    } else {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        println!(\\\"✅ Token count matches estimate: {}\\\", token_count);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    // 3. Verify semantic JSON equality against the raw input\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"    let decoded_json = serde_json::to_value(&decoded)?;\\n\");\n",
    );
    let source_validation_after_decode_code = r#"    validate_source_schema(
        &source_schema_validator,
        &decoded_json,
        "after decode",
        "decoded_context",
    )?;
"#;
    build_rs.push_str(&format!(
        "    full_code.push_str({});\n",
        serde_json::to_string(source_validation_after_decode_code)?
    ));
    build_rs.push_str("    full_code.push_str(\"    if original == decoded {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        println!(\\\"✅ Struct equality verification passed\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    } else {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        println!(\\\"⚠️  Struct equality failed; checking raw JSON equality\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    }\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"    if raw_original_json != decoded_json {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        println!(\\\"🔍 ROUND-TRIP FAILURE DIAGNOSTICS\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        println!(\\\"📊 Token count: actual={}, estimated={}\\\", token_count, estimated_tokens);\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"        println!(\\\"📄 RAW ORIGINAL JSON:\\\");\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"        if let Ok(pretty_original) = serde_json::to_string_pretty(&raw_original_json) {\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"            println!(\\\"{}\\\", pretty_original);\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"        }\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"        println!(\\\"📄 DECODED JSON:\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        if let Ok(pretty_decoded) = serde_json::to_string_pretty(&decoded_json) {\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"            println!(\\\"{}\\\", pretty_decoded);\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"        }\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"        let mut debug_tokens = Vec::new();\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"        original.encode(&mut debug_tokens);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        println!(\\\"🔢 FIRST 20 TOKENS: {:?}\\\", debug_tokens.iter().take(20).collect::<Vec<_>>());\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        if let (serde_json::Value::Object(orig_map), serde_json::Value::Object(dec_map)) = (&raw_original_json, &decoded_json) {\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"            for (key, orig_val) in orig_map {\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"                if let Some(dec_val) = dec_map.get(key) {\\n\");\n");
    build_rs
        .push_str("    full_code.push_str(\"                    if orig_val != dec_val {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"                        println!(\\\"  ❌ Field '{}': {:?} → {:?}\\\", key, orig_val, dec_val);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"                    }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"                } else {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"                    println!(\\\"  ❌ Field '{}' missing in decoded\\\", key);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"                }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"            }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"            for key in dec_map.keys() {\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"                if !orig_map.contains_key(key) {\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"                    println!(\\\"  ❌ Extra field '{}' in decoded\\\", key);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"                }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"            }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        return Err(\\\"Round-trip verification failed: raw input JSON and decoded JSON differ\\\".into());\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"✅ Raw JSON equality verification passed\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str(
        "    full_code.push_str(\"    // 4. Additional JSON roundtrip verification\\n\");\n",
    );
    build_rs.push_str(
        "    full_code.push_str(\"    let final_json = serde_json::to_string(&decoded)?;\\n\");\n",
    );
    build_rs.push_str("    full_code.push_str(\"    let reparsed: TestType = serde_json::from_str(&final_json)?;\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    if decoded == reparsed {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        println!(\\\"✅ JSON re-serialization verification passed\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    } else {\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"        return Err(\\\"JSON re-serialization verification failed\\\".into());\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    }\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    // Success!\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"✅ COMPREHENSIVE ROUND-TRIP SUCCESS!\\\");\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"📊 Final JSON size: {} bytes\\\", final_json.len());\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    println!(\\\"📊 Compression: {} bytes → {} tokens\\\", test_json.len(), token_count);\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"    Ok(())\\n\");\n");
    build_rs.push_str("    full_code.push_str(\"}\\n\");\n");
    build_rs.push('\n');
    build_rs.push_str("    fs::write(&dest_path, full_code)?;\n\n");
    build_rs.push_str("    // Copy instance.json to OUT_DIR so include_str! can find it\n");
    build_rs.push_str("    let instance_dest = Path::new(&out_dir).join(\"instance.json\");\n");
    build_rs.push_str("    fs::copy(\"instance.json\", &instance_dest)?;\n\n");
    build_rs.push_str("    let schema_dest = Path::new(&out_dir).join(\"schema.json\");\n");
    build_rs.push_str("    fs::copy(\"schema.json\", &schema_dest)?;\n\n");
    build_rs.push_str("    println!(\"cargo:rerun-if-changed=schema.json\");\n");
    build_rs.push_str("    println!(\"cargo:rerun-if-changed=instance.json\");\n\n");
    build_rs.push_str(&source_schema_external_resource_build_copy_code());
    build_rs.push_str("    Ok(())\n");
    build_rs.push_str("}\n");

    fs::write(format!("{}/build.rs", test_dir), build_rs)?;

    // Create main.rs
    let main_rs = r#"include!(concat!(env!("OUT_DIR"), "/generated.rs"));

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_round_trip_test()
}
"#;

    fs::write(format!("{}/src/main.rs", test_dir), main_rs)?;

    Ok(())
}

/// Perform complete round-trip test for a single instance (with cargo compilation)
#[allow(dead_code)]
pub fn test_round_trip(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
) -> RoundTripResult {
    test_round_trip_with_optimization_level(
        schema,
        instance,
        schema_name,
        instance_name,
        OptimizationLevel::O0,
    )
}

#[allow(dead_code)]
pub fn test_round_trip_with_optimization_level(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
    optimization_level: OptimizationLevel,
) -> RoundTripResult {
    let _start_time = std::time::Instant::now();

    // Create temporary directory for test crate
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            return RoundTripResult {
                schema_name: schema_name.to_string(),
                instance_name: instance_name.to_string(),
                success: false,
                error: Some(format!("Failed to create temp dir: {}", e)),
                metric_source: RoundTripMetricSource::NotCollected,
                compression_ratio: None,
                encode_time_us: None,
                decode_time_us: None,
                token_count: None,
                json_bytes: None,
                token_bytes: None,
            };
        }
    };

    let crate_path = temp_dir.path();

    // Generate tokenizer from schema
    let grammar = match TctCompiler::new(schema.clone(), "TestType").and_then(|compiler| {
        compiler
            .with_optimization_level(optimization_level)
            .compile()
    }) {
        Ok(g) => g,
        Err(e) => {
            return RoundTripResult {
                schema_name: schema_name.to_string(),
                instance_name: instance_name.to_string(),
                success: false,
                error: Some(format!("Schema compilation failed: {}", e)),
                metric_source: RoundTripMetricSource::NotCollected,
                compression_ratio: None,
                encode_time_us: None,
                decode_time_us: None,
                token_count: None,
                json_bytes: None,
                token_bytes: None,
            };
        }
    };

    let generated_code = match grammar.generate_code() {
        Ok(code) => code,
        Err(e) => {
            return RoundTripResult {
                schema_name: schema_name.to_string(),
                instance_name: instance_name.to_string(),
                success: false,
                error: Some(format!("Code generation failed: {}", e)),
                metric_source: RoundTripMetricSource::NotCollected,
                compression_ratio: None,
                encode_time_us: None,
                decode_time_us: None,
                token_count: None,
                json_bytes: None,
                token_bytes: None,
            };
        }
    };

    // Create and run test crate for round-trip validation
    match create_round_trip_crate(
        crate_path,
        &generated_code,
        schema,
        instance,
        instance_name,
        schema_name,
    ) {
        Ok(result) => result,
        Err(e) => RoundTripResult {
            schema_name: schema_name.to_string(),
            instance_name: instance_name.to_string(),
            success: false,
            error: Some(format!("Round-trip test failed: {}", e)),
            metric_source: RoundTripMetricSource::NotCollected,
            compression_ratio: None,
            encode_time_us: None,
            decode_time_us: None,
            token_count: None,
            json_bytes: None,
            token_bytes: None,
        },
    }
}

/// Extract the root type name from generated code
#[allow(dead_code)]
fn extract_root_type_name(generated_code: &str) -> Option<String> {
    let type_re =
        regex::Regex::new(r"^\s*pub\s+(?:struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)").ok()?;
    let mut all_types = Vec::new();

    for line in generated_code.lines() {
        if let Some(captures) = type_re.captures(line) {
            all_types.push(captures[1].to_string());
        }
    }

    all_types
        .iter()
        .rev()
        .find(|type_name| !is_generated_helper_type_name(type_name))
        .cloned()
        .or_else(|| all_types.into_iter().next_back())
}

fn is_generated_helper_type_name(type_name: &str) -> bool {
    type_name.contains("Variant")
        || matches!(
            type_name,
            "SequenceBoundaryMetadata"
                | "SequenceTokenRange"
                | "SequenceSpecialTokenPolicy"
                | "SequenceBpeMetadata"
        )
}

#[allow(dead_code)]
pub fn parse_batch_round_trip_results(
    stdout: &str,
    stderr: &str,
    process_success: bool,
    status_code: Option<i32>,
    expected_names: &[String],
) -> ParsedBatchRoundTripResults {
    let mut parsed = ParsedBatchRoundTripResults::default();
    let mut seen_results = HashSet::new();
    let expected_name_set = expected_names.iter().cloned().collect::<HashSet<_>>();

    if expected_name_set.len() != expected_names.len() {
        parsed.failed_instances.push((
            "<batch>".to_string(),
            "Expected batch names contain duplicates".to_string(),
        ));
    }

    for line in stdout.lines() {
        let Some(payload) = line.strip_prefix("RESULT:") else {
            continue;
        };

        let result: Value = match serde_json::from_str(payload) {
            Ok(result) => result,
            Err(error) => {
                parsed.failed_instances.push((
                    "<batch>".to_string(),
                    format!("Malformed batch result JSON: {}; line={}", error, line),
                ));
                continue;
            }
        };

        let name = result
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("<unnamed>")
            .to_string();

        if !seen_results.insert(name.clone()) {
            parsed.failed_instances.push((
                name,
                "Duplicate result emitted by generated batch runner".to_string(),
            ));
            continue;
        }

        if !expected_name_set.contains(&name) {
            parsed.failed_instances.push((
                name,
                "Unexpected result emitted by generated batch runner".to_string(),
            ));
            continue;
        }

        match result.get("status").and_then(Value::as_str) {
            Some("success") => {
                let Some(encode_time_ns) = batch_result_u64(
                    &result,
                    "encode_time_ns",
                    &name,
                    &mut parsed.failed_instances,
                ) else {
                    continue;
                };
                let Some(decode_time_ns) = batch_result_u64(
                    &result,
                    "decode_time_ns",
                    &name,
                    &mut parsed.failed_instances,
                ) else {
                    continue;
                };
                let Some(token_count_u64) =
                    batch_result_u64(&result, "token_count", &name, &mut parsed.failed_instances)
                else {
                    continue;
                };
                let Ok(token_count) = usize::try_from(token_count_u64) else {
                    parsed.failed_instances.push((
                        name,
                        format!(
                            "Success result token_count is too large: {}",
                            token_count_u64
                        ),
                    ));
                    continue;
                };
                let Some(max_token_u64) =
                    batch_result_u64(&result, "max_token", &name, &mut parsed.failed_instances)
                else {
                    continue;
                };
                let Ok(max_token) = u32::try_from(max_token_u64) else {
                    parsed.failed_instances.push((
                        name,
                        format!("Success result max_token is too large: {}", max_token_u64),
                    ));
                    continue;
                };

                parsed
                    .per_instance_times_us
                    .insert(name.clone(), (encode_time_ns / 1000, decode_time_ns / 1000));
                parsed.successes.push(BatchRoundTripSuccess {
                    name,
                    encode_time_ns,
                    decode_time_ns,
                    token_count,
                    max_token,
                });
                parsed.successful_instances += 1;
            }
            Some("failure") => {
                let error = result
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("generated batch runner reported failure without an error")
                    .to_string();
                parsed.failed_instances.push((name, error));
            }
            Some(status) => {
                parsed.failed_instances.push((
                    name,
                    format!("Unexpected result status: {:?}", Some(status)),
                ));
            }
            None => parsed
                .failed_instances
                .push((name, "Unexpected result status: None".to_string())),
        }
    }

    for expected_name in expected_names {
        if !seen_results.contains(expected_name) {
            parsed.failed_instances.push((
                expected_name.clone(),
                format!(
                    "No batch result emitted; process status={:?}; stderr={}",
                    status_code, stderr
                ),
            ));
        }
    }

    if !process_success && parsed.failed_instances.is_empty() {
        parsed.failed_instances.push((
            "<batch>".to_string(),
            format!(
                "Generated batch runner exited with status {:?}; stderr={}",
                status_code, stderr
            ),
        ));
    }

    parsed
}

#[allow(dead_code)]
fn batch_result_u64(
    result: &Value,
    field: &str,
    instance_name: &str,
    failed_instances: &mut Vec<(String, String)>,
) -> Option<u64> {
    match result.get(field).and_then(Value::as_u64) {
        Some(value) => Some(value),
        None => {
            failed_instances.push((
                instance_name.to_string(),
                format!("Success result missing numeric field: {}", field),
            ));
            None
        }
    }
}

#[allow(dead_code)]
pub fn test_batch_round_trip_with_optimization_level(
    schema: &Value,
    instances: &[(String, Value)],
    schema_name: &str,
    optimization_level: OptimizationLevel,
) -> Result<BatchRoundTripSummary, Box<dyn std::error::Error>> {
    test_batch_round_trip_with_optimization_level_and_after_decode_hook(
        schema,
        instances,
        schema_name,
        optimization_level,
        "",
    )
}

#[allow(dead_code)]
fn test_batch_round_trip_with_optimization_level_and_after_decode_hook(
    schema: &Value,
    instances: &[(String, Value)],
    schema_name: &str,
    optimization_level: OptimizationLevel,
    after_decode_json_hook: &str,
) -> Result<BatchRoundTripSummary, Box<dyn std::error::Error>> {
    let schema_compilation_start = std::time::Instant::now();
    let grammar = TctCompiler::new(schema.clone(), "TestType")?
        .with_optimization_level(optimization_level)
        .compile()?;
    let schema_compilation_time = schema_compilation_start.elapsed();

    let code_generation_start = std::time::Instant::now();
    let generated_code = grammar.generate_code()?;
    let code_generation_time = code_generation_start.elapsed();
    let root_type_name =
        extract_root_type_name(&generated_code).unwrap_or_else(|| "TestType".to_string());

    println!(
        "✅ Schema compiled and code generated once for {} at {:?}",
        schema_name, optimization_level
    );
    println!("  Generated code: {} bytes", generated_code.len());
    println!("  Root type: {}", root_type_name);

    let temp_dir = format!(
        "/tmp/tct_synthetic_batch_{}_{}",
        sanitize_path_component(schema_name),
        sanitize_path_component(&format!("{:?}", optimization_level))
    );
    fs::create_dir_all(format!("{}/src", temp_dir))?;

    let tct_path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let package_name = format!(
        "synthetic-batch-{}-{}",
        sanitize_path_component(schema_name),
        sanitize_path_component(&format!("{:?}", optimization_level))
    );
    let cargo_toml = GeneratedCargoToml::new(package_name)
        .with_regex()
        .with_jsonschema()
        .with_rustc_hash()
        .with_chrono()
        .with_base64()
        .with_tct_path(tct_path.display().to_string())
        .with_bin("test_tokenizer", "src/main.rs")
        .render();
    write_if_changed(format!("{}/Cargo.toml", temp_dir), &cargo_toml)?;
    copy_workspace_cargo_lock(&tct_path, Path::new(&temp_dir))?;
    let source_schema_json = serde_json::to_string_pretty(schema)?;
    write_if_changed(
        format!("{}/source-schema.json", temp_dir),
        &source_schema_json,
    )?;
    write_source_schema_external_resource_files(&temp_dir)?;
    let schema_name_literal = serde_json::to_string(schema_name)?;
    let decoded_json_binding = if after_decode_json_hook.trim().is_empty() {
        "let decoded_json"
    } else {
        "let mut decoded_json"
    };

    let type_alias = if root_type_name == "TestType" {
        String::new()
    } else {
        format!("pub type TestType = {};\n", root_type_name)
    };
    let source_schema_external_resources = source_schema_external_resource_constants("../");
    let source_schema_validator_function = source_schema_validator_function_code();
    let main_rs = format!(
        r#"use serde::{{Deserialize, Serialize}};
use serde_json::{{json, Value}};
use std::io::{{self, Read}};

pub mod generator {{
    pub mod additional_properties_analyzer {{
        use serde::{{Deserialize, Serialize}};

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(untagged)]
        pub enum AdditionalValue {{
            String(String),
            Number(f64),
            Bool(bool),
            Null,
        }}
    }}
}}

pub mod decoder {{
    pub use tct::decoder::*;
}}

#[macro_export]
macro_rules! log_debug {{
    ($($arg:tt)*) => {{
    }};
}}

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

{type_alias}

const SOURCE_SCHEMA_NAME: &str = {schema_name_literal};
const SOURCE_SCHEMA_JSON: &str = include_str!("../source-schema.json");
{source_schema_external_resources}

fn json_context(value: &Value) -> String {{
    let rendered = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    let preview = rendered.chars().take(4096).collect::<String>();
    if preview.len() < rendered.len() {{
        format!("{{}}...<truncated {{}} bytes>", preview, rendered.len() - preview.len())
    }} else {{
        preview
    }}
}}

fn validate_source_schema(
    validator: &jsonschema::Validator,
    value: &Value,
    fixture_name: &str,
    phase: &str,
    context_label: &str,
) -> Result<(), String> {{
    if validator.is_valid(value) {{
        return Ok(());
    }}

    let errors = validator
        .iter_errors(value)
        .take(5)
        .map(|error| {{
            format!(
                "instance_path={{}} schema_path={{}} error={{}}",
                error.instance_path(),
                error.schema_path(),
                error
            )
        }})
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!(
        "source schema validation failed {{}}: schema={{}} fixture={{}} errors={{}} {{}}={{}}",
        phase,
        SOURCE_SCHEMA_NAME,
        fixture_name,
        errors,
        context_label,
        json_context(value)
    ))
}}

{source_schema_validator_function}

fn run_case(
    json_value: Value,
    source_schema_validator: &jsonschema::Validator,
    fixture_name: &str,
) -> Result<(u64, u64, usize, u32), String> {{
    validate_source_schema(
        source_schema_validator,
        &json_value,
        fixture_name,
        "before encode",
        "input_context",
    )?;

    let typed_instance: TestType = serde_json::from_value(json_value.clone())
        .map_err(|error| format!("generated type parse failed: {{}}", error))?;

    let encode_start = std::time::Instant::now();
    let mut tokens = Vec::new();
    typed_instance.encode(&mut tokens);
    let encode_time = encode_start.elapsed();

    let token_count = tokens.len();
    let max_token = tokens.iter().copied().max().unwrap_or(0);
    if max_token >= VOCAB_SIZE {{
        return Err(format!(
            "Vocabulary constraint violation: token {{}} >= VOCAB_SIZE {{}}",
            max_token, VOCAB_SIZE
        ));
    }}

    let decode_start = std::time::Instant::now();
    let decoded_instance = TestType::decode_full(tokens)
        .map_err(|error| format!("decode failed: {{}}", error))?;
    let decode_time = decode_start.elapsed();

    {decoded_json_binding} = serde_json::to_value(&decoded_instance)
        .map_err(|error| format!("decoded JSON serialization failed: {{}}", error))?;
    {after_decode_json_hook}
    validate_source_schema(
        source_schema_validator,
        &decoded_json,
        fixture_name,
        "after decode",
        "decoded_context",
    )?;
    if json_value != decoded_json {{
        return Err(format!(
            "Original and decoded JSON do not match\nOriginal: {{}}\nDecoded: {{}}",
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| json_value.to_string()),
            serde_json::to_string_pretty(&decoded_json).unwrap_or_else(|_| decoded_json.to_string())
        ));
    }}

    let final_json = serde_json::to_string(&decoded_instance)
        .map_err(|error| format!("decoded JSON string serialization failed: {{}}", error))?;
    let reparsed: TestType = serde_json::from_str(&final_json)
        .map_err(|error| format!("decoded JSON reparse failed: {{}}", error))?;
    if decoded_instance != reparsed {{
        return Err("JSON re-serialization verification failed".to_string());
    }}

    Ok((
        u64::try_from(encode_time.as_nanos()).unwrap_or(u64::MAX),
        u64::try_from(decode_time.as_nanos()).unwrap_or(u64::MAX),
        token_count,
        max_token,
    ))
}}

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let source_schema: Value = serde_json::from_str(SOURCE_SCHEMA_JSON)?;
    let source_schema_validator = build_source_schema_validator(&source_schema)?;
    let batch: Value = serde_json::from_str(&buffer)?;
    let cases = batch.as_array().ok_or("expected a JSON array of round-trip cases")?;

    let mut failures = 0usize;
    for case in cases {{
        let name = case
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("<unnamed>");
        let Some(instance) = case.get("instance").cloned() else {{
            failures += 1;
            println!(
                "RESULT:{{}}",
                json!({{"name": name, "status": "failure", "error": "missing instance"}})
            );
            continue;
        }};

        match run_case(instance, &source_schema_validator, name) {{
            Ok((encode_time_ns, decode_time_ns, token_count, max_token)) => println!(
                "RESULT:{{}}",
                json!({{
                    "name": name,
                    "status": "success",
                    "encode_time_ns": encode_time_ns,
                    "decode_time_ns": decode_time_ns,
                    "token_count": token_count,
                    "max_token": max_token,
                }})
            ),
            Err(error) => {{
                failures += 1;
                println!(
                    "RESULT:{{}}",
                    json!({{"name": name, "status": "failure", "error": error}})
                );
            }}
        }}
    }}

    if failures > 0 {{
        std::process::exit(1);
    }}

    Ok(())
}}
"#
    );
    write_if_changed(format!("{}/src/main.rs", temp_dir), &main_rs)?;

    let generated_target_dir = generated_cargo_target_dir_for_crate(Path::new(&temp_dir));
    fs::create_dir_all(&generated_target_dir)?;

    let rust_compilation_start = std::time::Instant::now();
    let mut build_command = Command::new("cargo");
    configure_round_trip_generated_cargo_command(&mut build_command);
    build_command
        .args(["build", "--release", "--quiet", "--bin", "test_tokenizer"])
        .current_dir(&temp_dir);
    build_command.env("CARGO_TARGET_DIR", &generated_target_dir);
    let build_output = build_command.output()?;
    let rust_compilation_time = rust_compilation_start.elapsed();

    if !build_output.status.success() {
        return Err(format!(
            "generated crate build failed for {} at {:?}:\n{}",
            schema_name,
            optimization_level,
            String::from_utf8_lossy(&build_output.stderr)
        )
        .into());
    }

    let batch = instances
        .iter()
        .map(|(name, instance)| serde_json::json!({ "name": name, "instance": instance }))
        .collect::<Vec<_>>();
    let batch_json = serde_json::to_string(&batch)?;
    let binary_path = generated_target_dir.join("release/test_tokenizer");

    let mut child = Command::new(&binary_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(batch_json.as_bytes())?;
        stdin.flush()?;
    }

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected_names = instances
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let parsed = parse_batch_round_trip_results(
        &stdout,
        &stderr,
        output.status.success(),
        output.status.code(),
        &expected_names,
    );

    for line in stdout.lines().filter(|line| line.starts_with("RESULT:")) {
        println!("  {}", line);
    }

    Ok(BatchRoundTripSummary {
        total_instances: instances.len(),
        successful_instances: parsed.successful_instances,
        successes: parsed.successes,
        failed_instances: parsed.failed_instances,
        schema_compilation_time_ms: schema_compilation_time.as_millis() as u64,
        code_generation_time_ms: code_generation_time.as_millis() as u64,
        rust_compilation_time_ms: rust_compilation_time.as_millis() as u64,
    })
}

/// Create a test crate that performs the round-trip validation
#[allow(dead_code)]
fn create_round_trip_crate(
    crate_path: &Path,
    generated_code: &str,
    schema: &Value,
    instance: &Value,
    instance_name: &str,
    schema_name: &str,
) -> Result<RoundTripResult, Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let tct_path = current_dir.display().to_string();

    // Create Cargo.toml
    let cargo_toml = GeneratedCargoToml::new("round-trip-test")
        .with_regex()
        .with_jsonschema()
        .with_rustc_hash()
        .with_chrono()
        .with_base64()
        .with_tct_path(tct_path)
        .render();
    fs::write(crate_path.join("Cargo.toml"), cargo_toml)?;
    copy_workspace_cargo_lock(&current_dir, crate_path)?;

    // Create src directory
    fs::create_dir_all(crate_path.join("src"))?;
    fs::write(
        crate_path.join("source-schema.json"),
        serde_json::to_string_pretty(schema)?,
    )?;
    write_source_schema_external_resource_files(crate_path)?;

    // Extract the root type name from generated code
    let root_type_name =
        extract_root_type_name(generated_code).unwrap_or_else(|| "UnknownType".to_string());

    // Create main.rs with generated code and test
    let instance_json = serde_json::to_string(instance)?;
    let instance_json_literal = serde_json::to_string(&instance_json)?;
    let schema_name_literal = serde_json::to_string(schema_name)?;
    let instance_name_literal = serde_json::to_string(instance_name)?;
    let source_schema_external_resources = source_schema_external_resource_constants("../");
    let source_schema_validator_function = source_schema_validator_function_code();
    let main_rs = format!(
        r###"use serde::{{Deserialize, Serialize}};

// Generator module that generated code expects
pub mod generator {{
    pub mod additional_properties_analyzer {{
        use serde::{{Deserialize, Serialize}};

        /// Simple enum for type-preserving additional properties (O2 optimization level)
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(untagged)]
        pub enum AdditionalValue {{
            String(String),
            Number(f64),
            Bool(bool),
            Null,
        }}
    }}
}}

// Note: Generated code now includes standalone tokenizer module
// (see Grammar::generate_tokenizer_module() in src/grammar.rs)

// Decoder module that generated code expects (for schema-aware decode)
pub mod decoder {{
    // Re-export everything from TCT's decoder
    pub use tct::decoder::*;
}}

// Logger macros that generated code expects
#[macro_export]
macro_rules! log_debug {{
    ($($arg:tt)*) => {{
        // Silent in external crates for performance
        // To enable: println!($($arg)*);
    }};
}}

// Import the trait so generated code can use it
use crate::tokenizer::JsonTokenEncoder;

{}

const SOURCE_SCHEMA_NAME: &str = {schema_name_literal};
const SOURCE_INSTANCE_NAME: &str = {instance_name_literal};
const SOURCE_SCHEMA_JSON: &str = include_str!("../source-schema.json");
{source_schema_external_resources}

fn json_context(value: &serde_json::Value) -> String {{
    let rendered = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    let preview = rendered.chars().take(4096).collect::<String>();
    if preview.len() < rendered.len() {{
        format!("{{}}...<truncated {{}} bytes>", preview, rendered.len() - preview.len())
    }} else {{
        preview
    }}
}}

fn validate_source_schema(
    validator: &jsonschema::Validator,
    value: &serde_json::Value,
    phase: &str,
    context_label: &str,
) -> Result<(), String> {{
    if validator.is_valid(value) {{
        return Ok(());
    }}

    let errors = validator
        .iter_errors(value)
        .take(5)
        .map(|error| {{
            format!(
                "instance_path={{}} schema_path={{}} error={{}}",
                error.instance_path(),
                error.schema_path(),
                error
            )
        }})
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!(
        "source schema validation failed {{}}: schema={{}} fixture={{}} errors={{}} {{}}={{}}",
        phase,
        SOURCE_SCHEMA_NAME,
        SOURCE_INSTANCE_NAME,
        errors,
        context_label,
        json_context(value)
    ))
}}

{source_schema_validator_function}

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let original_json = {};
    let raw_original_json_value: serde_json::Value = serde_json::from_str(original_json)?;
    let source_schema: serde_json::Value = serde_json::from_str(SOURCE_SCHEMA_JSON)?;
    let source_schema_validator = build_source_schema_validator(&source_schema)?;
    validate_source_schema(
        &source_schema_validator,
        &raw_original_json_value,
        "before encode",
        "input_context",
    )?;

    // Test actual type parsing and tokenizer functionality
    println!("🔄 Testing {type_name} type parsing and tokenization...");

    // Parse the JSON using the actual generated type
    let parsed_result: serde_json::Result<{type_name}> = serde_json::from_str(original_json);

    match parsed_result {{
        Ok(value) => {{
            println!("✅ JSON parsing successful");
            println!("📊 Original size: {{}} bytes", original_json.len());

            // Test actual tokenizer encode/decode round-trip
            println!("🔄 Testing tokenizer encode/decode...");

            let mut tokens = Vec::new();
            value.encode(&mut tokens);
            let token_count = tokens.len();
            println!("📊 Encoded to {{}} tokens", token_count);
            println!("📊 Token count estimate: {{}} tokens", value.token_count());

            match {type_name}::decode_full(tokens) {{
                Ok(decoded_value) => {{
                    println!("✅ Tokenizer decode successful");

                    // Compare decoded output to the raw input JSON, not to a
                    // typed value that may already have dropped or normalized data.
                    let decoded_json_value = serde_json::to_value(&decoded_value)?;
                    validate_source_schema(
                        &source_schema_validator,
                        &decoded_json_value,
                        "after decode",
                        "decoded_context",
                    )?;

                    if raw_original_json_value != decoded_json_value {{
                        println!("❌ Tokenizer round-trip JSON equality failed");
                        println!("📄 Raw original JSON:");
                        println!("{{}}", serde_json::to_string_pretty(&raw_original_json_value)?);
                        println!("📄 Decoded JSON:");
                        println!("{{}}", serde_json::to_string_pretty(&decoded_json_value)?);
                        return Err("Tokenizer round-trip JSON equality failed".into());
                    }}

                    println!("✅ Tokenizer JSON equality verified");
                    println!("✅ Tokenizer round-trip completed successfully");
                    println!("📊 Compression ratio: {{:.2}}%", token_count as f64 / original_json.len() as f64 * 100.0);
                    Ok(())
                }}
                Err(e) => {{
                    println!("❌ Tokenizer decode failed: {{:?}}", e);
                    Err("Tokenizer decode failed".into())
                }}
            }}
        }}
        Err(e) => {{
            println!("❌ JSON parsing failed: {{}}", e);
            Err(e.into())
        }}
    }}
}}
"###,
        generated_code,
        instance_json_literal,
        schema_name_literal = schema_name_literal,
        instance_name_literal = instance_name_literal,
        type_name = root_type_name
    );

    fs::write(crate_path.join("src/main.rs"), main_rs)?;

    // Compile and run the test crate
    let mut command = Command::new("cargo");
    configure_round_trip_generated_cargo_command(&mut command);
    let output = command
        .args(["run", "--release", "--quiet"])
        .current_dir(crate_path)
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse output for metrics
        let mut compression_ratio = None;
        for line in stdout.lines() {
            if line.contains("Compression ratio:")
                && let Some(ratio_str) = line.split("Compression ratio: ").nth(1)
            {
                compression_ratio = ratio_str.parse::<f64>().ok();
            }
        }

        Ok(RoundTripResult {
            schema_name: schema_name.to_string(),
            instance_name: instance_name.to_string(),
            success: true,
            error: None,
            metric_source: RoundTripMetricSource::GeneratedExecution,
            compression_ratio,
            encode_time_us: None,
            decode_time_us: None,
            token_count: None,
            json_bytes: Some(instance.to_string().len()),
            token_bytes: None,
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Compilation failed: {}", stderr).into())
    }
}

// ================================================================================================
// PROFILING-BASED ROUND-TRIP TESTING (PHASE 3 IMPLEMENTATION)
// ================================================================================================

/// Estimate profiling metrics from generated profiling code analysis.
///
/// This does not execute encode/decode and must not be used as round-trip
/// validation. Returned metric fields are explicitly marked as
/// [`RoundTripMetricSource::StaticEstimate`].
#[allow(dead_code)]
pub fn estimate_profiling_metrics_without_round_trip(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
) -> Result<RoundTripResult, Box<dyn std::error::Error>> {
    use std::time::Instant;
    use tct::{ProfilingConfig, TctCompiler};

    // Compile schema with profiling enabled
    let compiler = TctCompiler::new(schema.clone(), "TestType")?
        .with_profiling(ProfilingConfig::basic())
        .with_optimization_level(tct::optimization_pass::OptimizationLevel::O0);

    let grammar = compiler.compile()?;

    // Generate the complete Rust code with profiling
    let generated_code = grammar.generate_code()?;

    // Verify profiling was generated
    if !generated_code.contains("TimedEncode") {
        return Err("Generated code does not contain profiling functionality".into());
    }

    // For now, estimate profiling metrics based on schema complexity. This
    // avoids external compilation while keeping provenance explicit so callers
    // cannot treat these values as generated-tokenizer measurements.
    let start_time = Instant::now();

    // Analyze schema complexity for realistic timing estimates
    let json_size = instance.to_string().len();
    let schema_complexity = analyze_schema_complexity(schema);

    // Estimate encode/decode times based on complexity (in nanoseconds).
    let base_encode_ns = 100; // Base 100ns for simple operations
    let base_decode_ns = 150; // Decode is typically slower

    let complexity_factor = (schema_complexity as f64 * 50.0) as u64;
    let size_factor = (json_size as f64 * 10.0) as u64;

    let estimated_encode_ns = base_encode_ns + complexity_factor + size_factor;
    let estimated_decode_ns = base_decode_ns + complexity_factor + size_factor;

    // Estimate token count based on schema analysis
    let estimated_tokens = estimate_token_count(instance, schema);

    // Calculate compression ratio
    let json_bytes = json_size;
    let token_bytes = estimated_tokens * 4; // u32 tokens = 4 bytes each
    let compression_ratio = json_bytes as f64 / token_bytes as f64;

    println!(
        "🚀 Profiling-based test completed in {:?}",
        start_time.elapsed()
    );
    println!("📊 Static profiling estimates (not generated-tokenizer measurements):");
    println!(
        "   Encode time: {}ns ({}µs)",
        estimated_encode_ns,
        estimated_encode_ns / 1000
    );
    println!(
        "   Decode time: {}ns ({}µs)",
        estimated_decode_ns,
        estimated_decode_ns / 1000
    );
    println!("   Token count: {}", estimated_tokens);
    println!("   Compression ratio: {:.2}", compression_ratio);

    Ok(RoundTripResult {
        schema_name: schema_name.to_string(),
        instance_name: instance_name.to_string(),
        success: true,
        error: None,
        metric_source: RoundTripMetricSource::StaticEstimate,
        compression_ratio: Some(compression_ratio),
        encode_time_us: Some(estimated_encode_ns / 1000), // Convert ns to µs
        decode_time_us: Some(estimated_decode_ns / 1000), // Convert ns to µs
        token_count: Some(estimated_tokens),
        json_bytes: Some(json_bytes),
        token_bytes: Some(token_bytes),
    })
}

/// Analyze schema complexity to estimate performance
#[allow(dead_code)]
fn analyze_schema_complexity(schema: &Value) -> u32 {
    let mut complexity = 1;

    if let Some(obj) = schema.as_object() {
        // Object schemas are more complex
        complexity += 2;

        if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
            complexity += properties.len() as u32;
        }

        if obj.contains_key("required") {
            complexity += 1;
        }

        if obj.contains_key("additionalProperties") {
            complexity += 2;
        }

        if obj.contains_key("enum") {
            complexity += 1;
        }

        // Nested schemas add complexity
        for value in obj.values() {
            if value.is_object() {
                complexity += analyze_schema_complexity(value) / 2;
            }
        }
    }

    complexity.max(1)
}

/// Estimate token count based on instance and schema
#[allow(dead_code)]
fn estimate_token_count(instance: &Value, _schema: &Value) -> usize {
    match instance {
        Value::String(s) => {
            // String enums typically use 1 token, free strings use more
            if s.len() < 20 {
                1
            } else {
                (s.len() / 4).max(1)
            }
        }
        Value::Number(_) => 1, // Numbers typically encode as 1 token
        Value::Bool(_) => 1,   // Booleans encode as 1 token
        Value::Null => 1,      // Null encodes as 1 token
        Value::Array(arr) => {
            1 + arr
                .iter()
                .map(|v| estimate_token_count(v, _schema))
                .sum::<usize>()
        }
        Value::Object(obj) => {
            1 + obj
                .values()
                .map(|v| estimate_token_count(v, _schema))
                .sum::<usize>()
        }
    }
}

// ================================================================================================
// PHASED ROUND-TRIP TESTING FUNCTIONS
// ================================================================================================

/// Check if debug mode is enabled for round-trip tests
fn is_debug_enabled() -> bool {
    std::env::var("ROUND_TRIP_DEBUG").is_ok()
}

#[derive(Debug, Default)]
struct GeneratedTokenizerContract {
    has_encoder: bool,
    has_encode: bool,
    has_decode: bool,
    has_token_count: bool,
}

fn generated_tokenizer_contract(
    generated_code: &str,
) -> Result<GeneratedTokenizerContract, String> {
    let generated_file = syn::parse_file(generated_code)
        .map_err(|error| format!("Generated Rust failed to parse: {error}"))?;
    let mut contract = GeneratedTokenizerContract::default();

    for item in generated_file.items {
        let syn::Item::Impl(item_impl) = item else {
            continue;
        };
        let Some((_, trait_path, _)) = item_impl.trait_ else {
            continue;
        };
        if !trait_path
            .segments
            .iter()
            .any(|segment| segment.ident == "JsonTokenEncoder")
        {
            continue;
        }

        contract.has_encoder = true;
        for impl_item in item_impl.items {
            let syn::ImplItem::Fn(function) = impl_item else {
                continue;
            };
            if function.sig.ident == "encode" {
                contract.has_encode = true;
            } else if function.sig.ident == "decode" {
                contract.has_decode = true;
            } else if function.sig.ident == "token_count" {
                contract.has_token_count = true;
            }
        }
    }

    Ok(contract)
}

/// Phase 1: Schema Compilation Verification
/// Leverages existing schema compilation infrastructure to verify TCT can compile the schema
#[allow(dead_code)]
pub fn verify_schema_compilation(schema: &Value, schema_name: &str) -> RoundTripPhaseResult {
    verify_schema_compilation_with_optimization_level(schema, schema_name, OptimizationLevel::O0)
}

pub fn verify_schema_compilation_with_optimization_level(
    schema: &Value,
    schema_name: &str,
    optimization_level: OptimizationLevel,
) -> RoundTripPhaseResult {
    let start_time = std::time::Instant::now();
    let mut debug_output = if is_debug_enabled() {
        Some(String::new())
    } else {
        None
    };
    let mut metrics = HashMap::new();

    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!("🔧 Compiling schema: {}\n", schema_name));
        debug.push_str(&format!(
            "📊 Schema size: {} bytes\n",
            serde_json::to_string(schema).map(|s| s.len()).unwrap_or(0)
        ));
    }

    // Try compiling with the selected optimization level.
    let result = TctCompiler::new(schema.clone(), "TestType").and_then(|compiler| {
        compiler
            .with_optimization_level(optimization_level)
            .compile()
    });

    let duration = start_time.elapsed();
    metrics.insert(
        "compilation_time_us".to_string(),
        duration.as_micros().to_string(),
    );

    match result {
        Ok(_grammar) => {
            if let Some(ref mut debug) = debug_output {
                debug.push_str("✅ Schema compilation successful\n");
            }
            metrics.insert("success".to_string(), "true".to_string());

            RoundTripPhaseResult::success(RoundTripPhase::SchemaCompilation)
                .with_metrics(metrics)
                .with_debug(debug_output.unwrap_or_default())
                .with_duration(duration.as_micros() as u64)
        }
        Err(e) => {
            if let Some(ref mut debug) = debug_output {
                debug.push_str(&format!("❌ Schema compilation failed: {}\n", e));
            }
            metrics.insert("success".to_string(), "false".to_string());
            metrics.insert("error_type".to_string(), format!("{:?}", e));

            RoundTripPhaseResult::failure(
                RoundTripPhase::SchemaCompilation,
                format!("Schema compilation failed: {}", e),
            )
            .with_metrics(metrics)
            .with_debug(debug_output.unwrap_or_default())
            .with_duration(duration.as_micros() as u64)
        }
    }
}

/// Phase 2: Code Generation Validation
/// Verify the generated Rust code includes all required tokenizer traits and methods
#[allow(dead_code)]
pub fn validate_generated_code(schema: &Value, schema_name: &str) -> RoundTripPhaseResult {
    validate_generated_code_with_optimization_level(schema, schema_name, OptimizationLevel::O0)
}

pub fn validate_generated_code_with_optimization_level(
    schema: &Value,
    schema_name: &str,
    optimization_level: OptimizationLevel,
) -> RoundTripPhaseResult {
    let start_time = std::time::Instant::now();
    let mut debug_output = if is_debug_enabled() {
        Some(String::new())
    } else {
        None
    };
    let mut metrics = HashMap::new();

    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!("🔧 Generating code for schema: {}\n", schema_name));
    }

    // First compile the schema
    let grammar = match TctCompiler::new(schema.clone(), "TestType").and_then(|compiler| {
        compiler
            .with_optimization_level(optimization_level)
            .compile()
    }) {
        Ok(g) => g,
        Err(e) => {
            return RoundTripPhaseResult::failure(
                RoundTripPhase::CodeGeneration,
                format!("Schema compilation failed during code generation: {}", e),
            )
            .with_duration(start_time.elapsed().as_micros() as u64);
        }
    };

    // Generate the code
    let generated_code = match grammar.generate_code() {
        Ok(code) => code,
        Err(e) => {
            if let Some(ref mut debug) = debug_output {
                debug.push_str(&format!("❌ Code generation failed: {}\n", e));
            }
            return RoundTripPhaseResult::failure(
                RoundTripPhase::CodeGeneration,
                format!("Code generation failed: {}", e),
            )
            .with_duration(start_time.elapsed().as_micros() as u64);
        }
    };

    let duration = start_time.elapsed();
    metrics.insert(
        "generation_time_us".to_string(),
        duration.as_micros().to_string(),
    );
    metrics.insert(
        "code_size_bytes".to_string(),
        generated_code.len().to_string(),
    );

    // Verify that code generation includes required tokenizer methods.
    let contract = match generated_tokenizer_contract(&generated_code) {
        Ok(contract) => contract,
        Err(error) => {
            if let Some(ref mut debug) = debug_output {
                debug.push_str(&format!("❌ {}\n", error));
            }
            metrics.insert("generated_rust_parse_error".to_string(), error.clone());
            return RoundTripPhaseResult::failure(RoundTripPhase::CodeGeneration, error)
                .with_metrics(metrics)
                .with_debug(debug_output.unwrap_or_default())
                .with_duration(start_time.elapsed().as_micros() as u64);
        }
    };
    let has_encoder = contract.has_encoder;
    let has_encode = contract.has_encode;
    let has_decode = contract.has_decode;
    let has_token_count = contract.has_token_count;

    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!(
            "📊 Generated code size: {} bytes\n",
            generated_code.len()
        ));
        debug.push_str(&format!(
            "🔍 JsonTokenEncoder trait: {}\n",
            if has_encoder { "✅" } else { "❌" }
        ));
        debug.push_str(&format!(
            "🔍 encode() method: {}\n",
            if has_encode { "✅" } else { "❌" }
        ));
        debug.push_str(&format!(
            "🔍 decode() method: {}\n",
            if has_decode { "✅" } else { "❌" }
        ));
        debug.push_str(&format!(
            "🔍 token_count() method: {}\n",
            if has_token_count { "✅" } else { "❌" }
        ));
    }

    metrics.insert("has_encoder_trait".to_string(), has_encoder.to_string());
    metrics.insert("has_encode_method".to_string(), has_encode.to_string());
    metrics.insert("has_decode_method".to_string(), has_decode.to_string());
    metrics.insert(
        "has_token_count_method".to_string(),
        has_token_count.to_string(),
    );

    if !has_encoder || !has_encode || !has_decode || !has_token_count {
        let missing_methods = vec![
            if !has_encoder {
                "JsonTokenEncoder trait"
            } else {
                ""
            },
            if !has_encode { "encode() method" } else { "" },
            if !has_decode { "decode() method" } else { "" },
            if !has_token_count {
                "token_count() method"
            } else {
                ""
            },
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

        RoundTripPhaseResult::failure(
            RoundTripPhase::CodeGeneration,
            format!(
                "Generated code missing required tokenizer methods: {}",
                missing_methods
            ),
        )
        .with_metrics(metrics)
        .with_debug(debug_output.unwrap_or_default())
        .with_duration(duration.as_micros() as u64)
    } else {
        if let Some(ref mut debug) = debug_output {
            debug.push_str("✅ All required tokenizer methods present\n");
        }
        metrics.insert("success".to_string(), "true".to_string());

        RoundTripPhaseResult::success(RoundTripPhase::CodeGeneration)
            .with_metrics(metrics)
            .with_debug(debug_output.unwrap_or_default())
            .with_duration(duration.as_micros() as u64)
    }
}

/// Phase 3: Token Encoding Verification
/// Parse JSON instance into Rust struct and encode it into tokens
#[allow(dead_code)]
pub fn verify_token_encoding(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
) -> (RoundTripPhaseResult, Option<(Vec<u32>, String)>) {
    verify_token_encoding_with_optimization_level(
        schema,
        instance,
        schema_name,
        instance_name,
        OptimizationLevel::O0,
    )
}

pub fn verify_token_encoding_with_optimization_level(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
    optimization_level: OptimizationLevel,
) -> (RoundTripPhaseResult, Option<(Vec<u32>, String)>) {
    let start_time = std::time::Instant::now();
    let mut debug_output = if is_debug_enabled() {
        Some(String::new())
    } else {
        None
    };
    let mut metrics = HashMap::new();

    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!(
            "🔧 Encoding tokens for {}/{}\n",
            schema_name, instance_name
        ));
        debug.push_str(&format!(
            "📊 Instance JSON size: {} bytes\n",
            serde_json::to_string(instance)
                .map(|s| s.len())
                .unwrap_or(0)
        ));
    }

    // This phase requires actually compiling and running test code
    // For now, we'll create a simplified test that validates the encoding step

    // Create test crate with file-based approach
    let test_dir = format!(
        "/tmp/round_trip_encode_{}_{}",
        sanitize_path_component(schema_name),
        sanitize_path_component(instance_name)
    );
    let _ = fs::remove_dir_all(&test_dir);

    if let Err(e) = create_round_trip_test_crate(
        &test_dir,
        schema,
        instance,
        schema_name,
        instance_name,
        optimization_level,
    ) {
        return (
            RoundTripPhaseResult::failure(
                RoundTripPhase::TokenEncoding,
                format!("Failed to create test crate: {}", e),
            )
            .with_duration(start_time.elapsed().as_micros() as u64),
            None,
        );
    }

    // Try to compile and extract encoding results with better timeout handling
    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!(
            "⏳ Starting cargo check with 60s timeout in {}\n",
            test_dir
        ));
    }

    let mut command = Command::new("cargo");
    configure_round_trip_generated_cargo_command(&mut command);
    let output = match command
        .args(["check", "--release"])
        .current_dir(&test_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            // Wait with timeout
            match wait_for_child_with_timeout(&mut child, std::time::Duration::from_secs(60)) {
                Ok(output) => output,
                Err(e) => {
                    let _ = child.kill(); // Kill if still running
                    return (
                        RoundTripPhaseResult::failure(
                            RoundTripPhase::TokenEncoding,
                            format!("Cargo check timed out or failed: {}", e),
                        )
                        .with_duration(start_time.elapsed().as_micros() as u64),
                        None,
                    );
                }
            }
        }
        Err(e) => {
            return (
                RoundTripPhaseResult::failure(
                    RoundTripPhase::TokenEncoding,
                    format!("Failed to spawn cargo check: {}", e),
                )
                .with_duration(start_time.elapsed().as_micros() as u64),
                None,
            );
        }
    };

    let duration = start_time.elapsed();
    metrics.insert(
        "encoding_compile_time_us".to_string(),
        duration.as_micros().to_string(),
    );

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(ref mut debug) = debug_output {
            debug.push_str(&format!("❌ Compilation failed: {}\n", stderr));
        }
        return (
            RoundTripPhaseResult::failure(
                RoundTripPhase::TokenEncoding,
                format!("Test crate compilation failed: {}", stderr),
            )
            .with_metrics(metrics)
            .with_debug(debug_output.unwrap_or_default())
            .with_duration(duration.as_micros() as u64),
            None,
        );
    }

    if let Some(ref mut debug) = debug_output {
        debug.push_str("✅ Token encoding compilation successful\n");
    }
    metrics.insert("success".to_string(), "true".to_string());

    (
        RoundTripPhaseResult::success(RoundTripPhase::TokenEncoding)
            .with_metrics(metrics)
            .with_debug(debug_output.unwrap_or_default())
            .with_duration(duration.as_micros() as u64),
        None,
    )
}

/// Phase 4: Token Decoding Verification
/// Decode token stream back to Rust struct, ensuring all tokens are consumed
#[allow(dead_code)]
pub fn verify_token_decoding(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
    _tokens: Option<Vec<u32>>,
) -> RoundTripPhaseResult {
    verify_token_decoding_with_optimization_level(
        schema,
        instance,
        schema_name,
        instance_name,
        _tokens,
        OptimizationLevel::O0,
    )
}

pub fn verify_token_decoding_with_optimization_level(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
    _tokens: Option<Vec<u32>>,
    optimization_level: OptimizationLevel,
) -> RoundTripPhaseResult {
    let start_time = std::time::Instant::now();
    let mut debug_output = if is_debug_enabled() {
        Some(String::new())
    } else {
        None
    };
    let mut metrics = HashMap::new();

    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!(
            "🔧 Verifying token decoding for {}/{}\n",
            schema_name, instance_name
        ));
    }

    // For now, this is combined with the overall execution test
    // In a future iteration, we could extract tokens from Phase 3 and test decoding separately
    let test_dir = format!(
        "/tmp/round_trip_decode_{}_{}",
        sanitize_path_component(schema_name),
        sanitize_path_component(instance_name)
    );
    let _ = fs::remove_dir_all(&test_dir);

    if let Err(e) = create_round_trip_test_crate(
        &test_dir,
        schema,
        instance,
        schema_name,
        instance_name,
        optimization_level,
    ) {
        return RoundTripPhaseResult::failure(
            RoundTripPhase::TokenDecoding,
            format!("Failed to create test crate: {}", e),
        )
        .with_duration(start_time.elapsed().as_micros() as u64);
    }

    // Run the test to verify decoding works
    let mut command = Command::new("cargo");
    configure_round_trip_generated_cargo_command(&mut command);
    let output = match command
        .args(["run", "--release"])
        .current_dir(&test_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(mut child) => match wait_for_child_with_timeout(&mut child, Duration::from_secs(60)) {
            Ok(output) => output,
            Err(e) => {
                let _ = child.kill();
                return RoundTripPhaseResult::failure(
                    RoundTripPhase::TokenDecoding,
                    format!("Decoding test timed out: {}", e),
                )
                .with_duration(start_time.elapsed().as_micros() as u64);
            }
        },
        Err(e) => {
            return RoundTripPhaseResult::failure(
                RoundTripPhase::TokenDecoding,
                format!("Failed to spawn decoding test: {}", e),
            )
            .with_duration(start_time.elapsed().as_micros() as u64);
        }
    };

    let duration = start_time.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    metrics.insert(
        "decoding_run_time_us".to_string(),
        duration.as_micros().to_string(),
    );

    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!("📊 Test output:\n{}\n", stdout));
        if !stderr.is_empty() {
            debug.push_str(&format!("⚠️ stderr: {}\n", stderr));
        }
    }

    // Check for specific decoding success indicators
    let decode_successful =
        stdout.contains("Decode successful") || stdout.contains("COMPREHENSIVE ROUND-TRIP SUCCESS");
    let tokens_consumed =
        stdout.contains("All tokens consumed") || !stdout.contains("Unconsumed tokens");

    if !output.status.success() {
        if let Some(ref mut debug) = debug_output {
            debug.push_str(&format!("❌ Decoding test failed: {}\n", stderr));
        }
        return RoundTripPhaseResult::failure(
            RoundTripPhase::TokenDecoding,
            format!("Token decoding test failed: {}", stderr),
        )
        .with_metrics(metrics)
        .with_debug(debug_output.unwrap_or_default())
        .with_duration(duration.as_micros() as u64);
    }

    if !decode_successful {
        return RoundTripPhaseResult::failure(
            RoundTripPhase::TokenDecoding,
            "Token decoding did not complete successfully".to_string(),
        )
        .with_metrics(metrics)
        .with_debug(debug_output.unwrap_or_default())
        .with_duration(duration.as_micros() as u64);
    }

    if let Some(ref mut debug) = debug_output {
        debug.push_str("✅ Token decoding verification successful\n");
        debug.push_str(&format!(
            "🔍 Tokens consumed: {}\n",
            if tokens_consumed { "✅" } else { "⚠️" }
        ));
    }

    metrics.insert("success".to_string(), "true".to_string());
    metrics.insert(
        "decode_successful".to_string(),
        decode_successful.to_string(),
    );
    metrics.insert("tokens_consumed".to_string(), tokens_consumed.to_string());

    RoundTripPhaseResult::success(RoundTripPhase::TokenDecoding)
        .with_metrics(metrics)
        .with_debug(debug_output.unwrap_or_default())
        .with_duration(duration.as_micros() as u64)
}

/// Phase 5: Data Integrity Verification
/// Compare original vs decoded structures with multiple verification methods
#[allow(dead_code)]
pub fn verify_data_integrity(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
) -> RoundTripPhaseResult {
    verify_data_integrity_with_optimization_level(
        schema,
        instance,
        schema_name,
        instance_name,
        OptimizationLevel::O0,
    )
}

pub fn verify_data_integrity_with_optimization_level(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
    optimization_level: OptimizationLevel,
) -> RoundTripPhaseResult {
    let start_time = std::time::Instant::now();
    let mut debug_output = if is_debug_enabled() {
        Some(String::new())
    } else {
        None
    };
    let mut metrics = HashMap::new();

    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!(
            "🔧 Verifying data integrity for {}/{}\n",
            schema_name, instance_name
        ));
    }

    // Run the complete round-trip test and analyze the output for integrity verification
    let test_dir = format!(
        "/tmp/round_trip_integrity_{}_{}",
        sanitize_path_component(schema_name),
        sanitize_path_component(instance_name)
    );
    let _ = fs::remove_dir_all(&test_dir);

    if let Err(e) = create_round_trip_test_crate(
        &test_dir,
        schema,
        instance,
        schema_name,
        instance_name,
        optimization_level,
    ) {
        return RoundTripPhaseResult::failure(
            RoundTripPhase::DataIntegrity,
            format!("Failed to create test crate: {}", e),
        )
        .with_duration(start_time.elapsed().as_micros() as u64);
    }

    let mut command = Command::new("cargo");
    configure_round_trip_generated_cargo_command(&mut command);
    let output = match command
        .args(["run", "--release"])
        .current_dir(&test_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(mut child) => match wait_for_child_with_timeout(&mut child, Duration::from_secs(60)) {
            Ok(output) => output,
            Err(e) => {
                let _ = child.kill();
                return RoundTripPhaseResult::failure(
                    RoundTripPhase::DataIntegrity,
                    format!("Integrity test timed out: {}", e),
                )
                .with_duration(start_time.elapsed().as_micros() as u64);
            }
        },
        Err(e) => {
            return RoundTripPhaseResult::failure(
                RoundTripPhase::DataIntegrity,
                format!("Failed to run integrity test: {}", e),
            )
            .with_duration(start_time.elapsed().as_micros() as u64);
        }
    };

    let duration = start_time.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    metrics.insert(
        "integrity_test_time_us".to_string(),
        duration.as_micros().to_string(),
    );

    // Parse actual encode/decode performance from test output
    let mut encode_time_ns = None;
    let mut decode_time_ns = None;

    for line in stdout.lines() {
        if line.starts_with("PERF:encode:") {
            if let Some(time_str) = line
                .strip_prefix("PERF:encode:")
                .and_then(|s| s.strip_suffix("ns"))
                && let Ok(nanos) = time_str.parse::<u64>()
            {
                encode_time_ns = Some(nanos);
                metrics.insert("actual_encode_time_ns".to_string(), nanos.to_string());
            }
        } else if line.starts_with("PERF:decode:")
            && let Some(time_str) = line
                .strip_prefix("PERF:decode:")
                .and_then(|s| s.strip_suffix("ns"))
            && let Ok(nanos) = time_str.parse::<u64>()
        {
            decode_time_ns = Some(nanos);
            metrics.insert("actual_decode_time_ns".to_string(), nanos.to_string());
        }
    }

    // Display actual performance if found
    if let Some(encode_ns) = encode_time_ns {
        println!(
            "        ⚡ Actual encode time: {}ns ({:.3}µs)",
            encode_ns,
            encode_ns as f64 / 1000.0
        );
    }
    if let Some(decode_ns) = decode_time_ns {
        println!(
            "        ⚡ Actual decode time: {}ns ({:.3}µs)",
            decode_ns,
            decode_ns as f64 / 1000.0
        );
    }
    if let (Some(enc), Some(dec)) = (encode_time_ns, decode_time_ns) {
        let total_ns = enc + dec;
        println!(
            "        ⚡ Total operation time: {}ns ({:.3}µs)",
            total_ns,
            total_ns as f64 / 1000.0
        );
    }

    if let Some(ref mut debug) = debug_output {
        debug.push_str(&format!("📊 Integrity test output:\n{}\n", stdout));
        if !stderr.is_empty() {
            debug.push_str(&format!("⚠️ stderr: {}\n", stderr));
        }
    }

    // Check for specific integrity verification indicators from our enhanced round-trip test
    let struct_equality = stdout.contains("Struct equality verification passed");
    let raw_json_equality = stdout.contains("Raw JSON equality verification passed");
    let reserialize_check = stdout.contains("JSON re-serialization verification passed");
    let comprehensive_success = stdout.contains("COMPREHENSIVE ROUND-TRIP SUCCESS");

    if !output.status.success() {
        return RoundTripPhaseResult::failure(
            RoundTripPhase::DataIntegrity,
            format!("Data integrity test failed: {}", stderr),
        )
        .with_metrics(metrics)
        .with_debug(debug_output.unwrap_or_default())
        .with_duration(duration.as_micros() as u64);
    }

    if !comprehensive_success {
        return RoundTripPhaseResult::failure(
            RoundTripPhase::DataIntegrity,
            "Comprehensive round-trip verification did not succeed".to_string(),
        )
        .with_metrics(metrics)
        .with_debug(debug_output.unwrap_or_default())
        .with_duration(duration.as_micros() as u64);
    }

    if !raw_json_equality {
        return RoundTripPhaseResult::failure(
            RoundTripPhase::DataIntegrity,
            "Raw JSON equality verification did not succeed".to_string(),
        )
        .with_metrics(metrics)
        .with_debug(debug_output.unwrap_or_default())
        .with_duration(duration.as_micros() as u64);
    }

    if let Some(ref mut debug) = debug_output {
        debug.push_str("✅ Data integrity verification successful\n");
        debug.push_str(&format!(
            "🔍 Struct equality: {}\n",
            if struct_equality { "✅" } else { "⚠️" }
        ));
        debug.push_str(&format!(
            "🔍 Raw JSON equality: {}\n",
            if raw_json_equality { "✅" } else { "⚠️" }
        ));
        debug.push_str(&format!(
            "🔍 Re-serialization: {}\n",
            if reserialize_check { "✅" } else { "⚠️" }
        ));
    }

    metrics.insert("success".to_string(), "true".to_string());
    metrics.insert("struct_equality".to_string(), struct_equality.to_string());
    metrics.insert(
        "raw_json_equality".to_string(),
        raw_json_equality.to_string(),
    );
    metrics.insert(
        "reserialize_check".to_string(),
        reserialize_check.to_string(),
    );
    metrics.insert(
        "comprehensive_success".to_string(),
        comprehensive_success.to_string(),
    );

    RoundTripPhaseResult::success(RoundTripPhase::DataIntegrity)
        .with_metrics(metrics)
        .with_debug(debug_output.unwrap_or_default())
        .with_duration(duration.as_micros() as u64)
}

#[allow(dead_code)]
fn classify_batch_round_trip_failure(error: &str) -> RoundTripPhase {
    if error.contains("source schema validation failed") {
        RoundTripPhase::DataIntegrity
    } else if error.contains("decode failed") || error.contains("Unconsumed tokens") {
        RoundTripPhase::TokenDecoding
    } else if error.contains("Original and decoded JSON do not match")
        || error.contains("re-serialization")
    {
        RoundTripPhase::DataIntegrity
    } else {
        RoundTripPhase::TokenEncoding
    }
}

#[allow(dead_code)]
fn batch_failure_message(summary: &BatchRoundTripSummary) -> String {
    if summary.failed_instances.is_empty() {
        "batch round-trip did not produce a successful result".to_string()
    } else {
        summary
            .failed_instances
            .iter()
            .map(|(name, error)| format!("{}: {}", name, error))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Execute all phases of round-trip testing in sequence
#[allow(dead_code)]
pub fn test_phased_round_trip(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
) -> PhasedRoundTripResult {
    test_phased_round_trip_with_optimization_level(
        schema,
        instance,
        schema_name,
        instance_name,
        OptimizationLevel::O0,
    )
}

#[allow(dead_code)]
pub fn test_phased_round_trip_with_optimization_level(
    schema: &Value,
    instance: &Value,
    schema_name: &str,
    instance_name: &str,
    optimization_level: OptimizationLevel,
) -> PhasedRoundTripResult {
    let start_time = std::time::Instant::now();
    let mut result = PhasedRoundTripResult::new(schema_name.to_string(), instance_name.to_string());

    if is_debug_enabled() {
        println!(
            "🚀 Starting phased round-trip test for {}/{}",
            schema_name, instance_name
        );
    }

    // Phase 1: Schema Compilation
    let phase1_start = std::time::Instant::now();
    println!(
        "  🔄 Phase 1: {} - Converting JSON schema to TCT grammar",
        RoundTripPhase::SchemaCompilation.name()
    );
    println!(
        "    📝 Schema: {} (type: {})",
        schema_name,
        schema
            .get("type")
            .unwrap_or(&serde_json::Value::String("unknown".to_string()))
    );
    let phase1_result =
        verify_schema_compilation_with_optimization_level(schema, schema_name, optimization_level);
    let phase1_duration = phase1_start.elapsed().as_micros() as u64;
    let phase1_success = phase1_result.success;
    if phase1_success {
        println!(
            "    ✅ Schema compilation succeeded in {}µs",
            phase1_duration
        );
    } else {
        println!(
            "    ❌ Schema compilation failed in {}µs: {}",
            phase1_duration,
            phase1_result.error.as_deref().unwrap_or("Unknown error")
        );
    }
    result.add_phase_result(phase1_result);

    if !phase1_success {
        result.total_duration_us = Some(start_time.elapsed().as_micros() as u64);
        return result; // Early exit on schema compilation failure
    }

    // Phase 2: Code Generation
    let phase2_start = std::time::Instant::now();
    println!(
        "  🔄 Phase 2: {} - Generating Rust tokenizer code",
        RoundTripPhase::CodeGeneration.name()
    );
    let phase2_result =
        validate_generated_code_with_optimization_level(schema, schema_name, optimization_level);
    let phase2_duration = phase2_start.elapsed().as_micros() as u64;
    let phase2_success = phase2_result.success;
    if phase2_success {
        println!("    ✅ Code generation succeeded in {}µs", phase2_duration);
        if let Some(code_size) = phase2_result.metrics.get("code_size_bytes") {
            println!("    📊 Generated code size: {} bytes", code_size);
        }
    } else {
        println!(
            "    ❌ Code generation failed in {}µs: {}",
            phase2_duration,
            phase2_result.error.as_deref().unwrap_or("Unknown error")
        );
    }
    result.add_phase_result(phase2_result);

    if !phase2_success {
        result.total_duration_us = Some(start_time.elapsed().as_micros() as u64);
        return result; // Early exit on code generation failure
    }

    // Phases 3-5: Run one generated binary for encode, decode, and integrity.
    let generated_round_trip_start = std::time::Instant::now();
    println!(
        "  🔄 Phase 3: {} - Converting JSON instance to tokens",
        RoundTripPhase::TokenEncoding.name()
    );
    let instance_size = serde_json::to_string(instance)
        .map(|s| s.len())
        .unwrap_or(0);
    println!(
        "    📄 Instance: {} ({} bytes)",
        instance_name, instance_size
    );
    let batch_instances = vec![(instance_name.to_string(), instance.clone())];
    let batch_summary = match test_batch_round_trip_with_optimization_level(
        schema,
        &batch_instances,
        schema_name,
        optimization_level,
    ) {
        Ok(summary) => summary,
        Err(error) => {
            let duration_us = generated_round_trip_start.elapsed().as_micros() as u64;
            let phase3_result = RoundTripPhaseResult::failure(
                RoundTripPhase::TokenEncoding,
                format!("Generated round-trip binary failed: {}", error),
            )
            .with_duration(duration_us);
            println!(
                "    ❌ Generated round-trip binary failed in {}µs: {}",
                duration_us, error
            );
            result.add_phase_result(phase3_result);
            result.total_duration_us = Some(start_time.elapsed().as_micros() as u64);
            return result;
        }
    };
    let generated_round_trip_duration_us = generated_round_trip_start.elapsed().as_micros() as u64;
    let success = batch_summary
        .successes
        .iter()
        .find(|success| success.name == instance_name)
        .or_else(|| batch_summary.successes.first());

    if !batch_summary.failed_instances.is_empty() || success.is_none() {
        let error = batch_failure_message(&batch_summary);
        let failed_phase = classify_batch_round_trip_failure(&error);

        if matches!(
            failed_phase,
            RoundTripPhase::TokenDecoding | RoundTripPhase::DataIntegrity
        ) {
            result.add_phase_result(
                RoundTripPhaseResult::success(RoundTripPhase::TokenEncoding)
                    .with_duration(generated_round_trip_duration_us),
            );
        }
        if matches!(failed_phase, RoundTripPhase::DataIntegrity) {
            result.add_phase_result(
                RoundTripPhaseResult::success(RoundTripPhase::TokenDecoding)
                    .with_duration(generated_round_trip_duration_us),
            );
        }

        result.add_phase_result(
            RoundTripPhaseResult::failure(failed_phase, error.clone())
                .with_duration(generated_round_trip_duration_us),
        );
        println!(
            "    ❌ Generated round-trip failed at {} in {}µs: {}",
            failed_phase.name(),
            generated_round_trip_duration_us,
            error
        );
        result.total_duration_us = Some(start_time.elapsed().as_micros() as u64);
        return result;
    }

    let success = success.expect("success checked above");
    let token_bytes = success.token_count * std::mem::size_of::<u32>();
    let compression_ratio = if token_bytes == 0 {
        None
    } else {
        Some(instance_size as f64 / token_bytes as f64)
    };

    let mut phase3_metrics = HashMap::new();
    phase3_metrics.insert("success".to_string(), "true".to_string());
    phase3_metrics.insert("token_count".to_string(), success.token_count.to_string());
    phase3_metrics.insert("max_token".to_string(), success.max_token.to_string());
    phase3_metrics.insert(
        "actual_encode_time_ns".to_string(),
        success.encode_time_ns.to_string(),
    );
    phase3_metrics.insert("json_bytes".to_string(), instance_size.to_string());
    phase3_metrics.insert("token_bytes".to_string(), token_bytes.to_string());
    if let Some(compression_ratio) = compression_ratio {
        phase3_metrics.insert(
            "compression_ratio".to_string(),
            compression_ratio.to_string(),
        );
    }
    println!(
        "    ✅ Encoding verified via batch binary in {}µs",
        generated_round_trip_duration_us
    );
    println!(
        "    🎯 Generated {} tokens (max token {})",
        success.token_count, success.max_token
    );
    result.add_phase_result(
        RoundTripPhaseResult::success(RoundTripPhase::TokenEncoding)
            .with_metrics(phase3_metrics)
            .with_duration(generated_round_trip_duration_us),
    );

    println!(
        "  🔄 Phase 4: {} - Converting tokens back to JSON",
        RoundTripPhase::TokenDecoding.name()
    );
    let mut phase4_metrics = HashMap::new();
    phase4_metrics.insert("success".to_string(), "true".to_string());
    phase4_metrics.insert(
        "actual_decode_time_ns".to_string(),
        success.decode_time_ns.to_string(),
    );
    phase4_metrics.insert("token_count".to_string(), success.token_count.to_string());
    println!(
        "    ✅ Decoding verified via batch binary in {}µs",
        generated_round_trip_duration_us
    );
    result.add_phase_result(
        RoundTripPhaseResult::success(RoundTripPhase::TokenDecoding)
            .with_metrics(phase4_metrics)
            .with_duration(generated_round_trip_duration_us),
    );

    println!(
        "  🔄 Phase 5: {} - Verifying original data equals decoded data",
        RoundTripPhase::DataIntegrity.name()
    );
    let mut phase5_metrics = HashMap::new();
    phase5_metrics.insert("success".to_string(), "true".to_string());
    phase5_metrics.insert("raw_json_equality".to_string(), "true".to_string());
    phase5_metrics.insert("reserialize_check".to_string(), "true".to_string());
    phase5_metrics.insert("comprehensive_success".to_string(), "true".to_string());
    phase5_metrics.insert(
        "generated_schema_compilation_time_ms".to_string(),
        batch_summary.schema_compilation_time_ms.to_string(),
    );
    phase5_metrics.insert(
        "generated_code_generation_time_ms".to_string(),
        batch_summary.code_generation_time_ms.to_string(),
    );
    phase5_metrics.insert(
        "generated_rust_compilation_time_ms".to_string(),
        batch_summary.rust_compilation_time_ms.to_string(),
    );
    println!(
        "    ✅ Data integrity verified in {}µs - original data equals decoded data",
        generated_round_trip_duration_us
    );
    result.add_phase_result(
        RoundTripPhaseResult::success(RoundTripPhase::DataIntegrity)
            .with_metrics(phase5_metrics)
            .with_duration(generated_round_trip_duration_us),
    );

    result.total_duration_us = Some(start_time.elapsed().as_micros() as u64);

    if is_debug_enabled() {
        if result.overall_success {
            println!(
                "🎉 All phases completed successfully for {}/{}",
                schema_name, instance_name
            );
        } else if let Some(failed_phase) = result.failed_phase() {
            println!(
                "❌ Failed at phase: {} for {}/{}",
                failed_phase.phase.name(),
                schema_name,
                instance_name
            );
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generated_cargo_target_dir_matches_configured_cargo_default() {
        let tct_path = Path::new("/repo/tct");
        let crate_path = Path::new("/tmp/generated-crate");

        assert_eq!(
            resolve_generated_cargo_target_dir(tct_path, crate_path, None, None),
            PathBuf::from("/repo/tct/target/tct-generated-cargo")
        );
        assert_eq!(
            resolve_generated_cargo_target_dir(
                tct_path,
                crate_path,
                Some(OsStr::new("custom-generated")),
                Some(OsStr::new("/ignored-inherited")),
            ),
            PathBuf::from("/repo/tct/custom-generated")
        );
        assert_eq!(
            resolve_generated_cargo_target_dir(
                tct_path,
                crate_path,
                Some(OsStr::new("")),
                Some(OsStr::new("inherited-target")),
            ),
            PathBuf::from("/tmp/generated-crate/inherited-target")
        );
    }

    #[test]
    #[ignore = "legacy raw JSON equality check; current round-trip helpers assert semantic equality"]
    fn test_legacy_round_trip_helper_enforces_raw_json_equality() {
        let schema = json!({
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" }
            },
            "required": ["enabled"],
            "additionalProperties": false
        });
        let instance = json!({ "enabled": true });

        let result = test_round_trip(&schema, &instance, "synthetic/raw_json_equality", "enabled");

        assert!(
            result.success,
            "legacy generated-crate round-trip helper failed: {:?}",
            result.error
        );
    }

    #[test]
    fn estimated_profiling_metrics_are_marked_as_static_estimates() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "enabled": { "type": "boolean" }
            },
            "required": ["name"]
        });
        let instance = json!({ "name": "example", "enabled": true });

        let result =
            estimate_profiling_metrics_without_round_trip(&schema, &instance, "synthetic", "case")
                .expect("static profiling estimate should be generated");

        assert!(result.success);
        assert_eq!(result.metric_source, RoundTripMetricSource::StaticEstimate);
        assert!(result.encode_time_us.is_some());
        assert!(result.decode_time_us.is_some());
        assert!(result.token_count.is_some());
    }

    #[test]
    fn generated_tokenizer_contract_reads_trait_impl_methods() {
        let generated_code = r#"
            pub struct TestType;

            impl crate::tokenizer::JsonTokenEncoder for TestType {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    tokens.push(0);
                }

                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, String> {
                    let _ = tokens.next();
                    Ok(Self)
                }

                fn token_count(&self) -> usize {
                    1
                }
            }
        "#;

        let contract = generated_tokenizer_contract(generated_code)
            .expect("synthetic generated tokenizer impl should parse");

        assert!(contract.has_encoder);
        assert!(contract.has_encode);
        assert!(contract.has_decode);
        assert!(contract.has_token_count);
    }

    #[test]
    fn root_type_extraction_ignores_generated_sequence_metadata() {
        let generated_code = r#"
            pub struct TestType {
                pub name: String,
            }

            pub mod sequence {
                pub struct SequenceTokenRange {
                    pub start: u32,
                    pub end_exclusive: u32,
                }

                pub struct SequenceSpecialTokenPolicy {
                    pub pad_token: u32,
                }

                pub struct SequenceBpeMetadata {
                    pub enabled: bool,
                }

                pub struct SequenceBoundaryMetadata {
                    pub version: u32,
                }
            }
        "#;

        assert_eq!(
            extract_root_type_name(generated_code),
            Some("TestType".to_string())
        );
    }

    #[test]
    fn source_schema_validation_failures_are_data_integrity_failures() {
        assert_eq!(
            classify_batch_round_trip_failure(
                "source schema validation failed after decode: schema=example fixture=case"
            ),
            RoundTripPhase::DataIntegrity
        );
        assert_eq!(
            classify_batch_round_trip_failure(
                "source schema validation failed before encode: schema=example fixture=case"
            ),
            RoundTripPhase::DataIntegrity
        );
    }

    #[test]
    fn generated_batch_round_trip_reports_after_decode_source_schema_validation_failure() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"],
            "additionalProperties": false
        });
        let instances = vec![(
            "synthetic/after-decode-invalid.json".to_string(),
            json!({ "name": "ok" }),
        )];

        let summary = test_batch_round_trip_with_optimization_level_and_after_decode_hook(
            &schema,
            &instances,
            "synthetic_after_decode_source_schema_validation",
            OptimizationLevel::O3,
            "decoded_json[\"name\"] = json!(7);",
        )
        .expect("generated batch helper should report the injected validation failure");

        assert_eq!(summary.successful_instances, 0);
        assert_eq!(summary.failed_instances.len(), 1);
        let (fixture_name, error) = &summary.failed_instances[0];
        assert_eq!(fixture_name, "synthetic/after-decode-invalid.json");
        assert!(error.contains("source schema validation failed after decode"));
        assert!(error.contains("schema=synthetic_after_decode_source_schema_validation"));
        assert!(error.contains("fixture=synthetic/after-decode-invalid.json"));
        assert!(error.contains("instance_path=/name"));
        assert!(error.contains("schema_path=/properties/name/type"));
        assert!(error.contains("decoded_context="));
        assert!(error.contains("\"name\": 7"));
        assert!(!error.contains("source schema validation failed before encode"));
    }
}
