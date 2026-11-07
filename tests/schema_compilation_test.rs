//! Schema Compilation Test Suite
//!
//! This test suite validates TCT schema compilation (schema → Rust code generation) without
//! executing round-trip tests. Focus is on compilation success and code generation quality.
//!
//! Test Tiers:
//! - Popular: Most important schemas for CI (fast)
//! - Common: Standard schemas for comprehensive testing
//! - Specialized: Domain-specific schemas
//! - Comprehensive: All SchemaStore schemas (feature-gated)

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use tct::{OptimizationLevel, TctCompiler};

#[cfg(feature = "round-trip-tests")]
mod round_trip_helpers;
#[cfg(feature = "round-trip-tests")]
use round_trip_helpers::{
    RoundTripMetricSource, RoundTripResult, load_instances_for_schema, test_round_trip,
};

#[cfg(feature = "round-trip-tests")]
const UNSUPPORTED_ROUND_TRIP_FIXTURES: &[(&str, &str)] = &[];

#[cfg(any(feature = "comprehensive-tests", feature = "extended-benchmarks"))]
fn benchmark_output_dir() -> std::path::PathBuf {
    std::env::var_os("TCT_BENCHMARK_OUTPUT_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("target/tct-benchmarks"))
}

#[cfg(any(feature = "comprehensive-tests", feature = "extended-benchmarks"))]
fn write_benchmark_output(file_name: &str, contents: String, label: &str) {
    let output_dir = benchmark_output_dir();
    if let Err(e) = fs::create_dir_all(&output_dir) {
        eprintln!(
            "Warning: Could not create benchmark output directory {}: {}",
            output_dir.display(),
            e
        );
        return;
    }

    let output_path = output_dir.join(file_name);
    if let Err(e) = fs::write(&output_path, contents) {
        eprintln!(
            "Warning: Could not save {} to {}: {}",
            label,
            output_path.display(),
            e
        );
    } else {
        println!("💾 {} saved to {}", label, output_path.display());
    }
}

/// Results from testing a schema
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchemaTestResult {
    pub schema_name: String,
    pub category: String,
    pub tier: Option<String>, // New field for tier information
    pub success: bool,
    pub error_type: Option<String>,
    pub error_detail: Option<String>,
    pub compilation_time_ms: Option<f64>,
    pub code_size_bytes: Option<usize>,
    pub has_optimizations: bool,
}

// Schema tier mapping (should match round_trip_test.rs)
const SCHEMA_TIERS: &[(&str, &[&str])] = &[
    (
        "popular",
        &[
            "commitlintrc",
            "package",
            "tsconfig",
            "cargo",
            "docker-compose",
            "github-workflow",
        ],
    ),
    ("common", &["all-contributors", "bower", "apibuilder"]),
    ("specialized", &["buf.plugin", "chutzpah", "detekt-1.22.0"]),
];

const EXPECTED_POPULAR_SCHEMA_FAILURES: &[(&str, &str)] = &[
    ("popular/gitlab-ci", "UnsupportedFeature"),
    ("popular/travis", "UnsupportedFeature"),
    ("popular/drone", "UnsupportedFeature"),
    ("popular/dependabot", "UnsupportedFeature"),
    ("popular/tslint", "UnsupportedFeature"),
];

const EXPECTED_COMMON_SCHEMA_FAILURES: &[(&str, &str)] = &[
    ("common/github-issue-forms", "UnsupportedFeature"),
    ("common/drone", "UnsupportedFeature"),
    ("common/jfrog-pipelines", "UnsupportedFeature"),
];

const EXPECTED_SPECIALIZED_SCHEMA_FAILURES: &[(&str, &str)] = &[
    (
        "specialized/azure-deviceupdate-import-manifest-4.0",
        "InvalidInput",
    ),
    ("specialized/utam-page-object", "UnsupportedFeature"),
    ("specialized/gitea-issue-forms", "UnsupportedFeature"),
    ("specialized/minecraft-biome", "UnsupportedFeature"),
    ("specialized/lsdlschema-2.0", "SchemaValidation"),
];

const EXPECTED_CONFIGURATION_SCHEMA_FAILURES: &[(&str, &str)] = &[(
    "configuration/AnyWork_Automation_Configuration",
    "UnsupportedFeature",
)];

const EXPECTED_API_SCHEMA_FAILURES: &[(&str, &str)] = &[];

/// Schema tiers for tiered testing approach
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaTier {
    Popular,
    Common,
    Specialized,
    Comprehensive, // All schemas from SchemaStore repository
    Legacy,        // For backwards compatibility with old categorized schemas
}

// Get schema count from environment variable (same as round-trip tests)
fn get_schema_count() -> usize {
    std::env::var("SCHEMA_COUNT")
        .ok()
        .and_then(|s| {
            if s == "all" {
                Some(usize::MAX)
            } else {
                s.parse().ok()
            }
        })
        .unwrap_or(5) // Default to 5 schemas for compilation tests (faster than round-trip)
}

impl SchemaTier {
    fn from_path(path: &str) -> Self {
        if path.contains("/popular/") {
            SchemaTier::Popular
        } else if path.contains("/common/") {
            SchemaTier::Common
        } else if path.contains("/specialized/") {
            SchemaTier::Specialized
        } else if path.contains("schemastore") || path.contains("comprehensive") {
            SchemaTier::Comprehensive
        } else {
            SchemaTier::Legacy
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            SchemaTier::Popular => "popular",
            SchemaTier::Common => "common",
            SchemaTier::Specialized => "specialized",
            SchemaTier::Comprehensive => "comprehensive",
            SchemaTier::Legacy => "legacy",
        }
    }
}

/// Schema categories for organized testing (legacy, maintained for backwards compatibility)
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaCategory {
    Api,
    Configuration,
    CloudService,
    Data,
    Other,
}

impl SchemaCategory {
    fn from_path(path: &str) -> Self {
        if path.contains("/api/") {
            SchemaCategory::Api
        } else if path.contains("/configuration/") {
            SchemaCategory::Configuration
        } else if path.contains("/cloud_service/") {
            SchemaCategory::CloudService
        } else if path.contains("/data/") {
            SchemaCategory::Data
        } else {
            SchemaCategory::Other
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            SchemaCategory::Api => "api",
            SchemaCategory::Configuration => "configuration",
            SchemaCategory::CloudService => "cloud_service",
            SchemaCategory::Data => "data",
            SchemaCategory::Other => "other",
        }
    }
}

/// Load schemas with tiered structure (critical, important, extended)
fn load_tiered_schemas() -> Vec<(String, Value, SchemaTier)> {
    let mut schemas = Vec::new();
    let tiers = ["popular", "common", "specialized"];

    for tier in &tiers {
        let tier_dir = format!("schemas/{}", tier);
        if let Ok(entries) = fs::read_dir(&tier_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json")
                    && let Some(schema) = load_schema_file(&path)
                {
                    let name = format!("{}/{}", tier, path.file_stem().unwrap().to_string_lossy());
                    let tier_enum = SchemaTier::from_path(&path.to_string_lossy());
                    schemas.push((name, schema, tier_enum));
                }
            }
        }
    }

    schemas
}

