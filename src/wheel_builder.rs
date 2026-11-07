//! Python Wheel Builder for TCT
//!
//! Generates schema-specific Python wheels with PyO3 bindings.
//! Instead of exposing the full TCT compiler, creates pre-compiled wheels
//! containing only encode/decode functionality for a specific schema.

use crate::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};
use crate::generator::external_schemas::vendored_external_schema_resources;
use crate::sequence_bundle::{SequenceBundleManifest, SequenceBundleSchema};
use crate::{OptimizationLevel, TctCompiler};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

mod combined_lib;

use combined_lib::generate_combined_lib;

/// Configuration for Python wheel generation
#[derive(Debug, Clone)]
pub struct WheelConfig {
    /// Path to JSON schema file
    pub schema_path: String,
    /// Root type name (e.g., "GithubWorkflow")
    pub type_name: String,
    /// Python package name (e.g., "tct-workflow-tokenizer")
    pub package_name: String,
    /// Package version (default: "0.1.0")
    pub version: String,
    /// Package description (optional)
    pub description: Option<String>,
    /// Output directory for wheel (default: "wheels/")
    pub output_dir: String,
    /// BPE merge file path for compression
    pub bpe_merges_path: Option<String>,
    /// Optimization profile used for generated tokenizer code
    pub optimization_level: OptimizationLevel,
    /// Maximum base vocabulary size for multi-token encoding
    pub max_base_vocab: Option<u32>,
    /// Maximum total raw structural vocabulary
    pub max_vocab: Option<usize>,
    /// Suppress informational output
    pub quiet: bool,
    /// Enable streaming-friendly encoding for LLM autocompletion - default: false
    pub streaming_encode: bool,
    /// Digit-aligned decimal text encoding for unbounded numbers - default: false
    pub decimal_number_encoding: bool,
}

/// One schema-specific generated wheel module assembled into a bundle wrapper.
#[derive(Debug, Clone)]
pub struct BundleWheelSchemaConfig {
    /// Stable schema id used by the bundle API.
    pub schema_id: String,
    /// Importable Python module name for an existing generated schema wheel.
    pub module_name: String,
    /// Bundle-level schema-decision token.
    pub schema_token: u32,
    /// Raw schema-local generated vocabulary size.
    pub raw_vocab_size: u32,
    /// Raw schema-local PAD token. Raw payload tokens must be below this value.
    pub raw_pad_token: u32,
    /// Start of this schema's payload range in the bundle vocabulary.
    pub global_payload_start: u32,
    /// Whether the schema-local generated wheel already applies BPE internally.
    pub bpe_enabled: bool,
    /// Optional downstream sentinel text for Hugging Face-style wrappers.
    pub schema_sentinel: Option<String>,
    /// Optional generated wheel or artifact reference.
    pub generated_artifact: Option<String>,
}

impl BundleWheelSchemaConfig {
    #[must_use]
    pub fn new(
        schema_id: impl Into<String>,
        module_name: impl Into<String>,
        schema_token: u32,
        raw_vocab_size: u32,
        raw_pad_token: u32,
        global_payload_start: u32,
        bpe_enabled: bool,
    ) -> Self {
        Self {
            schema_id: schema_id.into(),
            module_name: module_name.into(),
            schema_token,
            raw_vocab_size,
            raw_pad_token,
            global_payload_start,
            bpe_enabled,
            schema_sentinel: None,
            generated_artifact: None,
        }
    }

    #[must_use]
    pub fn with_schema_sentinel(mut self, sentinel: impl Into<String>) -> Self {
        self.schema_sentinel = Some(sentinel.into());
        self
    }

    #[must_use]
    pub fn with_generated_artifact(mut self, artifact: impl Into<String>) -> Self {
        self.generated_artifact = Some(artifact.into());
        self
    }

    fn to_manifest_schema(&self) -> SequenceBundleSchema {
        let mut schema = SequenceBundleSchema::new(
            &self.schema_id,
            self.schema_token,
            self.raw_vocab_size,
            self.raw_pad_token,
            self.global_payload_start,
            self.bpe_enabled,
        );
        if let Some(sentinel) = &self.schema_sentinel {
            schema = schema.with_schema_sentinel(sentinel);
        }
        if let Some(artifact) = &self.generated_artifact {
            schema = schema.with_generated_artifact(artifact);
        }
        schema
    }
}

/// Configuration for an assembled pure-Python bundle wrapper.
///
/// The assembled wrapper imports existing schema-specific generated wheel
/// modules and dispatches raw payloads through their `encode` and `decode`
/// functions. It does not combine multiple generated Rust tokenizers in one
/// crate.
#[derive(Debug, Clone)]
pub struct BundleWheelConfig {
    /// Python package name for the assembled wrapper.
    pub package_name: String,
    /// Package version.
    pub version: String,
    /// Package description.
    pub description: Option<String>,
    /// Output directory for the generated package source.
    pub output_dir: String,
    /// Schema wheel modules included in the wrapper.
    pub schemas: Vec<BundleWheelSchemaConfig>,
    /// Optional explicit bundle vocabulary size. Inferred from ranges otherwise.
    pub vocab_size: Option<u32>,
    /// Suppress informational output.
    pub quiet: bool,
}

impl BundleWheelConfig {
    #[must_use]
    pub fn new(package_name: impl Into<String>, schemas: Vec<BundleWheelSchemaConfig>) -> Self {
        Self {
            package_name: package_name.into(),
            version: "0.1.0".to_string(),
            description: None,
            output_dir: "wheels".to_string(),
            schemas,
            vocab_size: None,
            quiet: false,
        }
    }

    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn with_output_dir(mut self, output_dir: impl Into<String>) -> Self {
        self.output_dir = output_dir.into();
        self
    }

    #[must_use]
    pub fn with_vocab_size(mut self, vocab_size: u32) -> Self {
        self.vocab_size = Some(vocab_size);
        self
    }

    #[must_use]
    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    pub fn manifest(&self) -> SequenceBundleManifest {
        let schemas: Vec<SequenceBundleSchema> = self
            .schemas
            .iter()
            .map(BundleWheelSchemaConfig::to_manifest_schema)
            .collect();
        let vocab_size = self
            .vocab_size
            .unwrap_or_else(|| infer_bundle_vocab_size(&schemas));
        SequenceBundleManifest::new(vocab_size, schemas)
    }
}

impl WheelConfig {
    /// Create new wheel configuration with required fields
    pub fn new(
        schema_path: impl Into<String>,
        type_name: impl Into<String>,
        package_name: impl Into<String>,
    ) -> Self {
        Self {
            schema_path: schema_path.into(),
            type_name: type_name.into(),
            package_name: package_name.into(),
            version: "0.1.0".to_string(),
            description: None,
            output_dir: "wheels".to_string(),
            bpe_merges_path: None,
            optimization_level: OptimizationLevel::O3,
            max_base_vocab: None,
            max_vocab: None,
            quiet: false,
            streaming_encode: false,
            decimal_number_encoding: false,
        }
    }

    /// Set package version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set package description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set output directory
    pub fn with_output_dir(mut self, output_dir: impl Into<String>) -> Self {
        self.output_dir = output_dir.into();
        self
    }

    /// Set BPE merge file path
    pub fn with_bpe_merges(mut self, bpe_merges_path: impl Into<String>) -> Self {
        self.bpe_merges_path = Some(bpe_merges_path.into());
        self
    }

    /// Set optimization profile for generated tokenizer code
    pub fn with_optimization_level(mut self, optimization_level: OptimizationLevel) -> Self {
        self.optimization_level = optimization_level;
        self
    }

    /// Enable digit-aligned decimal text encoding for unbounded numbers
    pub fn with_decimal_number_encoding(mut self) -> Self {
        self.decimal_number_encoding = true;
        self
    }

    /// Set maximum base vocabulary size for multi-token encoding
    pub fn with_max_base_vocab(mut self, max_base_vocab: u32) -> Self {
        self.max_base_vocab = Some(max_base_vocab);
        self
    }

    pub fn with_max_vocabulary(mut self, max_vocab: usize) -> Self {
        self.max_vocab = Some(max_vocab);
        self
    }

    /// Enable quiet mode (suppress output)
    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Enable streaming-friendly encoding for LLM autocompletion
    ///
    /// When enabled, generates:
    /// - decode_prefix() for truncation-tolerant decoding
    /// - All types derive Default
    ///
    /// Token format is identical to non-streaming mode (full compatibility).
    ///
    /// Default: false (disabled)
    pub fn with_streaming_encode(mut self, enable: bool) -> Self {
        self.streaming_encode = enable;
        self
    }
}

