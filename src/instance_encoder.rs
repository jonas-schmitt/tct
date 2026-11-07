//! Instance Encoder Module
//!
//! Provides reusable infrastructure for encoding JSON instances using TCT-generated tokenizers.
//!
//! This module creates temporary Cargo projects with generated tokenizer code, compiles them,
//! and uses them to encode JSON instances to token sequences. This approach ensures that
//! instance encoding uses the exact same tokenizer logic as the generated code.
//!
//! # Architecture
//!
//! The encoding process follows these steps:
//! 1. Generate TCT tokenizer code from schema
//! 2. Create temporary Cargo project with generated code
//! 3. Compile the project to create encoder binary
//! 4. Use the binary to encode each JSON instance
//! 5. Collect and return token sequences
//!
//! # Example
//!
//! ```ignore
//! use tct::TctCompiler;
//! use tct::instance_encoder::InstanceEncoder;
//!
//! // Compile schema to get tokenizer code
//! let compiler = TctCompiler::new(schema, "MyType")?;
//! let grammar = compiler.compile()?;
//! let code = grammar.generate_code()?;
//!
//! // Create encoder with root type name
//! let encoder = InstanceEncoder::new(&code, &grammar.root_type_name)?;
//!
//! // Encode instances
//! let tokens = encoder.encode_file(Path::new("instance.json"))?;
//! let sequences = encoder.encode_batch(&paths)?;
//! ```

use crate::generated_crate::GeneratedCargoToml;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

mod generated_helper;
use generated_helper::{GeneratedHelperMainInput, render_generated_helper_main};
mod helper_project;
use helper_project::build_generated_helper;
mod encode_failure;
mod metrics;
mod process;
pub use encode_failure::{
    EncodeFailure, EncodeFailureClass, UnencodableFailureEntry, UnencodableReport,
    classify_encode_failure_message,
};
pub use metrics::{
    BatchAllocationMetrics, BatchDecodePrefixMetrics, BatchEncodeHotPathMetrics,
    BatchEncodePhaseMetrics, BatchEncodeValidationPhaseMetrics, BatchParentIoMetrics,
    BatchPrimaryRuntimeMetrics, BatchRuntimeMetrics, token_sequence_checksum,
};
/// Instance encoder that can encode JSON instances using TCT-generated tokenizers
///
/// This struct manages a temporary Cargo project with the generated tokenizer code
/// and provides methods to encode individual files or batches of files.
pub struct InstanceEncoder {
    project_dir: PathBuf,
    encoder_bin: PathBuf,
    allocation_metrics_enabled: bool,
}

impl InstanceEncoder {
    /// Create a new instance encoder from generated tokenizer code
    ///
    /// This creates a temporary Cargo project, generates the necessary files,
    /// and compiles the encoder binary.
    ///
    /// # Arguments
    /// * `tokenizer_code` - Generated TCT tokenizer code (from `Grammar::generate_code()`)
    /// * `root_type_name` - Name of the root type to deserialize (e.g., "Workflow")
    ///
    /// # Returns
    /// An encoder ready to encode JSON instances, or an error if compilation fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - Temporary directory creation fails
    /// - Code generation fails
    /// - Cargo compilation fails
    pub fn new(tokenizer_code: &str, root_type_name: &str) -> Result<Self, Box<dyn Error>> {
        Self::new_with_options(tokenizer_code, root_type_name, false)
    }

    /// Create a new instance encoder with generated-process allocation metrics enabled.
    ///
    /// This is intended for measurement workflows. Ordinary encoders should use
    /// `new()` so temporary generated helpers do not install a counting global
    /// allocator.
    pub fn new_with_allocation_metrics(
        tokenizer_code: &str,
        root_type_name: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Self::new_with_options(tokenizer_code, root_type_name, true)
    }

    fn new_with_options(
        tokenizer_code: &str,
        root_type_name: &str,
        allocation_metrics_enabled: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let built =
            build_generated_helper(tokenizer_code, root_type_name, allocation_metrics_enabled)?;

        Ok(Self {
            project_dir: built.project_dir,
            encoder_bin: built.encoder_bin,
            allocation_metrics_enabled,
        })
    }

