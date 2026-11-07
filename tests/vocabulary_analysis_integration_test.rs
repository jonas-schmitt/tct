//! Vocabulary Analysis Integration Test Suite
//!
//! This test suite validates the complete end-to-end functionality of the
//! vocabulary analysis feature, including CLI integration, analysis engine,
//! recommendation system, and output formatting.

use serde_json::Value;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Test complete vocabulary analysis workflow with complex schema
#[test]
fn test_vocabulary_analysis_end_to_end_integration() {
    // Create a comprehensive test schema
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "title": "ComplexSchema",
        "type": "object",
        "properties": {{
            "user": {{
                "type": "object",
                "properties": {{
                    "id": {{ "type": "integer", "minimum": 1, "maximum": 9999 }},
                    "username": {{ "type": "string", "minLength": 3, "maxLength": 20 }},
                    "status": {{ "enum": ["active", "pending", "suspended", "deleted"] }},
                    "role": {{ "enum": ["admin", "user", "moderator"] }},
                    "preferences": {{
                        "type": "object",
                        "properties": {{
                            "theme": {{ "enum": ["light", "dark"] }},
                            "notifications": {{ "type": "boolean" }}
                        }}
                    }}
                }}
            }},
            "metadata": {{
                "type": "object",
                "properties": {{
                    "created_at": {{ "type": "string", "format": "date-time" }},
                    "tags": {{
                        "type": "array",
                        "items": {{ "type": "string" }}
                    }}
                }}
            }}
        }},
        "required": ["user", "metadata"]
    }}"#
    )
    .expect("Failed to write to temp file");

    // Test text format output
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "tct",
            "--",
            temp_file.path().to_str().unwrap(),
            "--analyze",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(
        output.status.success(),
        "CLI should succeed with complex schema"
    );

    let stdout_text = String::from_utf8_lossy(&output.stdout);

    // Verify all major sections are present
    assert!(
        stdout_text.contains("📊 Overview"),
        "Should contain overview section"
    );
    assert!(
        stdout_text.contains("🏗️  Schema Composition"),
        "Should contain composition section"
    );
    assert!(
        stdout_text.contains("🎯 Optimal Vocabulary Size"),
        "Should contain optimal size recommendation"
    );
    assert!(
        stdout_text.contains("🔧 Recommended Compression Strategies"),
        "Should contain strategies section"
    );
    assert!(
        stdout_text.contains("📈 Vocabulary Efficiency Analysis"),
        "Should contain efficiency analysis"
    );
    assert!(
        stdout_text.contains("📝 Summary"),
        "Should contain summary section"
    );

    // Verify specific analysis content
    assert!(
        stdout_text.contains("Total positions analyzed:"),
        "Should show position count"
    );
    assert!(
        stdout_text.contains("Total vocabulary required:"),
        "Should show total vocabulary"
    );
    assert!(
        stdout_text.contains("Schema complexity score:"),
        "Should show complexity score"
    );
    assert!(
        stdout_text.contains("enum_optimization"),
        "Should recommend enum optimization"
    );
}

/// Test JSON format output with validation
#[test]
fn test_vocabulary_analysis_json_output_integration() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "title": "TestSchema",
        "type": "object",
        "properties": {{
            "status": {{ "enum": ["active", "pending"] }},
            "priority": {{ "type": "integer", "minimum": 1, "maximum": 5 }}
        }}
    }}"#
    )
    .expect("Failed to write to temp file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "tct",
            "--",
            temp_file.path().to_str().unwrap(),
            "--analyze",
            "--format",
            "json",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(
        output.status.success(),
        "CLI should succeed with JSON format"
    );

    let stdout_text = String::from_utf8_lossy(&output.stdout);

    // Parse JSON to verify structure
    let json_output: Value =
        serde_json::from_str(&stdout_text).expect("Output should be valid JSON");

    // Verify JSON structure
    assert!(
        json_output.get("schema_file").is_some(),
        "Should contain schema_file"
    );
    assert!(
        json_output.get("analysis").is_some(),
        "Should contain analysis section"
    );
    assert!(
        json_output.get("recommendations").is_some(),
        "Should contain recommendations section"
    );

    // Verify analysis section
    let analysis = json_output.get("analysis").unwrap();
    assert!(
        analysis.get("total_positions").is_some(),
        "Should contain total_positions"
    );
    assert!(
        analysis.get("max_vocabulary_size").is_some(),
        "Should contain max_vocabulary_size"
    );
    assert!(
        analysis.get("complexity_metrics").is_some(),
        "Should contain complexity_metrics"
    );

    // Verify recommendations section
    let recommendations = json_output.get("recommendations").unwrap();
    assert!(
        recommendations.get("optimal_vocab_size").is_some(),
        "Should contain optimal_vocab_size"
    );
    assert!(
        recommendations.get("compression_strategies").is_some(),
        "Should contain compression_strategies"
    );
    assert!(
        recommendations.get("efficiency_analysis").is_some(),
        "Should contain efficiency_analysis"
    );
    assert!(
        recommendations.get("warnings").is_some(),
        "Should contain warnings"
    );

    // Verify efficiency analysis array structure
    let efficiency_analysis = recommendations.get("efficiency_analysis").unwrap();
    assert!(
        efficiency_analysis.is_array(),
        "Efficiency analysis should be an array"
    );
    let first_efficiency = &efficiency_analysis.as_array().unwrap()[0];
    assert!(
        first_efficiency.get("vocab_size").is_some(),
        "Should contain vocab_size"
    );
    assert!(
        first_efficiency.get("efficiency_score").is_some(),
        "Should contain efficiency_score"
    );
    assert!(
        first_efficiency
            .get("multi_token_encoding_needed")
            .is_some(),
        "Should contain multi_token_encoding_needed"
    );
}

