//! Round-Trip Testing
//!
//! Tests multiple instances against a SINGLE compiled schema to avoid recompilation overhead.
//! This test:
//! 1. Compiles schema ONCE
//! 2. Generates and compiles Rust code ONCE
//! 3. Tests ALL instances against the same compiled tokenizer
//!
//! Usage:
//! - Default smoke gate: cargo test --test round_trip_test test_round_trip
//! - Single schema: SCHEMA_NAME=package cargo test --test round_trip_test test_round_trip
//! - Default schema BPE smoke: ROUND_TRIP_BPE_PRESET=legacy-tct-500 cargo test --test round_trip_test test_round_trip
//! - Full default matrix: ROUND_TRIP_FULL=1 cargo test --test round_trip_test test_round_trip

use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tct::TctCompiler;
use tct::generated_crate::{
    GENERATED_EXTERNAL_BUILD_ENV, GeneratedCargoToml, configure_generated_cargo_command,
    copy_workspace_cargo_lock,
};
use tct::provenance::sha256_hex;

mod round_trip_helpers;
use round_trip_helpers::{
    InstanceFixtureScope, build_source_schema_validator, generated_cargo_target_dir_for_crate,
    instance_search_patterns_for_schema, mixed_fixture_matches_schema,
    parse_batch_round_trip_results, schema_base_name_for,
    source_schema_external_resource_constants, source_schema_validator_function_code,
    test_batch_round_trip_with_optimization_level, write_source_schema_external_resource_files,
};

#[derive(Debug, serde::Serialize)]
struct RoundTripRunReport {
    mode: &'static str,
    default_smoke: bool,
    fixture_selection: &'static str,
    max_instances: Option<usize>,
    build_profile: &'static str,
    round_trip_scenarios_env: Option<String>,
    bpe_merges: Option<String>,
    bpe_preset: Option<String>,
    max_base_vocab: Option<String>,
    schemas: Vec<RoundTripSchemaReport>,
}

#[derive(Debug, serde::Serialize)]
struct RoundTripSchemaReport {
    schema_name: String,
    schema_folder: String,
    instance_count: usize,
    instances: Vec<String>,
    scenarios: Vec<RoundTripScenarioReport>,
}

#[derive(Debug, serde::Serialize)]
struct RoundTripScenarioReport {
    id: String,
    name: String,
    total_instances: usize,
    successful_instances: usize,
    failed_instances: Vec<RoundTripFailureReport>,
    schema_compilation_time_ms: u64,
    code_generation_time_ms: u64,
    rust_compilation_time_ms: u64,
    generated_code_sha256: String,
}

#[derive(Debug, serde::Serialize)]
struct RoundTripFailureReport {
    instance: String,
    error: String,
}

#[derive(Debug)]
struct MultiInstanceResult {
    _schema_name: String,
    total_instances: usize,
    successful_instances: usize,
    failed_instances: Vec<(String, String)>, // (instance_name, error)
    schema_compilation_time_ms: u64,
    code_generation_time_ms: u64,
    rust_compilation_time_ms: u64,
    generated_code_sha256: String,
    _per_instance_times_us: HashMap<String, (u64, u64)>, // (encode_time, decode_time)
}

#[derive(Debug, Clone)]
struct RoundTripSchema {
    name: String,
    folder: String,
}

struct RoundTripPlan {
    schemas: Vec<RoundTripSchema>,
    default_smoke: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildProfile {
    Debug,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureSelection {
    DefaultSmokeManifest,
    MaxInstances(usize),
    All,
}

const DEFAULT_ROUND_TRIP_SCHEMAS: &[(&str, &str)] = &[
    ("kubernetes", "popular"),
    ("eslintrc", "popular"),
    ("tsconfig", "popular"),
];

const DEFAULT_ROUND_TRIP_MAX_INSTANCES: usize = 3;
const LEGACY_TCT_500_BPE_PRESET: &str = "legacy-tct-500";
const FOCUSED_ROUND_TRIP_BPE_SMOKE_SCHEMAS: &[(&str, &str)] =
    &[("github-workflow", "popular"), ("package", "popular")];
const DEFAULT_SMOKE_FIXTURES: &[(&str, &str, &[&str])] = &[
    (
        "kubernetes",
        "popular",
        &[
            "kubernetes/synthetic/public_apiservice.json",
            "kubernetes/synthetic/public_deployment.json",
            "kubernetes/synthetic/public_service.json",
        ],
    ),
    (
        "eslintrc",
        "popular",
        &[
            "eslintrc/synthetic/public_minimal.json",
            "eslintrc/synthetic/public_overrides.json",
            "eslintrc/synthetic/public_rule_options.json",
        ],
    ),
    (
        "tsconfig",
        "popular",
        &[
            "tsconfig/synthetic/public_strict.json",
            "tsconfig/apollographql_apolloserver_tsconfig.json",
            "tsconfig/nestjs_nest_tsconfig.json",
        ],
    ),
];

const FOCUSED_SMOKE_FIXTURES: &[(&str, &str, &[&str])] = &[
    (
        "package",
        "popular",
        &[
            "package/synthetic/public_cli.json",
            "package/synthetic/public_library.json",
            "package/real/vscode.json",
        ],
    ),
    (
        "github-workflow",
        "popular",
        &[
            "github-workflow/real/actions_checkout_test.json",
            "github-workflow/real/complete.json",
            "github-workflow/real/partial.json",
        ],
    ),
    (
        "cargo",
        "popular",
        &[
            "cargo/real/serde.json",
            "cargo/synthetic/public_workspace.json",
            "cargo/real/tokio.json",
        ],
    ),
    (
        "docker-compose",
        "popular",
        &[
            "docker-compose/synthetic/public_gateway.json",
            "docker-compose/synthetic/public_worker.json",
            "docker-compose/synthetic/webapp.json",
        ],
    ),
    (
        "appsettings",
        "popular",
        &[
            "appsettings/synthetic/public_http_logging.json",
            "appsettings/synthetic/public_database.json",
            "appsettings/synthetic/development.json",
        ],
    ),
    (
        "commitlintrc",
        "popular",
        &[
            "commitlintrc/synthetic/public_rules.json",
            "commitlintrc/synthetic/basic.json",
        ],
    ),
    (
        "jest",
        "popular",
        &[
            "jest/synthetic/public_unit.json",
            "jest/synthetic/public_projects.json",
            "jest/synthetic/basic.json",
        ],
    ),
    (
        "ava",
        "popular",
        &[
            "ava/synthetic/public_serial.json",
            "ava/synthetic/public_typescript.json",
            "ava/synthetic/comprehensive-config.json",
        ],
    ),
    (
        "babelrc",
        "popular",
        &[
            "babelrc/synthetic/public_plugins.json",
            "babelrc/synthetic/public_environments.json",
            "babelrc/synthetic/react.json",
        ],
    ),
    (
        "cypress",
        "popular",
        &[
            "cypress/synthetic/public_e2e.json",
            "cypress/synthetic/public_component.json",
            "cypress/synthetic/basic.json",
        ],
    ),
    (
        "codecov",
        "popular",
        &[
            "codecov/synthetic/public_status.json",
            "codecov/synthetic/basic.json",
        ],
    ),
    (
        "api-response",
        "api",
        &[
            "api-response/synthetic/public_success.json",
            "api-response/synthetic/public_error.json",
        ],
    ),
    (
        "microservice-config",
        "configuration",
        &[
            "microservice-config/synthetic/public_minimal.json",
            "microservice-config/synthetic/public_featured.json",
        ],
    ),
];

impl BuildProfile {
    fn from_env() -> Self {
        match std::env::var("ROUND_TRIP_BUILD_PROFILE") {
            Ok(value) => match value.as_str() {
                "debug" => Self::Debug,
                "release" => Self::Release,
                _ => panic!(
                    "Invalid ROUND_TRIP_BUILD_PROFILE value: {} (expected debug or release)",
                    value
                ),
            },
            Err(_) => Self::Debug,
        }
    }

    fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &["build", "--bin", "test_tokenizer"],
            Self::Release => &["build", "--release", "--bin", "test_tokenizer"],
        }
    }