/// Load schemas from schemas directory (legacy function for backwards compatibility)
fn load_schemas() -> Vec<(String, Value, SchemaCategory)> {
    let mut schemas = Vec::new();
    let schemas_dir = "schemas";

    if let Ok(entries) = fs::read_dir(schemas_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Skip archived directory completely
            if path_str.contains("/archived") {
                continue;
            }

            if path.is_dir() {
                // Skip the new tiered directories to avoid duplicates
                let dir_name = path.file_name().unwrap().to_string_lossy();
                if matches!(
                    dir_name.as_ref(),
                    "popular" | "common" | "specialized" | "archived"
                ) {
                    continue;
                }

                // Recursively load from legacy subdirectories
                if let Ok(sub_entries) = fs::read_dir(&path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.extension().and_then(|s| s.to_str()) == Some("json")
                            && let Some(schema) = load_schema_file(&sub_path)
                        {
                            let name = format!(
                                "{}/{}",
                                path.file_name().unwrap().to_string_lossy(),
                                sub_path.file_stem().unwrap().to_string_lossy()
                            );
                            let category = SchemaCategory::from_path(&sub_path.to_string_lossy());
                            schemas.push((name, schema, category));
                        }
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Skip analysis/report files
                let filename = path.file_name().unwrap().to_string_lossy();
                if !filename.contains("report")
                    && !filename.contains("analysis")
                    && let Some(schema) = load_schema_file(&path)
                {
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    let category = SchemaCategory::from_path(&path.to_string_lossy());
                    schemas.push((name, schema, category));
                }
            }
        }
    }

    schemas
}

/// Load and parse a schema file
fn load_schema_file(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

/// Load all schemas from SchemaStore repository (comprehensive testing)
#[cfg(feature = "comprehensive-tests")]
fn load_schemastore_schemas() -> Result<Vec<(String, Value, SchemaTier)>, Box<dyn std::error::Error>>
{
    use std::process::Command;

    let temp_dir = "/tmp/tct-schemastore-test";
    let schemastore_path = format!("{}/schemastore", temp_dir);
    let schemas_path = format!("{}/src/schemas/json", schemastore_path);

    // Create temp directory
    fs::create_dir_all(temp_dir)?;

    // Clone SchemaStore if not present
    if !Path::new(&schemastore_path).exists() {
        println!("🌐 Cloning SchemaStore repository (one-time setup)...");
        let output = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "https://github.com/SchemaStore/schemastore.git",
                &schemastore_path,
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to clone SchemaStore: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        println!("✅ SchemaStore cloned successfully");
    } else {
        println!("📂 Using existing SchemaStore clone");
    }

    // Load schemas from SchemaStore
    let mut schemas = Vec::new();

    if let Ok(entries) = fs::read_dir(&schemas_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(schema) = load_schema_file(&path) {
                    let name = format!(
                        "schemastore/{}",
                        path.file_stem().unwrap().to_string_lossy()
                    );
                    schemas.push((name, schema, SchemaTier::Comprehensive));
                }
            }
        }
    }

    println!("📊 Loaded {} schemas from SchemaStore", schemas.len());
    Ok(schemas)
}

/// Clean up SchemaStore temporary directory
#[cfg(feature = "comprehensive-tests")]
fn cleanup_schemastore() {
    let temp_dir = "/tmp/tct-schemastore-test";

    // Check if user wants to keep the directory for debugging
    if std::env::var("KEEP_SCHEMASTORE").is_ok() {
        println!(
            "🔧 Keeping SchemaStore directory for debugging: {}",
            temp_dir
        );
        return;
    }

    if Path::new(temp_dir).exists() {
        if let Err(e) = fs::remove_dir_all(temp_dir) {
            eprintln!(
                "⚠️  Warning: Could not remove temp directory {}: {}",
                temp_dir, e
            );
        } else {
            println!("🧹 Cleaned up temporary SchemaStore directory");
        }
    }
}

