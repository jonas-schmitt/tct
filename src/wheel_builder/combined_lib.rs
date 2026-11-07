//! Combined library generation for assembled wheel packages.

use std::error::Error;

/// Generate combined lib.rs with generated code + PyO3 wrapper
///
/// # Arguments
/// * `type_name` - Root type name (e.g., "GithubWorkflow", "K8S")
/// * `package_name` - Python package name (e.g., "tct-workflow", "tct-k8s")
/// * `tokenizer_code` - Generated Rust tokenizer code
/// * `has_bpe` - Whether BPE compression is enabled
/// * `has_streaming` - Whether streaming mode (decode_prefix) is enabled
pub(super) fn generate_combined_lib(
    type_name: &str,
    package_name: &str,
    tokenizer_code: &str,
    has_bpe: bool,
    has_streaming: bool,
) -> Result<String, Box<dyn Error>> {
    let module_name = package_name.replace('-', "_");

    // Build PyO3 wrapper with conditional Mode 2 support and BPE compression
    // Generate encode/decode implementations based on whether BPE is enabled
    // Typed (de)serialization only: avoid Value intermediate round-trips that
    // double-materialize JSON trees on every encode/decode measurement.
    let (encode_body, decode_body) = if has_bpe {
        // BPE enabled: Use codec wrapper
        let enc = r#"    use crate::tokenizer::JsonTokenEncoder;

    let json_owned = json_str.to_owned();
    let result: Result<Vec<u32>, String> = py.detach(move || {
        let instance = serde_json::from_str::<__TYPE_NAME__>(&json_owned)
            .map_err(|e| format!("JSON parse or schema validation failed: {}", e))?;

        // Apply BPE compression via codec wrapper
        Ok(codec::encode(&instance))
    });
    result.map_err(PyValueError::new_err)"#
            .replace("__TYPE_NAME__", type_name);

        let dec = r#"    use crate::tokenizer::JsonTokenEncoder;

    let result: Result<(String, usize, usize), String> = py.detach(move || {
        // Decode with BPE expansion via codec wrapper
        let (instance, consumed, total) = codec::decode::<__TYPE_NAME__>(tokens)
            .map_err(|e| format!("Decode failed: {}", e))?;

        let json_str = serde_json::to_string(&instance)
            .map_err(|e| format!("JSON serialization failed: {}", e))?;

        Ok((json_str, consumed, total))
    });
    result.map_err(PyRuntimeError::new_err)"#
            .replace("__TYPE_NAME__", type_name);

        (enc, dec)
    } else {
        // No BPE: Direct encoding
        let enc = r#"    use crate::tokenizer::JsonTokenEncoder;

    let json_owned = json_str.to_owned();
    let result: Result<Vec<u32>, String> = py.detach(move || {
        let instance = serde_json::from_str::<__TYPE_NAME__>(&json_owned)
            .map_err(|e| format!("JSON parse or schema validation failed: {}", e))?;

        let mut tokens = Vec::new();
        instance.encode(&mut tokens);
        Ok(tokens)
    });
    result.map_err(PyValueError::new_err)"#
            .replace("__TYPE_NAME__", type_name);

        let dec = r#"    use crate::tokenizer::JsonTokenEncoder;

    let result: Result<(String, usize, usize), String> = py.detach(move || {
        let total = tokens.len();
        let mut token_iter = tokens.into_iter().peekable();
        let instance = __TYPE_NAME__::decode(&mut token_iter)
            .map_err(|e| format!("Decode failed: {}", e))?;
        let consumed = total - token_iter.count();

        let json_str = serde_json::to_string(&instance)
            .map_err(|e| format!("JSON serialization failed: {}", e))?;

        Ok((json_str, consumed, total))
    });
    result.map_err(PyRuntimeError::new_err)"#
            .replace("__TYPE_NAME__", type_name);

        (enc, dec)
    };

    // Streaming mode functions (decode_prefix for truncation-tolerant decoding)
    let streaming_functions = if has_streaming {
        if has_bpe {
            // BPE-aware decode_prefix: expand BPE tokens before decoding
            format!(
                r#"

/// Decode from potentially truncated token stream (streaming mode)
///
/// Returns partial struct with defaults when EOF is encountered.
/// Useful for LLM autocompletion where token stream may be incomplete.
/// BPE tokens are automatically expanded before decoding.
///
/// # Arguments
/// * `tokens` - Token list (may be truncated/incomplete, BPE compressed)
///
/// # Returns
/// Tuple of (json_string, fields_decoded, is_complete)
/// - json_string: JSON with decoded fields (remaining fields get defaults)
/// - fields_decoded: Number of fields successfully decoded
/// - is_complete: True if all fields were decoded
#[pyfunction]
fn decode_prefix(py: Python<'_>, tokens: Vec<u32>) -> PyResult<(String, usize, bool)> {{
    let result: Result<(String, usize, bool), String> = py.detach(move || {{
        // First expand BPE tokens back to base tokens
        let base_tokens = bpe::decode(&tokens, VOCAB_SIZE);
        let mut token_iter = base_tokens.into_iter();
        let result = {}::decode_prefix(&mut token_iter);

        let json_str = serde_json::to_string(&result.instance)
            .map_err(|e| format!("JSON serialization failed: {{}}", e))?;

        Ok((json_str, result.fields_decoded, result.complete))
    }});
    result.map_err(PyRuntimeError::new_err)
}}
"#,
                type_name
            )
        } else {
            // Non-BPE decode_prefix: direct decoding
            format!(
                r#"

/// Decode from potentially truncated token stream (streaming mode)
///
/// Returns partial struct with defaults when EOF is encountered.
/// Useful for LLM autocompletion where token stream may be incomplete.
///
/// # Arguments
/// * `tokens` - Token list (may be truncated/incomplete)
///
/// # Returns
/// Tuple of (json_string, fields_decoded, is_complete)
/// - json_string: JSON with decoded fields (remaining fields get defaults)
/// - fields_decoded: Number of fields successfully decoded
/// - is_complete: True if all fields were decoded
#[pyfunction]
fn decode_prefix(tokens: Vec<u32>) -> PyResult<(String, usize, bool)> {{
    let mut token_iter = tokens.into_iter();
    let result = {}::decode_prefix(&mut token_iter);

    let json_str = serde_json::to_string(&result.instance)
        .map_err(|e| PyRuntimeError::new_err(format!("JSON serialization failed: {{}}", e)))?;

    Ok((json_str, result.fields_decoded, result.complete))
}}
"#,
                type_name
            )
        }
    } else {
        String::new()
    };

    // Cloneable decode-state handle: stores the wheel-space prefix and classifies
    // each append via the generated full-prefix decode oracle. Optional root-token
    // restriction lets the host enforce projected branch candidates without
    // replaying host-side masks alone. True O(1) structural transitions remain
    // future work.
    let decode_state_functions = if has_streaming {
        let classify_fn_body = if has_bpe {
            format!(
                r#"
    let base_tokens = bpe::decode(tokens, VOCAB_SIZE);
    let total = base_tokens.len();
    let mut iter = base_tokens.into_iter().peekable();
    match {type_name}::decode(&mut iter) {{
        Ok(_) => {{
            let remaining = iter.count();
            if remaining == 0 {{
                "complete".to_string()
            }} else {{
                "dead".to_string()
            }}
        }}
        Err(error) => {{
            let message = error.to_string();
            if message.contains("Unexpected end of token stream") {{
                let _ = total;
                "prefix_ok".to_string()
            }} else {{
                "dead".to_string()
            }}
        }}
    }}
"#,
                type_name = type_name
            )
        } else {
            format!(
                r#"
    let total = tokens.len();
    let mut iter = tokens.iter().copied().peekable();
    match {type_name}::decode(&mut iter) {{
        Ok(_) => {{
            let remaining = iter.count();
            if remaining == 0 {{
                "complete".to_string()
            }} else {{
                "dead".to_string()
            }}
        }}
        Err(error) => {{
            let message = error.to_string();
            if message.contains("Unexpected end of token stream") {{
                let _ = total;
                "prefix_ok".to_string()
            }} else {{
                "dead".to_string()
            }}
        }}
    }}
"#,
                type_name = type_name
            )
        };
        let render_body = if has_bpe {
            format!(
                r#"
        let base_tokens = bpe::decode(&self.tokens, VOCAB_SIZE);
        let mut iter = base_tokens.into_iter();
        let result = {type_name}::decode_prefix(&mut iter);
        let json_str = serde_json::to_string(&result.instance)
            .map_err(|e| PyRuntimeError::new_err(format!("JSON serialization failed: {{}}", e)))?;
        Ok((json_str, result.fields_decoded, result.complete))
"#,
                type_name = type_name
            )
        } else {
            format!(
                r#"
        let mut iter = self.tokens.iter().copied();
        let result = {type_name}::decode_prefix(&mut iter);
        let json_str = serde_json::to_string(&result.instance)
            .map_err(|e| PyRuntimeError::new_err(format!("JSON serialization failed: {{}}", e)))?;
        Ok((json_str, result.fields_decoded, result.complete))
"#,
                type_name = type_name
            )
        };
        let (terminal_key_body, terminal_path_body) = if has_bpe {
            (
                format!(
                    r#"
        if !root_token_allowed(
            &self.tokens,
            self.allowed_root_tokens.as_deref(),
        ) {{
            return None;
        }}
        let base_tokens = bpe::decode(&self.tokens, VOCAB_SIZE);
        tokenizer::terminal_key_export::current_terminal_key_path::<{type_name}>(&base_tokens)
            .map(|(key, _path)| key)
"#,
                    type_name = type_name
                ),
                format!(
                    r#"
        if !root_token_allowed(
            &self.tokens,
            self.allowed_root_tokens.as_deref(),
        ) {{
            return None;
        }}
        let base_tokens = bpe::decode(&self.tokens, VOCAB_SIZE);
        tokenizer::terminal_key_export::current_terminal_key_path::<{type_name}>(&base_tokens)
            .map(|(_key, path)| path)
"#,
                    type_name = type_name
                ),
            )
        } else {
            (
                format!(
                    r#"
        if !root_token_allowed(
            &self.tokens,
            self.allowed_root_tokens.as_deref(),
        ) {{
            return None;
        }}
        tokenizer::terminal_key_export::current_terminal_key_path::<{type_name}>(&self.tokens)
            .map(|(key, _path)| key)
"#,
                    type_name = type_name
                ),
                format!(
                    r#"
        if !root_token_allowed(
            &self.tokens,
            self.allowed_root_tokens.as_deref(),
        ) {{
            return None;
        }}
        tokenizer::terminal_key_export::current_terminal_key_path::<{type_name}>(&self.tokens)
            .map(|(_key, path)| path)
"#,
                    type_name = type_name
                ),
            )
        };
        format!(
            r#"

/// Classify a wheel-space token prefix via full-prefix decode replay.
///
/// Returns one of: `"complete"`, `"prefix_ok"`, `"dead"`. Empty prefixes are
/// `"prefix_ok"`. This is the oracle behind `DecodeState::status`.
fn classify_decode_tokens(tokens: &[u32]) -> String {{
    use crate::tokenizer::JsonTokenEncoder;
    if tokens.is_empty() {{
        return "prefix_ok".to_string();
    }}
{classify_fn_body}
}}

/// True when unrestricted, or when the first wheel-space token is in `allowed`.
///
/// Empty prefixes always pass (no root choice committed yet). Restriction is on
/// the first stored encode-space token (raw or BPE-compressed), matching the
/// stream `encode()` emits — host maps branch ids to those root tokens.
fn root_token_allowed(tokens: &[u32], allowed: Option<&[u32]>) -> bool {{
    match (allowed, tokens.first()) {{
        (None, _) | (_, None) => true,
        (Some(allowed), Some(first)) => allowed.contains(first),
    }}
}}

/// Cloneable generated decoder state for host-side candidate probing.
///
/// Appends are classified by replaying the full prefix through the generated
/// decoder (oracle). Optional `allowed_root_tokens` rejects prefixes whose
/// first wheel-space token is outside the set. Callers may `clone_state()`
/// before speculative appends.
#[pyclass]
#[derive(Clone)]
struct DecodeState {{
    tokens: Vec<u32>,
    /// When set, first wheel-space token must be in this list or status is dead.
    allowed_root_tokens: Option<Vec<u32>>,
}}

#[pymethods]
impl DecodeState {{
    #[new]
    fn new() -> Self {{
        Self {{
            tokens: Vec::new(),
            allowed_root_tokens: None,
        }}
    }}

    /// Independent clone for speculative candidate probing (preserves restriction).
    fn clone_state(&self) -> Self {{
        self.clone()
    }}

    /// Tokens accepted so far (raw or BPE-compressed, matching encode()).
    fn tokens(&self) -> Vec<u32> {{
        self.tokens.clone()
    }}

    /// Optional allowed first-token set, or `None` when unrestricted.
    fn allowed_root_tokens(&self) -> Option<Vec<u32>> {{
        self.allowed_root_tokens.clone()
    }}

    fn len(&self) -> usize {{
        self.tokens.len()
    }}

    /// Classify the current prefix without appending.
    ///
    /// Returns one of: `"complete"`, `"prefix_ok"`, `"dead"`.
    fn status(&self) -> String {{
        if !root_token_allowed(
            &self.tokens,
            self.allowed_root_tokens.as_deref(),
        ) {{
            return "dead".to_string();
        }}
        classify_decode_tokens(&self.tokens)
    }}

    /// Append one token and return the new status string.
    fn try_append(&mut self, token: u32) -> String {{
        self.tokens.push(token);
        self.status()
    }}

    /// Truncation-tolerant render of the current prefix.
    ///
    /// Returns `(json_string, fields_decoded, is_complete)`.
    fn render_prefix(&self) -> PyResult<(String, usize, bool)> {{
{render_body}
    }}

    /// Terminal object-property key at the current host-value cursor, if any.
    ///
    /// Returns `Some(key)` when the raw prefix is positioned immediately before
    /// a host-value byte (or mid host-value before VALUE_END) for a schema
    /// object property. Returns `None` when the prefix is complete, dead, or
    /// waiting on structure. Never inspects gold JSON.
    fn current_terminal_key(&self) -> Option<String> {{
{terminal_key_body}
    }}

    /// Dotted schema path for the current host-value cursor (array indices as
    /// decimal segments). Same `None` conditions as `current_terminal_key`.
    fn current_terminal_path(&self) -> Option<String> {{
{terminal_path_body}
    }}
}}

/// Construct an empty unrestricted cloneable decoder state.
#[pyfunction]
fn begin_decode_state() -> DecodeState {{
    DecodeState::new()
}}

/// Construct a decoder state that rejects prefixes whose first wheel-space token
/// is outside `allowed_root_tokens`.
///
/// Empty prefixes remain `"prefix_ok"`. Hosts should pass the encode-space root
/// discriminator tokens that correspond to projected branch / tool candidates.
#[pyfunction]
fn begin_decode_state_restricted(allowed_root_tokens: Vec<u32>) -> DecodeState {{
    DecodeState {{
        tokens: Vec::new(),
        allowed_root_tokens: Some(allowed_root_tokens),
    }}
}}
"#,
            classify_fn_body = classify_fn_body,
            render_body = render_body,
            terminal_key_body = terminal_key_body,
            terminal_path_body = terminal_path_body
        )
    } else {
        String::new()
    };

    // PyO3 module registrations for streaming mode (conditional)
    let streaming_registrations = if has_streaming {
        r#"
    // Streaming mode (truncation-tolerant decoding)
    m.add_function(wrap_pyfunction!(decode_prefix, m)?)?;
    m.add_function(wrap_pyfunction!(begin_decode_state, m)?)?;
    m.add_function(wrap_pyfunction!(begin_decode_state_restricted, m)?)?;
    m.add_class::<DecodeState>()?;
"#
        .to_string()
    } else {
        String::new()
    };

    let encode_sequence_body = r#"    let json_owned = json_str.to_owned();
    let result: Result<Vec<u32>, String> = py.detach(move || {
        let instance = serde_json::from_str::<__TYPE_NAME__>(&json_owned)
            .map_err(|e| format!("JSON parse or schema validation failed: {}", e))?;

        Ok(sequence::encode(&instance))
    });
    result.map_err(PyValueError::new_err)"#
        .replace("__TYPE_NAME__", type_name);

    let decode_sequence_body =
        r#"    let result: Result<(String, usize, usize), String> = py.detach(move || {
        let (instance, consumed, total) = sequence::decode::<__TYPE_NAME__>(&tokens)
            .map_err(|e| format!("Decode failed: {}", e))?;

        let json_str = serde_json::to_string(&instance)
            .map_err(|e| format!("JSON serialization failed: {}", e))?;

        Ok((json_str, consumed, total))
    });
    result.map_err(PyRuntimeError::new_err)"#
            .replace("__TYPE_NAME__", type_name);

    let token_roles_body = r#"    let result: Result<Vec<u8>, String> = py.detach(move || {
        // roles are defined over RAW (pre-BPE) streams; expand merges first
        let base_tokens = __BPE_EXPAND__;
        tokenizer::role_export::token_roles::<__TYPE_NAME__>(&base_tokens)
            .map_err(|e| format!("Role derivation failed: {}", e))
    });
    result.map_err(PyRuntimeError::new_err)"#
        .replace("__TYPE_NAME__", type_name)
        .replace(
            "__BPE_EXPAND__",
            if has_bpe {
                "bpe::decode(&tokens, VOCAB_SIZE)"
            } else {
                "tokens"
            },
        );

    let strict_prefix_roles_body =
        r#"    let result: Result<(Vec<u8>, bool), String> = py.detach(move || {
        let base_tokens = __BPE_STRICT_EXPAND__;
        tokenizer::role_export::strict_prefix_roles::<__TYPE_NAME__>(&base_tokens)
            .map_err(|e| format!("Strict prefix role derivation failed: {}", e))
    });
    result.map_err(PyRuntimeError::new_err)"#
            .replace("__TYPE_NAME__", type_name)
            .replace(
                "__BPE_STRICT_EXPAND__",
                if has_bpe {
                    "bpe::decode_strict(&tokens, VOCAB_SIZE)\n            .map_err(|e| format!(\"Strict BPE expansion failed: {}\", e))?"
                } else {
                    "tokens"
                },
            );

    let mode1_and_windowed = format!(
        r#"
// PyO3 bindings (uses generated tokenizer code above)
use pyo3::prelude::*;
use pyo3::exceptions::{{PyValueError, PyRuntimeError}};

/// Encode JSON string to tokens.
///
/// Uses all compiled optimizations, including BPE when configured.
#[pyfunction]
fn encode(py: Python<'_>, json_str: &str) -> PyResult<Vec<u32>> {{
{}
}}