    fn target_dir(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }
}

impl FixtureSelection {
    fn max_instances(self) -> Option<usize> {
        match self {
            Self::DefaultSmokeManifest => Some(DEFAULT_ROUND_TRIP_MAX_INSTANCES),
            Self::MaxInstances(max_instances) => Some(max_instances),
            Self::All => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::DefaultSmokeManifest => "default_smoke_manifest",
            Self::MaxInstances(_) => "max_instances",
            Self::All => "all",
        }
    }
}

impl MultiInstanceResult {
    fn success_rate(&self) -> f64 {
        if self.total_instances == 0 {
            0.0
        } else {
            self.successful_instances as f64 / self.total_instances as f64
        }
    }
}

/// Verify that BASE_VOCAB_SIZE and VOCAB_SIZE constants are present and correct.
fn verify_vocabulary_constants(code: &str, grammar: &tct::Grammar, has_bpe: bool) {
    println!("\n🔍 Verifying vocabulary constants...");

    // Extract BASE_VOCAB_SIZE
    let base_vocab_size = extract_constant_value(code, "BASE_VOCAB_SIZE");
    println!("  • BASE_VOCAB_SIZE: {}", base_vocab_size);

    // Extract VOCAB_SIZE
    let vocab_size = extract_constant_value(code, "VOCAB_SIZE");
    println!("  • VOCAB_SIZE: {}", vocab_size);

    let bpe_merges = grammar
        .bpe_merge_table
        .as_ref()
        .map(|b| b.merges.len() as u32)
        .unwrap_or(0);

    assert_eq!(
        base_vocab_size, grammar.base_vocab,
        "generated BASE_VOCAB_SIZE should match Grammar.base_vocab"
    );

    let base_vocab = grammar.base_vocab;

    let bpe_start = base_vocab;
    let total_base = bpe_start + bpe_merges;

    if has_bpe {
        let generated_bpe_start = extract_constant_value(code, "BPE_START");
        println!("  • BPE_START: {}", generated_bpe_start);
        assert_eq!(
            generated_bpe_start, bpe_start,
            "BPE_START mismatch: got {}, expected {} (base={})",
            generated_bpe_start, bpe_start, base_vocab_size
        );
    }

    // VOCAB_SIZE = base_vocab + bpe_merges + PAD
    let expected_vocab_size = total_base + 1;

    if has_bpe {
        println!("  • BPE merges: {}", bpe_merges);
    }

    println!("  • Expected VOCAB_SIZE: {}", expected_vocab_size);

    // Verify VOCAB_SIZE matches expected
    assert_eq!(
        vocab_size, expected_vocab_size,
        "VOCAB_SIZE mismatch: got {}, expected {} (base={}, bpe={})",
        vocab_size, expected_vocab_size, base_vocab_size, bpe_merges
    );

    println!("  ✅ Vocabulary constants verified!");
}

/// Extract constant value from generated code
fn extract_constant_value(code: &str, constant_name: &str) -> u32 {
    // Try different patterns to be more robust
    let patterns = vec![
        format!("pub const {}: u32", constant_name),
        format!("pub const {} : u32", constant_name),
        format!("const {}: u32", constant_name),
        format!("const {} : u32", constant_name),
    ];

    for pattern in &patterns {
        if let Some(line) = code.lines().find(|line| line.contains(pattern))
            && let Some(value) = line
                .split('=')
                .nth(1)
                .and_then(|part| part.split(';').next())
                .map(|s| s.trim())
                .map(|s| s.trim_end_matches("u32")) // Remove type suffix
                .and_then(|num| num.parse::<u32>().ok())
        {
            return value;
        }
    }

    panic!("{} constant not found in generated code", constant_name)
}

fn default_smoke_fixture_paths(schema_name: &str, tier: &str) -> Option<&'static [&'static str]> {
    let schema_base_name = schema_base_name_for(schema_name);

    DEFAULT_SMOKE_FIXTURES
        .iter()
        .chain(FOCUSED_SMOKE_FIXTURES.iter())
        .find(|(manifest_schema, manifest_tier, _)| {
            *manifest_schema == schema_base_name && *manifest_tier == tier
        })
        .map(|(_, _, paths)| *paths)
}

fn instance_type_for_fixture_path(relative_path: &str) -> &'static str {
    if relative_path.contains("/synthetic/") {
        "synthetic"
    } else if relative_path.contains("/large-scale/") {
        "large-scale"
    } else if relative_path.contains("/real/") {
        "real"
    } else {
        "mixed"
    }
}