/// Test custom comparison sizes integration
#[test]
fn test_vocabulary_analysis_custom_comparison_integration() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "type": "object",
        "properties": {{
            "large_enum": {{ "enum": [
                "option1", "option2", "option3", "option4", "option5",
                "option6", "option7", "option8", "option9", "option10"
            ] }}
        }}
    }}"#
    )
    .expect("Failed to write to temp file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "tct",
            "--",
            temp_file.path().to_str().unwrap(),
            "--analyze",
            "--compare",
            "8,16,32",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(
        output.status.success(),
        "CLI should succeed with custom comparison sizes"
    );

    let stdout_text = String::from_utf8_lossy(&output.stdout);

    // Verify that custom comparison sizes are reflected in the analysis
    assert!(
        stdout_text.contains("Analyzed 3 vocabulary sizes: [8, 16, 32]"),
        "Should show custom vocab sizes in summary"
    );

    // Verify efficiency table shows the custom sizes
    assert!(
        stdout_text.contains("8           "),
        "Should show vocab size 8 in efficiency table"
    );
    assert!(
        stdout_text.contains("16          "),
        "Should show vocab size 16 in efficiency table"
    );
    assert!(
        stdout_text.contains("32          "),
        "Should show vocab size 32 in efficiency table"
    );

    // Should show multi-token encoding needed for smaller vocabularies
    assert!(
        stdout_text.contains("Yes"),
        "Should indicate multi-token encoding needed for insufficient vocab size"
    );
}

/// Test error handling integration
#[test]
fn test_vocabulary_analysis_error_handling_integration() {
    // Test with invalid JSON schema
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "type": "invalid_type",
        "properties": {{}}
    }}"#
    )
    .expect("Failed to write to temp file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "tct",
            "--",
            temp_file.path().to_str().unwrap(),
            "--analyze",
        ])
        .output()
        .expect("Failed to run CLI");

    // Should handle invalid schema gracefully
    let stderr_text = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        assert!(
            stderr_text.contains("failed") || stderr_text.contains("error"),
            "Should provide meaningful error message"
        );
    }
}

/// Test help integration
#[test]
fn test_vocabulary_analysis_help_integration() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "tct", "--", "--help"])
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success(), "Help should succeed");

    let help_text = String::from_utf8_lossy(&output.stdout);

    // Verify vocabulary analysis options are documented
    assert!(
        help_text.contains("--analyze"),
        "Help should mention --analyze option"
    );
    assert!(
        help_text.contains("--format"),
        "Help should mention --format option"
    );
    assert!(
        help_text.contains("--compare"),
        "Help should mention --compare option"
    );
    assert!(
        help_text.contains("vocabulary requirements"),
        "Help should describe vocabulary analysis"
    );
}

/// Test edge cases integration
#[test]
fn test_vocabulary_analysis_edge_cases_integration() {
    // Test with minimal schema
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "type": "string"
    }}"#
    )
    .expect("Failed to write to temp file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "tct",
            "--",
            temp_file.path().to_str().unwrap(),
            "--analyze",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success(), "CLI should handle minimal schema");

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_text.contains("📊 Overview"),
        "Should still provide analysis overview"
    );
    assert!(
        stdout_text.contains("Schema analysis completed successfully"),
        "Should complete successfully"
    );
}