/// Decode tokens with consumption tracking (detects surplus tokens)
///
/// Returns (json_string, tokens_consumed, total_tokens).
/// Always tracks token consumption for error detection.
///
/// # Example
/// ```python
/// json_str, consumed, total = tct.decode(tokens)
/// if consumed < total:
///     print(f"Warning: {{total - consumed}} surplus tokens not consumed")
/// ```
#[pyfunction]
fn decode(py: Python<'_>, tokens: Vec<u32>) -> PyResult<(String, usize, usize)> {{
{}
}}

/// Per-token roles for a raw or BPE stream (0=structure, 1=value byte,
/// 2=end-of-value marker, 3=oneOf/enum variant selector aka discriminator),
/// derived by replaying decode. BPE streams are expanded first, so the returned
/// roles align with the EXPANDED tokens.
#[pyfunction]
fn token_roles(py: Python<'_>, tokens: Vec<u32>) -> PyResult<Vec<u8>> {{
{}
}}

/// Strictly classify a raw complete or valid-incomplete prefix.
///
/// Returns `(roles, complete)`. Each role aligns to one expanded raw token;
/// malformed prefixes, recovery-only tokens, and surplus tokens are rejected.
#[pyfunction]
fn strict_prefix_roles(py: Python<'_>, tokens: Vec<u32>) -> PyResult<(Vec<u8>, bool)> {{
{}
}}