/// Load instances for a schema from various directories
fn load_instances_for_schema(
    schema_name: &str,
    tier: &str,
    fixture_selection: FixtureSelection,
) -> Vec<(String, Value)> {
    let mut instances = Vec::new();
    let mut real_count = 0;
    let mut synthetic_count = 0;
    let mut invalid_count = 0;
    let mut candidate_paths: Vec<(PathBuf, &'static str)> = Vec::new();

    // Extract just the schema name from "tier/name" format
    let schema_base_name = schema_base_name_for(schema_name);

    println!(
        "📂 Loading instances for schema: {} (tier: {})",
        schema_base_name, tier
    );

    let fixture_scope = if fixture_selection == FixtureSelection::All {
        InstanceFixtureScope::IncludeLargeScale
    } else {
        InstanceFixtureScope::Standard
    };
    let search_patterns = instance_search_patterns_for_schema(schema_name, tier, fixture_scope);

    let explicit_smoke_paths = if fixture_selection == FixtureSelection::DefaultSmokeManifest {
        default_smoke_fixture_paths(schema_name, tier)
    } else {
        None
    };

    if let Some(smoke_paths) = explicit_smoke_paths {
        println!(
            "  📌 Loading {} explicit default smoke fixtures",
            smoke_paths.len()
        );
        for relative_path in smoke_paths {
            let path = Path::new("tests/data/instances").join(relative_path);
            println!("    • {}", path.display());
            candidate_paths.push((path, instance_type_for_fixture_path(relative_path)));
        }
    } else {
        if fixture_selection == FixtureSelection::DefaultSmokeManifest {
            println!(
                "  ⚠️ No explicit smoke fixture manifest for {}/{}; falling back to first {} discovered fixtures",
                tier, schema_base_name, DEFAULT_ROUND_TRIP_MAX_INSTANCES
            );
        }

        for (instances_dir, instance_type) in search_patterns {
            println!("  🔍 Searching in: {}", instances_dir);

            if let Ok(dir) = fs::read_dir(&instances_dir) {
                let mut found_in_dir = 0;
                let mut paths = dir.flatten().map(|entry| entry.path()).collect::<Vec<_>>();
                paths.sort();

                for path in paths {
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        let filename = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");

                        if filename == "metadata" {
                            continue;
                        }

                        // For direct JSON files in schema directory, filter by schema-bearing names.
                        if instance_type == "mixed"
                            && !mixed_fixture_matches_schema(filename, schema_base_name)
                        {
                            continue;
                        }

                        candidate_paths.push((path, instance_type));
                        found_in_dir += 1;
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

        candidate_paths.sort_by(|(left, _), (right, _)| left.cmp(right));

        if let Some(max) = fixture_selection.max_instances()
            && candidate_paths.len() > max
        {
            println!("  🔧 Limiting to {} instances", max);
            candidate_paths.truncate(max);
        }
    }

    for (path, instance_type) in candidate_paths {
        match fs::read_to_string(&path) {
            Ok(content) => {
                // Check for 404 errors (defensive)
                if content.trim() == "404: Not Found" {
                    println!("    ⚠️ Skipping 404 error file: {}", path.display());
                    invalid_count += 1;
                    continue;
                }

                match serde_json::from_str::<Value>(&content) {
                    Ok(instance) => {
                        let name_path = path.strip_prefix("tests/data/instances").unwrap_or(&path);
                        let mut name_path = name_path.to_path_buf();
                        name_path.set_extension("");
                        let name = name_path
                            .to_string_lossy()
                            .trim_start_matches(std::path::MAIN_SEPARATOR)
                            .to_string();
                        instances.push((name.clone(), instance));

                        if instance_type == "real" {
                            real_count += 1;
                        } else if instance_type == "synthetic" || instance_type == "nested" {
                            synthetic_count += 1;
                        } else {
                            real_count += 1;
                        }

                        println!("    ✅ Loaded {} instance: {}", instance_type, name);
                    }
                    Err(_) => {
                        println!("    ❌ Invalid JSON in: {}", path.display());
                        invalid_count += 1;
                    }
                }
            }
            Err(_) => {
                println!("    ❌ Could not read file: {}", path.display());
                invalid_count += 1;
            }
        }
    }

    let mut seen_names = HashSet::new();
    for (name, _) in &instances {
        assert!(
            seen_names.insert(name.clone()),
            "Duplicate round-trip instance id after fixture discovery: {}",
            name
        );
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

/// Test scenarios for comprehensive validation
#[derive(Debug, Clone, Copy)]
struct TestScenario {
    id: &'static str,
    name: &'static str,
    opt_level: tct::OptimizationLevel,
}

const TEST_SCENARIOS: &[TestScenario] = &[
    TestScenario {
        id: "o0",
        name: "O0 (no optimization)",
        opt_level: tct::OptimizationLevel::O0,
    },
    TestScenario {
        id: "o2",
        name: "O2 (full optimization)",
        opt_level: tct::OptimizationLevel::O2,
    },
    TestScenario {
        id: "o3",
        name: "O3 (experimental bit-packing)",
        opt_level: tct::OptimizationLevel::O3,
    },
    TestScenario {
        id: "os",
        name: "Os (size profile)",
        opt_level: tct::OptimizationLevel::Os,
    },
    TestScenario {
        id: "ovocab",
        name: "OVocab (vocabulary profile)",
        opt_level: tct::OptimizationLevel::OVocab,
    },
];

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

fn round_trip_plan() -> RoundTripPlan {
    let default_smoke = !env_flag("ROUND_TRIP_FULL");

    if let Ok(schema_name) = std::env::var("SCHEMA_NAME") {
        let schema_folder =
            std::env::var("SCHEMA_FOLDER").unwrap_or_else(|_| "popular".to_string());
        let mode = if default_smoke {
            "focused smoke"
        } else {
            "focused full"
        };
        println!(
            "ℹ️  SCHEMA_NAME set; running {} round-trip schema: {}",
            mode, schema_name
        );
        return RoundTripPlan {
            schemas: vec![RoundTripSchema {
                name: schema_name,
                folder: schema_folder,
            }],
            default_smoke,
        };
    }

    if std::env::var("SCHEMA_FOLDER").is_ok() {
        println!(
            "ℹ️  SCHEMA_FOLDER is ignored unless SCHEMA_NAME is set; using default round-trip schemas"
        );
    }

    let mode = if default_smoke {
        "default smoke"
    } else {
        "full default"
    };

    println!(
        "ℹ️  SCHEMA_NAME not set; running {} round-trip schemas: {}",
        mode,
        DEFAULT_ROUND_TRIP_SCHEMAS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(", ")
    );

    RoundTripPlan {
        schemas: DEFAULT_ROUND_TRIP_SCHEMAS
            .iter()
            .map(|(name, folder)| RoundTripSchema {
                name: (*name).to_string(),
                folder: (*folder).to_string(),
            })
            .collect(),
        default_smoke,
    }
}

fn fixture_selection_for_run(default_smoke: bool) -> FixtureSelection {
    match std::env::var("MAX_INSTANCES") {
        Ok(value) => FixtureSelection::MaxInstances(
            value
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("Invalid MAX_INSTANCES value: {}", value)),
        ),
        Err(_) if default_smoke => FixtureSelection::DefaultSmokeManifest,
        Err(_) => FixtureSelection::All,
    }
}

fn scenarios_to_test(default_smoke: bool, has_compression: bool) -> Vec<&'static TestScenario> {
    if let Ok(value) = std::env::var("ROUND_TRIP_SCENARIOS") {
        let requested = value
            .split(',')
            .map(|part| part.trim().to_ascii_lowercase())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if requested.iter().any(|part| part == "all") {
            return TEST_SCENARIOS.iter().collect::<Vec<_>>();
        }

        let scenarios = requested
            .iter()
            .map(|id| {
                TEST_SCENARIOS
                    .iter()
                    .find(|scenario| scenario.id == id)
                    .unwrap_or_else(|| {
                        panic!(
                            "Invalid ROUND_TRIP_SCENARIOS entry: {} (expected o0, o2, o3, os, ovocab, or all)",
                            id
                        )
                    })
            })
            .collect::<Vec<_>>();

        if scenarios.is_empty() {
            panic!("ROUND_TRIP_SCENARIOS did not select any scenarios");
        }

        return scenarios;
    }

    if has_compression {
        println!("ℹ️  BPE detected; using the selected round-trip scenario set");
    }

    if default_smoke {
        TEST_SCENARIOS
            .iter()
            .filter(|s| s.id == "o3")
            .collect::<Vec<_>>()
    } else {
        TEST_SCENARIOS.iter().collect::<Vec<_>>()
    }
}

fn bpe_merges_path_for_schema(schema_name: &str) -> Option<String> {
    resolve_bpe_merges_path(
        schema_name,
        std::env::var("BPE_MERGES").ok().as_deref(),
        std::env::var("ROUND_TRIP_BPE_PRESET").ok().as_deref(),
    )
}

fn resolve_bpe_merges_path(
    schema_name: &str,
    explicit_bpe_merges: Option<&str>,
    bpe_preset: Option<&str>,
) -> Option<String> {
    match (explicit_bpe_merges, bpe_preset) {
        (Some(_), Some(_)) => {
            panic!("BPE_MERGES and ROUND_TRIP_BPE_PRESET are mutually exclusive")
        }
        (Some(path), None) => Some(path.to_string()),
        (None, Some(LEGACY_TCT_500_BPE_PRESET)) => {
            let path = format!("schemas/bpe/{schema_name}-tct-bpe-500.json");
            if !Path::new(&path).is_file() {
                panic!(
                    "ROUND_TRIP_BPE_PRESET={LEGACY_TCT_500_BPE_PRESET} missing merge table: {path}"
                );
            }
            Some(path)
        }
        (None, Some(value)) => {
            panic!(
                "Invalid ROUND_TRIP_BPE_PRESET value: {value} (expected {LEGACY_TCT_500_BPE_PRESET})"
            )
        }
        (None, None) => None,
    }
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
        "schema".to_string()
    } else {
        sanitized
    }
}

fn write_if_changed(path: impl AsRef<Path>, contents: &str) {
    let path = path.as_ref();
    if fs::read_to_string(path)
        .map(|existing| existing == contents)
        .unwrap_or(false)
    {
        return;
    }

    fs::write(path, contents)
        .unwrap_or_else(|e| panic!("Failed to write {}: {}", path.display(), e));
}

/// Test multiple instances of a single schema efficiently
#[test]
fn test_round_trip() {
    let plan = round_trip_plan();
    let fixture_selection = fixture_selection_for_run(plan.default_smoke);
    let build_profile = BuildProfile::from_env();
    let mut report = RoundTripRunReport {
        mode: if plan.default_smoke { "smoke" } else { "full" },
        default_smoke: plan.default_smoke,
        fixture_selection: fixture_selection.id(),
        max_instances: fixture_selection.max_instances(),
        build_profile: build_profile.id(),
        round_trip_scenarios_env: std::env::var("ROUND_TRIP_SCENARIOS").ok(),
        bpe_merges: std::env::var("BPE_MERGES").ok(),
        bpe_preset: std::env::var("ROUND_TRIP_BPE_PRESET").ok(),
        max_base_vocab: std::env::var("MAX_BASE_VOCAB").ok(),
        schemas: Vec::new(),
    };

    if plan.default_smoke {
        match fixture_selection {
            FixtureSelection::DefaultSmokeManifest => println!(
                "ℹ️  Default smoke gate uses O3, explicit fixture manifests, and {:?} generated builds",
                build_profile
            ),
            FixtureSelection::MaxInstances(max_instances) => println!(
                "ℹ️  Default smoke gate uses O3, the first {} discovered instances per schema, and {:?} generated builds",
                max_instances, build_profile
            ),
            FixtureSelection::All => println!(
                "ℹ️  Default smoke gate uses O3, all discovered instances, and {:?} generated builds",
                build_profile
            ),
        }
        println!("   Set ROUND_TRIP_FULL=1 for all instances and all scenarios.");
    }
    println!(
        "   Set ROUND_TRIP_BUILD_PROFILE=release to compile generated crates in release mode."
    );

    for schema in plan.schemas {
        let schema_report = run_round_trip_for_schema(
            &schema.name,
            &schema.folder,
            fixture_selection,
            plan.default_smoke,
            build_profile,
        );
        report.schemas.push(schema_report);
    }

    write_round_trip_report(&report);
    assert_round_trip_report_success(&report);
}

#[test]
fn test_named_optimization_profiles_generated_round_trip() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "timestamp": {"type": "string", "format": "date-time"},
            "status": {"enum": ["active", "inactive"]},
            "optional_a": {"type": "string"},
            "optional_b": {"type": "string"},
            "optional_c": {"type": "string"},
            "optional_d": {"type": "string"}
        },
        "required": ["timestamp", "status"]
    });
    let instances = vec![
        (
            "minimal".to_string(),
            json!({"timestamp": "not-a-rfc3339-date", "status": "active"}),
        ),
        (
            "with_optionals".to_string(),
            json!({
                "timestamp": "still-annotation-only",
                "status": "inactive",
                "optional_a": "a",
                "optional_c": "c"
            }),
        ),
    ];

    for optimization_level in [
        tct::OptimizationLevel::Os,
        tct::OptimizationLevel::O3,
        tct::OptimizationLevel::OVocab,
    ] {
        let summary = test_batch_round_trip_with_optimization_level(
            &schema,
            &instances,
            "named_profile_smoke",
            optimization_level,
        )
        .unwrap_or_else(|error| {
            panic!("{optimization_level} generated round trip failed: {error}")
        });

        assert_eq!(summary.total_instances, instances.len());
        assert_eq!(
            summary.successful_instances,
            instances.len(),
            "{optimization_level} generated round trip failures: {:?}",
            summary.failed_instances
        );
    }
}