/// Build a Python wheel for the specified schema
///
/// # Returns
/// Path to the generated wheel file
pub fn build_wheel(config: &WheelConfig) -> Result<PathBuf, Box<dyn Error>> {
    if !config.quiet {
        eprintln!("🔧 TCT Wheel Builder");
        eprintln!("  Schema: {}", config.schema_path);
        eprintln!("  Package: {}", config.package_name);
        eprintln!("  Type name: {}", config.type_name);
        eprintln!("  Output: {}", config.output_dir);
        if let Some(bpe) = &config.bpe_merges_path {
            eprintln!("  BPE merges: {}", bpe);
        }
        eprintln!();
    }

    // Step 1: Compile schema to generate tokenizer code
    if !config.quiet {
        eprintln!("📖 Step 1: Compiling schema...");
    }
    let CompiledWheelCode {
        code: tokenizer_code,
        has_bpe,
        has_streaming,
        component_files,
    } = compile_schema_to_code(config)?;
    if !config.quiet {
        eprintln!(
            "   ✅ Schema compiled ({} bytes of code)",
            tokenizer_code.len()
        );
        if has_bpe {
            eprintln!("   ✅ BPE compression enabled in Python bindings");
        }
        if has_streaming {
            eprintln!("   ✅ Streaming mode enabled (decode_prefix available)");
        }
    }

    // Step 2: Create package directory structure
    if !config.quiet {
        eprintln!("📁 Step 2: Creating package directory...");
    }
    let package_dir = PathBuf::from(&config.output_dir).join(&config.package_name);
    create_package_structure(&package_dir)?;
    write_schema_component_files(&package_dir, component_files)?;
    if !config.quiet {
        eprintln!("   ✅ Package structure created: {}", package_dir.display());
    }

    // Step 3: Generate combined lib.rs (generated code + PyO3 wrapper)
    if !config.quiet {
        eprintln!("🔧 Step 3: Generating lib.rs...");
    }
    let combined_code = generate_combined_lib(
        &config.type_name,
        &config.package_name,
        &tokenizer_code,
        has_bpe,
        has_streaming,
    )?;
    let lib_path = package_dir.join("src").join("lib.rs");
    fs::write(&lib_path, combined_code)?;
    if !config.quiet {
        eprintln!("   ✅ lib.rs generated (tokenizer + PyO3 wrapper)");
    }

    // Step 4: Generate Cargo.toml
    if !config.quiet {
        eprintln!("📝 Step 4: Generating Cargo.toml...");
    }
    let cargo_toml = generate_cargo_toml(&config.package_name, &config.version)?;
    let cargo_path = package_dir.join("Cargo.toml");
    fs::write(&cargo_path, cargo_toml)?;
    let tct_path = std::env::current_dir()?;
    copy_workspace_cargo_lock(&tct_path, &package_dir)?;
    if !config.quiet {
        eprintln!("   ✅ Cargo.toml generated");
    }

    // Step 5: Generate pyproject.toml
    if !config.quiet {
        eprintln!("📝 Step 5: Generating pyproject.toml...");
    }
    let pyproject_toml = generate_pyproject_toml(
        &config.package_name,
        &config.version,
        config.description.as_ref(),
        component_files,
    )?;
    let pyproject_path = package_dir.join("pyproject.toml");
    fs::write(&pyproject_path, pyproject_toml)?;
    if !config.quiet {
        eprintln!("   ✅ pyproject.toml generated");
    }

    // Step 6: Build wheel with maturin
    if !config.quiet {
        eprintln!("🔨 Step 6: Building wheel with maturin...");
    }
    let build_started = std::time::SystemTime::now();
    build_wheel_with_maturin(&package_dir)?;
    if !config.quiet {
        eprintln!("   ✅ Wheel built successfully!");
    }

    // Step 7: Find and report wheel location
    let wheel_dir = wheel_output_dir(&package_dir);
    if wheel_dir.exists()
        && let Some(wheel_file) = find_wheel_file(&wheel_dir)?
    {
        // A stale wheel here means maturin built for a different interpreter
        // tag (or elsewhere) and we would silently report an outdated file —
        // this shipped a non-decimal wheel as "decimal" once. Fail loudly.
        let fresh = std::fs::metadata(&wheel_file)
            .and_then(|m| m.modified())
            .map(|mtime| mtime >= build_started)
            .unwrap_or(false);
        if !fresh {
            return Err(format!(
                "wheel {} predates this build; maturin likely targeted a different \
                 interpreter tag (set TCT_WHEEL_PYTHON or install maturin with the \
                 intended interpreter)",
                wheel_file.display()
            )
            .into());
        }
        if !config.quiet {
            eprintln!();
            eprintln!("🎉 SUCCESS!");
            eprintln!("   Wheel: {}", wheel_file.display());
            eprintln!();
            eprintln!("📦 Install with:");
            eprintln!("   pip install {}", wheel_file.display());
            eprintln!();
            eprintln!("🐍 Python API:");
            eprintln!("   from {} import (", config.package_name.replace('-', "_"));
            eprintln!("       encode, decode, vocab_size,");
            eprintln!(
                "       encode_sequence, decode_sequence, sequence_vocab_size, sequence_metadata_json,"
            );
            eprintln!("       strict_prefix_roles,");
            if has_streaming {
                eprintln!("       decode_prefix,  # Streaming mode");
            }
            eprintln!("   )");
        }
        return Ok(wheel_file);
    }

    Err("Failed to find generated wheel file".into())
}

/// Write a pure-Python package that bundles existing generated schema wheels.
///
/// The generated package imports each schema module by name and exposes
/// bundle-level `encode_sequence`, `decode_sequence`, `sequence_vocab_size`, and
/// `sequence_bundle_metadata_json` helpers. It writes package source only; use
/// standard Python packaging tools to build the pure-Python wheel from the
/// returned directory.
pub fn write_bundle_wrapper_package(config: &BundleWheelConfig) -> Result<PathBuf, Box<dyn Error>> {
    let package_dir = PathBuf::from(&config.output_dir).join(&config.package_name);
    let module_name = python_module_name(&config.package_name);

    if !config.quiet {
        eprintln!("🔧 TCT Bundle Wheel Wrapper");
        eprintln!("  Package: {}", config.package_name);
        eprintln!("  Schemas: {}", config.schemas.len());
        eprintln!("  Output: {}", package_dir.display());
    }

    create_bundle_package_structure(&package_dir, &module_name)?;

    let wrapper_source = generate_bundle_wrapper_module(config)?;
    fs::write(
        package_dir.join(&module_name).join("__init__.py"),
        wrapper_source,
    )?;

    let pyproject_toml = generate_bundle_pyproject_toml(
        &config.package_name,
        &config.version,
        config.description.as_ref(),
        &module_name,
    )?;
    fs::write(package_dir.join("pyproject.toml"), pyproject_toml)?;

    Ok(package_dir)
}