/// Test a single schema compilation (legacy version)
fn test_schema_compilation(
    schema: &Value,
    schema_name: &str,
    category: SchemaCategory,
) -> SchemaTestResult {
    let start_time = std::time::Instant::now();

    // Try compiling with O2 optimization (most common production setting)
    let result = TctCompiler::new(schema.clone(), "TestType").and_then(|compiler| {
        compiler
            .with_optimization_level(OptimizationLevel::O2)
            .compile()
    });

    let compilation_time = start_time.elapsed().as_secs_f64() * 1000.0;

    match result {
        Ok(grammar) => {
            // Try generating code to catch generation errors
            match grammar.generate_code() {
                Ok(code) => {
                    let code_str = code.to_string();
                    let has_optimizations = code_str.contains("🚀")
                        || code_str.contains("BoundedU8")
                        || code_str.contains("PatternEnumeration");

                    SchemaTestResult {
                        schema_name: schema_name.to_string(),
                        category: category.as_str().to_string(),
                        tier: None, // Legacy schemas don't have tier information
                        success: true,
                        error_type: None,
                        error_detail: None,
                        compilation_time_ms: Some(compilation_time),
                        code_size_bytes: Some(code_str.len()),
                        has_optimizations,
                    }
                }
                Err(e) => SchemaTestResult {
                    schema_name: schema_name.to_string(),
                    category: category.as_str().to_string(),
                    tier: None,
                    success: false,
                    error_type: Some("CodeGeneration".to_string()),
                    error_detail: Some(e.to_string()),
                    compilation_time_ms: Some(compilation_time),
                    code_size_bytes: None,
                    has_optimizations: false,
                },
            }
        }
        Err(e) => {
            let error_type = classify_error(&e);
            SchemaTestResult {
                schema_name: schema_name.to_string(),
                category: category.as_str().to_string(),
                tier: None,
                success: false,
                error_type: Some(error_type),
                error_detail: Some(e.to_string()),
                compilation_time_ms: Some(compilation_time),
                code_size_bytes: None,
                has_optimizations: false,
            }
        }
    }
}

/// Test a single schema compilation with tier information
fn test_tiered_schema_compilation(
    schema: &Value,
    schema_name: &str,
    tier: SchemaTier,
) -> SchemaTestResult {
    let start_time = std::time::Instant::now();

    // Try compiling with O2 optimization (most common production setting)
    let result = TctCompiler::new(schema.clone(), "TestType").and_then(|compiler| {
        compiler
            .with_optimization_level(OptimizationLevel::O2)
            .compile()
    });

    let compilation_time = start_time.elapsed().as_secs_f64() * 1000.0;

    match result {
        Ok(grammar) => {
            // Try generating code to catch generation errors
            match grammar.generate_code() {
                Ok(code) => {
                    let code_str = code.to_string();
                    let has_optimizations = code_str.contains("🚀")
                        || code_str.contains("BoundedU8")
                        || code_str.contains("PatternEnumeration");

                    SchemaTestResult {
                        schema_name: schema_name.to_string(),
                        category: "tiered".to_string(), // Use generic category for tiered schemas
                        tier: Some(tier.as_str().to_string()),
                        success: true,
                        error_type: None,
                        error_detail: None,
                        compilation_time_ms: Some(compilation_time),
                        code_size_bytes: Some(code_str.len()),
                        has_optimizations,
                    }
                }
                Err(e) => SchemaTestResult {
                    schema_name: schema_name.to_string(),
                    category: "tiered".to_string(),
                    tier: Some(tier.as_str().to_string()),
                    success: false,
                    error_type: Some("CodeGeneration".to_string()),
                    error_detail: Some(e.to_string()),
                    compilation_time_ms: Some(compilation_time),
                    code_size_bytes: None,
                    has_optimizations: false,
                },
            }
        }
        Err(e) => {
            let error_type = classify_error(&e);
            SchemaTestResult {
                schema_name: schema_name.to_string(),
                category: "tiered".to_string(),
                tier: Some(tier.as_str().to_string()),
                success: false,
                error_type: Some(error_type),
                error_detail: Some(e.to_string()),
                compilation_time_ms: Some(compilation_time),
                code_size_bytes: None,
                has_optimizations: false,
            }
        }
    }
}