fn run_round_trip_for_schema(
    schema_name: &str,
    schema_folder: &str,
    fixture_selection: FixtureSelection,
    default_smoke: bool,
    build_profile: BuildProfile,
) -> RoundTripSchemaReport {
    // Check if BPE is being used
    let has_compression =
        std::env::var("BPE_MERGES").is_ok() || std::env::var("ROUND_TRIP_BPE_PRESET").is_ok();
    let scenarios_to_test = scenarios_to_test(default_smoke, has_compression);

    println!(
        "\n🧪 Testing schema: {} from folder: {} with {} scenarios",
        schema_name,
        schema_folder,
        scenarios_to_test.len()
    );
    println!("═══════════════════════════════════════════════════════════════");

    // Load schema ONCE
    let schema_path = format!("schemas/{}/{}.json", schema_folder, schema_name);
    let schema_content = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read schema: {}", schema_path));
    let schema: Value = serde_json::from_str(&schema_content)
        .unwrap_or_else(|_| panic!("Failed to parse schema: {}", schema_path));

    println!("✅ Schema loaded from {}", schema_path);

    // Load all instances ONCE
    let instances = load_instances_for_schema(schema_name, schema_folder, fixture_selection);
    if instances.is_empty() {
        panic!("No instances found for schema: {}", schema_name);
    }

    println!(
        "✅ Found {} instances for {}\n",
        instances.len(),
        schema_name
    );

    // Test each scenario
    let mut all_results = Vec::new();

    for (scenario_idx, scenario) in scenarios_to_test.iter().enumerate() {
        println!(
            "\n📋 SCENARIO {}/{}: {}",
            scenario_idx + 1,
            scenarios_to_test.len(),
            scenario.name
        );
        println!("───────────────────────────────────────────────────────────────");

        let result = test_scenario(schema_name, &schema, &instances, scenario, build_profile);
        all_results.push((scenario, result));
    }

    // Print comprehensive summary
    println!("\n");
    println!("═══════════════════════════════════════════════════════════════");
    println!("📊 COMPREHENSIVE TEST RESULTS for {}", schema_name);
    println!("═══════════════════════════════════════════════════════════════");

    for (scenario, result) in &all_results {
        println!(
            "\n{}: {:.1}% ({}/{})",
            scenario.name,
            result.success_rate() * 100.0,
            result.successful_instances,
            result.total_instances
        );
        println!(
            "  Compilation: schema={}ms, code={}ms, rust={}ms",
            result.schema_compilation_time_ms,
            result.code_generation_time_ms,
            result.rust_compilation_time_ms
        );
    }

    let schema_passed = all_results.iter().all(|(_, result)| {
        result.successful_instances == result.total_instances && result.failed_instances.is_empty()
    });
    if schema_passed {
        println!("\n✅ All scenarios passed for schema: {}", schema_name);
    } else {
        println!(
            "\n❌ Completed schema {} with one or more failed scenarios",
            schema_name
        );
    }
    println!("═══════════════════════════════════════════════════════════════\n");

    RoundTripSchemaReport {
        schema_name: schema_name.to_string(),
        schema_folder: schema_folder.to_string(),
        instance_count: instances.len(),
        instances: instances.iter().map(|(name, _)| name.clone()).collect(),
        scenarios: all_results
            .into_iter()
            .map(|(scenario, result)| RoundTripScenarioReport {
                id: scenario.id.to_string(),
                name: scenario.name.to_string(),
                total_instances: result.total_instances,
                successful_instances: result.successful_instances,
                failed_instances: result
                    .failed_instances
                    .into_iter()
                    .map(|(instance, error)| RoundTripFailureReport { instance, error })
                    .collect(),
                schema_compilation_time_ms: result.schema_compilation_time_ms,
                code_generation_time_ms: result.code_generation_time_ms,
                rust_compilation_time_ms: result.rust_compilation_time_ms,
                generated_code_sha256: result.generated_code_sha256,
            })
            .collect(),
    }
}

