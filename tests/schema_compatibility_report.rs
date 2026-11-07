//! Individual Schema Compatibility Report Generator
//!
//! Generates detailed compatibility reports for individual schemas,
//! analyzing all instances and providing comprehensive phase-by-phase analysis.
//!
//! Usage:
//!   SCHEMA_NAME=package cargo test --test schema_compatibility_report -- --nocapture
//!   SCHEMA_NAME=tsconfig cargo test --test schema_compatibility_report -- --nocapture
//! Output: Creates {SCHEMA_NAME}_COMPATIBILITY_REPORT.md with detailed analysis

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::time::Instant;

mod round_trip_helpers;
use round_trip_helpers::{RoundTripPhase, load_instances_for_schema, test_phased_round_trip};

/// Statistics for a single schema's compatibility testing
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SchemaCompatibilityStats {
    schema_name: String,
    tier: String,
    total_instances: usize,
    real_instances: usize,
    synthetic_instances: usize,

    // Phase success counts
    schema_compilation_success: usize,
    code_generation_success: usize,
    token_encoding_success: usize,
    token_decoding_success: usize,
    data_integrity_success: usize,

    // Full round-trip success
    full_round_trip_success: usize,

    // Timing and metrics
    avg_compilation_time_us: Option<u64>,
    avg_token_count: Option<usize>,
    avg_compression_ratio: Option<f64>,

    // Error patterns
    common_errors: Vec<String>,
    trait_analysis_warnings: usize,
}

impl SchemaCompatibilityStats {
    fn new(schema_name: String, tier: String) -> Self {
        Self {
            schema_name,
            tier,
            total_instances: 0,
            real_instances: 0,
            synthetic_instances: 0,
            schema_compilation_success: 0,
            code_generation_success: 0,
            token_encoding_success: 0,
            token_decoding_success: 0,
            data_integrity_success: 0,
            full_round_trip_success: 0,
            avg_compilation_time_us: None,
            avg_token_count: None,
            avg_compression_ratio: None,
            common_errors: Vec::new(),
            trait_analysis_warnings: 0,
        }
    }

    fn success_rate(&self) -> f64 {
        if self.total_instances == 0 {
            0.0
        } else {
            self.full_round_trip_success as f64 / self.total_instances as f64
        }
    }

    fn phase_success_rate(&self, phase: RoundTripPhase) -> f64 {
        if self.total_instances == 0 {
            return 0.0;
        }

        let success_count = match phase {
            RoundTripPhase::SchemaCompilation => self.schema_compilation_success,
            RoundTripPhase::CodeGeneration => self.code_generation_success,
            RoundTripPhase::TokenEncoding => self.token_encoding_success,
            RoundTripPhase::TokenDecoding => self.token_decoding_success,
            RoundTripPhase::DataIntegrity => self.data_integrity_success,
        };

        success_count as f64 / self.total_instances as f64
    }
}

/// Overall compatibility report across all schemas
#[derive(Debug)]
struct CompatibilityReport {
    total_schemas: usize,
    schemas_with_instances: usize,
    total_instances: usize,

    // Tier breakdown
    popular_schemas: Vec<SchemaCompatibilityStats>,
    common_schemas: Vec<SchemaCompatibilityStats>,
    specialized_schemas: Vec<SchemaCompatibilityStats>,

    // Overall statistics
    overall_success_rate: f64,
    phase_success_rates: HashMap<String, f64>,

    // Issue analysis
    most_common_errors: Vec<(String, usize)>,
    schemas_needing_attention: Vec<String>,

    generation_time: Instant,
}

/// Load all schemas from a tier directory
fn load_schemas_from_tier(tier_name: &str) -> Vec<String> {
    let tier_dir = format!("schemas/{}", tier_name);
    let mut schemas = Vec::new();

    if let Ok(dir) = fs::read_dir(&tier_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && let Some(name) = path.file_stem().and_then(|s| s.to_str())
            {
                schemas.push(name.to_string());
            }
        }
    }

    schemas.sort();
    schemas
}

