//! CLI Integration Tests
//!
//! This test suite validates the command-line interface functionality
//! for both optimization levels and optimization passes.

use serde_json::json;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};

fn tct_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tct")
}

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn path_ends_with_ident(path: &syn::Path, name: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

fn type_ends_with_ident(ty: &syn::Type, name: &str) -> bool {
    matches!(ty, syn::Type::Path(type_path) if path_ends_with_ident(&type_path.path, name))
}

fn generated_file_has_enum(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Enum(item_enum) => item_enum.ident == name,
        _ => false,
    })
}

fn generated_file_has_trait_impl_for(file: &syn::File, type_name: &str, trait_name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Impl(item_impl) => {
            type_ends_with_ident(&item_impl.self_ty, type_name)
                && item_impl
                    .trait_
                    .as_ref()
                    .is_some_and(|(_, path, _)| path_ends_with_ident(path, trait_name))
        }
        _ => false,
    })
}

mod rust_output {
    use super::*;

    #[test]
    fn test_cli_dry_run_compiles_and_skips_output_write() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");
        let output_path = temp_dir.path().join("output.rs");

        let schema = json!({
            "title": "DryRunSchema",
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {"type": "string"},
                "enabled": {"type": "boolean"}
            }
        });

        fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap())
            .expect("Failed to write schema");

        let output = Command::new(tct_bin())
            .args([
                &schema_path.to_string_lossy(),
                "--dry-run",
                "-o",
                &output_path.to_string_lossy(),
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output.status.success(),
            "CLI command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(
            !output_path.exists(),
            "--dry-run should not create the requested Rust output file"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.trim().is_empty(),
            "--dry-run should suppress generated-code stdout"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Dry run completed")
                && stderr.contains("bytes=")
                && stderr.contains("sha256=")
                && stderr.contains("would write to:"),
            "--dry-run should report generated-code metadata on stderr: {stderr}"
        );
    }

    #[test]
    fn test_cli_stdout_output_is_valid_rust_source() {
        // Without -o the generated code goes to stdout, so `tct schema.json >
        // lib.rs` must produce a file that compiles; informational lines belong
        // on stderr.
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("stdout_schema.json");
        fs::write(
            &schema_path,
            serde_json::to_string_pretty(&json!({
                "title": "StdoutSchema",
                "type": "object",
                "required": ["name"],
                "properties": {"name": {"type": "string"}}
            }))
            .unwrap(),
        )
        .expect("Failed to write schema");

        let output = Command::new(tct_bin())
            .arg(&schema_path)
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output.status.success(),
            "CLI command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
        let file = parse_generated_code(&stdout);
        assert!(
            file.items
                .iter()
                .any(|item| matches!(item, syn::Item::Mod(module) if module.ident == "tokenizer")),
            "stdout should be the generated Rust module"
        );
    }

    #[test]
    fn test_cli_help_does_not_offer_the_removed_lib_flag() {
        // `--lib` wrapped the output in a second `tokenizer` module and produced
        // code that does not compile, so the flag was removed.
        let output = Command::new(tct_bin())
            .arg("--help")
            .output()
            .expect("Failed to execute CLI command");
        assert!(output.status.success());
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(
            !help.contains("--lib"),
            "--help should not list the removed --lib flag:\n{help}"
        );
    }

    #[test]
    fn test_cli_dry_run_rejects_python_target() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");

        fs::write(
            &schema_path,
            serde_json::to_string_pretty(&json!({
                "title": "DryRunSchema",
                "type": "object",
                "properties": {}
            }))
            .unwrap(),
        )
        .expect("Failed to write schema");

        let output = Command::new(tct_bin())
            .args([
                &schema_path.to_string_lossy(),
                "--target",
                "python",
                "--dry-run",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            !output.status.success(),
            "--dry-run should reject Python wheel generation until that preflight is explicit"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--dry-run is only supported for --target rust"),
            "Error should explain the target restriction: {stderr}"
        );
    }
}

/// CLI Optimization Level Tests
mod optimization_levels {
    use super::*;