fn write_round_trip_report(report: &RoundTripRunReport) {
    fs::create_dir_all("target")
        .unwrap_or_else(|e| panic!("Failed to create target directory for report: {}", e));
    let report_path = Path::new("target/tct-round-trip-report.json");
    let report_json = serde_json::to_string_pretty(report)
        .unwrap_or_else(|e| panic!("Failed to serialize round-trip report: {}", e));
    fs::write(report_path, report_json)
        .unwrap_or_else(|e| panic!("Failed to write {}: {}", report_path.display(), e));
    println!("📝 Wrote round-trip report to {}", report_path.display());
}

fn assert_round_trip_report_success(report: &RoundTripRunReport) {
    for schema in &report.schemas {
        for scenario in &schema.scenarios {
            assert!(
                scenario.successful_instances == scenario.total_instances
                    && scenario.failed_instances.is_empty(),
                "{} / {} expected all {} instances to pass, but {} passed and {} failed: {:?}",
                schema.schema_name,
                scenario.name,
                scenario.total_instances,
                scenario.successful_instances,
                scenario.failed_instances.len(),
                scenario.failed_instances
            );
        }
    }
}

fn test_scenario(
    schema_name: &str,
    schema: &Value,
    instances: &[(String, Value)],
    scenario: &TestScenario,
    build_profile: BuildProfile,
) -> MultiInstanceResult {
    // PHASE 1: Compile schema
    println!("\n🔧 Phase 1: Compiling schema...");
    println!("  Optimization: {:?}", scenario.opt_level);

    let schema_compilation_start = Instant::now();

    let mut compiler = TctCompiler::new(schema.clone(), "TestType")
        .unwrap_or_else(|e| panic!("Failed to create compiler for {}: {}", schema_name, e))
        .with_optimization_level(scenario.opt_level);

    // Apply BPE if specified
    let has_bpe = if let Some(bpe_path) = bpe_merges_path_for_schema(schema_name) {
        println!("  BPE merges: {}", bpe_path);
        compiler = compiler
            .with_bpe_merges(&bpe_path)
            .unwrap_or_else(|e| panic!("Failed to load BPE merges {}: {}", bpe_path, e));
        true
    } else {
        println!("  BPE merges: None");
        false
    };

    // Apply max_base_vocab if specified (P1-2)
    if let Ok(max_base_vocab_str) = std::env::var("MAX_BASE_VOCAB") {
        let max_base_vocab: u32 = max_base_vocab_str
            .parse()
            .unwrap_or_else(|_| panic!("Invalid MAX_BASE_VOCAB value: {}", max_base_vocab_str));
        println!("  MAX_BASE_VOCAB: {}", max_base_vocab);
        compiler = compiler
            .with_max_base_vocab(max_base_vocab)
            .unwrap_or_else(|e| panic!("Failed to set max_base_vocab {}: {}", max_base_vocab, e));
    } else {
        println!("  MAX_BASE_VOCAB: None (using default 256)");
    }

    let grammar = compiler
        .compile()
        .unwrap_or_else(|e| panic!("Failed to compile schema {}: {}", schema_name, e));

    let schema_compilation_time = schema_compilation_start.elapsed();
    println!(
        "✅ Schema compilation completed in {:?}",
        schema_compilation_time
    );

    // PHASE 2: Generate Rust code ONCE
    println!("🔧 Phase 2: Generating Rust code (ONCE)...");
    let code_gen_start = Instant::now();

    let generated_code = grammar
        .generate_code()
        .unwrap_or_else(|e| panic!("Failed to generate code for {}: {}", schema_name, e));

    let code_generation_time = code_gen_start.elapsed();
    let code_str = generated_code.to_string();
    let generated_code_sha256 = sha256_hex(code_str.as_bytes());
    println!("✅ Code generation completed in {:?}", code_generation_time);
    println!("📊 Generated code size: {} bytes", code_str.len());
    println!("📊 Generated code SHA-256: {}", generated_code_sha256);

    // Verify vocabulary constants are present and correct
    verify_vocabulary_constants(&code_str, &grammar, has_bpe);

    // PHASE 3: Compile Rust code
    println!("\n🔧 Phase 3: Compiling Rust code...");
    println!("  Build profile: {:?}", build_profile);
    let rust_compilation_start = Instant::now();

    // Create temporary directory for this test
    let schema_dir = sanitize_path_component(schema_name);
    let temp_dir = format!(
        "/tmp/tct_efficient_test_{}_{}_{}",
        schema_dir,
        scenario.id,
        build_profile.id()
    );
    fs::create_dir_all(&temp_dir)
        .unwrap_or_else(|e| panic!("Failed to create temp dir {}: {}", temp_dir, e));

    // Write Cargo.toml - link against TCT for tokenizer support
    let tct_path = std::env::current_dir().unwrap();
    let cargo_toml = GeneratedCargoToml::new(format!(
        "efficient-test-{}-{}-{}",
        schema_dir,
        scenario.id,
        build_profile.id()
    ))
    .with_regex()
    .with_jsonschema()
    .with_rustc_hash()
    .with_chrono()
    .with_base64()
    .with_tct_path(tct_path.display().to_string())
    .with_bin("test_tokenizer", "src/main.rs")
    .render();

    write_if_changed(format!("{}/Cargo.toml", temp_dir), &cargo_toml);
    copy_workspace_cargo_lock(&tct_path, Path::new(&temp_dir))
        .expect("failed to copy workspace Cargo.lock into generated round-trip crate");
    let source_schema_json = serde_json::to_string_pretty(schema)
        .unwrap_or_else(|e| panic!("Failed to serialize source schema: {}", e));
    write_if_changed(
        format!("{}/source-schema.json", temp_dir),
        &source_schema_json,
    );
    write_source_schema_external_resource_files(&temp_dir)
        .unwrap_or_else(|error| panic!("Failed to write external schema resources: {}", error));
    let schema_name_literal = serde_json::to_string(schema_name)
        .unwrap_or_else(|e| panic!("Failed to serialize schema name: {}", e));

    // Create src directory
    fs::create_dir_all(format!("{}/src", temp_dir))
        .unwrap_or_else(|e| panic!("Failed to create src dir: {}", e));

    // Comment out log_debug! macro calls (multi-line aware)
    let mut in_log = false;
    let mut lines = Vec::new();
    for line in code_str.lines() {
        let t = line.trim_start();
        if t.starts_with("log_debug!(") {
            in_log = true;
            lines.push(format!("// {}", line));
            if t.contains(");") {
                in_log = false;
            }
        } else if in_log {
            lines.push(format!("// {}", line));
            if t.ends_with(");") {
                in_log = false;
            }
        } else {
            lines.push(line.to_string());
        }
    }
    let code_str = lines.join("\n");

    // Generated code is standalone - includes tokenizer module
    let source_schema_external_resources = source_schema_external_resource_constants("../");
    let source_schema_validator_function = source_schema_validator_function_code();
    let main_rs = format!(
        r#"{}
use serde_json::{{Value, json}};
use std::io::{{self, Read}};
use crate::tokenizer::JsonTokenEncoder;

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
        .map_err(|e| format!("generated type parse failed: {{}}", e))?;

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
        .map_err(|e| format!("decode failed: {{}}", e))?;
    let decode_time = decode_start.elapsed();

    let decoded_json = serde_json::to_value(decoded_instance)
        .map_err(|e| format!("decoded JSON serialization failed: {{}}", e))?;
    validate_source_schema(
        source_schema_validator,
        &decoded_json,
        fixture_name,
        "after decode",
        "decoded_context",
    )?;

    if json_value == decoded_json {{
        Ok((
            u64::try_from(encode_time.as_nanos()).unwrap_or(u64::MAX),
            u64::try_from(decode_time.as_nanos()).unwrap_or(u64::MAX),
            token_count,
            max_token,
        ))
    }} else {{
        Err(format!(
            "Original and decoded JSON do not match\nOriginal: {{}}\nDecoded: {{}}",
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| json_value.to_string()),
            serde_json::to_string_pretty(&decoded_json).unwrap_or_else(|_| decoded_json.to_string())
        ))
    }}
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
"#,
        code_str
    );

    write_if_changed(format!("{}/src/main.rs", temp_dir), &main_rs);

    let generated_target_dir = generated_cargo_target_dir_for_crate(Path::new(&temp_dir));
    fs::create_dir_all(&generated_target_dir).unwrap_or_else(|e| {
        panic!(
            "Failed to create generated Cargo target dir {}: {}",
            generated_target_dir.display(),
            e
        )
    });
    println!(
        "  Generated Cargo target dir: {}",
        generated_target_dir.display()
    );

    // Compile the Rust project
    let mut build_command = std::process::Command::new("cargo");
    configure_generated_cargo_command(&mut build_command, &tct_path);
    build_command
        .args(build_profile.cargo_args())
        .current_dir(&temp_dir)
        .env(GENERATED_EXTERNAL_BUILD_ENV, "1");
    build_command.env("CARGO_TARGET_DIR", &generated_target_dir);
    let compile_output = build_command
        .output()
        .unwrap_or_else(|e| panic!("Failed to run cargo build: {}", e));

    let rust_compilation_time = rust_compilation_start.elapsed();

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        panic!("Rust compilation failed for {}:\n{}", schema_name, stderr);
    }

    println!(
        "✅ Rust compilation completed in {:?}",
        rust_compilation_time
    );

    // PHASE 4: Test ALL instances against the compiled tokenizer
    println!(
        "🔧 Phase 4: Testing {} instances (EFFICIENT)...",
        instances.len()
    );

    let binary_path = generated_target_dir
        .join(build_profile.target_dir())
        .join("test_tokenizer");

    let batch = instances
        .iter()
        .map(|(name, instance)| json!({ "name": name, "instance": instance }))
        .collect::<Vec<_>>();
    let batch_json = serde_json::to_string(&batch)
        .unwrap_or_else(|e| panic!("Failed to serialize batch: {}", e));

    let mut child = std::process::Command::new(&binary_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn test process: {}", e));

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(batch_json.as_bytes())
            .unwrap_or_else(|e| panic!("Failed to write batch to stdin: {}", e));
        stdin
            .flush()
            .unwrap_or_else(|e| panic!("Failed to flush stdin: {}", e));
        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("Failed to wait for child process: {}", e));

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

    for success in &parsed.successes {
        println!(
            "  ✅ {} (tokens={}, max={})",
            success.name, success.token_count, success.max_token
        );
    }

    for (instance_name, error) in &parsed.failed_instances {
        println!("  ❌ {}: {}", instance_name, error);
    }

    // Clean up - DISABLED for debugging
    // fs::remove_dir_all(&temp_dir).ok();
    println!("📁 Debug: Generated code preserved in {}", temp_dir);

    // Create result summary
    let result = MultiInstanceResult {
        _schema_name: schema_name.to_string(),
        total_instances: instances.len(),
        successful_instances: parsed.successful_instances,
        failed_instances: parsed.failed_instances,
        schema_compilation_time_ms: schema_compilation_time.as_millis() as u64,
        code_generation_time_ms: code_generation_time.as_millis() as u64,
        rust_compilation_time_ms: rust_compilation_time.as_millis() as u64,
        generated_code_sha256,
        _per_instance_times_us: parsed.per_instance_times_us,
    };

    // Print scenario summary
    println!("\n📊 Scenario Results:");
    println!(
        "  Success Rate: {:.1}% ({}/{})",
        result.success_rate() * 100.0,
        result.successful_instances,
        result.total_instances
    );

    if !result.failed_instances.is_empty() {
        println!("\n❌ Failed Instances:");
        for (instance_name, error) in &result.failed_instances {
            println!("  💥 {}: {}", instance_name, error);
        }
    }

    // Return result for comprehensive reporting
    result
}