/// Classify error types for better analysis
fn classify_error(error: &tct::TctError) -> String {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("unsupported") {
        "UnsupportedFeature".to_string()
    } else if error_str.contains("invalid") || error_str.contains("malformed") {
        "InvalidInput".to_string()
    } else if error_str.contains("schema") {
        "SchemaValidation".to_string()
    } else if error_str.contains("overflow") || error_str.contains("stack") {
        "StackOverflow".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Generate test report
fn generate_test_report(results: &[SchemaTestResult]) {
    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let success_rate = if total > 0 {
        successful as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    println!("\n=== Real-World Schema Test Results ===");
    println!("📊 Total schemas tested: {}", total);
    println!("✅ Successful compilations: {}", successful);
    println!("❌ Failed compilations: {}", total - successful);
    println!("📈 Success rate: {:.1}%", success_rate);

    // Category breakdown
    let mut category_stats: HashMap<String, (usize, usize)> = HashMap::new();
    for result in results {
        let entry = category_stats
            .entry(result.category.clone())
            .or_insert((0, 0));
        entry.0 += 1; // total
        if result.success {
            entry.1 += 1; // successful
        }
    }

    println!("\n📋 Results by category:");
    for (category, (total, successful)) in category_stats {
        let rate = if total > 0 {
            successful as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!("  {}: {}/{} ({:.1}%)", category, successful, total, rate);
    }

    // Tier breakdown (if tier information is available)
    let has_tier_info = results.iter().any(|r| r.tier.is_some());
    if has_tier_info {
        let mut tier_stats: HashMap<String, (usize, usize)> = HashMap::new();
        for result in results {
            if let Some(ref tier) = result.tier {
                let entry = tier_stats.entry(tier.clone()).or_insert((0, 0));
                entry.0 += 1; // total
                if result.success {
                    entry.1 += 1; // successful
                }
            }
        }

        println!("\n🎯 Results by tier:");
        for (tier, (total, successful)) in tier_stats {
            let rate = if total > 0 {
                successful as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            println!("  {}: {}/{} ({:.1}%)", tier, successful, total, rate);
        }
    }

    // Error analysis
    let mut error_counts: HashMap<String, usize> = HashMap::new();
    for result in results.iter().filter(|r| !r.success) {
        if let Some(ref error_type) = result.error_type {
            *error_counts.entry(error_type.clone()).or_insert(0) += 1;
        }
    }

    if !error_counts.is_empty() {
        println!("\n❌ Error types:");
        for (error_type, count) in error_counts {
            println!("  {}: {}", error_type, count);
        }
    }

    // Performance insights
    let avg_time: f64 = results
        .iter()
        .filter_map(|r| r.compilation_time_ms)
        .sum::<f64>()
        / results.len() as f64;

    let optimized_count = results.iter().filter(|r| r.has_optimizations).count();

    println!("\n⚡ Performance insights:");
    println!("  Average compilation time: {:.1}ms", avg_time);
    println!(
        "  Schemas with optimizations: {}/{} ({:.1}%)",
        optimized_count,
        total,
        optimized_count as f64 / total as f64 * 100.0
    );
}

fn assert_schema_compilation_expectations(
    label: &str,
    results: &[SchemaTestResult],
    expected_failures: &[(&str, &str)],
) {
    assert!(
        !results.is_empty(),
        "No {label} schema compilation results were produced"
    );

    let expected_by_schema: HashMap<&str, &str> = expected_failures.iter().copied().collect();
    let exercised: HashSet<&str> = results
        .iter()
        .map(|result| result.schema_name.as_str())
        .collect();

    let mut unexpected_failures = Vec::new();
    let mut unexpected_successes = Vec::new();
    let mut missing_expected_failures = Vec::new();

    for result in results {
        match expected_by_schema.get(result.schema_name.as_str()) {
            Some(_) if result.success => {
                unexpected_successes.push(format!(
                    "{} unexpectedly compiled successfully",
                    result.schema_name
                ));
            }
            Some(expected_error_type) => {
                let actual_error_type = result.error_type.as_deref().unwrap_or("Unknown");
                if actual_error_type != *expected_error_type {
                    unexpected_failures.push(format!(
                        "{} instead of expected {}",
                        describe_schema_failure(result),
                        expected_error_type
                    ));
                }
            }
            None if !result.success => {
                unexpected_failures.push(describe_schema_failure(result));
            }
            None => {}
        }
    }

    for (schema_name, expected_error_type) in expected_failures {
        if !exercised.contains(schema_name) {
            missing_expected_failures.push(format!(
                "{} expected to fail with {} but was not exercised",
                schema_name, expected_error_type
            ));
        }
    }

    assert!(
        unexpected_failures.is_empty()
            && unexpected_successes.is_empty()
            && missing_expected_failures.is_empty(),
        "{label} schema compilation expectations failed.\nUnexpected failures:\n{}\nUnexpected successes for expected-failure schemas:\n{}\nMissing expected-failure schemas:\n{}",
        format_schema_expectation_list(&unexpected_failures),
        format_schema_expectation_list(&unexpected_successes),
        format_schema_expectation_list(&missing_expected_failures)
    );
}

fn describe_schema_failure(result: &SchemaTestResult) -> String {
    let error_type = result.error_type.as_deref().unwrap_or("Unknown");
    match result.error_detail.as_deref() {
        Some(detail) if !detail.is_empty() => {
            format!(
                "{} failed with {}: {}",
                result.schema_name, error_type, detail
            )
        }
        _ => format!("{} failed with {}", result.schema_name, error_type),
    }
}

fn format_schema_expectation_list(items: &[String]) -> String {
    if items.is_empty() {
        "  <none>".to_string()
    } else {
        items
            .iter()
            .map(|item| format!("  - {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Popular schemas test - must-have schemas for CI/CD (fast, reliable)
#[test]
fn test_popular_schemas() {
    let schemas = load_tiered_schemas();
    let critical_schemas: Vec<_> = schemas
        .iter()
        .filter(|(_, _, tier)| *tier == SchemaTier::Popular)
        .collect();

    assert!(
        !critical_schemas.is_empty(),
        "No critical schemas found for testing"
    );

    println!("🔥 Testing {} critical schemas", critical_schemas.len());

    let mut results = Vec::new();
    for (name, schema, tier) in critical_schemas {
        print!("Testing critical: {} ... ", name);
        let result = test_tiered_schema_compilation(schema, name, tier.clone());
        if result.success {
            println!("✅");
        } else {
            println!("❌ ({})", result.error_type.as_deref().unwrap_or("Unknown"));
        }
        results.push(result);
    }

    generate_test_report(&results);

    assert_schema_compilation_expectations("popular", &results, EXPECTED_POPULAR_SCHEMA_FAILURES);
}

/// Popular schemas round-trip test - comprehensive validation with real instances
#[cfg(feature = "round-trip-tests")]
#[test]
fn test_critical_schemas_round_trip() {
    run_tiered_round_trip_suite(SchemaTier::Popular, "critical");
}

/// Common schemas round-trip test - comprehensive validation with real instances
#[cfg(feature = "round-trip-tests")]
#[test]
fn test_important_schemas_round_trip() {
    run_tiered_round_trip_suite(SchemaTier::Common, "important");
}

#[cfg(feature = "round-trip-tests")]
fn run_tiered_round_trip_suite(schema_tier: SchemaTier, label: &str) {
    let schemas = load_tiered_schemas();
    let selected_schemas: Vec<_> = schemas
        .iter()
        .filter(|(_, _, tier)| *tier == schema_tier)
        .collect();

    assert!(
        !selected_schemas.is_empty(),
        "No {} schemas found for round-trip testing",
        label
    );

    println!(
        "🔄 Round-trip testing {} {} schemas with real instances",
        selected_schemas.len(),
        label
    );

    let mut all_results = Vec::new();

    for (name, schema, _) in selected_schemas {
        // Load instances for this schema
        let instances = load_instances_for_schema(name, label);

        if instances.is_empty() {
            println!("⚠️  No instances found for {}, skipping", name);
            continue;
        }

        println!("🧪 Testing {}: {} instances", name, instances.len());

        for (instance_name, instance) in instances {
            print!("  Round-trip {}/{}... ", instance_name, name);

            let result = test_round_trip(schema, &instance, name, &instance_name);

            if result.success {
                println!("✅");
            } else {
                println!("❌ ({})", result.error.as_deref().unwrap_or("Unknown"));
            }

            all_results.push(result);
        }
    }

    // Generate round-trip report
    generate_round_trip_report(&all_results);

    println!("\n📊 Round-trip Summary:");
    println!("  Total round-trips: {}", all_results.len());
    println!(
        "  Successful: {}",
        all_results.iter().filter(|result| result.success).count()
    );

    assert_round_trip_expectations(label, &all_results);
}

#[cfg(feature = "round-trip-tests")]
fn assert_round_trip_expectations(label: &str, results: &[RoundTripResult]) {
    assert!(
        !results.is_empty(),
        "No {} round-trip fixtures were exercised",
        label
    );

    let unsupported = UNSUPPORTED_ROUND_TRIP_FIXTURES
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();

    let mut unexpected_failures = Vec::new();
    let mut unexpected_successes = Vec::new();

    for result in results {
        let fixture = (result.schema_name.as_str(), result.instance_name.as_str());
        let is_unsupported = unsupported.contains(&fixture);

        if result.success && is_unsupported {
            unexpected_successes.push(format!("{}/{}", result.schema_name, result.instance_name));
        } else if !result.success && !is_unsupported {
            unexpected_failures.push(format!(
                "{}/{}: {}",
                result.schema_name,
                result.instance_name,
                result
                    .error
                    .as_deref()
                    .unwrap_or("Unknown round-trip failure")
            ));
        }
    }

    assert!(
        unexpected_failures.is_empty() && unexpected_successes.is_empty(),
        "{} round-trip expectations failed.\nUnexpected failures:\n{}\nUnexpected successes for allowlisted fixtures:\n{}",
        label,
        if unexpected_failures.is_empty() {
            "  <none>".to_string()
        } else {
            unexpected_failures
                .iter()
                .map(|failure| format!("  - {}", failure))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if unexpected_successes.is_empty() {
            "  <none>".to_string()
        } else {
            unexpected_successes
                .iter()
                .map(|success| format!("  - {}", success))
                .collect::<Vec<_>>()
                .join("\n")
        }
    );
}

#[cfg(feature = "round-trip-tests")]
#[test]
fn test_round_trip_expectations_accept_all_successes() {
    let results = vec![round_trip_result(
        "popular/package",
        "package/real/basic",
        true,
    )];

    assert_round_trip_expectations("test", &results);
}

#[cfg(feature = "round-trip-tests")]
#[test]
#[should_panic(expected = "Unexpected failures")]
fn test_round_trip_expectations_reject_unallowlisted_failure() {
    let results = vec![round_trip_result(
        "popular/package",
        "package/real/basic",
        false,
    )];

    assert_round_trip_expectations("test", &results);
}

#[cfg(feature = "round-trip-tests")]
fn round_trip_result(schema_name: &str, instance_name: &str, success: bool) -> RoundTripResult {
    RoundTripResult {
        schema_name: schema_name.to_string(),
        instance_name: instance_name.to_string(),
        success,
        error: (!success).then(|| "synthetic failure".to_string()),
        metric_source: RoundTripMetricSource::NotCollected,
        compression_ratio: None,
        encode_time_us: None,
        decode_time_us: None,
        token_count: None,
        json_bytes: None,
        token_bytes: None,
    }
}

/// Basic test suite - critical schemas only for fast CI/CD (legacy compatibility)
#[test]
fn test_basic_real_world_schemas() {
    // Redirect to critical schemas test for backwards compatibility
    test_popular_schemas();
}

/// Common schemas test - commonly used schemas
#[test]
fn test_important_schemas() {
    let schemas = load_tiered_schemas();
    let important_schemas: Vec<_> = schemas
        .iter()
        .filter(|(_, _, tier)| *tier == SchemaTier::Common)
        .collect();

    if important_schemas.is_empty() {
        println!("⚠️  No important schemas found, skipping test");
        return;
    }

    println!("⭐ Testing {} important schemas", important_schemas.len());

    let mut results = Vec::new();
    for (name, schema, tier) in important_schemas {
        print!("Testing important: {} ... ", name);
        let result = test_tiered_schema_compilation(schema, name, tier.clone());
        if result.success {
            println!("✅");
        } else {
            println!("❌ ({})", result.error_type.as_deref().unwrap_or("Unknown"));
        }
        results.push(result);
    }

    generate_test_report(&results);

    assert_schema_compilation_expectations("common", &results, EXPECTED_COMMON_SCHEMA_FAILURES);
}

/// Generate round-trip test report
#[cfg(feature = "round-trip-tests")]
fn generate_round_trip_report(results: &[RoundTripResult]) {
    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let success_rate = if total > 0 {
        successful as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    println!("\n🔄 === Round-Trip Test Results ===");
    println!("📊 Total round-trips tested: {}", total);
    println!("✅ Successful round-trips: {}", successful);
    println!("❌ Failed round-trips: {}", total - successful);
    println!("📈 Round-trip success rate: {:.1}%", success_rate);

    // Compression analysis
    let compressions: Vec<f64> = results.iter().filter_map(|r| r.compression_ratio).collect();

    if !compressions.is_empty() {
        let avg_compression = compressions.iter().sum::<f64>() / compressions.len() as f64;
        let best_compression = compressions.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let worst_compression = compressions.iter().fold(0.0f64, |a, &b| a.max(b));

        println!("\n📦 Compression Analysis:");
        println!("  Average compression ratio: {:.2}", avg_compression);
        println!(
            "  Best compression: {:.2} (smaller is better)",
            best_compression
        );
        println!("  Worst compression: {:.2}", worst_compression);
    }

    // Error analysis
    let mut error_counts: HashMap<String, usize> = HashMap::new();
    for result in results.iter().filter(|r| !r.success) {
        if let Some(ref error) = result.error {
            // Classify errors by type
            let error_type = if error.contains("compilation") {
                "Compilation"
            } else if error.contains("generation") {
                "CodeGeneration"
            } else if error.contains("round-trip") {
                "RoundTrip"
            } else {
                "Other"
            };
            *error_counts.entry(error_type.to_string()).or_insert(0) += 1;
        }
    }

    if !error_counts.is_empty() {
        println!("\n❌ Round-trip error breakdown:");
        for (error_type, count) in error_counts {
            println!("  {}: {}", error_type, count);
        }
    }
}

/// Specialized schemas test - additional diverse schemas for comprehensive testing
#[test]
fn test_extended_schemas() {
    let schemas = load_tiered_schemas();
    let extended_schemas: Vec<_> = schemas
        .iter()
        .filter(|(_, _, tier)| *tier == SchemaTier::Specialized)
        .collect();

    if extended_schemas.is_empty() {
        println!("⚠️  No extended schemas found, skipping test");
        return;
    }

    println!("🔬 Testing {} extended schemas", extended_schemas.len());

    let mut results = Vec::new();
    for (name, schema, tier) in extended_schemas {
        print!("Testing extended: {} ... ", name);
        let result = test_tiered_schema_compilation(schema, name, tier.clone());
        if result.success {
            println!("✅");
        } else {
            println!("❌ ({})", result.error_type.as_deref().unwrap_or("Unknown"));
        }
        results.push(result);
    }

    generate_test_report(&results);

    assert_schema_compilation_expectations(
        "specialized",
        &results,
        EXPECTED_SPECIALIZED_SCHEMA_FAILURES,
    );
}

/// Comprehensive SchemaStore test - all schemas from SchemaStore repository (feature-gated)
#[cfg(feature = "comprehensive-tests")]
#[test]
#[ignore = "report-only: clones live SchemaStore; run manually with --ignored --nocapture"]
fn test_comprehensive_schemastore() {
    use std::panic;

    // Set up panic cleanup to ensure temp directory is removed
    let _cleanup_on_panic = || {
        cleanup_schemastore();
    };

    // Use catch_unwind to ensure cleanup on panic
    let test_result = panic::catch_unwind(|| {
        // Load all schemas from SchemaStore
        let schemas = match load_schemastore_schemas() {
            Ok(schemas) => schemas,
            Err(e) => {
                panic!("Failed to load SchemaStore schemas: {}", e);
            }
        };

        assert!(
            !schemas.is_empty(),
            "No schemas found in SchemaStore repository"
        );

        println!(
            "🌍 Testing {} comprehensive schemas from SchemaStore",
            schemas.len()
        );

        let mut results = Vec::new();
        let start_time = std::time::Instant::now();

        for (i, (name, schema, tier)) in schemas.iter().enumerate() {
            // Progress indicator for large test suite
            if i % 50 == 0 {
                println!("Progress: {}/{} schemas tested", i, schemas.len());
            }

            print!("Testing comprehensive: {} ... ", name);
            let result = test_tiered_schema_compilation(schema, name, tier.clone());
            if result.success {
                println!("✅");
            } else {
                println!("❌ ({})", result.error_type.as_deref().unwrap_or("Unknown"));
            }
            results.push(result);
        }

        let total_time = start_time.elapsed();
        println!("⏱️  Total testing time: {:.2}s", total_time.as_secs_f64());

        // Generate comprehensive report
        generate_comprehensive_report(&results);

        // Save detailed results for analysis
        save_comprehensive_results(&results);

        let successful = results.iter().filter(|r| r.success).count();
        let success_rate = successful as f64 / results.len() as f64;

        println!(
            "📊 Comprehensive test success rate: {:.1}%",
            success_rate * 100.0
        );
    });

    // Always cleanup, regardless of test result
    cleanup_schemastore();

    // Re-panic if the test failed
    if let Err(panic_payload) = test_result {
        panic::resume_unwind(panic_payload);
    }
}

/// Generate comprehensive report with detailed statistics
#[cfg(feature = "comprehensive-tests")]
fn generate_comprehensive_report(results: &[SchemaTestResult]) {
    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let success_rate = if total > 0 {
        successful as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    println!("\n🌍 === Comprehensive SchemaStore Test Results ===");
    println!("📊 Total schemas tested: {}", total);
    println!("✅ Successful compilations: {}", successful);
    println!("❌ Failed compilations: {}", total - successful);
    println!("📈 Overall success rate: {:.1}%", success_rate);

    // Error analysis with counts
    let mut error_counts: HashMap<String, usize> = HashMap::new();
    let mut failed_schemas: Vec<&str> = Vec::new();

    for result in results.iter().filter(|r| !r.success) {
        failed_schemas.push(&result.schema_name);
        if let Some(ref error_type) = result.error_type {
            *error_counts.entry(error_type.clone()).or_insert(0) += 1;
        }
    }

    if !error_counts.is_empty() {
        println!("\n❌ Error breakdown:");
        let mut sorted_errors: Vec<_> = error_counts.into_iter().collect();
        sorted_errors.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count, descending

        for (error_type, count) in sorted_errors {
            let percentage = count as f64 / (total - successful) as f64 * 100.0;
            println!("  {}: {} ({:.1}%)", error_type, count, percentage);
        }
    }

    // Performance statistics
    let compilation_times: Vec<f64> = results
        .iter()
        .filter_map(|r| r.compilation_time_ms)
        .collect();

    if !compilation_times.is_empty() {
        let avg_time = compilation_times.iter().sum::<f64>() / compilation_times.len() as f64;
        let max_time = compilation_times.iter().fold(0.0f64, |a, &b| a.max(b));
        let min_time = compilation_times
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));

        println!("\n⚡ Performance statistics:");
        println!("  Average compilation time: {:.1}ms", avg_time);
        println!("  Fastest compilation: {:.1}ms", min_time);
        println!("  Slowest compilation: {:.1}ms", max_time);
    }

    let optimized_count = results.iter().filter(|r| r.has_optimizations).count();
    println!(
        "  Schemas with optimizations: {}/{} ({:.1}%)",
        optimized_count,
        total,
        optimized_count as f64 / total as f64 * 100.0
    );

    // Top 10 failed schemas (for debugging priority)
    if !failed_schemas.is_empty() {
        println!("\n🔍 Sample failed schemas (first 10):");
        for (i, schema_name) in failed_schemas.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, schema_name);
        }
        if failed_schemas.len() > 10 {
            println!("  ... and {} more", failed_schemas.len() - 10);
        }
    }
}

/// Save comprehensive results for detailed analysis
#[cfg(feature = "comprehensive-tests")]
fn save_comprehensive_results(results: &[SchemaTestResult]) {
    // Calculate error breakdown separately
    let mut error_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for result in results.iter().filter(|r| !r.success) {
        if let Some(ref error_type) = result.error_type {
            *error_counts.entry(error_type.clone()).or_insert(0) += 1;
        }
    }

    let benchmark_data = serde_json::json!({
        "test_type": "comprehensive_schemastore",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "total_schemas": results.len(),
        "successful_schemas": results.iter().filter(|r| r.success).count(),
        "success_rate": results.iter().filter(|r| r.success).count() as f64 / results.len() as f64,
        "avg_compilation_time_ms": results.iter()
            .filter_map(|r| r.compilation_time_ms)
            .sum::<f64>() / results.len() as f64,
        "schemas_with_optimizations": results.iter().filter(|r| r.has_optimizations).count(),
        "error_breakdown": error_counts,
        "detailed_results": results
    });

    write_benchmark_output(
        "comprehensive_schemastore_results.json",
        serde_json::to_string_pretty(&benchmark_data).unwrap(),
        "Comprehensive results",
    );

    // Generate compatibility matrix CSV for easier analysis
    let mut csv_content =
        String::from("schema_name,success,error_type,compilation_time_ms,has_optimizations\n");
    for result in results {
        csv_content.push_str(&format!(
            "{},{},{},{},{}\n",
            result.schema_name,
            result.success,
            result.error_type.as_deref().unwrap_or(""),
            result.compilation_time_ms.unwrap_or(0.0),
            result.has_optimizations
        ));
    }

    write_benchmark_output(
        "schema_compatibility_matrix.csv",
        csv_content,
        "Schema compatibility matrix",
    );
}

/// Specialized test suite - full tiered schema collection validation (feature-gated)
#[cfg(feature = "extended-benchmarks")]
#[test]
#[ignore = "report-only: writes extended schema benchmark output; run manually with --ignored --nocapture"]
fn test_extended_real_world_schemas() {
    let tiered_schemas = load_tiered_schemas();
    let legacy_schemas = load_schemas();

    // Combine tiered and legacy schemas for comprehensive testing
    let mut all_results = Vec::new();

    // Test tiered schemas
    if !tiered_schemas.is_empty() {
        println!("🔍 Testing {} tiered schemas", tiered_schemas.len());

        for (name, schema, tier) in tiered_schemas {
            print!("Testing tiered: {} ... ", name);
            let result = test_tiered_schema_compilation(&schema, &name, tier);
            if result.success {
                println!("✅");
            } else {
                println!("❌ ({})", result.error_type.as_deref().unwrap_or("Unknown"));
            }
            all_results.push(result);
        }
    }

    // Test legacy schemas (excluding those already in tiered structure)
    if !legacy_schemas.is_empty() {
        println!("🔍 Testing {} legacy schemas", legacy_schemas.len());

        for (name, schema, category) in legacy_schemas {
            print!("Testing legacy: {} ... ", name);
            let result = test_schema_compilation(&schema, &name, category);
            if result.success {
                println!("✅");
            } else {
                println!("❌ ({})", result.error_type.as_deref().unwrap_or("Unknown"));
            }
            all_results.push(result);
        }
    }

    assert!(
        !all_results.is_empty(),
        "No schemas found in schemas directory for extended testing"
    );

    generate_test_report(&all_results);

    let successful = all_results.iter().filter(|r| r.success).count();
    let success_rate = successful as f64 / all_results.len() as f64;
    println!(
        "📊 Extended schema compilation success rate: {:.1}%",
        success_rate * 100.0
    );

    // Save results for regression tracking
    save_benchmark_results(&all_results);
}

/// Save benchmark results for regression detection
#[cfg(feature = "extended-benchmarks")]
fn save_benchmark_results(results: &[SchemaTestResult]) {
    let benchmark_data = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "total_schemas": results.len(),
        "successful_schemas": results.iter().filter(|r| r.success).count(),
        "success_rate": results.iter().filter(|r| r.success).count() as f64 / results.len() as f64,
        "avg_compilation_time_ms": results.iter()
            .filter_map(|r| r.compilation_time_ms)
            .sum::<f64>() / results.len() as f64,
        "schemas_with_optimizations": results.iter().filter(|r| r.has_optimizations).count(),
        "results": results
    });

    write_benchmark_output(
        "latest_extended_results.json",
        serde_json::to_string_pretty(&benchmark_data).unwrap(),
        "Benchmark results",
    );
}

/// Category-specific tests for focused validation
#[test]
fn test_configuration_schemas() {
    let schemas = load_schemas();
    let config_schemas: Vec<_> = schemas
        .iter()
        .filter(|(_, _, category)| *category == SchemaCategory::Configuration)
        .collect();

    if config_schemas.is_empty() {
        println!("⚠️  No configuration schemas found, skipping category test");
        return;
    }

    let mut results = Vec::new();
    for (name, schema, category) in config_schemas {
        let result = test_schema_compilation(schema, name, category.clone());
        results.push(result);
    }

    println!("\n📋 Configuration Schema Results:");
    generate_test_report(&results);

    assert_schema_compilation_expectations(
        "configuration",
        &results,
        EXPECTED_CONFIGURATION_SCHEMA_FAILURES,
    );
}

#[test]
fn test_api_schemas() {
    let schemas = load_schemas();
    let api_schemas: Vec<_> = schemas
        .iter()
        .filter(|(_, _, category)| *category == SchemaCategory::Api)
        .collect();

    if api_schemas.is_empty() {
        println!("⚠️  No API schemas found, skipping category test");
        return;
    }

    let mut results = Vec::new();
    for (name, schema, category) in api_schemas {
        let result = test_schema_compilation(schema, name, category.clone());
        results.push(result);
    }

    println!("\n📋 API Schema Results:");
    generate_test_report(&results);

    assert_schema_compilation_expectations("api", &results, EXPECTED_API_SCHEMA_FAILURES);
}

/// Simple compilation test using SCHEMA_COUNT environment variable
/// This test is parallel to the round-trip test but only tests compilation
#[test]
fn test_schema_compilation_by_count() {
    let total_count = get_schema_count();
    let mut schemas = Vec::new();

    // Load schemas in priority order: popular → common → specialized
    for (tier_name, schema_names) in SCHEMA_TIERS {
        for schema_name in *schema_names {
            if schemas.len() >= total_count && total_count != usize::MAX {
                break;
            }

            let path = format!("schemas/{}/{}.json", tier_name, schema_name);
            if let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(schema) = serde_json::from_str::<Value>(&content)
            {
                let name = format!("{}/{}", tier_name, schema_name);
                schemas.push((name, schema));
            }
        }
        if schemas.len() >= total_count && total_count != usize::MAX {
            break;
        }
    }

    assert!(
        !schemas.is_empty(),
        "No schemas found for compilation testing"
    );

    if total_count == usize::MAX {
        println!(
            "🔧 Testing compilation of all {} available schemas",
            schemas.len()
        );
    } else {
        println!(
            "🔧 Testing compilation of {} schemas (SCHEMA_COUNT={})",
            schemas.len(),
            total_count
        );
    }

    let mut successful = 0;
    let mut failures = Vec::new();

    for (name, schema) in &schemas {
        print!("  Compiling {}... ", name);
        match TctCompiler::new(schema.clone(), "TestSchema") {
            Ok(_compiler) => {
                println!("✅");
                successful += 1;
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                failures.push(format!("{name}: {e}"));
            }
        }
    }

    println!("📊 Compilation Results:");
    println!("  Total schemas: {}", schemas.len());
    println!("  Successful: {}", successful);
    println!("  Failed: {}", failures.len());

    assert!(
        failures.is_empty(),
        "SCHEMA_COUNT compilation gate had unexpected failures:\n{}",
        format_schema_expectation_list(&failures)
    );
}