    /// Test that -O0 flag generates code successfully
    ///
    /// NOTE: Enum encoding is always index-based (0, 1, 2...) regardless of optimization
    /// level, as this is the natural and efficient representation for enum variants.
    /// The O0 flag primarily affects other optimizations like pattern analysis and
    /// vocabulary optimization.
    #[test]
    fn test_cli_cli_o0_flag_generates_unoptimized_code() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");
        let output_path = temp_dir.path().join("output.rs");

        // Create test schema with enum
        let schema = json!({
            "title": "TestEnum",
            "type": "string",
            "enum": ["red", "green", "blue"]
        });

        fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap())
            .expect("Failed to write schema");

        // Run CLI with -O0 flag
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &schema_path.to_string_lossy(),
                "-O0",
                "-o",
                &output_path.to_string_lossy(),
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output.status.success(),
            "CLI command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let generated_code =
            fs::read_to_string(&output_path).expect("Failed to read generated code");
        let generated_file = parse_generated_code(&generated_code);

        // Verify code was generated successfully
        assert!(
            generated_file_has_trait_impl_for(&generated_file, "TestEnum", "JsonTokenEncoder"),
            "O0 should generate valid encoder implementation"
        );
        assert!(
            generated_file_has_enum(&generated_file, "TestEnum"),
            "O0 should generate the enum type"
        );

        println!("✅ CLI -O0 flag correctly generates code");
    }

    /// Test that -O2 flag generates optimized code (index-based enums)
    #[test]
    fn test_cli_cli_o2_flag_generates_optimized_code() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");
        let output_path = temp_dir.path().join("output.rs");

        // Create test schema with enum
        let schema = json!({
            "title": "TestEnum",
            "type": "string",
            "enum": ["red", "green", "blue"]
        });

        fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap())
            .expect("Failed to write schema");

        // Run CLI with -O2 flag (default optimization)
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &schema_path.to_string_lossy(),
                "-O2",
                "-o",
                &output_path.to_string_lossy(),
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output.status.success(),
            "CLI command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let generated_code =
            fs::read_to_string(&output_path).expect("Failed to read generated code");

        // O2 should use index encoding
        assert!(
            generated_code.contains("0u32") && generated_code.contains("1u32"),
            "O2 should generate index-based enum optimization"
        );

        println!("✅ CLI -O2 flag correctly generates optimized code");
    }

    /// Test that CLI uses -O2 optimization by default
    #[test]
    fn test_cli_cli_default_uses_o2_optimization() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");
        let output_path = temp_dir.path().join("output.rs");

        let schema = json!({
            "title": "TestEnum",
            "type": "string",
            "enum": ["red", "green", "blue"]
        });

        fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap())
            .expect("Failed to write schema");

        // Run CLI without explicit optimization level
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &schema_path.to_string_lossy(),
                "-o",
                &output_path.to_string_lossy(),
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output.status.success(),
            "CLI command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let generated_code =
            fs::read_to_string(&output_path).expect("Failed to read generated code");

        // Default should use O2 optimization (index encoding)
        assert!(
            generated_code.contains("0u32") && generated_code.contains("1u32"),
            "Default should use O2 optimization"
        );

        println!("✅ CLI correctly defaults to O2 optimization level");
    }

    /// Test that -O3 flag works (should be more aggressive than -O2)
    #[test]
    fn test_cli_cli_o3_flag_works() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");
        let output_path = temp_dir.path().join("output.rs");

        let schema = json!({
            "title": "TestComplexEnum",
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                },
                "priority": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10
                }
            }
        });

        fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap())
            .expect("Failed to write schema");

        // Run CLI with -O3 flag
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &schema_path.to_string_lossy(),
                "-O3",
                "-o",
                &output_path.to_string_lossy(),
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output.status.success(),
            "CLI command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let generated_code =
            fs::read_to_string(&output_path).expect("Failed to read generated code");

        // O3 should at least include O2 optimizations
        assert!(
            generated_code.contains("0u32") || generated_code.contains("BoundedU8"),
            "O3 should include enum or integer optimizations"
        );

        println!("✅ CLI -O3 flag works correctly");
    }

    /// Test that size and vocabulary profiles are accepted by the CLI.
    #[test]
    fn test_cli_cli_size_and_vocab_profiles_work() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");

        let schema = json!({
            "title": "ProfileSchema",
            "type": "object",
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

        fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap())
            .expect("Failed to write schema");

        for (args, label, output_name) in [
            (vec!["-Os"], "-Os", "output_os.rs"),
            (vec!["-OVocab"], "-OVocab", "output_ovocab.rs"),
            (
                vec!["--optimization", "Os"],
                "--optimization Os",
                "output_os_long.rs",
            ),
            (
                vec!["--optimization", "OVocab"],
                "--optimization OVocab",
                "output_ovocab_long.rs",
            ),
        ] {
            let output_path = temp_dir.path().join(output_name);
            let mut command = Command::new("cargo");
            command.args(["run", "--bin", "tct", "--"]);
            command.arg(&schema_path);
            command.args(args);
            command.arg("-o").arg(&output_path);
            let output = command.output().expect("Failed to execute CLI command");

            assert!(
                output.status.success(),
                "CLI command failed for {}: {}",
                label,
                String::from_utf8_lossy(&output.stderr)
            );

            let generated_code =
                fs::read_to_string(&output_path).expect("Failed to read generated code");
            assert!(
                generated_code.contains("ProfileSchema"),
                "{} should generate the root type",
                label
            );
            assert!(
                !generated_code.contains("timestamp_opt"),
                "{} should not enable O3-only format-specialized strings",
                label
            );
            assert!(
                generated_code.contains("presence_mask_0"),
                "{} should enable no-vocabulary-growth bit-packed optional field presence",
                label
            );
        }

        println!("✅ CLI accepts size and vocabulary profiles");
    }

    /// Test that invalid optimization level produces error
    #[test]
    fn test_cli_cli_invalid_optimization_level_error() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");

        let schema = json!({
            "title": "TestEnum",
            "type": "string",
            "enum": ["red", "green", "blue"]
        });

        fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap())
            .expect("Failed to write schema");

        // Run CLI with invalid optimization level
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &schema_path.to_string_lossy(),
                "-O9", // Invalid level
            ])
            .output()
            .expect("Failed to execute CLI command");

        // Should fail with invalid optimization level
        assert!(
            !output.status.success(),
            "CLI should fail with invalid optimization level"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid") || stderr.contains("unknown"),
            "Error message should mention invalid optimization level"
        );

        println!("✅ CLI correctly handles invalid optimization level");
    }

    /// Test that help includes optimization documentation
    #[test]
    fn test_cli_cli_help_includes_optimization_documentation() {
        let output = Command::new("cargo")
            .args(["run", "--bin", "tct", "--", "--help"])
            .output()
            .expect("Failed to execute CLI help command");

        assert!(output.status.success(), "CLI help command should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should mention optimization levels
        assert!(
            stdout.contains("-O") || stdout.contains("optimization"),
            "Help should document optimization levels"
        );
        assert!(
            stdout.contains("Os") && stdout.contains("OVocab"),
            "Help should document size and vocabulary optimization profiles"
        );

        println!("✅ CLI help includes optimization documentation");
    }

    /// Test that different optimization levels produce different outputs
    ///
    /// Uses a schema with bounded integers which will be optimized by BoundedIntegerOptimizationPass.
    /// Simple enums don't benefit from optimization passes, so we need a schema
    /// with features that are actually optimized.
    #[test]
    fn test_cli_cli_optimization_levels_produce_different_outputs() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let schema_path = temp_dir.path().join("test_schema.json");
        let output_o0_path = temp_dir.path().join("output_o0.rs");
        let output_o2_path = temp_dir.path().join("output_o2.rs");

        // Use a schema with bounded integers - these are optimized by BoundedIntegerOptimizationPass
        // O0 will use generic integer encoding, O2 will use optimized bounded encoding
        let schema = json!({
            "title": "TestBoundedInt",
            "type": "object",
            "properties": {
                "percentage": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "active", "completed"]
                }
            },
            "required": ["percentage", "status"]
        });

        fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap())
            .expect("Failed to write schema");

        // Generate with -O0
        let output_o0 = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &schema_path.to_string_lossy(),
                "-O0",
                "-o",
                &output_o0_path.to_string_lossy(),
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output_o0.status.success(),
            "O0 compilation failed: {:?}",
            String::from_utf8_lossy(&output_o0.stderr)
        );

        // Generate with -O2
        let output_o2 = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &schema_path.to_string_lossy(),
                "-O2",
                "-o",
                &output_o2_path.to_string_lossy(),
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output_o2.status.success(),
            "O2 compilation failed: {:?}",
            String::from_utf8_lossy(&output_o2.stderr)
        );

        let code_o0 = fs::read_to_string(&output_o0_path).expect("Failed to read O0 output");
        let code_o2 = fs::read_to_string(&output_o2_path).expect("Failed to read O2 output");

        // Outputs should be different - O2 applies BoundedIntegerOptimizationPass
        // which changes how bounded integers are encoded
        assert_ne!(
            code_o0, code_o2,
            "Different optimization levels should produce different code.\n\
             O2 should optimize bounded integers (0-100) differently than O0.\n\
             If this fails, the optimization passes may not be applying correctly."
        );

        println!("✅ CLI optimization levels produce different outputs");
    }
}