#[test]
fn test_default_smoke_fixture_manifest_is_explicit_and_existing() {
    let mut manifest_schemas = HashSet::new();

    for (schema_name, schema_folder) in DEFAULT_ROUND_TRIP_SCHEMAS {
        let fixtures =
            default_smoke_fixture_paths(schema_name, schema_folder).unwrap_or_else(|| {
                panic!(
                    "Default round-trip schema {}/{} is missing explicit smoke fixtures",
                    schema_folder, schema_name
                )
            });

        assert_eq!(
            fixtures.len(),
            DEFAULT_ROUND_TRIP_MAX_INSTANCES,
            "Default smoke fixture count changed for {}",
            schema_name
        );

        for relative_path in fixtures {
            let path = Path::new("tests/data/instances").join(relative_path);
            assert!(
                path.exists(),
                "Default smoke fixture does not exist: {}",
                path.display()
            );
        }

        manifest_schemas.insert((*schema_name, *schema_folder));
    }

    for (schema_name, schema_folder, _) in DEFAULT_SMOKE_FIXTURES {
        assert!(
            manifest_schemas.contains(&(*schema_name, *schema_folder)),
            "Smoke fixture manifest contains non-default schema {}/{}",
            schema_folder,
            schema_name
        );
    }
}

#[test]
fn test_focused_smoke_fixture_manifests_are_explicit_and_existing() {
    for (schema_name, schema_folder, fixtures) in FOCUSED_SMOKE_FIXTURES {
        assert!(
            !fixtures.is_empty() && fixtures.len() <= DEFAULT_ROUND_TRIP_MAX_INSTANCES,
            "Focused smoke fixture count for {} should be in 1..={}",
            schema_name,
            DEFAULT_ROUND_TRIP_MAX_INSTANCES
        );

        let mut seen = std::collections::HashSet::new();
        for relative_path in *fixtures {
            assert!(
                seen.insert(*relative_path),
                "Focused smoke fixture is duplicated for {}/{}: {}",
                schema_folder,
                schema_name,
                relative_path
            );
            let path = Path::new("tests/data/instances").join(relative_path);
            assert!(
                path.exists(),
                "Focused smoke fixture does not exist for {}/{}: {}",
                schema_folder,
                schema_name,
                path.display()
            );
        }
    }
}