/// Test a single schema and return compatibility statistics
fn test_schema_compatibility(schema_name: &str, tier: &str) -> SchemaCompatibilityStats {
    let mut stats = SchemaCompatibilityStats::new(schema_name.to_string(), tier.to_string());

    println!("\n🧪 Testing schema: {}/{}", tier, schema_name);

    // Load schema
    let schema_path = format!("schemas/{}/{}.json", tier, schema_name);
    let schema = match fs::read_to_string(&schema_path) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(schema) => schema,
            Err(e) => {
                println!("  ❌ Failed to parse schema: {}", e);
                stats
                    .common_errors
                    .push(format!("Schema parse error: {}", e));
                return stats;
            }
        },
        Err(e) => {
            println!("  ❌ Failed to read schema file: {}", e);
            stats.common_errors.push(format!("File read error: {}", e));
            return stats;
        }
    };

    // Load instances
    let instances = load_instances_for_schema(schema_name, tier);
    if instances.is_empty() {
        println!("  ⚠️ No instances found for {}", schema_name);
        return stats;
    }

    stats.total_instances = instances.len();
    // Note: Enhanced logging in load_instances_for_schema will show real vs synthetic breakdown

    let mut compilation_times = Vec::new();
    let mut token_counts = Vec::new();
    let mut compression_ratios = Vec::new();

    // Test each instance
    for (instance_name, instance) in instances {
        println!("  🔄 Testing instance: {}", instance_name);
        if instance_name.contains("/real/") {
            stats.real_instances += 1;
        } else if instance_name.contains("/synthetic/") {
            stats.synthetic_instances += 1;
        }

        let result = test_phased_round_trip(&schema, &instance, schema_name, &instance_name);

        // Count phase successes
        for phase_result in &result.phases {
            match phase_result.phase {
                RoundTripPhase::SchemaCompilation => {
                    if phase_result.success {
                        stats.schema_compilation_success += 1;
                        if let Some(duration) = phase_result.duration_us {
                            compilation_times.push(duration);
                        }
                    } else if let Some(error) = &phase_result.error {
                        stats.common_errors.push(error.clone());
                    }
                }
                RoundTripPhase::CodeGeneration => {
                    if phase_result.success {
                        stats.code_generation_success += 1;
                    } else if let Some(error) = &phase_result.error {
                        stats.common_errors.push(error.clone());
                    }
                }
                RoundTripPhase::TokenEncoding => {
                    if phase_result.success {
                        stats.token_encoding_success += 1;
                        // Extract token count from metrics if available
                        if let Some(token_count_str) = phase_result.metrics.get("token_count")
                            && let Ok(count) = token_count_str.parse::<usize>()
                        {
                            token_counts.push(count);
                        }
                        // Extract compression ratio if available
                        if let Some(ratio_str) = phase_result.metrics.get("compression_ratio")
                            && let Ok(ratio) = ratio_str.parse::<f64>()
                        {
                            compression_ratios.push(ratio);
                        }
                    } else if let Some(error) = &phase_result.error {
                        stats.common_errors.push(error.clone());
                    }
                }
                RoundTripPhase::TokenDecoding => {
                    if phase_result.success {
                        stats.token_decoding_success += 1;
                    } else if let Some(error) = &phase_result.error {
                        stats.common_errors.push(error.clone());
                    }
                }
                RoundTripPhase::DataIntegrity => {
                    if phase_result.success {
                        stats.data_integrity_success += 1;
                    } else if let Some(error) = &phase_result.error {
                        stats.common_errors.push(error.clone());
                    }
                }
            }
        }

        if result.overall_success {
            stats.full_round_trip_success += 1;
            println!("    ✅ Full round-trip success");
        } else if let Some(failed_phase) = result.failed_phase() {
            println!("    ❌ Failed at: {}", failed_phase.phase.name());
        }
    }

    // Calculate averages
    if !compilation_times.is_empty() {
        stats.avg_compilation_time_us =
            Some(compilation_times.iter().sum::<u64>() / compilation_times.len() as u64);
    }
    if !token_counts.is_empty() {
        stats.avg_token_count = Some(token_counts.iter().sum::<usize>() / token_counts.len());
    }
    if !compression_ratios.is_empty() {
        stats.avg_compression_ratio =
            Some(compression_ratios.iter().sum::<f64>() / compression_ratios.len() as f64);
    }

    println!(
        "  📊 Success rate: {:.1}% ({}/{})",
        stats.success_rate() * 100.0,
        stats.full_round_trip_success,
        stats.total_instances
    );

    stats
}