/// CLI Optimization Pass Tests
mod optimization_passes {
    use super::*;

    #[test]
    fn test_cli_cli_enable_pass_flag() {
        // Create a temporary schema file
        let mut temp_schema = NamedTempFile::new().unwrap();
        let schema_content = r#"{
            "title": "CLITest",
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                }
            }
        }"#;
        temp_schema.write_all(schema_content.as_bytes()).unwrap();

        // Test --enable-pass flag
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &temp_schema.path().to_string_lossy(),
                "--enable-pass",
                "enum_optimization",
                "--quiet",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(output.status.success(), "CLI command should succeed");

        let stdout = String::from_utf8(output.stdout).unwrap();

        // Check that enum optimization is applied (should see token indices 0u32, 1u32)
        assert!(
            stdout.contains("0u32"),
            "Should see enum index optimization"
        );
        assert!(
            stdout.contains("1u32"),
            "Should see enum index optimization"
        );
    }

    #[test]
    fn test_cli_cli_enable_pass_overrides_default_profile() {
        // Create a temporary schema file
        let mut temp_schema = NamedTempFile::new().unwrap();
        let schema_content = r#"{
            "title": "CLITest",
            "type": "object",
            "properties": {
                "count": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10
                }
            }
        }"#;
        temp_schema.write_all(schema_content.as_bytes()).unwrap();

        // Enabling only enum optimization must not silently fall back to the
        // default O3 profile, which would also enable bounded integers.
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &temp_schema.path().to_string_lossy(),
                "--enable-pass",
                "enum_optimization",
                "--quiet",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(output.status.success(), "CLI command should succeed");

        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            !stdout.contains("let offset = (self.0 - 1i64) as u32"),
            "--enable-pass enum_optimization should not enable bounded integer optimization"
        );
    }

    #[test]
    fn test_cli_cli_vocabulary_enable_pass_reports_stats() {
        let mut temp_schema = NamedTempFile::new().unwrap();
        let schema_content = r#"{
            "title": "CLITest",
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                },
                "count": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10
                }
            }
        }"#;
        temp_schema.write_all(schema_content.as_bytes()).unwrap();

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &temp_schema.path().to_string_lossy(),
                "--enable-pass",
                "vocabulary_optimization",
                "--metrics",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            output.status.success(),
            "CLI should accept vocabulary_optimization as a named pass"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Vocabulary Optimization"),
            "Metrics output should include vocabulary statistics for vocabulary_optimization: {stdout}"
        );
    }

    #[test]
    fn test_cli_cli_disable_all_passes_flag() {
        // Create a temporary schema file
        let mut temp_schema = NamedTempFile::new().unwrap();
        let schema_content = r#"{
            "title": "CLITest",
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                },
                "count": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10
                }
            }
        }"#;
        temp_schema.write_all(schema_content.as_bytes()).unwrap();

        // Test --disable-all-passes flag
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &temp_schema.path().to_string_lossy(),
                "--disable-all-passes",
                "--quiet",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(output.status.success(), "CLI command should succeed");

        let stdout = String::from_utf8(output.stdout).unwrap();

        // Should not see bounded integer optimization when all passes disabled
        // Note: Enum index encoding (0u32, 1u32) is always used as the natural representation
        assert!(
            !stdout.contains("BoundedU8") && !stdout.contains("BoundedU16"),
            "Should not see bounded integer optimization when all passes disabled"
        );
    }

    #[test]
    fn test_cli_cli_multiple_enable_pass_flags() {
        // Create a temporary schema file
        let mut temp_schema = NamedTempFile::new().unwrap();
        let schema_content = r#"{
            "title": "CLITest",
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                },
                "count": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10
                }
            }
        }"#;
        temp_schema.write_all(schema_content.as_bytes()).unwrap();

        // Test multiple --enable-pass flags
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &temp_schema.path().to_string_lossy(),
                "--enable-pass",
                "enum_optimization",
                "--enable-pass",
                "integer_optimization",
                "--quiet",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(output.status.success(), "CLI command should succeed");

        let stdout = String::from_utf8(output.stdout).unwrap();

        // Enum index encoding is enabled by the enum_optimization pass.
        assert!(stdout.contains("0u32"), "Should see enum index encoding");
        assert!(
            stdout.contains("let offset = (self.0 - 1i64) as u32"),
            "Should enable bounded integer optimization"
        );
    }

    #[test]
    fn test_cli_cli_invalid_pass_name_error() {
        // Create a temporary schema file
        let mut temp_schema = NamedTempFile::new().unwrap();
        let schema_content = r#"{
            "title": "CLITest",
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                }
            }
        }"#;
        temp_schema.write_all(schema_content.as_bytes()).unwrap();

        // Test invalid pass name
        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &temp_schema.path().to_string_lossy(),
                "--enable-pass",
                "invalid_pass_name",
                "--quiet",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            !output.status.success(),
            "CLI should reject unknown optimization pass names"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Unknown optimization pass") || stderr.contains("invalid_pass_name"),
            "Error message should mention problem with pass name: {stderr}"
        );
    }

    #[test]
    fn test_cli_cli_mixed_enable_pass_invalid_name_error() {
        let mut temp_schema = NamedTempFile::new().unwrap();
        let schema_content = r#"{
            "title": "CLITest",
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                },
                "count": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10
                }
            }
        }"#;
        temp_schema.write_all(schema_content.as_bytes()).unwrap();

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &temp_schema.path().to_string_lossy(),
                "--enable-pass",
                "enum_optimization",
                "--enable-pass",
                "invalid_pass_name",
                "--quiet",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            !output.status.success(),
            "CLI should reject unknown optimization pass names even when mixed with valid names"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Unknown optimization pass: 'invalid_pass_name'")
                && stderr.contains("Use one of: enum_optimization"),
            "Error should identify the mixed invalid pass and include the pass suggestion: {stderr}"
        );
    }

    #[test]
    fn test_cli_cli_reserved_pattern_pass_error() {
        let mut temp_schema = NamedTempFile::new().unwrap();
        let schema_content = r#"{
            "title": "CLITest",
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "pattern": "^[a-z]{3}$"
                }
            }
        }"#;
        temp_schema.write_all(schema_content.as_bytes()).unwrap();

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "tct",
                "--",
                &temp_schema.path().to_string_lossy(),
                "--enable-pass",
                "pattern_optimization",
                "--quiet",
            ])
            .output()
            .expect("Failed to execute CLI command");

        assert!(
            !output.status.success(),
            "CLI should reject reserved pattern optimization"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("pattern_optimization is reserved")
                && stderr.contains("patternProperties validation is always schema-driven"),
            "Error should explain reserved pattern optimization semantics: {stderr}"
        );
    }

    #[test]
    fn test_cli_cli_help_includes_pass_documentation() {
        let output = Command::new("cargo")
            .args(["run", "--bin", "tct", "--", "--help"])
            .output()
            .expect("Failed to execute CLI help command");

        assert!(output.status.success(), "CLI help command should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should mention optimization passes
        assert!(
            stdout.contains("--enable-pass") || stdout.contains("optimization"),
            "Help should document optimization pass flags"
        );
    }
}