#[test]
fn test_smoke_fixture_manifests_validate_against_source_schemas() {
    validate_fixture_manifest_source_schemas("default smoke", DEFAULT_SMOKE_FIXTURES);
    validate_fixture_manifest_source_schemas("focused smoke", FOCUSED_SMOKE_FIXTURES);
}

fn validate_fixture_manifest_source_schemas(
    manifest_name: &str,
    manifest: &[(&str, &str, &[&str])],
) {
    for (schema_name, schema_folder, fixtures) in manifest {
        let schema_path = Path::new("schemas")
            .join(schema_folder)
            .join(format!("{}.json", schema_name));
        let schema_content = fs::read_to_string(&schema_path).unwrap_or_else(|error| {
            panic!(
                "{} schema fixture manifest references unreadable schema {}: {}",
                manifest_name,
                schema_path.display(),
                error
            )
        });
        let schema: Value = serde_json::from_str(&schema_content).unwrap_or_else(|error| {
            panic!(
                "{} schema fixture manifest references invalid schema JSON {}: {}",
                manifest_name,
                schema_path.display(),
                error
            )
        });
        let validator =
            build_source_schema_validator(&schema, schema_name).unwrap_or_else(|error| {
                panic!(
                    "{} schema fixture manifest could not build source validator for {}: {}",
                    manifest_name, schema_name, error
                )
            });

        for relative_path in *fixtures {
            let path = Path::new("tests/data/instances").join(relative_path);
            let instance_content = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!(
                    "{} schema fixture manifest references unreadable fixture {}: {}",
                    manifest_name,
                    path.display(),
                    error
                )
            });
            let instance: Value = serde_json::from_str(&instance_content).unwrap_or_else(|error| {
                panic!(
                    "{} schema fixture manifest references invalid fixture JSON {}: {}",
                    manifest_name,
                    path.display(),
                    error
                )
            });
            let errors = validator
                .iter_errors(&instance)
                .take(5)
                .map(|error| {
                    format!(
                        "instance_path={} schema_path={} error={}",
                        error.instance_path(),
                        error.schema_path(),
                        error
                    )
                })
                .collect::<Vec<_>>();
            assert!(
                errors.is_empty(),
                "{} fixture {} does not validate against schema {}: {}",
                manifest_name,
                path.display(),
                schema_path.display(),
                errors.join("; ")
            );
        }
    }
}

#[test]
fn test_focused_smoke_manifest_can_select_target_schema_fixtures() {
    let fixtures = default_smoke_fixture_paths("github-workflow", "popular").unwrap();

    assert!(
        fixtures.contains(&"github-workflow/real/actions_checkout_test.json"),
        "Focused smoke selection should be able to include target schema fixture manifests"
    );
}

#[test]
fn test_round_trip_bpe_preset_resolves_default_schema_merges() {
    for (schema_name, _) in DEFAULT_ROUND_TRIP_SCHEMAS {
        let path =
            resolve_bpe_merges_path(schema_name, None, Some(LEGACY_TCT_500_BPE_PRESET)).unwrap();
        assert!(
            Path::new(&path).is_file(),
            "BPE preset should resolve to an existing merge table for {schema_name}: {path}"
        );
    }
}

#[test]
fn test_round_trip_bpe_preset_resolves_focused_schema_merges() {
    for (schema_name, schema_folder) in FOCUSED_ROUND_TRIP_BPE_SMOKE_SCHEMAS {
        assert!(
            default_smoke_fixture_paths(schema_name, schema_folder).is_some(),
            "Focused BPE smoke schema {schema_folder}/{schema_name} should have explicit smoke fixtures"
        );
        let path =
            resolve_bpe_merges_path(schema_name, None, Some(LEGACY_TCT_500_BPE_PRESET)).unwrap();
        assert!(
            Path::new(&path).is_file(),
            "BPE preset should resolve to an existing merge table for focused schema {schema_name}: {path}"
        );
    }
}

#[test]
fn test_explicit_bpe_merges_override_without_preset() {
    assert_eq!(
        resolve_bpe_merges_path("kubernetes", Some("custom-bpe.json"), None),
        Some("custom-bpe.json".to_string())
    );
}

#[test]
fn test_default_smoke_manifest_can_select_direct_schema_fixtures() {
    let fixtures = default_smoke_fixture_paths("tsconfig", "popular").unwrap();

    assert!(
        fixtures.contains(&"tsconfig/apollographql_apolloserver_tsconfig.json"),
        "Explicit smoke selection should be able to include direct fixture files"
    );
}

#[test]
fn test_large_scale_fixtures_are_full_mode_only() {
    let smoke_patterns = instance_search_patterns_for_schema(
        "github-workflow",
        "popular",
        InstanceFixtureScope::Standard,
    );
    assert!(
        !smoke_patterns
            .iter()
            .any(|(path, _)| path.ends_with("/large-scale")),
        "Smoke discovery should not enumerate large-scale fixture directories"
    );

    let full_patterns = instance_search_patterns_for_schema(
        "github-workflow",
        "popular",
        InstanceFixtureScope::IncludeLargeScale,
    );
    assert!(
        full_patterns.iter().any(|(path, instance_type)| {
            path == "tests/data/instances/github-workflow/large-scale"
                && *instance_type == "large-scale"
        }),
        "Full discovery should include the github-workflow large-scale fixture directory"
    );
}

#[test]
fn test_mixed_fixture_matching_accepts_schema_suffixes() {
    assert!(mixed_fixture_matches_schema("tsconfig_strict", "tsconfig"));
    assert!(mixed_fixture_matches_schema(
        "apollographql_apolloserver_tsconfig",
        "tsconfig"
    ));
    assert!(mixed_fixture_matches_schema(
        "vuejs_core_package",
        "package"
    ));
    assert!(!mixed_fixture_matches_schema("eslint", "tsconfig"));
}

#[test]
fn test_full_tsconfig_discovery_includes_direct_mixed_fixtures() {
    let instances = load_instances_for_schema("tsconfig", "popular", FixtureSelection::All);
    let instance_names = instances
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<HashSet<_>>();

    assert!(instance_names.contains("tsconfig/synthetic/public_strict"));
    assert!(instance_names.contains("tsconfig/apollographql_apolloserver_tsconfig"));
    assert!(instance_names.contains("tsconfig/nestjs_nest_tsconfig"));
    assert!(instance_names.contains("tsconfig/prisma_prisma_tsconfig"));
    assert!(instance_names.contains("tsconfig/trpc_trpc_tsconfig"));
}