/// Generate comprehensive compatibility report
fn generate_compatibility_report() -> CompatibilityReport {
    let start_time = Instant::now();

    println!("🚀 Starting comprehensive schema compatibility analysis...");

    // Load all schemas from each tier
    let popular_schema_names = load_schemas_from_tier("popular");
    let common_schema_names = load_schemas_from_tier("common");
    let specialized_schema_names = load_schemas_from_tier("specialized");

    println!("📊 Found schemas:");
    println!("  • Popular: {} schemas", popular_schema_names.len());
    println!("  • Common: {} schemas", common_schema_names.len());
    println!(
        "  • Specialized: {} schemas",
        specialized_schema_names.len()
    );

    let mut report = CompatibilityReport {
        total_schemas: popular_schema_names.len()
            + common_schema_names.len()
            + specialized_schema_names.len(),
        schemas_with_instances: 0,
        total_instances: 0,
        popular_schemas: Vec::new(),
        common_schemas: Vec::new(),
        specialized_schemas: Vec::new(),
        overall_success_rate: 0.0,
        phase_success_rates: HashMap::new(),
        most_common_errors: Vec::new(),
        schemas_needing_attention: Vec::new(),
        generation_time: start_time,
    };

    // Test popular schemas
    println!(
        "\n🌟 Testing Popular Schemas ({} schemas)",
        popular_schema_names.len()
    );
    for schema_name in popular_schema_names {
        let stats = test_schema_compatibility(&schema_name, "popular");
        if stats.total_instances > 0 {
            report.schemas_with_instances += 1;
            report.total_instances += stats.total_instances;
        }
        if stats.success_rate() < 0.5 {
            report
                .schemas_needing_attention
                .push(format!("popular/{}", schema_name));
        }
        report.popular_schemas.push(stats);
    }

    // Test common schemas
    println!(
        "\n🔧 Testing Common Schemas ({} schemas)",
        common_schema_names.len()
    );
    for schema_name in common_schema_names {
        let stats = test_schema_compatibility(&schema_name, "common");
        if stats.total_instances > 0 {
            report.schemas_with_instances += 1;
            report.total_instances += stats.total_instances;
        }
        if stats.success_rate() < 0.5 {
            report
                .schemas_needing_attention
                .push(format!("common/{}", schema_name));
        }
        report.common_schemas.push(stats);
    }

    // Test specialized schemas
    println!(
        "\n🎯 Testing Specialized Schemas ({} schemas)",
        specialized_schema_names.len()
    );
    for schema_name in specialized_schema_names {
        let stats = test_schema_compatibility(&schema_name, "specialized");
        if stats.total_instances > 0 {
            report.schemas_with_instances += 1;
            report.total_instances += stats.total_instances;
        }
        if stats.success_rate() < 0.5 {
            report
                .schemas_needing_attention
                .push(format!("specialized/{}", schema_name));
        }
        report.specialized_schemas.push(stats);
    }

    // Calculate overall statistics
    let all_schemas: Vec<&SchemaCompatibilityStats> = report
        .popular_schemas
        .iter()
        .chain(report.common_schemas.iter())
        .chain(report.specialized_schemas.iter())
        .collect();

    let total_successful_instances: usize =
        all_schemas.iter().map(|s| s.full_round_trip_success).sum();

    report.overall_success_rate = if report.total_instances > 0 {
        total_successful_instances as f64 / report.total_instances as f64
    } else {
        0.0
    };

    // Calculate phase success rates
    let phases = [
        RoundTripPhase::SchemaCompilation,
        RoundTripPhase::CodeGeneration,
        RoundTripPhase::TokenEncoding,
        RoundTripPhase::TokenDecoding,
        RoundTripPhase::DataIntegrity,
    ];

    for phase in phases {
        let phase_success: usize = all_schemas
            .iter()
            .map(|s| match phase {
                RoundTripPhase::SchemaCompilation => s.schema_compilation_success,
                RoundTripPhase::CodeGeneration => s.code_generation_success,
                RoundTripPhase::TokenEncoding => s.token_encoding_success,
                RoundTripPhase::TokenDecoding => s.token_decoding_success,
                RoundTripPhase::DataIntegrity => s.data_integrity_success,
            })
            .sum();

        let phase_rate = if report.total_instances > 0 {
            phase_success as f64 / report.total_instances as f64
        } else {
            0.0
        };

        report
            .phase_success_rates
            .insert(phase.name().to_string(), phase_rate);
    }

    // Collect most common errors
    let mut error_counts: HashMap<String, usize> = HashMap::new();
    for schema in &all_schemas {
        for error in &schema.common_errors {
            *error_counts.entry(error.clone()).or_insert(0) += 1;
        }
    }

    let mut errors: Vec<(String, usize)> = error_counts.into_iter().collect();
    errors.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    report.most_common_errors = errors.into_iter().take(10).collect();

    report
}