/// Get vocabulary size
#[pyfunction]
fn vocab_size() -> u32 {{
    VOCAB_SIZE
}}

/// Get PAD token value
#[pyfunction]
fn pad_token() -> u32 {{
    PAD_TOKEN
}}

/// Get BOS token value for model-facing sequences.
#[pyfunction]
fn bos_token() -> u32 {{
    SEQUENCE_BOS_TOKEN
}}

/// Get EOS token value for model-facing sequences.
#[pyfunction]
fn eos_token() -> u32 {{
    SEQUENCE_EOS_TOKEN
}}

/// Get PAD token value for model-facing sequences.
#[pyfunction]
fn sequence_pad_token() -> u32 {{
    SEQUENCE_PAD_TOKEN
}}

/// Get schema-decision token value for the single generated schema.
#[pyfunction]
fn schema_token() -> u32 {{
    SEQUENCE_SCHEMA_TOKEN
}}

/// Get payload token offset for model-facing sequences.
#[pyfunction]
fn sequence_payload_offset() -> u32 {{
    SEQUENCE_PAYLOAD_OFFSET
}}

/// Get vocabulary size for model-facing sequences with BOS/EOS.
#[pyfunction]
fn sequence_vocab_size() -> u32 {{
    SEQUENCE_VOCAB_SIZE
}}

