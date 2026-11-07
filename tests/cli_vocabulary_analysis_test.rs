//! CLI Vocabulary Analysis Test Suite
//!
//! This test suite validates the CLI argument parsing for vocabulary analysis
//! functionality. Tests ensure proper argument validation and error handling.

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Test that CLI shows help for analyze option
#[test]
fn test_cli_vocabulary_analyze_help_shown() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "tct", "--", "--help"])
        .output()
        .expect("Failed to run CLI");

    let help_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        help_text.contains("--analyze"),
        "CLI help should mention --analyze option"
    );
    assert!(
        help_text.contains("vocabulary requirements"),
        "CLI help should describe vocabulary analysis"
    );
}

/// Test that analyze flag requires an input file
#[test]
fn test_cli_vocabulary_analyze_requires_input() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "tct", "--", "--analyze"])
        .output()
        .expect("Failed to run CLI");

    // Should fail because no input file provided
    assert!(!output.status.success(), "CLI should require input file");
}

/// Test basic analyze functionality with valid schema
#[test]
fn test_cli_vocabulary_analyze_basic_functionality() {
    // Create a temporary schema file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "title": "TestSchema",
        "type": "object",
        "properties": {{
            "name": {{ "type": "string" }},
            "status": {{ "enum": ["active", "pending"] }}
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
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(
        output.status.success(),
        "CLI should succeed with valid schema"
    );

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_text.contains("Schema Vocabulary Analysis"),
        "Should show analysis header"
    );
    assert!(
        stdout_text.contains("📊 Overview"),
        "Should show analysis overview section"
    );
}

/// Test JSON format output
#[test]
fn test_cli_vocabulary_analyze_json_format() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "title": "TestSchema",
        "type": "object",
        "properties": {{
            "name": {{ "type": "string" }}
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
    assert!(
        stdout_text.contains("\"schema_file\""),
        "Should show JSON output with schema file"
    );
    assert!(
        stdout_text.contains("\"analysis\""),
        "Should show analysis section in JSON"
    );
}

/// Test custom comparison sizes
#[test]
fn test_cli_vocabulary_analyze_custom_compare_sizes() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "title": "TestSchema",
        "type": "object",
        "properties": {{
            "name": {{ "type": "string" }}
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
            "16,32,64",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(
        output.status.success(),
        "CLI should succeed with custom comparison sizes"
    );

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_text.contains("📈 Vocabulary Efficiency Analysis"),
        "Should show efficiency analysis section"
    );
}

/// Test invalid comparison sizes
#[test]
fn test_cli_vocabulary_analyze_invalid_compare_sizes() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"{{
        "title": "TestSchema",
        "type": "object"
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
            "invalid,64",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(
        !output.status.success(),
        "CLI should fail with invalid comparison sizes"
    );

    let stderr_text = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_text.contains("Invalid vocabulary size"),
        "Should show error for invalid sizes"
    );
}

/// Test that format option requires analyze
#[test]
fn test_cli_vocabulary_format_requires_analyze() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp_file, r#"{{ "type": "object" }}"#).expect("Failed to write to temp file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "tct",
            "--",
            temp_file.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(
        !output.status.success(),
        "CLI should fail when format used without analyze"
    );
}

/// Test that compare option requires analyze
#[test]
fn test_cli_vocabulary_compare_requires_analyze() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp_file, r#"{{ "type": "object" }}"#).expect("Failed to write to temp file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "tct",
            "--",
            temp_file.path().to_str().unwrap(),
            "--compare",
            "64,128",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(
        !output.status.success(),
        "CLI should fail when compare used without analyze"
    );
}