#[test]
fn test_parse_batch_results_accounts_for_success_failure_and_missing() {
    let expected_names = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
    let stdout = concat!(
        "noise before result\n",
        r#"RESULT:{"name":"alpha","status":"success","encode_time_ns":1500,"decode_time_ns":2600,"token_count":4,"max_token":17}"#,
        "\n",
        r#"RESULT:{"name":"beta","status":"failure","error":"round trip mismatch"}"#,
        "\n"
    );

    let parsed =
        parse_batch_round_trip_results(stdout, "stderr context", false, Some(1), &expected_names);

    assert_eq!(parsed.successful_instances, 1);
    assert_eq!(parsed.successes.len(), 1);
    assert_eq!(parsed.successes[0].name, "alpha");
    assert_eq!(parsed.successes[0].token_count, 4);
    assert_eq!(parsed.successes[0].max_token, 17);
    assert_eq!(parsed.per_instance_times_us.get("alpha"), Some(&(1, 2)));
    assert_eq!(parsed.failed_instances.len(), 2);
    assert!(
        parsed
            .failed_instances
            .iter()
            .any(|(name, error)| { name == "beta" && error == "round trip mismatch" })
    );
    assert!(parsed.failed_instances.iter().any(|(name, error)| {
        name == "gamma"
            && error.contains("No batch result emitted")
            && error.contains("stderr context")
    }));
}

#[test]
fn test_parse_batch_results_flags_protocol_errors() {
    let expected_names = vec![
        "alpha".to_string(),
        "bad_status".to_string(),
        "missing_metric".to_string(),
    ];
    let stdout = concat!(
        "RESULT:{not valid json}\n",
        r#"RESULT:{"name":"alpha","status":"success","encode_time_ns":0,"decode_time_ns":0,"token_count":1,"max_token":0}"#,
        "\n",
        r#"RESULT:{"name":"alpha","status":"success","encode_time_ns":0,"decode_time_ns":0,"token_count":1,"max_token":0}"#,
        "\n",
        r#"RESULT:{"name":"extra","status":"success","encode_time_ns":0,"decode_time_ns":0,"token_count":1,"max_token":0}"#,
        "\n",
        r#"RESULT:{"name":"bad_status","status":"skipped"}"#,
        "\n",
        r#"RESULT:{"name":"missing_metric","status":"success","encode_time_ns":0,"decode_time_ns":0,"token_count":1}"#,
        "\n"
    );

    let parsed = parse_batch_round_trip_results(stdout, "", true, Some(0), &expected_names);

    assert_eq!(parsed.successful_instances, 1);
    assert_eq!(parsed.failed_instances.len(), 5);
    assert!(
        parsed
            .failed_instances
            .iter()
            .any(|(name, error)| name == "<batch>" && error.contains("Malformed batch result JSON"))
    );
    assert!(
        parsed
            .failed_instances
            .iter()
            .any(|(name, error)| { name == "alpha" && error.contains("Duplicate result") })
    );
    assert!(
        parsed
            .failed_instances
            .iter()
            .any(|(name, error)| { name == "extra" && error.contains("Unexpected result") })
    );
    assert!(parsed.failed_instances.iter().any(|(name, error)| {
        name == "bad_status" && error.contains("Unexpected result status")
    }));
    assert!(parsed.failed_instances.iter().any(|(name, error)| {
        name == "missing_metric" && error.contains("missing numeric field: max_token")
    }));
}

#[test]
fn test_parse_batch_results_flags_nonzero_exit_without_result_failure() {
    let expected_names = vec!["alpha".to_string()];
    let stdout = r#"RESULT:{"name":"alpha","status":"success","encode_time_ns":0,"decode_time_ns":0,"token_count":1,"max_token":0}"#;

    let parsed =
        parse_batch_round_trip_results(stdout, "process died", false, Some(101), &expected_names);

    assert_eq!(parsed.successful_instances, 1);
    assert_eq!(parsed.failed_instances.len(), 1);
    assert_eq!(parsed.failed_instances[0].0, "<batch>");
    assert!(parsed.failed_instances[0].1.contains("status Some(101)"));
    assert!(parsed.failed_instances[0].1.contains("process died"));
}

#[test]
fn test_generated_batch_round_trip_reports_source_schema_validation_failure() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "required": ["name"],
        "additionalProperties": false
    });
    let instances = vec![(
        "synthetic/invalid-source-schema-fixture.json".to_string(),
        json!({ "name": 7 }),
    )];

    let summary = test_batch_round_trip_with_optimization_level(
        &schema,
        &instances,
        "synthetic_source_schema_validation",
        tct::OptimizationLevel::O3,
    )
    .expect("generated batch helper should report validation failure as a batch result");

    assert_eq!(summary.successful_instances, 0);
    assert_eq!(summary.failed_instances.len(), 1);
    let (instance_name, error) = &summary.failed_instances[0];
    assert_eq!(
        instance_name,
        "synthetic/invalid-source-schema-fixture.json"
    );
    assert!(error.contains("source schema validation failed before encode"));
    assert!(error.contains("schema=synthetic_source_schema_validation"));
    assert!(error.contains("fixture=synthetic/invalid-source-schema-fixture.json"));
    assert!(error.contains("instance_path=/name"));
    assert!(error.contains("schema_path=/properties/name/type"));
    assert!(error.contains("input_context="));
    assert!(error.contains("\"name\": 7"));
}

#[test]
fn test_generated_typed_unbounded_integer_u64_max_matches_dynamic_value() {
    let schema = json!({
        "type": "object",
        "properties": {
            "typed": { "type": "integer" },
            "dynamic": {
                "type": "object",
                "additionalProperties": true
            }
        },
        "required": ["typed", "dynamic"],
        "additionalProperties": false
    });

    let instances = vec![
        (
            "dynamic_u64_control".to_string(),
            json!({
                "typed": 0,
                "dynamic": { "max": 18446744073709551615u64 }
            }),
        ),
        (
            "typed_u64_max".to_string(),
            json!({
                "typed": 18446744073709551615u64,
                "dynamic": { "max": 18446744073709551615u64 }
            }),
        ),
    ];

    let summary = test_batch_round_trip_with_optimization_level(
        &schema,
        &instances,
        "typed_unbounded_integer_u64",
        tct::OptimizationLevel::O3,
    )
    .expect("generated batch helper should run typed u64 regression");

    assert_eq!(summary.total_instances, instances.len());
    assert_eq!(
        summary.successful_instances,
        instances.len(),
        "typed/dynamic u64 regression failures: {:?}",
        summary.failed_instances
    );
    assert!(summary.failed_instances.is_empty());
}

#[test]
fn test_generated_overwide_bounded_integer_preserves_max_value() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {
                "type": "integer",
                "minimum": 0,
                "maximum": 5_000_000_000i64
            }
        },
        "required": ["value"],
        "additionalProperties": false
    });

    let instances = vec![
        ("min_value".to_string(), json!({ "value": 0 })),
        (
            "max_value".to_string(),
            json!({ "value": 5_000_000_000i64 }),
        ),
    ];

    let summary = test_batch_round_trip_with_optimization_level(
        &schema,
        &instances,
        "overwide_bounded_integer",
        tct::OptimizationLevel::O3,
    )
    .expect("generated batch helper should run overwide bounded integer regression");

    assert_eq!(summary.total_instances, instances.len());
    assert_eq!(
        summary.successful_instances,
        instances.len(),
        "overwide bounded integer regression failures: {:?}",
        summary.failed_instances
    );
    assert!(summary.failed_instances.is_empty());
}
