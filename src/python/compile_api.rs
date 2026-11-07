//! Compiler bindings of the root `tct` Python module.
//!
//! `tct.compile` mirrors the Rust-output mode of the `tct` command line tool and
//! returns the generated Rust source as a string; `tct.analyze` returns the
//! vocabulary analysis of `tct --analyze --format json` as a dictionary. Building
//! a Python wheel for a schema is still done with `tct --target python`.
//!
//! The compilation itself lives in plain Rust functions ([`compile_schema`],
//! [`root_type_name`]) so it can be tested without an interpreter; the
//! `#[pyfunction]` wrappers only convert arguments, release the GIL while the
//! compiler runs, and translate errors and warnings.

use std::ffi::CString;
use std::path::PathBuf;

use pyo3::exceptions::{PyUserWarning, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};
use serde_json::Value;

use crate::vocabulary_analyzer::{DEFAULT_COMPARISON_SIZES, analyze_to_report};
use crate::{OptimizationLevel, TctCompiler};

pyo3::create_exception!(
    tct,
    CompileError,
    PyValueError,
    "The schema or the compile options were rejected by the compiler."
);

/// Root type name used when neither a name nor a usable schema title is given.
const DEFAULT_ROOT_NAME: &str = "SchemaRoot";

/// Label echoed as `schema_file` in analysis reports of in-memory schemas.
const ANALYSIS_SCHEMA_LABEL: &str = "<schema>";

/// Options of one compilation; they mirror the command line flags that apply to
/// Rust output.
#[derive(Debug, Clone)]
pub(crate) struct CompileOptions {
    /// Optimization level spelling accepted by `tct -O` (`O0`, `2`, `os`, ...).
    pub optimization: String,
    /// Also generate `decode_prefix` (`--streaming`).
    pub streaming: bool,
    /// Force bit-packed optional-field presence (`--bit-packing`).
    pub bit_packing: bool,
    /// Encode unbounded numbers as decimal text (`--decimal-numbers`).
    pub decimal_numbers: bool,
    /// Trained BPE merge table to embed (`--bpe-merges`).
    pub bpe_merges: Option<PathBuf>,
    /// Raw vocabulary cap (`--max-vocab`).
    pub max_vocabulary: Option<usize>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            optimization: OptimizationLevel::O3.to_string(),
            streaming: false,
            bit_packing: false,
            decimal_numbers: false,
            bpe_merges: None,
            max_vocabulary: None,
        }
    }
}

/// Result of a successful compilation.
#[derive(Debug, Clone)]
pub(crate) struct Compiled {
    /// Generated Rust source.
    pub code: String,
    /// Schema quality findings, one line each.
    pub warnings: Vec<String>,
}

/// Root type name for a schema: the explicit name, else the schema `title`,
/// else [`DEFAULT_ROOT_NAME`], converted to UpperCamelCase like the CLI does.
pub(crate) fn root_type_name(schema: &Value, name: Option<&str>) -> String {
    let source = name.or_else(|| schema.get("title").and_then(Value::as_str));
    match source {
        Some(text) if !text.trim().is_empty() => crate::utils::to_upper_camel_case(text),
        _ => DEFAULT_ROOT_NAME.to_string(),
    }
}