/// Generate the pure-Python module source for a bundle wrapper.
pub fn generate_bundle_wrapper_module(
    config: &BundleWheelConfig,
) -> Result<String, Box<dyn Error>> {
    let manifest = config.manifest();
    manifest.validate()?;
    let mut module_names = BTreeMap::new();
    for schema in &config.schemas {
        if schema.module_name.is_empty() {
            return Err(format!("schema {} has empty Python module name", schema.schema_id).into());
        }
        module_names.insert(schema.schema_id.clone(), schema.module_name.clone());
    }
    for schema in &manifest.schemas {
        if !module_names.contains_key(&schema.schema_id) {
            return Err(format!("schema {} has no Python module mapping", schema.schema_id).into());
        }
    }

    let manifest_json = serde_json::to_string(&manifest)?;
    let module_names_json = serde_json::to_string(&module_names)?;
    let manifest_literal = serde_json::to_string(&manifest_json)?;
    let module_names_literal = serde_json::to_string(&module_names_json)?;

    Ok(format!(
        r#"# Generated by TCT. Do not edit by hand.
import importlib
import json

_MANIFEST_JSON = {manifest_literal}
_SCHEMA_MODULES_JSON = {module_names_literal}
_MANIFEST = json.loads(_MANIFEST_JSON)
_SCHEMA_MODULE_NAMES = json.loads(_SCHEMA_MODULES_JSON)
_SCHEMAS_BY_ID = {{schema["schema_id"]: schema for schema in _MANIFEST["schemas"]}}
_SCHEMAS_BY_TOKEN = {{int(schema["schema_token"]): schema for schema in _MANIFEST["schemas"]}}
_MODULE_CACHE = {{}}


class SequenceBundleError(ValueError):
    pass


def sequence_bundle_metadata_json():
    return _MANIFEST_JSON


def sequence_vocab_size():
    return int(_MANIFEST["vocab_size"])


def sequence_pad_token():
    return int(_MANIFEST["special_tokens"]["pad_token"])


def bos_token():
    return int(_MANIFEST["special_tokens"]["bos_token"])


def eos_token():
    return int(_MANIFEST["special_tokens"]["eos_token"])


def schema_token(schema_id):
    return int(_schema_by_id(schema_id)["schema_token"])


def generation_mask_for_schema_token(schema_token):
    schema = _schema_by_token(schema_token)
    payload = schema["global_payload"]
    return list(range(int(payload["start"]), int(payload["end_exclusive"]))) + [eos_token()]


def encode_sequence(schema_id, json_str):
    schema = _schema_by_id(schema_id)
    module = _module_for_schema_id(schema_id)
    payload = module.encode(json_str)
    sequence = [bos_token(), int(schema["schema_token"])]
    raw_payload = schema["raw_payload"]
    global_payload = schema["global_payload"]
    raw_start = int(raw_payload["start"])
    raw_end = int(raw_payload["end_exclusive"])
    global_start = int(global_payload["start"])
    for offset, token in enumerate(payload):
        token = int(token)
        if token < raw_start or token >= raw_end:
            raise SequenceBundleError(
                f"schema {{schema_id}} local payload token {{token}} at offset {{offset}} "
                "is outside raw payload range"
            )
        sequence.append(global_start + token - raw_start)
    sequence.append(eos_token())
    return sequence


def decode_sequence(tokens):
    tokens = [int(token) for token in tokens]
    if not tokens or tokens[0] != bos_token():
        actual = tokens[0] if tokens else None
        raise SequenceBundleError(f"sequence must start with BOS token {{bos_token()}}; got {{actual}}")
    if len(tokens) < 2:
        raise SequenceBundleError("sequence is missing schema token at position 1")

    schema = _schema_by_token(tokens[1])
    try:
        eos_index = next(
            index for index, token in enumerate(tokens[2:], start=2) if token == eos_token()
        )
    except StopIteration as error:
        raise SequenceBundleError(f"sequence for schema token {{tokens[1]}} is missing EOS") from error

    pad = sequence_pad_token()
    for offset, token in enumerate(tokens[eos_index + 1 :], start=eos_index + 1):
        if token != pad:
            raise SequenceBundleError(
                f"token {{token}} at sequence offset {{offset}} after EOS is not expected PAD {{pad}}"
            )

    local_payload = []
    global_payload = schema["global_payload"]
    raw_payload = schema["raw_payload"]
    global_start = int(global_payload["start"])
    global_end = int(global_payload["end_exclusive"])
    raw_start = int(raw_payload["start"])
    for offset, token in enumerate(tokens[2:eos_index]):
        if token < global_start or token >= global_end:
            raise SequenceBundleError(
                f"schema {{schema['schema_id']}} global payload token {{token}} at offset {{offset}} "
                "is outside selected schema range"
            )
        local_payload.append(raw_start + token - global_start)

    module = _module_for_schema_id(schema["schema_id"])
    decoded_json, consumed, total = module.decode(local_payload)
    if int(consumed) != int(total):
        raise SequenceBundleError(
            f"schema {{schema['schema_id']}} raw decode left {{int(total) - int(consumed)}} surplus tokens"
        )
    return schema["schema_id"], decoded_json, eos_index + 1, len(tokens)


def _schema_by_id(schema_id):
    try:
        return _SCHEMAS_BY_ID[schema_id]
    except KeyError as error:
        raise SequenceBundleError(f"unknown schema id {{schema_id}}") from error


def _schema_by_token(schema_token):
    try:
        return _SCHEMAS_BY_TOKEN[int(schema_token)]
    except KeyError as error:
        raise SequenceBundleError(f"unknown schema token {{schema_token}}") from error


def _module_for_schema_id(schema_id):
    _schema_by_id(schema_id)
    if schema_id not in _MODULE_CACHE:
        _MODULE_CACHE[schema_id] = importlib.import_module(_SCHEMA_MODULE_NAMES[schema_id])
    return _MODULE_CACHE[schema_id]
"#,
        manifest_literal = manifest_literal,
        module_names_literal = module_names_literal
    ))
}

#[derive(Debug)]
struct CompiledWheelCode {
    code: String,
    has_bpe: bool,
    has_streaming: bool,
    component_files: &'static [SchemaComponentFile],
}

/// Compile a tokenizer and retain its applicable vendored component payload.
fn compile_schema_to_code(config: &WheelConfig) -> Result<CompiledWheelCode, Box<dyn Error>> {
    // Read schema
    let schema_content = fs::read_to_string(&config.schema_path)?;
    let schema: Value = serde_json::from_str(&schema_content)?;
    let component_files = schema_component_files(&schema, schema_content.as_bytes());

    let mut compiler = TctCompiler::new(schema, &config.type_name)?
        .with_optimization_level(config.optimization_level);

    // Apply streaming encode if enabled
    if config.streaming_encode {
        compiler = compiler.with_streaming_encode();
    }

    // Apply decimal number encoding if enabled
    if config.decimal_number_encoding {
        compiler = compiler.with_decimal_number_encoding();
    }

    // Track whether BPE compression is enabled
    let has_bpe = config.bpe_merges_path.is_some();

    // Apply BPE if provided
    if let Some(bpe_path) = &config.bpe_merges_path {
        compiler = compiler.with_bpe_merges(bpe_path)?;
    }

    // Apply max_base_vocab if provided
    if let Some(max_base_vocab) = config.max_base_vocab {
        compiler = compiler.with_max_base_vocab(max_base_vocab)?;
    }
    if let Some(max_vocab) = config.max_vocab {
        compiler = compiler.with_max_vocabulary(max_vocab)?;
    }

    // Compile grammar and generate code
    let grammar = compiler.compile()?;
    let code = grammar.generate_code()?;

    Ok(CompiledWheelCode {
        code,
        has_bpe,
        has_streaming: config.streaming_encode,
        component_files,
    })
}

/// Create package directory structure
fn create_package_structure(package_dir: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(package_dir)?;
    fs::create_dir_all(package_dir.join("src"))?;
    fs::write(package_dir.join("LICENSE"), include_bytes!("../LICENSE"))?;
    fs::write(package_dir.join("NOTICE"), include_bytes!("../NOTICE"))?;
    Ok(())
}

type SchemaComponentFile = (&'static str, &'static [u8]);

const ESLINT_SCHEMA_URI: &str = "https://json.schemastore.org/partial-eslint-plugins.json";

// Retain exact source and notices in the compiler binary, so generated packages
// do not depend on the source checkout or mutable upstream downloads.
const ESLINT_SCHEMA_COMPONENT_FILES: &[SchemaComponentFile] = &[
    (
        "schemas/external/partial-eslint-plugins.json",
        include_bytes!("../schemas/external/partial-eslint-plugins.json"),
    ),
    (
        "third_party/clearance/schemas/ESLINT_SOURCE_NOTICE.md",
        include_bytes!("../third_party/clearance/schemas/ESLINT_SOURCE_NOTICE.md"),
    ),
    (
        "third_party/clearance/schemas/embedded-schemas.json",
        include_bytes!("../third_party/clearance/schemas/embedded-schemas.json"),
    ),
    (
        "third_party/clearance/schemas/eslint-plugin-inputs.json",
        include_bytes!("../third_party/clearance/schemas/eslint-plugin-inputs.json"),
    ),
    (
        "third_party/clearance/schemas/eslint-schema-changes.json",
        include_bytes!("../third_party/clearance/schemas/eslint-schema-changes.json"),
    ),
    (
        "third_party/licenses/schemastore/LICENSE",
        include_bytes!("../third_party/licenses/schemastore/LICENSE"),
    ),
    (
        "third_party/licenses/schemastore/NOTICE",
        include_bytes!("../third_party/licenses/schemastore/NOTICE"),
    ),
    (
        "third_party/licenses/jsonschema-extractor/LICENSE",
        include_bytes!("../third_party/licenses/jsonschema-extractor/LICENSE"),
    ),
    (
        "third_party/licenses/schema-eslint-inputs/angular-eslint-eslint-plugin-18.0.1/LICENSE",
        include_bytes!(
            "../third_party/licenses/schema-eslint-inputs/angular-eslint-eslint-plugin-18.0.1/LICENSE"
        ),
    ),
    (
        "third_party/licenses/schema-eslint-inputs/typescript-eslint-eslint-plugin-7.13.1/LICENSE",
        include_bytes!(
            "../third_party/licenses/schema-eslint-inputs/typescript-eslint-eslint-plugin-7.13.1/LICENSE"
        ),
    ),
    (
        "third_party/licenses/schema-eslint-inputs/eslint-plugin-import-2.29.1/LICENSE",
        include_bytes!(
            "../third_party/licenses/schema-eslint-inputs/eslint-plugin-import-2.29.1/LICENSE"
        ),
    ),
    (
        "third_party/licenses/schema-eslint-inputs/eslint-plugin-import-2.29.1/memo-parser-LICENSE",
        include_bytes!(
            "../third_party/licenses/schema-eslint-inputs/eslint-plugin-import-2.29.1/memo-parser-LICENSE"
        ),
    ),
    (
        "third_party/licenses/schema-eslint-inputs/eslint-plugin-unicorn-54.0.0/license",
        include_bytes!(
            "../third_party/licenses/schema-eslint-inputs/eslint-plugin-unicorn-54.0.0/license"
        ),
    ),
    (
        "third_party/licenses/schema-eslint-inputs/eslint-plugin-vue-9.26.0/LICENSE",
        include_bytes!(
            "../third_party/licenses/schema-eslint-inputs/eslint-plugin-vue-9.26.0/LICENSE"
        ),
    ),
    (
        "third_party/licenses/schema-eslint-inputs/eslint-8.57.0/LICENSE",
        include_bytes!("../third_party/licenses/schema-eslint-inputs/eslint-8.57.0/LICENSE"),
    ),
];