/// Get the sequence-boundary policy version.
#[pyfunction]
fn sequence_boundary_version() -> u32 {{
    SEQUENCE_BOUNDARY_VERSION
}}

/// Get sequence-boundary metadata as JSON.
#[pyfunction]
fn sequence_metadata_json() -> PyResult<String> {{
    serde_json::to_string(&sequence::metadata())
        .map_err(|e| PyRuntimeError::new_err(format!("Metadata serialization failed: {{}}", e)))
}}

/// Encode JSON string as a model-facing sequence with BOS/EOS.
#[pyfunction]
fn encode_sequence(py: Python<'_>, json_str: &str) -> PyResult<Vec<u32>> {{
{}
}}

/// Decode a model-facing sequence with BOS/EOS.
#[pyfunction]
fn decode_sequence(py: Python<'_>, tokens: Vec<u32>) -> PyResult<(String, usize, usize)> {{
{}
}}{streaming_functions}{decode_state_functions}
"#,
        encode_body,
        decode_body,
        token_roles_body,
        strict_prefix_roles_body,
        encode_sequence_body,
        decode_sequence_body,
        streaming_functions = streaming_functions,
        decode_state_functions = decode_state_functions
    );

    // Module registration
    let module_registration = format!(
        r#"
/// Python module definition
#[pymodule]
fn {}(m: &Bound<'_, PyModule>) -> PyResult<()> {{
    // Mode 1: Complete encoding
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(vocab_size, m)?)?;

    // Special tokens
    m.add_function(wrap_pyfunction!(pad_token, m)?)?;
    m.add_function(wrap_pyfunction!(bos_token, m)?)?;
    m.add_function(wrap_pyfunction!(eos_token, m)?)?;
    m.add_function(wrap_pyfunction!(sequence_pad_token, m)?)?;
    m.add_function(wrap_pyfunction!(schema_token, m)?)?;
    m.add_function(wrap_pyfunction!(sequence_payload_offset, m)?)?;
    m.add_function(wrap_pyfunction!(sequence_vocab_size, m)?)?;
    m.add_function(wrap_pyfunction!(sequence_boundary_version, m)?)?;
    m.add_function(wrap_pyfunction!(sequence_metadata_json, m)?)?;
    m.add_function(wrap_pyfunction!(encode_sequence, m)?)?;
    m.add_function(wrap_pyfunction!(decode_sequence, m)?)?;
    m.add_function(wrap_pyfunction!(token_roles, m)?)?;
    m.add_function(wrap_pyfunction!(strict_prefix_roles, m)?)?;
{streaming_registrations}
    Ok(())
}}
"#,
        module_name,
        streaming_registrations = streaming_registrations
    );

    let pyo3_wrapper = format!("{}{}", mode1_and_windowed, module_registration);

    // Add no-op log_debug! macro for standalone code
    let log_macro_stub = r#"
// No-op logging macro for standalone generated code
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {};
}
"#;

    let combined = format!(
        "{}\n\n{}\n\n{}",
        log_macro_stub, tokenizer_code, pyo3_wrapper
    );
    Ok(combined)
}