/// Compile `schema` to Rust source. Errors are rendered to their message.
pub(crate) fn compile_schema(
    schema: Value,
    name: Option<&str>,
    options: &CompileOptions,
) -> Result<Compiled, String> {
    let level = OptimizationLevel::parse_cli(&options.optimization)?;
    let type_name = root_type_name(&schema, name);

    let mut compiler = TctCompiler::new(schema, type_name)
        .map_err(|error| error.to_string())?
        .with_optimization_level(level);
    if let Some(path) = &options.bpe_merges {
        compiler = compiler
            .with_bpe_merges(path)
            .map_err(|error| format!("invalid BPE merge file: {error}"))?;
    }
    if options.bit_packing {
        compiler = compiler.with_bit_packing_optimization();
    }
    if options.streaming {
        compiler = compiler.with_streaming_encode();
    }
    if options.decimal_numbers {
        compiler = compiler.with_decimal_number_encoding();
    }
    if let Some(max_vocabulary) = options.max_vocabulary {
        compiler = compiler
            .with_max_vocabulary(max_vocabulary)
            .map_err(|error| error.to_string())?;
    }

    let grammar = compiler.compile().map_err(|error| error.to_string())?;
    let code = grammar.generate_code().map_err(|error| error.to_string())?;
    let warnings = grammar
        .warnings
        .as_ref()
        .map(|warnings| {
            warnings
                .warnings
                .iter()
                .map(|warning| {
                    format!(
                        "{}: {} {}",
                        warning.schema_path, warning.message, warning.suggestion
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Compiled { code, warnings })
}

/// Read a schema given as JSON text (`str`), UTF-8 `bytes`, or any object that
/// `json.dumps` accepts.
fn schema_from_python(py: Python<'_>, schema: &Bound<'_, PyAny>) -> PyResult<Value> {
    let text: String = if let Ok(text) = schema.cast::<PyString>() {
        text.extract()?
    } else if let Ok(bytes) = schema.cast::<PyBytes>() {
        String::from_utf8(bytes.as_bytes().to_vec()).map_err(|error| {
            PyValueError::new_err(format!("schema bytes are not valid UTF-8: {error}"))
        })?
    } else {
        py.import("json")?
            .call_method1("dumps", (schema,))?
            .extract()?
    };
    serde_json::from_str(&text)
        .map_err(|error| PyValueError::new_err(format!("schema is not valid JSON: {error}")))
}

/// Raise each schema quality finding as a Python `UserWarning`.
fn emit_warnings(py: Python<'_>, warnings: &[String]) -> PyResult<()> {
    let category = py.get_type::<PyUserWarning>();
    for warning in warnings {
        // Python C strings cannot hold NUL; schema text may.
        let message = CString::new(warning.replace('\0', "")).unwrap_or_default();
        PyErr::warn(py, category.as_any(), &message, 1)?;
    }
    Ok(())
}

/// Compile a JSON Schema to generated Rust source.
///
/// `schema` is JSON text, UTF-8 bytes, or any JSON-serializable object. `name`
/// is the root type name (default: the schema `title`, else `SchemaRoot`). The
/// keyword options mirror the `tct` command line: `optimization` is one of
/// `O0`, `O2`, `O3` (default), `Os`, or `OVocab`; `streaming` adds
/// `decode_prefix`; `bit_packing` forces bit-packed optional-field presence;
/// `decimal_numbers` encodes unbounded numbers as decimal text; `bpe_merges` is
/// the path of a trained merge table; `max_vocabulary` caps the raw vocabulary.
///
/// Raises `tct.CompileError` (a `ValueError`) when the schema or an option is
/// rejected. Schema quality findings are reported as `UserWarning`s.
#[pyfunction]
#[pyo3(signature = (
    schema,
    name = None,
    *,
    optimization = "O3",
    streaming = false,
    bit_packing = false,
    decimal_numbers = false,
    bpe_merges = None,
    max_vocabulary = None
))]
#[allow(clippy::too_many_arguments)]
fn compile(
    py: Python<'_>,
    schema: &Bound<'_, PyAny>,
    name: Option<String>,
    optimization: &str,
    streaming: bool,
    bit_packing: bool,
    decimal_numbers: bool,
    bpe_merges: Option<PathBuf>,
    max_vocabulary: Option<usize>,
) -> PyResult<String> {
    let schema = schema_from_python(py, schema)?;
    let options = CompileOptions {
        optimization: optimization.to_string(),
        streaming,
        bit_packing,
        decimal_numbers,
        bpe_merges,
        max_vocabulary,
    };
    let compiled = py
        .detach(move || compile_schema(schema, name.as_deref(), &options))
        .map_err(CompileError::new_err)?;
    emit_warnings(py, &compiled.warnings)?;
    Ok(compiled.code)
}

/// Analyze the vocabulary requirements of a JSON Schema.
///
/// Returns the report of `tct --analyze --format json` as a dictionary.
/// `compare_sizes` lists the vocabulary budgets to compare (default
/// `[64, 128, 256, 512]`). Raises `tct.CompileError` for schemas the analysis
/// rejects.
#[pyfunction]
#[pyo3(signature = (schema, *, compare_sizes = None))]
fn analyze<'py>(
    py: Python<'py>,
    schema: &Bound<'py, PyAny>,
    compare_sizes: Option<Vec<usize>>,
) -> PyResult<Bound<'py, PyAny>> {
    let schema = schema_from_python(py, schema)?;
    let sizes = compare_sizes.unwrap_or_else(|| DEFAULT_COMPARISON_SIZES.to_vec());
    let report = py
        .detach(move || analyze_to_report(&schema, &sizes, ANALYSIS_SCHEMA_LABEL))
        .map_err(|error| CompileError::new_err(format!("vocabulary analysis failed: {error}")))?;
    let text = serde_json::to_string(&report)
        .map_err(|error| PyValueError::new_err(format!("failed to serialize report: {error}")))?;
    py.import("json")?.call_method1("loads", (text,))
}