// Declared vendored references are bundled conservatively, including unused
// definitions; this does not trace which resources survive compilation. A root
// input is recognized as the pinned component only when its original bytes match.
fn schema_component_files(schema: &Value, schema_source: &[u8]) -> &'static [SchemaComponentFile] {
    let needs_component_payload = (schema.get("$id").and_then(Value::as_str)
        == Some(ESLINT_SCHEMA_URI)
        && schema_source == include_bytes!("../schemas/external/partial-eslint-plugins.json"))
        || vendored_external_schema_resources(schema)
            .iter()
            .any(|resource| resource.uri == ESLINT_SCHEMA_URI);
    if needs_component_payload {
        ESLINT_SCHEMA_COMPONENT_FILES
    } else {
        &[]
    }
}

fn write_schema_component_files(
    package_dir: &Path,
    files: &[SchemaComponentFile],
) -> std::io::Result<()> {
    for (relative_path, contents) in files {
        let destination = package_dir.join(relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(destination, contents)?;
    }
    Ok(())
}

fn create_bundle_package_structure(
    package_dir: &Path,
    module_name: &str,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(package_dir)?;
    fs::create_dir_all(package_dir.join(module_name))?;
    fs::write(package_dir.join("LICENSE"), include_bytes!("../LICENSE"))?;
    fs::write(package_dir.join("NOTICE"), include_bytes!("../NOTICE"))?;
    Ok(())
}

fn infer_bundle_vocab_size(schemas: &[SequenceBundleSchema]) -> u32 {
    let reserved_floor = 3u32;
    schemas
        .iter()
        .flat_map(|schema| {
            [
                schema.schema_token.saturating_add(1),
                schema.global_payload.end_exclusive,
            ]
        })
        .max()
        .unwrap_or(reserved_floor)
        .max(reserved_floor)
}

fn python_module_name(package_name: &str) -> String {
    package_name.replace('-', "_")
}

/// Generate Cargo.toml for the package
fn generate_cargo_toml(package_name: &str, version: &str) -> Result<String, Box<dyn Error>> {
    let lib_name = package_name.replace('-', "_");
    Ok(GeneratedCargoToml::new(package_name)
        .version(version)
        .with_cdylib(lib_name)
        .with_pyo3_extension()
        .with_rustc_hash()
        .with_regex()
        .with_lazy_static()
        .with_jsonschema()
        .with_chrono()
        .with_base64()
        // Release wheels are measurement-facing: enable LTO so timed encode
        // paths match the release codegen profile used by benchmark helpers.
        .with_release_lto()
        .render())
}

/// Generate pyproject.toml for the package
fn generate_pyproject_toml(
    package_name: &str,
    version: &str,
    description: Option<&String>,
    component_files: &[SchemaComponentFile],
) -> Result<String, Box<dyn Error>> {
    let desc = description
        .map(|s| s.as_str())
        .unwrap_or("TCT schema-specific tokenizer");
    let included_files = ["LICENSE", "NOTICE"]
        .into_iter()
        .chain(component_files.iter().map(|(path, _)| *path))
        .map(|path| format!("{path:?}"))
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        r#"[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "{}"
version = "{}"
description = "{}"
requires-python = ">=3.8"
license = {{text = "Apache-2.0"}}
classifiers = [
    "Programming Language :: Rust",
    "Programming Language :: Python :: Implementation :: CPython",
    "Programming Language :: Python :: Implementation :: PyPy",
]

[project.urls]
Repository = "https://github.com/jonas-schmitt/tct"

[tool.maturin]
include = [{}]
"#,
        package_name, version, desc, included_files
    ))
}

fn generate_bundle_pyproject_toml(
    package_name: &str,
    version: &str,
    description: Option<&String>,
    module_name: &str,
) -> Result<String, Box<dyn Error>> {
    let desc = description
        .map(|s| s.as_str())
        .unwrap_or("TCT multi-schema tokenizer bundle wrapper");

    Ok(format!(
        r#"[build-system]
requires = ["setuptools>=61"]
build-backend = "setuptools.build_meta"

[project]
name = "{}"
version = "{}"
description = "{}"
requires-python = ">=3.8"
license = {{text = "Apache-2.0"}}
classifiers = [
    "Programming Language :: Python :: 3",
    "Topic :: Text Processing",
]

[project.urls]
Repository = "https://github.com/jonas-schmitt/tct"

[tool.setuptools]
packages = ["{}"]
license-files = ["LICENSE", "NOTICE"]
"#,
        package_name, version, desc, module_name
    ))
}

/// Build wheel using maturin
fn build_wheel_with_maturin(package_dir: &Path) -> Result<(), Box<dyn Error>> {
    ensure_maturin_available()?;

    let tct_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let status = configured_maturin_build_command(package_dir, &tct_path).status()?;

    if !status.success() {
        return Err("Maturin build failed".into());
    }

    Ok(())
}

fn configured_maturin_build_command(package_dir: &Path, tct_path: &Path) -> process::Command {
    let mut command = process::Command::new("maturin");
    configure_generated_cargo_command(&mut command, tct_path);
    command
        .args(["build", "--release", "--out"])
        .arg(absolute_wheel_output_dir(package_dir, tct_path))
        .current_dir(package_dir);
    // Pin the target interpreter; without -i maturin picks whatever python it
    // finds, which can produce a differently-tagged wheel than the caller
    // expects (and step 7 would then report a stale file).
    if let Some(python) = std::env::var_os("TCT_WHEEL_PYTHON") {
        command.arg("-i").arg(python);
    }
    command
}

/// Where maturin is told to drop the wheel, and where step 7 goes looking.
///
/// Absolute by construction: maturin is spawned with `current_dir(package_dir)`,
/// so a path relative to the tct root would be resolved a second time inside the
/// package and the wheel would land at `<pkg>/<pkg>/target/wheels`.
///
/// Relative `package_dir` values are resolved against the process cwd (the TCT
/// repo root in production CLI use). Prefer [`absolute_wheel_output_dir`] when a
/// distinct `tct_path` is already known (tests, nested callers).
fn wheel_output_dir(package_dir: &Path) -> PathBuf {
    let tct_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    absolute_wheel_output_dir(package_dir, &tct_path)
}

/// Absolute maturin `--out` directory under `package_dir`, resolving a relative
/// package path against `tct_path` rather than against maturin’s package cwd.
fn absolute_wheel_output_dir(package_dir: &Path, tct_path: &Path) -> PathBuf {
    if package_dir.is_absolute() {
        return package_dir.join("target").join("wheels");
    }
    // `tct_path` may be "." in tests; force an absolute base so maturin cannot
    // re-resolve a still-relative --out against current_dir(package_dir).
    let base = if tct_path.is_absolute() {
        tct_path.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(tct_path),
            Err(_) => tct_path.to_path_buf(),
        }
    };
    base.join(package_dir).join("target").join("wheels")
}

fn ensure_maturin_available() -> Result<(), Box<dyn Error>> {
    ensure_maturin_available_in_path(std::env::var_os("PATH").as_deref())
}

fn ensure_maturin_available_in_path(path_env: Option<&OsStr>) -> Result<(), Box<dyn Error>> {
    if find_executable_on_path("maturin", path_env).is_some() {
        return Ok(());
    }

    Err(Box::new(crate::TctError::Build {
        message: "Required Python wheel build tool `maturin` was not found on PATH".to_string(),
        file_path: "maturin".to_string(),
        suggestion: "Install maturin in the active Python environment, for example `python -m pip install maturin`, then rerun the wheel build.".to_string(),
        source: None,
    }))
}