/// Write compatibility report to markdown file
fn write_report_to_file(report: &CompatibilityReport) -> Result<(), std::io::Error> {
    let mut content = String::new();

    content.push_str("# Schema Compatibility Report\n\n");
    content.push_str(&format!(
        "**Generated**: {}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));
    content.push_str(&format!(
        "**Analysis Duration**: {:.2}s\n\n",
        report.generation_time.elapsed().as_secs_f64()
    ));

    // Executive summary
    content.push_str("## Executive Summary\n\n");
    content.push_str(&format!("- **Total Schemas**: {}\n", report.total_schemas));
    content.push_str(&format!(
        "- **Schemas with Test Instances**: {}\n",
        report.schemas_with_instances
    ));
    content.push_str(&format!(
        "- **Total Test Instances**: {}\n",
        report.total_instances
    ));
    content.push_str(&format!(
        "- **Overall Success Rate**: {:.1}%\n\n",
        report.overall_success_rate * 100.0
    ));

    // Phase success rates
    content.push_str("### Phase Success Rates\n\n");
    content.push_str("| Phase | Success Rate |\n");
    content.push_str("|-------|-------------|\n");
    for (phase, rate) in &report.phase_success_rates {
        content.push_str(&format!("| {} | {:.1}% |\n", phase, rate * 100.0));
    }
    content.push('\n');

    // Tier breakdown
    content.push_str("## Tier Analysis\n\n");

    // Popular schemas
    content.push_str("### Popular Schemas\n\n");
    content.push_str("| Schema | Instances | Success Rate | Issues |\n");
    content.push_str("|--------|-----------|--------------|--------|\n");
    for schema in &report.popular_schemas {
        let issues = if schema.success_rate() < 1.0 {
            format!(
                "{} failures",
                schema.total_instances - schema.full_round_trip_success
            )
        } else {
            "None".to_string()
        };
        content.push_str(&format!(
            "| {} | {} | {:.1}% | {} |\n",
            schema.schema_name,
            schema.total_instances,
            schema.success_rate() * 100.0,
            issues
        ));
    }
    content.push('\n');

    // Common schemas (summary)
    let common_success_rate = if !report.common_schemas.is_empty() {
        report
            .common_schemas
            .iter()
            .map(|s| s.success_rate())
            .sum::<f64>()
            / report.common_schemas.len() as f64
    } else {
        0.0
    };
    content.push_str(&format!(
        "### Common Schemas ({} schemas)\n\n",
        report.common_schemas.len()
    ));
    content.push_str(&format!(
        "Average success rate: {:.1}%\n\n",
        common_success_rate * 100.0
    ));

    // Specialized schemas (summary)
    let specialized_success_rate = if !report.specialized_schemas.is_empty() {
        report
            .specialized_schemas
            .iter()
            .map(|s| s.success_rate())
            .sum::<f64>()
            / report.specialized_schemas.len() as f64
    } else {
        0.0
    };
    content.push_str(&format!(
        "### Specialized Schemas ({} schemas)\n\n",
        report.specialized_schemas.len()
    ));
    content.push_str(&format!(
        "Average success rate: {:.1}%\n\n",
        specialized_success_rate * 100.0
    ));

    // Issues analysis
    content.push_str("## Issues Analysis\n\n");

    content.push_str("### Schemas Needing Attention (< 50% success rate)\n\n");
    if report.schemas_needing_attention.is_empty() {
        content.push_str("None - all schemas have ≥50% success rate.\n\n");
    } else {
        for schema in &report.schemas_needing_attention {
            content.push_str(&format!("- {}\n", schema));
        }
        content.push('\n');
    }

    content.push_str("### Most Common Error Patterns\n\n");
    content.push_str("| Error | Frequency |\n");
    content.push_str("|-------|----------|\n");
    for (error, count) in &report.most_common_errors {
        content.push_str(&format!("| {} | {} |\n", error, count));
    }
    content.push('\n');

    // Recommendations
    content.push_str("## Recommendations\n\n");
    content.push_str("Based on this analysis:\n\n");

    if report.overall_success_rate >= 0.8 {
        content.push_str("✅ **Overall compatibility is good** (≥80% success rate)\n\n");
    } else if report.overall_success_rate >= 0.5 {
        content.push_str("⚠️ **Moderate compatibility issues** (50-80% success rate)\n\n");
    } else {
        content.push_str("❌ **Significant compatibility issues** (<50% success rate)\n\n");
    }

    if !report.schemas_needing_attention.is_empty() {
        content.push_str(&format!(
            "**Priority**: Address {} schemas with <50% success rate\n\n",
            report.schemas_needing_attention.len()
        ));
    }

    if let Some((most_common_error, _)) = report.most_common_errors.first() {
        content.push_str(&format!(
            "**Focus**: Address most common error pattern: {}\n\n",
            most_common_error
        ));
    }

    content.push_str("---\n");
    content.push_str("*Generated by schema_compatibility_report.rs*\n");

    fs::write("SCHEMA_COMPATIBILITY_REPORT.md", content)?;
    Ok(())
}

/// Find which tier contains the specified schema
fn find_schema_tier(schema_name: &str) -> Option<String> {
    let tiers = vec!["popular", "common", "specialized"];

    for tier in &tiers {
        let schema_path = format!("schemas/{}/{}.json", tier, schema_name);
        if fs::metadata(&schema_path).is_ok() {
            return Some(tier.to_string());
        }
    }

    None
}

/// Generate individual schema compatibility report content
fn generate_individual_schema_report(
    schema_name: &str,
    tier: &str,
    stats: &SchemaCompatibilityStats,
) -> String {
    let timestamp = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    format!(
        r#"# Individual Schema Compatibility Report: {}

**Generated:** {}
**Schema:** {} (tier: {})

## 📊 Summary

| Metric | Value |
|--------|-------|
| **Total Instances** | {} |
| **Real Instances** | {} |
| **Synthetic Instances** | {} |
| **Overall Success Rate** | {:.1}% ({}/{}) |

## 🔄 Phase-by-Phase Analysis

| Phase | Success Rate | Success Count | Status |
|-------|-------------|---------------|---------|
| **Schema Compilation** | {:.1}% | {}/{} | {} |
| **Code Generation** | {:.1}% | {}/{} | {} |
| **Token Encoding** | {:.1}% | {}/{} | {} |
| **Token Decoding** | {:.1}% | {}/{} | {} |
| **Data Integrity** | {:.1}% | {}/{} | {} |

## 🎯 Performance Metrics

- **Average Compilation Time:** {}μs
- **Average Code Generation Time:** {}μs
- **Average Token Encoding Time:** {}μs
- **Average Token Decoding Time:** {}μs
- **Total Analysis Time:** {}μs

{}

## 📝 Analysis Notes

{}

---

*Report generated by TCT Individual Schema Compatibility Analyzer*
"#,
        schema_name.to_uppercase(),
        timestamp,
        schema_name,
        tier,
        stats.total_instances,
        stats.real_instances,
        stats.synthetic_instances,
        if stats.total_instances > 0 {
            stats.full_round_trip_success as f64 / stats.total_instances as f64 * 100.0
        } else {
            0.0
        },
        stats.full_round_trip_success,
        stats.total_instances,
        // Phase analysis
        stats.phase_success_rate(RoundTripPhase::SchemaCompilation) * 100.0,
        stats.schema_compilation_success,
        stats.total_instances,
        if stats.schema_compilation_success == stats.total_instances {
            "✅"
        } else if stats.schema_compilation_success == 0 {
            "❌"
        } else {
            "⚠️"
        },
        stats.phase_success_rate(RoundTripPhase::CodeGeneration) * 100.0,
        stats.code_generation_success,
        stats.total_instances,
        if stats.code_generation_success == stats.total_instances {
            "✅"
        } else if stats.code_generation_success == 0 {
            "❌"
        } else {
            "⚠️"
        },
        stats.phase_success_rate(RoundTripPhase::TokenEncoding) * 100.0,
        stats.token_encoding_success,
        stats.total_instances,
        if stats.token_encoding_success == stats.total_instances {
            "✅"
        } else if stats.token_encoding_success == 0 {
            "❌"
        } else {
            "⚠️"
        },
        stats.phase_success_rate(RoundTripPhase::TokenDecoding) * 100.0,
        stats.token_decoding_success,
        stats.total_instances,
        if stats.token_decoding_success == stats.total_instances {
            "✅"
        } else if stats.token_decoding_success == 0 {
            "❌"
        } else {
            "⚠️"
        },
        stats.phase_success_rate(RoundTripPhase::DataIntegrity) * 100.0,
        stats.data_integrity_success,
        stats.total_instances,
        if stats.data_integrity_success == stats.total_instances {
            "✅"
        } else if stats.data_integrity_success == 0 {
            "❌"
        } else {
            "⚠️"
        },
        // Performance metrics
        stats.avg_compilation_time_us.unwrap_or(0),
        0u64, // avg_generation_time_us - not tracked separately yet
        0u64, // avg_encoding_time_us - not tracked separately yet
        0u64, // avg_decoding_time_us - not tracked separately yet
        0u64, // total_analysis_time_us - not tracked separately yet
        // Error analysis
        if !stats.common_errors.is_empty() {
            let error_section = format!(
                "\n## ⚠️ Common Errors\n\n{}\n",
                stats
                    .common_errors
                    .iter()
                    .map(|e| format!("- {}", e))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            error_section
        } else {
            String::new()
        },
        // Notes
        if stats.total_instances == 0 {
            "⚠️ **No test instances found** - Consider adding real-world examples for this schema."
        } else if stats.full_round_trip_success == stats.total_instances {
            "✅ **Perfect compatibility** - All instances pass complete round-trip testing."
        } else if stats.schema_compilation_success < stats.total_instances {
            "❌ **Schema compilation issues** - The schema has fundamental compatibility problems."
        } else if stats.code_generation_success < stats.total_instances {
            "⚠️ **Code generation issues** - The schema compiles but has code generation problems."
        } else {
            "⚠️ **Runtime issues** - Schema compiles and generates code but has runtime problems."
        }
    )
}

/// Generate individual schema compatibility report
/// ⚠️ SLOW TEST: Varies by schema - Run manually with:
/// `SCHEMA_NAME=commitlintrc cargo test --test schema_compatibility_report test_individual -- --ignored --nocapture`
#[test]
#[ignore = "report-only: writes an individual compatibility report"]
fn test_individual_schema_compatibility_report() {
    let schema_name = std::env::var("SCHEMA_NAME").unwrap_or_else(|_| {
        println!("⚠️ SCHEMA_NAME not set, using 'commitlintrc' as default");
        "commitlintrc".to_string()
    });

    println!(
        "🧪 Generating compatibility report for schema: {}",
        schema_name
    );

    // Find the schema in any tier
    let tier = find_schema_tier(&schema_name)
        .unwrap_or_else(|| panic!("Schema '{}' not found in any tier", schema_name));

    println!("📂 Schema found in tier: {}", tier);

    // Generate comprehensive analysis for this schema
    let result = test_schema_compatibility(&schema_name, &tier);

    println!("\n{}", "=".repeat(60));
    println!(
        "📋 SCHEMA COMPATIBILITY ANALYSIS: {}",
        schema_name.to_uppercase()
    );
    println!("{}", "=".repeat(60));
    println!("Tier: {}", tier);
    println!("Total instances: {}", result.total_instances);
    println!("Real instances: {}", result.real_instances);
    println!("Synthetic instances: {}", result.synthetic_instances);

    if result.total_instances > 0 {
        println!("\nPhase Success Rates:");
        println!(
            "  Schema Compilation: {:.1}% ({}/{})",
            result.phase_success_rate(RoundTripPhase::SchemaCompilation) * 100.0,
            result.schema_compilation_success,
            result.total_instances
        );
        println!(
            "  Code Generation: {:.1}% ({}/{})",
            result.phase_success_rate(RoundTripPhase::CodeGeneration) * 100.0,
            result.code_generation_success,
            result.total_instances
        );
        println!(
            "  Token Encoding: {:.1}% ({}/{})",
            result.phase_success_rate(RoundTripPhase::TokenEncoding) * 100.0,
            result.token_encoding_success,
            result.total_instances
        );
        println!(
            "  Token Decoding: {:.1}% ({}/{})",
            result.phase_success_rate(RoundTripPhase::TokenDecoding) * 100.0,
            result.token_decoding_success,
            result.total_instances
        );
        println!(
            "  Data Integrity: {:.1}% ({}/{})",
            result.phase_success_rate(RoundTripPhase::DataIntegrity) * 100.0,
            result.data_integrity_success,
            result.total_instances
        );

        let overall_success_rate =
            result.full_round_trip_success as f64 / result.total_instances as f64;
        println!(
            "\n🎯 Overall Success Rate: {:.1}% ({}/{})",
            overall_success_rate * 100.0,
            result.full_round_trip_success,
            result.total_instances
        );

        if !result.common_errors.is_empty() {
            println!("\n⚠️ Common Errors:");
            for error in &result.common_errors {
                println!("  - {}", error);
            }
        }
    } else {
        println!("⚠️ No instances found for testing");
    }

    // Generate and write individual schema report
    let report_content = generate_individual_schema_report(&schema_name, &tier, &result);
    let report_filename = format!("{}_COMPATIBILITY_REPORT.md", schema_name.to_uppercase());

    fs::write(&report_filename, report_content).unwrap_or_else(|e| {
        panic!(
            "Failed to write individual schema report to {}: {}",
            report_filename, e
        )
    });
    println!(
        "\n✅ Individual schema report written to: {}",
        report_filename
    );

    println!("{}", "=".repeat(60));
}

/// ⚠️ VERY SLOW TEST: ~2869s (48 minutes) runtime - Run manually with:
/// `cargo test --test schema_compatibility_report test_generate_comprehensive -- --ignored --nocapture`
#[test]
#[ignore = "report-only: writes SCHEMA_COMPATIBILITY_REPORT.md"]
fn test_generate_comprehensive_compatibility_report() {
    println!("🚀 Starting comprehensive schema compatibility analysis...");

    let report = generate_compatibility_report();

    println!("\n{}", "=".repeat(60));
    println!("📋 FINAL COMPATIBILITY SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Total Schemas: {}", report.total_schemas);
    println!("Schemas with Instances: {}", report.schemas_with_instances);
    println!("Total Test Instances: {}", report.total_instances);
    println!(
        "Overall Success Rate: {:.1}%",
        report.overall_success_rate * 100.0
    );

    println!("\nPhase Success Rates:");
    for (phase, rate) in &report.phase_success_rates {
        println!("  {}: {:.1}%", phase, rate * 100.0);
    }

    if !report.schemas_needing_attention.is_empty() {
        println!(
            "\n⚠️ Schemas needing attention ({}):",
            report.schemas_needing_attention.len()
        );
        for schema in &report.schemas_needing_attention {
            println!("  - {}", schema);
        }
    }

    println!("\n🔄 Writing detailed report to SCHEMA_COMPATIBILITY_REPORT.md...");
    write_report_to_file(&report)
        .unwrap_or_else(|e| panic!("Failed to write SCHEMA_COMPATIBILITY_REPORT.md: {}", e));
    println!("✅ Report written successfully!");

    println!("{}", "=".repeat(60));
}