/// Register the compiler bindings on the `tct` module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(compile, module)?)?;
    module.add_function(wrap_pyfunction!(analyze, module)?)?;
    module.add("CompileError", module.py().get_type::<CompileError>())?;
    module.add("OPTIMIZATION_LEVELS", ("O0", "O2", "O3", "Os", "OVocab"))?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::tests::with_python;
    use super::*;
    use pyo3::types::PyDict;
    use serde_json::json;

    fn task_schema() -> Value {
        json!({
            "title": "task item",
            "type": "object",
            "properties": {
                "status": {"enum": ["active", "pending", "completed"]},
                "priority": {"type": "integer", "minimum": 1, "maximum": 5}
            },
            "required": ["status"],
            "additionalProperties": false
        })
    }

    #[test]
    fn root_type_name_prefers_name_then_title_then_default() {
        let schema = task_schema();
        assert_eq!(root_type_name(&schema, Some("my_task")), "MyTask");
        assert_eq!(root_type_name(&schema, None), "TaskItem");
        assert_eq!(
            root_type_name(&json!({"type": "object"}), None),
            "SchemaRoot"
        );
        assert_eq!(
            root_type_name(&json!({"title": "   "}), None),
            "SchemaRoot",
            "a blank title must not become the root name"
        );
    }

    #[test]
    fn compile_schema_generates_rust_for_the_named_root() {
        let compiled = compile_schema(task_schema(), Some("Task"), &CompileOptions::default())
            .expect("task schema should compile");
        assert!(compiled.code.contains("pub struct Task"));
        assert!(compiled.code.contains("pub mod tokenizer"));
        assert!(!compiled.code.contains("fn decode_prefix"));
    }

    #[test]
    fn compile_schema_streaming_adds_prefix_decoding() {
        let options = CompileOptions {
            streaming: true,
            ..CompileOptions::default()
        };
        let compiled = compile_schema(task_schema(), Some("Task"), &options).unwrap();
        assert!(compiled.code.contains("fn decode_prefix"));
    }

    #[test]
    fn compile_schema_accepts_every_optimization_spelling_of_the_cli() {
        let reference = |spelling: &str| {
            let options = CompileOptions {
                optimization: spelling.to_string(),
                ..CompileOptions::default()
            };
            compile_schema(task_schema(), Some("Task"), &options)
                .unwrap_or_else(|error| panic!("{spelling}: {error}"))
                .code
        };
        assert_eq!(reference("2"), reference("O2"));
        assert_eq!(reference("o2"), reference("-O2"));
        assert_eq!(reference("os"), reference("size"));
        assert_eq!(reference("vocab"), reference("OVocab"));
        assert_ne!(reference("O0"), reference("O2"));
    }

    #[test]
    fn compile_schema_reports_invalid_options_and_schemas() {
        let bad_level = CompileOptions {
            optimization: "O9".to_string(),
            ..CompileOptions::default()
        };
        let error = compile_schema(task_schema(), Some("Task"), &bad_level).unwrap_err();
        assert!(error.contains("invalid optimization level"), "{error}");

        let missing_merges = CompileOptions {
            bpe_merges: Some(PathBuf::from("/nonexistent/merges.json")),
            ..CompileOptions::default()
        };
        let error = compile_schema(task_schema(), Some("Task"), &missing_merges).unwrap_err();
        assert!(error.contains("invalid BPE merge file"), "{error}");

        let too_small = CompileOptions {
            max_vocabulary: Some(100),
            ..CompileOptions::default()
        };
        assert!(compile_schema(task_schema(), Some("Task"), &too_small).is_err());

        let dangling_ref = json!({
            "type": "object",
            "properties": {"x": {"$ref": "#/definitions/missing"}}
        });
        let error =
            compile_schema(dangling_ref, Some("Broken"), &CompileOptions::default()).unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn schema_from_python_accepts_text_bytes_and_objects() {
        let expected = json!({"type": "object", "properties": {"a": {"type": "string"}}});
        with_python(|py| {
            let text = r#"{"type": "object", "properties": {"a": {"type": "string"}}}"#;
            let from_text = schema_from_python(py, PyString::new(py, text).as_any()).unwrap();
            let from_bytes =
                schema_from_python(py, PyBytes::new(py, text.as_bytes()).as_any()).unwrap();
            let dict = PyDict::new(py);
            dict.set_item("type", "object").unwrap();
            let properties = PyDict::new(py);
            let a = PyDict::new(py);
            a.set_item("type", "string").unwrap();
            properties.set_item("a", a).unwrap();
            dict.set_item("properties", properties).unwrap();
            let from_object = schema_from_python(py, dict.as_any()).unwrap();

            assert_eq!(from_text, expected);
            assert_eq!(from_bytes, expected);
            assert_eq!(from_object, expected);

            assert!(schema_from_python(py, PyString::new(py, "{not json").as_any()).is_err());
            assert!(schema_from_python(py, PyBytes::new(py, &[0xff, 0xfe]).as_any()).is_err());
        });
    }

    #[test]
    fn analysis_report_uses_the_default_budgets() {
        let report = analyze_to_report(
            &task_schema(),
            &DEFAULT_COMPARISON_SIZES,
            ANALYSIS_SCHEMA_LABEL,
        )
        .unwrap();
        assert_eq!(report["schema_file"], ANALYSIS_SCHEMA_LABEL);
        let sizes: Vec<u64> = report["recommendations"]["efficiency_analysis"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["vocab_size"].as_u64().unwrap())
            .collect();
        assert_eq!(sizes, vec![64, 128, 256, 512]);
    }
}