fn find_executable_on_path(program: &str, path_env: Option<&OsStr>) -> Option<PathBuf> {
    let paths = path_env?;

    for dir in std::env::split_paths(paths) {
        let candidate = dir.join(program);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }

        #[cfg(windows)]
        {
            let candidate = dir.join(format!("{program}.exe"));
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

/// Find the generated wheel file
fn find_wheel_file(wheel_dir: &Path) -> Result<Option<PathBuf>, Box<dyn Error>> {
    if !wheel_dir.exists() {
        return Ok(None);
    }

    // Newest wheel wins: the directory can hold wheels from earlier builds
    // (other versions/tags), and reporting one of those defeats the
    // staleness check above.
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(wheel_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("whl") {
            continue;
        }
        let mtime = entry.metadata()?.modified()?;
        if newest.as_ref().is_none_or(|(t, _)| mtime > *t) {
            newest = Some((mtime, path));
        }
    }

    Ok(newest.map(|(_, path)| path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated_crate::generated_cargo_target_dir;
    use quote::ToTokens;
    use tempfile::tempdir;

    fn parse_generated_rust(code: &str) -> syn::File {
        syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
    }

    fn generated_rust_has_function(code: &str, name: &str) -> bool {
        let file = parse_generated_rust(code);
        file.items
            .iter()
            .any(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == name))
    }

    fn generated_function_tokens_contain(code: &str, name: &str, expected: &str) -> bool {
        let file = parse_generated_rust(code);
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == name => Some(function),
                _ => None,
            })
            .unwrap_or_else(|| panic!("generated function `{name}` should exist"))
            .to_token_stream()
            .to_string()
            .contains(expected)
    }

    #[test]
    fn test_wheel_config_defaults_and_builders() {
        let config = WheelConfig::new("schema.json", "RootType", "tct-root")
            .with_version("1.2.3")
            .with_description("Schema tokenizer")
            .with_output_dir("dist")
            .with_bpe_merges("merges.json")
            .with_optimization_level(OptimizationLevel::Os)
            .with_max_base_vocab(512)
            .with_max_vocabulary(257)
            .with_quiet(true)
            .with_streaming_encode(true);

        assert_eq!(config.schema_path, "schema.json");
        assert_eq!(config.type_name, "RootType");
        assert_eq!(config.package_name, "tct-root");
        assert_eq!(config.version, "1.2.3");
        assert_eq!(config.description.as_deref(), Some("Schema tokenizer"));
        assert_eq!(config.output_dir, "dist");
        assert_eq!(config.bpe_merges_path.as_deref(), Some("merges.json"));
        assert_eq!(config.optimization_level, OptimizationLevel::Os);
        assert_eq!(config.max_base_vocab, Some(512));
        assert_eq!(config.max_vocab, Some(257));
        assert!(config.quiet);
        assert!(config.streaming_encode);
    }

    #[test]
    fn test_bundle_wheel_config_builds_valid_manifest() {
        let config = BundleWheelConfig::new(
            "tct-bundle-demo",
            vec![
                BundleWheelSchemaConfig::new("kubernetes", "tct_kubernetes", 3, 18, 17, 100, true)
                    .with_schema_sentinel("<|schema:kubernetes|>")
                    .with_generated_artifact("wheels/tct_kubernetes.whl"),
                BundleWheelSchemaConfig::new("tsconfig", "tct_tsconfig", 4, 12, 11, 100, false)
                    .with_schema_sentinel("<|schema:tsconfig|>"),
            ],
        )
        .with_version("2.0.0")
        .with_description("Bundle wrapper")
        .with_output_dir("dist")
        .with_quiet(true);

        let manifest = config.manifest();

        manifest.validate().unwrap();
        assert_eq!(config.package_name, "tct-bundle-demo");
        assert_eq!(config.version, "2.0.0");
        assert_eq!(config.description.as_deref(), Some("Bundle wrapper"));
        assert_eq!(config.output_dir, "dist");
        assert!(config.quiet);
        assert_eq!(manifest.vocab_size, 117);
        assert_eq!(manifest.schemas[0].schema_id, "kubernetes");
        assert_eq!(
            manifest.schemas[0].schema_sentinel.as_deref(),
            Some("<|schema:kubernetes|>")
        );
        assert_eq!(
            manifest.schemas[0].generated_artifact.as_deref(),
            Some("wheels/tct_kubernetes.whl")
        );
        assert_eq!(manifest.schemas[1].global_payload.start, 100);
    }

    #[test]
    fn test_create_package_structure_creates_src_dir() {
        let dir = tempdir().unwrap();
        let package_dir = dir.path().join("tct-package");

        create_package_structure(&package_dir).unwrap();

        assert!(package_dir.is_dir());
        assert!(package_dir.join("src").is_dir());
    }

    #[test]
    fn test_create_bundle_package_structure_creates_python_package_dir() {
        let dir = tempdir().unwrap();
        let package_dir = dir.path().join("tct-bundle-package");

        create_bundle_package_structure(&package_dir, "tct_bundle_package").unwrap();

        assert!(package_dir.is_dir());
        assert!(package_dir.join("tct_bundle_package").is_dir());
    }

    #[test]
    fn test_package_structures_include_repository_license() {
        let dir = tempdir().unwrap();
        let schema_package = dir.path().join("schema");
        let bundle_package = dir.path().join("bundle");
        create_package_structure(&schema_package).unwrap();
        create_bundle_package_structure(&bundle_package, "bundle").unwrap();

        for package in [schema_package, bundle_package] {
            assert_eq!(
                fs::read(package.join("LICENSE")).unwrap(),
                include_bytes!("../LICENSE")
            );
            assert_eq!(
                fs::read(package.join("NOTICE")).unwrap(),
                include_bytes!("../NOTICE")
            );
        }
    }

    #[test]
    fn test_schema_component_payload_preserves_direct_and_transitive_source() {
        let ledger: Value = serde_json::from_str(include_str!(
            "../third_party/clearance/schemas/embedded-schemas.json"
        ))
        .unwrap();
        let component = ledger["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["local_path"] == "schemas/external/partial-eslint-plugins.json")
            .unwrap();
        for schema_source in [
            r##"{"$ref": "https://json.schemastore.org/partial-eslint-plugins.json#/definitions/ruleString"}"##,
            r#"{"properties": {"eslint": {"$ref": "https://json.schemastore.org/eslintrc.json"}}}"#,
            include_str!("../schemas/external/partial-eslint-plugins.json"),
        ] {
            let schema: Value = serde_json::from_str(schema_source).unwrap();
            let component_files = schema_component_files(&schema, schema_source.as_bytes());
            let dir = tempdir().unwrap();
            create_package_structure(dir.path()).unwrap();
            write_schema_component_files(dir.path(), component_files).unwrap();
            let pyproject =
                generate_pyproject_toml("tct-component", "0.1.0", None, component_files).unwrap();
            for (path, _) in component_files {
                assert!(pyproject.contains(&format!("\"{path}\"")));
            }
            assert_eq!(
                fs::read(
                    dir.path()
                        .join("schemas/external/partial-eslint-plugins.json")
                )
                .unwrap(),
                include_bytes!("../schemas/external/partial-eslint-plugins.json")
            );
            assert_eq!(
                fs::read(
                    dir.path()
                        .join("third_party/clearance/schemas/ESLINT_SOURCE_NOTICE.md")
                )
                .unwrap(),
                include_bytes!("../third_party/clearance/schemas/ESLINT_SOURCE_NOTICE.md")
            );
            for path in component["retained_notices"].as_array().unwrap() {
                let path = path.as_str().unwrap();
                assert_eq!(
                    fs::read(dir.path().join(path)).unwrap(),
                    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap(),
                    "component notice bytes must match: {path}"
                );
            }
        }
    }

    #[test]
    fn test_schema_component_payload_rejects_unverified_root_id() {
        let schema = serde_json::json!({"$id": ESLINT_SCHEMA_URI, "type": "string"});
        let source = serde_json::to_vec(&schema).unwrap();
        assert!(schema_component_files(&schema, &source).is_empty());
    }

    #[test]
    fn test_schema_component_payload_rejects_reordered_root_source() {
        let original = include_bytes!("../schemas/external/partial-eslint-plugins.json");
        let schema: Value = serde_json::from_slice(original).unwrap();
        let fields = schema
            .as_object()
            .unwrap()
            .iter()
            .rev()
            .map(|(key, value)| {
                format!(
                    "{}: {}",
                    serde_json::to_string(key).unwrap(),
                    serde_json::to_string(value).unwrap()
                )
            })
            .collect::<Vec<_>>();
        let reordered_source = format!("{{{}}}\n", fields.join(",\n"));
        let reordered: Value = serde_json::from_str(&reordered_source).unwrap();
        assert_eq!(reordered, schema);
        assert_ne!(reordered_source.as_bytes(), original);
        assert!(schema_component_files(&reordered, reordered_source.as_bytes()).is_empty());
    }

    #[test]
    fn test_schema_component_payload_conservatively_includes_unused_declared_refs() {
        let schema = serde_json::json!({
            "type": "string",
            "$defs": {"unused": {"$ref": ESLINT_SCHEMA_URI}}
        });
        let source = serde_json::to_vec(&schema).unwrap();
        assert_eq!(
            schema_component_files(&schema, &source),
            ESLINT_SCHEMA_COMPONENT_FILES
        );
    }

    #[test]
    fn test_schema_component_payload_omits_unrelated_kubernetes_and_bundle_packages() {
        let kubernetes: Value =
            serde_json::from_str(include_str!("../schemas/popular/kubernetes.json")).unwrap();
        let dir = tempdir().unwrap();
        create_package_structure(dir.path()).unwrap();
        let files = schema_component_files(
            &kubernetes,
            include_bytes!("../schemas/popular/kubernetes.json"),
        );
        assert!(files.is_empty());
        write_schema_component_files(dir.path(), files).unwrap();
        assert!(!dir.path().join("schemas").exists());
        assert!(!dir.path().join("third_party").exists());

        let bundle = dir.path().join("bundle");
        create_bundle_package_structure(&bundle, "demo_bundle").unwrap();
        assert!(!bundle.join("schemas").exists());
        assert!(!bundle.join("third_party").exists());
    }

    #[test]
    fn test_generate_cargo_toml_pins_wheel_dependencies() {
        let cargo_toml = generate_cargo_toml("tct-demo-tokenizer", "2.0.0").unwrap();

        assert!(cargo_toml.contains("name = \"tct-demo-tokenizer\""));
        assert!(cargo_toml.contains("version = \"2.0.0\""));
        assert!(cargo_toml.contains("edition = \"2024\""));
        assert!(cargo_toml.contains("rust-version = \"1.95\""));
        assert!(cargo_toml.contains("serde = { version = \"=1.0.228\", features = [\"derive\"] }"));
        assert!(cargo_toml.contains("serde_json = \"=1.0.149\""));
        assert!(
            cargo_toml
                .contains("pyo3 = { version = \"=0.29.3\", features = [\"extension-module\"] }")
        );
        assert!(cargo_toml.contains("rustc-hash = \"=2.1.2\""));
        assert!(cargo_toml.contains("regex = \"=1.12.3\""));
        assert!(cargo_toml.contains("lazy_static = \"=1.5.0\""));
        assert!(
            cargo_toml.contains("jsonschema = { version = \"=0.46.2\", default-features = false }")
        );
        assert!(cargo_toml.contains("chrono = { version = \"=0.4.44\", features = [\"serde\"] }"));
        assert!(cargo_toml.contains("base64 = \"=0.22.1\""));
        assert!(cargo_toml.contains("name = \"tct_demo_tokenizer\""));
        assert!(cargo_toml.contains("crate-type = [\"cdylib\"]"));
        assert!(cargo_toml.contains("[profile.release]\nlto = true\ncodegen-units = 1"));
    }

    /// Regression: `--out` is derived from `package_dir`, and the caller passes
    /// a path relative to the tct root (`wheels/<pkg>`). maturin runs with
    /// `current_dir(package_dir)`, so a relative `--out` resolves a second time
    /// inside the package: the wheel landed at
    /// `wheels/<pkg>/wheels/<pkg>/target/wheels/` while step 7 looked in
    /// `wheels/<pkg>/target/wheels/` and reported "Failed to find generated
    /// wheel file" on an otherwise successful build.
    #[test]
    fn test_maturin_out_dir_is_absolute_for_a_relative_package_dir() {
        let command = configured_maturin_build_command(
            Path::new("wheels/tct-demo-tokenizer"),
            Path::new("."),
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let out_dir = Path::new(&args[3]);

        assert!(
            out_dir.is_absolute(),
            "--out must not be re-resolved against current_dir(package_dir): {args:?}"
        );
        assert!(
            out_dir.ends_with("wheels/tct-demo-tokenizer/target/wheels"),
            "--out must still point at the package's own target/wheels: {args:?}"
        );
    }

    #[test]
    fn test_maturin_build_command_uses_generated_toolchain_environment() {
        let package_dir = tempdir().unwrap();
        let tct_dir = tempdir().unwrap();
        fs::write(
            tct_dir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.95.0\"\n",
        )
        .unwrap();

        let command = configured_maturin_build_command(package_dir.path(), tct_dir.path());
        let envs = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();

        assert_eq!(command.get_program().to_string_lossy(), "maturin");
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(args.len(), 4);
        assert_eq!(args[0], "build");
        assert_eq!(args[1], "--release");
        assert_eq!(args[2], "--out");
        assert!(args[3].ends_with("target/wheels"), "{args:?}");
        assert_eq!(command.get_current_dir(), Some(package_dir.path()));
        assert_eq!(
            envs.get("RUSTFLAGS").and_then(|value| value.as_deref()),
            Some(crate::generated_crate::GENERATED_CARGO_RUSTFLAGS)
        );
        assert!(
            envs.get("RUSTUP_TOOLCHAIN")
                .and_then(|value| value.as_deref())
                .is_some_and(|value| !value.trim().is_empty())
        );
        // Same policy production uses (honours ambient TCT_GENERATED_CARGO_TARGET_DIR).
        let expected_target_dir = generated_cargo_target_dir(
            tct_dir.path(),
            std::env::var_os("TCT_GENERATED_CARGO_TARGET_DIR").as_deref(),
            std::env::var_os("CARGO_TARGET_DIR").as_deref(),
        );
        assert_eq!(
            envs.get("CARGO_TARGET_DIR")
                .and_then(|value| value.as_ref())
                .map(|value| std::ffi::OsString::from(value.as_str())),
            expected_target_dir
        );
    }

    #[test]
    fn test_maturin_build_command_absolutizes_relative_wheel_output() {
        let tct_dir = tempdir().unwrap();
        let package_dir = Path::new("wheels/tct-demo");

        let command = configured_maturin_build_command(package_dir, tct_dir.path());
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            PathBuf::from(&args[3]),
            tct_dir.path().join("wheels/tct-demo/target/wheels")
        );
    }

    #[test]
    fn test_maturin_preflight_reports_missing_tool() {
        let empty_path = tempdir().unwrap();

        let err = ensure_maturin_available_in_path(Some(empty_path.path().as_os_str()))
            .expect_err("missing maturin should fail before process spawn");
        let message = err.to_string();

        assert!(message.contains("maturin"));
        assert!(message.contains("not found on PATH"));
        assert!(message.contains("python -m pip install maturin"));
    }

    #[test]
    fn test_maturin_preflight_accepts_executable_on_path() {
        let dir = tempdir().unwrap();
        let maturin = dir.path().join("maturin");
        fs::write(&maturin, "").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&maturin).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&maturin, permissions).unwrap();
        }

        ensure_maturin_available_in_path(Some(dir.path().as_os_str())).unwrap();
    }

    #[test]
    fn test_generate_pyproject_toml_uses_default_and_custom_description() {
        let default_pyproject = generate_pyproject_toml("tct-demo", "0.1.0", None, &[]).unwrap();
        assert!(default_pyproject.contains("requires = [\"maturin>=1.0,<2.0\"]"));
        assert!(default_pyproject.contains("name = \"tct-demo\""));
        assert!(default_pyproject.contains("version = \"0.1.0\""));
        assert!(default_pyproject.contains("description = \"TCT schema-specific tokenizer\""));
        assert!(default_pyproject.contains("requires-python = \">=3.8\""));
        assert!(default_pyproject.contains("license = {text = \"Apache-2.0\"}"));
        assert!(
            default_pyproject.contains("Repository = \"https://github.com/jonas-schmitt/tct\"")
        );
        assert!(default_pyproject.contains("include = [\"LICENSE\", \"NOTICE\"]"));

        let description = "Custom tokenizer".to_string();
        let custom_pyproject =
            generate_pyproject_toml("tct-demo", "0.2.0", Some(&description), &[]).unwrap();
        assert!(custom_pyproject.contains("version = \"0.2.0\""));
        assert!(custom_pyproject.contains("description = \"Custom tokenizer\""));
    }

    #[test]
    fn test_generate_bundle_pyproject_toml_uses_setuptools_package() {
        let pyproject =
            generate_bundle_pyproject_toml("tct-bundle-demo", "0.3.0", None, "tct_bundle_demo")
                .unwrap();

        assert!(pyproject.contains("requires = [\"setuptools>=61\"]"));
        assert!(pyproject.contains("build-backend = \"setuptools.build_meta\""));
        assert!(pyproject.contains("name = \"tct-bundle-demo\""));
        assert!(pyproject.contains("version = \"0.3.0\""));
        assert!(pyproject.contains("packages = [\"tct_bundle_demo\"]"));
        assert!(pyproject.contains("license = {text = \"Apache-2.0\"}"));
        assert!(pyproject.contains("Repository = \"https://github.com/jonas-schmitt/tct\""));
        assert!(pyproject.contains("license-files = [\"LICENSE\", \"NOTICE\"]"));
    }

    #[test]
    fn test_find_wheel_file_handles_missing_empty_and_matching_dirs() {
        let dir = tempdir().unwrap();
        let missing_dir = dir.path().join("missing");
        assert!(find_wheel_file(&missing_dir).unwrap().is_none());

        let wheel_dir = dir.path().join("wheels");
        fs::create_dir_all(&wheel_dir).unwrap();
        fs::write(wheel_dir.join("notes.txt"), "not a wheel").unwrap();
        assert!(find_wheel_file(&wheel_dir).unwrap().is_none());

        let wheel_path = wheel_dir.join("demo-0.1.0-py3-none-any.whl");
        fs::write(&wheel_path, "wheel").unwrap();
        assert_eq!(find_wheel_file(&wheel_dir).unwrap(), Some(wheel_path));
    }

    #[test]
    fn test_generate_bundle_wrapper_module_exports_dispatch_api() {
        let config = BundleWheelConfig::new(
            "tct-bundle-demo",
            vec![
                BundleWheelSchemaConfig::new("kubernetes", "tct_kubernetes", 3, 18, 17, 100, true)
                    .with_schema_sentinel("<|schema:kubernetes|>"),
                BundleWheelSchemaConfig::new("tsconfig", "tct_tsconfig", 4, 12, 11, 100, false),
            ],
        )
        .with_quiet(true);

        let source = generate_bundle_wrapper_module(&config).unwrap();

        assert!(source.contains("import importlib"));
        assert!(source.contains("class SequenceBundleError(ValueError):"));
        assert!(source.contains("def sequence_bundle_metadata_json():"));
        assert!(source.contains("def sequence_vocab_size():"));
        assert!(source.contains("def generation_mask_for_schema_token(schema_token):"));
        assert!(source.contains("def encode_sequence(schema_id, json_str):"));
        assert!(source.contains("payload = module.encode(json_str)"));
        assert!(source.contains("sequence = [bos_token(), int(schema[\"schema_token\"])]"));
        assert!(source.contains("def decode_sequence(tokens):"));
        assert!(source.contains("schema = _schema_by_token(tokens[1])"));
        assert!(source.contains("decoded_json, consumed, total = module.decode(local_payload)"));
        assert!(
            source
                .contains("return schema[\"schema_id\"], decoded_json, eos_index + 1, len(tokens)")
        );
        assert!(source.contains("schema_sentinel"));
        assert!(source.contains("<|schema:kubernetes|>"));
        assert!(source.contains("tct_kubernetes"));
    }

    #[test]
    fn test_write_bundle_wrapper_package_writes_python_sources() {
        let dir = tempdir().unwrap();
        let config = BundleWheelConfig::new(
            "tct-bundle-demo",
            vec![BundleWheelSchemaConfig::new(
                "tsconfig",
                "tct_tsconfig",
                3,
                12,
                11,
                4,
                false,
            )],
        )
        .with_output_dir(dir.path().display().to_string())
        .with_quiet(true);

        let package_dir = write_bundle_wrapper_package(&config).unwrap();

        let init_path = package_dir.join("tct_bundle_demo").join("__init__.py");
        let pyproject_path = package_dir.join("pyproject.toml");
        assert!(init_path.is_file());
        assert!(pyproject_path.is_file());
        let init = fs::read_to_string(init_path).unwrap();
        let pyproject = fs::read_to_string(pyproject_path).unwrap();
        assert!(init.contains("def encode_sequence(schema_id, json_str):"));
        assert!(pyproject.contains("packages = [\"tct_bundle_demo\"]"));
    }

    #[test]
    fn test_generated_bundle_wrapper_runtime_with_fake_modules_when_python_available() {
        let python_available = std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_ok();
        if !python_available {
            return;
        }

        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("tct_alpha.py"),
            r#"
def encode(json_str):
    return [0, 1]

def decode(tokens):
    tokens = list(tokens)
    if tokens != [0, 1]:
        raise ValueError(f"bad alpha payload: {tokens}")
    return '{"schema":"alpha"}', len(tokens), len(tokens)
"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("tct_beta.py"),
            r#"
def encode(json_str):
    return [0, 2]

def decode(tokens):
    tokens = list(tokens)
    if tokens != [0, 2]:
        raise ValueError(f"bad beta payload: {tokens}")
    return '{"schema":"beta"}', len(tokens), len(tokens)
"#,
        )
        .unwrap();

        let config = BundleWheelConfig::new(
            "tct-bundle-demo",
            vec![
                BundleWheelSchemaConfig::new("alpha", "tct_alpha", 3, 3, 2, 4, false),
                BundleWheelSchemaConfig::new("beta", "tct_beta", 4, 4, 3, 10, false),
            ],
        )
        .with_output_dir(dir.path().display().to_string())
        .with_quiet(true);
        let package_dir = write_bundle_wrapper_package(&config).unwrap();

        let pythonpath = format!("{}:{}", package_dir.display(), dir.path().display());
        let script = r#"