    /// Get the path to the temporary project directory
    ///
    /// Useful for debugging or manual inspection of the generated project
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// Get the path to the encoder binary
    pub fn encoder_binary(&self) -> &Path {
        &self.encoder_bin
    }

    /// Remove the temporary generated Cargo project after a successful run.
    ///
    /// Failed helper builds are reported before an `InstanceEncoder` exists, so
    /// this opt-in cleanup is intended for successful measurement workflows
    /// that no longer need to inspect the generated source.
    pub fn cleanup_project_dir(&self) -> Result<(), std::io::Error> {
        match fs::remove_dir_all(&self.project_dir) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

/// Expand glob pattern to list of file paths
///
/// # Arguments
/// * `pattern` - Glob pattern (e.g., "data/*.json")
///
/// # Returns
/// Vector of paths matching the pattern
///
/// # Errors
/// Returns an error if the glob pattern is invalid
pub fn expand_glob_pattern(pattern: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let paths: Vec<PathBuf> = glob::glob(pattern)?.filter_map(Result::ok).collect();
    Ok(paths)
}

/// Create a temporary Cargo project with the TCT tokenizer
///
/// This function generates all necessary files for a standalone encoder binary:
/// - Cargo.toml with dependencies
/// - src/main.rs with encoder binary implementation
/// - Generated tokenizer code (embedded in main.rs)
///
/// # Arguments
/// * `project_dir` - Directory where the project should be created
/// * `tokenizer_code` - Generated TCT tokenizer code
///
/// # Errors
/// Returns an error if file operations fail
fn create_encoder_project(
    project_dir: &Path,
    helper_name: &str,
    tokenizer_code: &str,
    root_type_name: &str,
    allocation_metrics_enabled: bool,
) -> Result<(), Box<dyn Error>> {
    // Create project structure
    fs::create_dir_all(project_dir)?;
    fs::create_dir_all(project_dir.join("src"))?;

    // Path of the TCT crate that the helper depends on
    let tct_path = crate::generated_crate::tct_source_dir()?;

    // Create Cargo.toml - link against TCT for tokenizer support
    let cargo_toml = GeneratedCargoToml::new(helper_name)
        .with_rustc_hash()
        .with_regex()
        .with_jsonschema()
        .with_rayon()
        .with_chrono()
        .with_base64()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(helper_name, "src/main.rs")
        .render();
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    let main_rs = render_generated_helper_main(GeneratedHelperMainInput {
        tokenizer_code,
        root_type_name,
        allocation_metrics_enabled,
    });

    fs::write(project_dir.join("src/main.rs"), main_rs)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;
    use std::fs;
    use tempfile::tempdir;

    fn parse_generated_file(code: &str) -> syn::File {
        syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
    }

    fn generated_file_tokens_contain(file: &syn::File, expected: &str) -> bool {
        file.to_token_stream().to_string().contains(expected)
    }

    #[test]
    fn test_expand_glob_pattern_no_matches() {
        let result = expand_glob_pattern("nonexistent/*.json").unwrap();
        assert_eq!(result.len(), 0, "Should return empty vector for no matches");
    }

    #[test]
    fn test_expand_glob_pattern_invalid() {
        let result = expand_glob_pattern("[invalid");
        assert!(result.is_err(), "Should fail on invalid glob pattern");
    }

    #[test]
    fn test_expand_glob_pattern_returns_matches() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("first.json");
        let second = dir.path().join("second.json");
        let ignored = dir.path().join("ignored.txt");
        fs::write(&first, "{}").unwrap();
        fs::write(&second, "{}").unwrap();
        fs::write(&ignored, "{}").unwrap();

        let pattern = dir.path().join("*.json").display().to_string();
        let mut result = expand_glob_pattern(&pattern).unwrap();
        result.sort();

        assert_eq!(result, vec![first, second]);
    }

    #[test]
    fn test_cleanup_project_dir_removes_generated_project() {
        let dir = tempdir().unwrap();
        let project_dir = dir.path().join("generated-helper");
        fs::create_dir_all(project_dir.join("src")).unwrap();
        fs::write(project_dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        let encoder = InstanceEncoder {
            project_dir: project_dir.clone(),
            encoder_bin: project_dir.join("target/release/tct-encoder"),
            allocation_metrics_enabled: false,
        };

        encoder.cleanup_project_dir().unwrap();
        assert!(!project_dir.exists());
        encoder.cleanup_project_dir().unwrap();
    }

    #[test]
    fn test_create_encoder_project_writes_base_encoder_main() {
        let dir = tempdir().unwrap();
        let tokenizer_code = r#"
pub mod tokenizer {
    pub trait JsonTokenEncoder {}
}

log_debug!(
    "hidden",
    42
);
"#;

        create_encoder_project(dir.path(), "tct-encoder", tokenizer_code, "RootType", false)
            .unwrap();

        let cargo_toml = fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
        let main_rs = fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
        let main_file = parse_generated_file(&main_rs);
        assert!(cargo_toml.contains("name = \"tct-encoder\""));
        assert!(generated_file_tokens_contain(
            &main_file,
            "let instance : RootType = serde_json :: from_str (& content) ?"
        ));
        assert!(generated_file_tokens_contain(
            &main_file,
            "serde_json :: from_str :: < RootType > (& content)"
        ));
        assert!(main_rs.contains("let instance = RootType::decode(&mut iter)?;"));
        assert!(main_rs.contains("\"decode-batch\" =>"));
        assert!(main_rs.contains("\"encode-batch-profile\" =>"));
        assert!(main_rs.contains("\"encode-batch-hot-path\" =>"));
        assert!(main_rs.contains("\"encode-batch-validation-profile\" =>"));
        assert!(main_rs.contains("match RootType::decode(&mut iter)"));
        assert!(main_rs.contains("start_allocation_tracking()"));
        assert!(main_rs.contains("finish_allocation_tracking(\"encode-batch\""));
        assert!(main_rs.contains("finish_batch_timing(\"encode-batch\""));
        assert!(main_rs.contains("finish_batch_timing(\"decode-batch\""));
        assert!(main_rs.contains("TCT_ENCODE_PHASE_METRICS"));
        assert!(main_rs.contains("TCT_ENCODE_HOT_PATH_METRICS"));
        assert!(main_rs.contains("TCT_ENCODE_VALIDATION_PHASE_METRICS"));
        assert!(main_rs.contains("finish_encode_phase_metrics("));
        assert!(main_rs.contains("finish_encode_hot_path_metrics("));
        assert!(main_rs.contains("finish_encode_validation_phase_metrics("));
        assert!(main_rs.contains("profile.base_encode_ns = elapsed_ns_u64(base_encode_start);"));
        assert!(generated_file_tokens_contain(
            &main_file,
            "serde_json :: from_value :: < RootType > (value)"
        ));
        assert!(main_rs.contains("false,\n            );"));
        assert!(!main_rs.contains("#[global_allocator]"));
        assert!(!main_rs.contains("TCT_ALLOCATION_METRICS"));
        assert!(main_rs.contains("// log_debug!("));
        assert!(main_rs.contains("//     \"hidden\","));
        assert!(!main_rs.contains("codec::encode(&instance)"));
    }

    #[test]
    fn test_create_encoder_project_can_enable_allocation_tracking() {
        let dir = tempdir().unwrap();
        let tokenizer_code = r#"
pub mod tokenizer {
    pub trait JsonTokenEncoder {}
}
"#;

        create_encoder_project(dir.path(), "tct-encoder", tokenizer_code, "RootType", true)
            .unwrap();

        let main_rs = fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
        assert!(main_rs.contains("#[global_allocator]"));
        assert!(main_rs.contains("TCT_ALLOCATION_METRICS"));
        assert!(main_rs.contains("finish_allocation_tracking(\"encode-batch\""));
        assert!(!main_rs.contains("codec::encode(&instance)"));
    }

    #[test]
    fn test_create_encoder_project_pins_generated_dependencies() {
        let dir = tempdir().unwrap();
        create_encoder_project(
            dir.path(),
            "tct-encoder",
            "pub mod tokenizer {}",
            "RootType",
            false,
        )
        .unwrap();

        let cargo_toml = fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();

        assert!(cargo_toml.contains("rust-version = \"1.95\""));
        assert!(cargo_toml.contains("serde = { version = \"=1.0.228\", features = [\"derive\"] }"));
        assert!(cargo_toml.contains("serde_json = \"=1.0.149\""));
        assert!(cargo_toml.contains("rustc-hash = \"=2.1.2\""));
        assert!(cargo_toml.contains("regex = \"=1.12.3\""));
        assert!(
            cargo_toml.contains("jsonschema = { version = \"=0.46.2\", default-features = false }")
        );
        assert!(cargo_toml.contains("rayon = \"=1.12.0\""));
        assert!(cargo_toml.contains("chrono = { version = \"=0.4.44\", features = [\"serde\"] }"));
        assert!(cargo_toml.contains("base64 = \"=0.22.1\""));
    }

    #[test]
    fn test_create_encoder_project_uses_helper_name_for_package_and_bin() {
        let dir = tempdir().unwrap();

        create_encoder_project(
            dir.path(),
            "tct-encoder-123-0",
            "pub mod tokenizer {}",
            "RootType",
            false,
        )
        .unwrap();

        let cargo_toml = fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();

        assert!(cargo_toml.contains("name = \"tct-encoder-123-0\""));
        assert!(cargo_toml.contains("[[bin]]"));
        assert!(cargo_toml.contains("path = \"src/main.rs\""));
    }

    #[test]
    fn test_create_encoder_project_writes_bpe_encoder_main_when_codec_exists() {
        let dir = tempdir().unwrap();
        let tokenizer_code = r#"
pub mod tokenizer {
    pub trait JsonTokenEncoder {}
}

pub mod codec {}
"#;

        create_encoder_project(dir.path(), "tct-encoder", tokenizer_code, "RootType", false)
            .unwrap();

        let main_rs = fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
        let main_file = parse_generated_file(&main_rs);
        assert!(main_rs.contains("let tokens = codec::encode(&instance);"));
        assert!(main_rs.contains("let (instance, _, _) = codec::decode::<RootType>(tokens)?;"));
        assert!(main_rs.contains("match codec::decode::<RootType>(tokens)"));
        assert!(main_rs.contains("\"decode-batch\" =>"));
        assert!(main_rs.contains("\"encode-batch-profile\" =>"));
        assert!(main_rs.contains("\"encode-batch-hot-path\" =>"));
        assert!(main_rs.contains("\"encode-batch-validation-profile\" =>"));
        assert!(main_rs.contains("let mut base_tokens = Vec::new();"));
        assert!(main_rs.contains("instance.encode(&mut base_tokens);"));
        assert!(generated_file_tokens_contain(
            &main_file,
            "let tokens = bpe :: encode_owned (base_tokens , VOCAB_SIZE)"
        ));
        assert!(main_rs.contains("profile.bpe_encode_ns = elapsed_ns_u64(bpe_encode_start);"));
        assert!(generated_file_tokens_contain(
            &main_file,
            "serde_json :: from_value :: < RootType > (value)"
        ));
        assert!(main_rs.contains("update_tct_token_checksum(token_checksum, tokens);"));
        assert!(main_rs.contains("true,\n            );"));
        assert!(main_rs.contains("\"bpe-encode-batch\" =>"));
        assert!(main_rs.contains("let operation_start = Instant::now();"));
        assert!(generated_file_tokens_contain(
            &main_file,
            "bpe :: encode_owned (tokens , VOCAB_SIZE)"
        ));
        assert!(main_rs.contains("finish_batch_timing(\"encode-batch\""));
        assert!(main_rs.contains("finish_batch_timing(\"decode-batch\""));
        assert!(main_rs.contains("finish_allocation_tracking(\"bpe-encode-batch\""));
        assert!(main_rs.contains("finish_batch_timing(\"bpe-encode-batch\""));
        assert!(main_rs.contains("\"bpe-decode-batch\" =>"));
        assert!(generated_file_tokens_contain(
            &main_file,
            "bpe :: decode (tokens , VOCAB_SIZE)"
        ));
        assert!(main_rs.contains("finish_allocation_tracking(\"bpe-decode-batch\""));
        assert!(main_rs.contains("finish_batch_timing(\"bpe-decode-batch\""));
        assert!(!main_rs.contains("let instance = RootType::decode(&mut iter)?;"));
    }
}