import tct_bundle_demo

alpha = tct_bundle_demo.encode_sequence("alpha", "{}")
assert alpha == [1, 3, 4, 5, 2], alpha
schema_id, json_str, consumed, total = tct_bundle_demo.decode_sequence(alpha + [0, 0])
assert (schema_id, json_str, consumed, total) == ("alpha", '{"schema":"alpha"}', 5, 7)

beta = tct_bundle_demo.encode_sequence("beta", "{}")
assert beta == [1, 4, 10, 12, 2], beta
schema_id, json_str, consumed, total = tct_bundle_demo.decode_sequence(beta)
assert (schema_id, json_str, consumed, total) == ("beta", '{"schema":"beta"}', 5, 5)

try:
    tct_bundle_demo.decode_sequence([1, 3, 10, 2])
    raise AssertionError("wrong-schema payload should fail")
except tct_bundle_demo.SequenceBundleError:
    pass

try:
    tct_bundle_demo.decode_sequence([1, 3, 4, 2, 4])
    raise AssertionError("non-PAD after EOS should fail")
except tct_bundle_demo.SequenceBundleError:
    pass
"#;

        let output = std::process::Command::new("python3")
            .arg("-c")
            .arg(script)
            .env("PYTHONPATH", pythonpath)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "generated bundle wrapper failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_generate_combined_lib_base_mode_exports_core_api() {
        let combined = generate_combined_lib(
            "RootType",
            "tct-demo-tokenizer",
            "pub const VOCAB_SIZE: u32 = 8;\npub const PAD_TOKEN: u32 = 0;",
            false,
            false,
        )
        .unwrap();

        assert!(combined.contains("fn tct_demo_tokenizer(m: &Bound<'_, PyModule>)"));
        assert!(
            combined.contains("fn encode(py: Python<'_>, json_str: &str) -> PyResult<Vec<u32>>")
        );
        assert!(combined.contains("m.add_function(wrap_pyfunction!(encode, m)?)?;"));
        assert!(combined.matches("py.detach(move ||").count() >= 4);
        assert!(combined.contains("let mut tokens = Vec::new();"));
        assert!(combined.contains("serde_json::from_str::<RootType>(&json_owned)"));
        assert!(!combined.contains("serde_json::from_value::<RootType>"));
        assert!(!combined.contains("serde_json::to_value(&instance)"));
        assert!(combined.contains("serde_json::to_string(&instance)"));
        assert!(combined.contains("let instance = RootType::decode(&mut token_iter)"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(decode, m)?)?;"));
        assert!(combined.contains("fn vocab_size() -> u32"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(vocab_size, m)?)?;"));
        assert!(combined.contains("fn pad_token() -> u32"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(pad_token, m)?)?;"));
        assert!(combined.contains("fn bos_token() -> u32"));
        assert!(combined.contains("SEQUENCE_BOS_TOKEN"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(bos_token, m)?)?;"));
        assert!(combined.contains("fn eos_token() -> u32"));
        assert!(combined.contains("SEQUENCE_EOS_TOKEN"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(eos_token, m)?)?;"));
        assert!(combined.contains("fn sequence_pad_token() -> u32"));
        assert!(generated_function_tokens_contain(
            &combined,
            "sequence_pad_token",
            "SEQUENCE_PAD_TOKEN"
        ));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(sequence_pad_token, m)?)?;"));
        assert!(combined.contains("fn schema_token() -> u32"));
        assert!(combined.contains("SEQUENCE_SCHEMA_TOKEN"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(schema_token, m)?)?;"));
        assert!(combined.contains("fn sequence_payload_offset() -> u32"));
        assert!(combined.contains("SEQUENCE_PAYLOAD_OFFSET"));
        assert!(
            combined.contains("m.add_function(wrap_pyfunction!(sequence_payload_offset, m)?)?;")
        );
        assert!(combined.contains("fn sequence_vocab_size() -> u32"));
        assert!(generated_function_tokens_contain(
            &combined,
            "sequence_vocab_size",
            "SEQUENCE_VOCAB_SIZE"
        ));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(sequence_vocab_size, m)?)?;"));
        assert!(combined.contains("fn sequence_boundary_version() -> u32"));
        assert!(
            combined.contains("m.add_function(wrap_pyfunction!(sequence_boundary_version, m)?)?;")
        );
        assert!(combined.contains("fn sequence_metadata_json() -> PyResult<String>"));
        assert!(generated_function_tokens_contain(
            &combined,
            "sequence_metadata_json",
            "serde_json :: to_string (& sequence :: metadata ())"
        ));
        assert!(
            combined.contains("m.add_function(wrap_pyfunction!(sequence_metadata_json, m)?)?;")
        );
        assert!(
            combined.contains(
                "fn encode_sequence(py: Python<'_>, json_str: &str) -> PyResult<Vec<u32>>"
            )
        );
        assert!(combined.contains("sequence::encode(&instance)"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(encode_sequence, m)?)?;"));
        assert!(combined.contains("fn decode_sequence(py: Python<'_>, tokens: Vec<u32>)"));
        assert!(combined.contains("sequence::decode::<RootType>(&tokens)"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(decode_sequence, m)?)?;"));
        assert!(combined.contains("fn strict_prefix_roles(py: Python<'_>, tokens: Vec<u32>)"));
        assert!(combined.contains("role_export::strict_prefix_roles::<RootType>"));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(strict_prefix_roles, m)?)?;"));
        assert!(!generated_rust_has_function(&combined, "decode_prefix"));
        assert!(!combined.contains("codec::encode(&instance)"));
    }

    #[test]
    fn test_generate_combined_lib_bpe_streaming_exports_decode_prefix() {
        let combined = generate_combined_lib(
            "RootType",
            "tct-demo-tokenizer",
            "pub const VOCAB_SIZE: u32 = 8;\npub const PAD_TOKEN: u32 = 0;",
            true,
            true,
        )
        .unwrap();

        assert!(combined.contains("codec::encode(&instance)"));
        assert!(combined.contains("codec::decode::<RootType>(tokens)"));
        assert!(combined.contains("serde_json::from_str::<RootType>(&json_owned)"));
        assert!(!combined.contains("serde_json::from_value::<RootType>"));
        assert!(!combined.contains("serde_json::to_value(&instance)"));
        assert!(combined.contains("serde_json::to_string(&instance)"));
        assert!(generated_rust_has_function(&combined, "decode_prefix"));
        assert!(combined.contains("py.detach(move ||"));
        assert!(generated_function_tokens_contain(
            &combined,
            "decode_prefix",
            "let base_tokens = bpe :: decode (& tokens , VOCAB_SIZE)"
        ));
        assert!(generated_function_tokens_contain(
            &combined,
            "decode_prefix",
            "RootType :: decode_prefix (& mut token_iter)"
        ));
        assert!(generated_function_tokens_contain(
            &combined,
            "decode_prefix",
            "serde_json :: to_string (& result . instance)"
        ));
        assert!(generated_function_tokens_contain(
            &combined,
            "tct_demo_tokenizer",
            "wrap_pyfunction ! (decode_prefix , m) ?"
        ));
        assert!(combined.contains("struct DecodeState"));
        assert!(generated_rust_has_function(
            &combined,
            "classify_decode_tokens"
        ));
        assert!(generated_rust_has_function(&combined, "root_token_allowed"));
        assert!(generated_rust_has_function(&combined, "begin_decode_state"));
        assert!(generated_rust_has_function(
            &combined,
            "begin_decode_state_restricted"
        ));
        assert!(combined.contains("m.add_function(wrap_pyfunction!(begin_decode_state, m)?)?;"));
        assert!(
            combined
                .contains("m.add_function(wrap_pyfunction!(begin_decode_state_restricted, m)?)?;")
        );
        assert!(combined.contains("m.add_class::<DecodeState>()?;"));
        assert!(combined.contains("fn clone_state(&self) -> Self"));
        assert!(combined.contains("fn try_append(&mut self, token: u32) -> String"));
        assert!(combined.contains("allowed_root_tokens: Option<Vec<u32>>"));
        assert!(combined.contains("fn allowed_root_tokens(&self) -> Option<Vec<u32>>"));
        assert!(combined.contains("fn current_terminal_key(&self) -> Option<String>"));
        assert!(combined.contains("fn current_terminal_path(&self) -> Option<String>"));
        assert!(combined.contains("terminal_key_export::current_terminal_key_path"));
    }

    #[test]
    fn test_generate_combined_lib_non_streaming_omits_decode_state() {
        let combined = generate_combined_lib(
            "RootType",
            "tct-demo-tokenizer",
            "pub const VOCAB_SIZE: u32 = 8;\npub const PAD_TOKEN: u32 = 0;",
            false,
            false,
        )
        .unwrap();

        assert!(!combined.contains("struct DecodeState"));
        assert!(!generated_rust_has_function(
            &combined,
            "begin_decode_state"
        ));
        assert!(!generated_rust_has_function(
            &combined,
            "begin_decode_state_restricted"
        ));
        assert!(!generated_rust_has_function(
            &combined,
            "classify_decode_tokens"
        ));
        assert!(!generated_rust_has_function(&combined, "decode_prefix"));
    }

    #[test]
    fn test_generate_combined_lib_streaming_no_bpe_exports_decode_state_helpers() {
        let combined = generate_combined_lib(
            "RootType",
            "tct-demo-tokenizer",
            "pub const VOCAB_SIZE: u32 = 8;\npub const PAD_TOKEN: u32 = 0;",
            false,
            true,
        )
        .unwrap();

        assert!(generated_rust_has_function(
            &combined,
            "classify_decode_tokens"
        ));
        assert!(generated_rust_has_function(&combined, "root_token_allowed"));
        assert!(generated_rust_has_function(&combined, "begin_decode_state"));
        assert!(generated_rust_has_function(
            &combined,
            "begin_decode_state_restricted"
        ));
        assert!(combined.contains("fn current_terminal_key(&self) -> Option<String>"));
        assert!(combined.contains("fn current_terminal_path(&self) -> Option<String>"));
        assert!(combined.contains("terminal_key_export::current_terminal_key_path"));
        // Non-BPE path feeds the stored token slice directly (no BPE expand).
        assert!(combined.contains(
            "tokenizer::terminal_key_export::current_terminal_key_path::<RootType>(&self.tokens)"
        ));
        assert!(combined.contains("RootType::decode(&mut iter)"));
        // Non-BPE classify uses the stored token slice directly (no BPE expand).
        assert!(combined.contains("let mut iter = tokens.iter().copied().peekable();"));
        assert!(!combined.contains("bpe::decode(tokens, VOCAB_SIZE)"));
    }

    #[test]
    fn test_compile_schema_to_code_reports_missing_schema_path() {
        let config =
            WheelConfig::new("does-not-exist.json", "RootType", "tct-demo").with_quiet(true);

        let error = compile_schema_to_code(&config).unwrap_err();

        assert!(
            error.to_string().contains("No such file") || error.to_string().contains("not found")
        );
    }

    #[test]
    fn test_compile_schema_to_code_retains_component_payload() {
        let dir = tempdir().unwrap();
        let schema_path = dir.path().join("schema.json");
        fs::write(
            &schema_path,
            serde_json::to_vec(&serde_json::json!({
                "type": "object",
                "properties": {"rule": {"type": "string"}},
                "allOf": [{
                    "$ref": "https://json.schemastore.org/partial-eslint-plugins.json"
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        let config = WheelConfig::new(
            schema_path.display().to_string(),
            "ComponentCarrier",
            "tct-component-carrier",
        )
        .with_quiet(true);
        let compiled = compile_schema_to_code(&config).unwrap();
        assert!(!compiled.code.is_empty());
        assert_eq!(compiled.component_files, ESLINT_SCHEMA_COMPONENT_FILES);
    }

    #[test]
    fn test_compile_schema_to_code_honors_optimization_profile() {
        let dir = tempdir().unwrap();
        let schema_path = dir.path().join("schema.json");
        fs::write(
            &schema_path,
            serde_json::to_string(&serde_json::json!({
                "type": "object",
                "properties": {
                    "timestamp": {"type": "string", "format": "date-time"}
                },
                "required": ["timestamp"]
            }))
            .unwrap(),
        )
        .unwrap();

        let os_config = WheelConfig::new(
            schema_path.display().to_string(),
            "ProfileWheel",
            "tct-profile-wheel",
        )
        .with_optimization_level(OptimizationLevel::Os)
        .with_quiet(true);
        let os_code = compile_schema_to_code(&os_config).unwrap().code;
        assert!(
            !os_code.contains("timestamp_opt"),
            "Os wheel generation must not use O3-only format-specialized date-time encoding"
        );

        let o3_config = WheelConfig::new(
            schema_path.display().to_string(),
            "ProfileWheel",
            "tct-profile-wheel",
        )
        .with_optimization_level(OptimizationLevel::O3)
        .with_quiet(true);
        let o3_code = compile_schema_to_code(&o3_config).unwrap().code;
        assert!(
            o3_code.contains("timestamp_opt"),
            "O3 wheel generation should keep format-specialized date-time encoding"
        );
    }
}
