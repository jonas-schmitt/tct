//! Structured classification of instance-encode failures.
//!
//! TCT closed objects reject undeclared keys that JSON Schema Draft may still
//! accept when `additionalProperties` is omitted. Prepare/corpus tools need
//! stable error **classes** (not only path lists) so agentic pipelines can
//! inventory unencodable mass without ad hoc string scraping.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Stable failure class for encode / prepare inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncodeFailureClass {
    /// Closed object / `deny_unknown_fields`: undeclared JSON key.
    UnknownField,
    /// Required property missing at deserialize.
    MissingField,
    /// Serde type mismatch (`invalid type`, wrong JSON kind).
    TypeMismatch,
    /// Unknown enum variant / const mismatch.
    UnknownVariant,
    /// File read failure or invalid JSON syntax (helper `read:` / bare JSON errors).
    IoOrParse,
    /// Empty token sequence treated as failure by the parent batch reader.
    EmptyTokens,
    /// Unclassified remainder.
    Other,
}

impl EncodeFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnknownField => "unknown_field",
            Self::MissingField => "missing_field",
            Self::TypeMismatch => "type_mismatch",
            Self::UnknownVariant => "unknown_variant",
            Self::IoOrParse => "io_or_parse",
            Self::EmptyTokens => "empty_tokens",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for EncodeFailureClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One failed encode attempt with path, raw message, and classified class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodeFailure {
    pub path: PathBuf,
    pub message: String,
    pub class: EncodeFailureClass,
}

impl EncodeFailure {
    pub fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        let message = message.into();
        let class = classify_encode_failure_message(&message);
        Self {
            path: path.into(),
            message,
            class,
        }
    }

    pub fn empty_tokens(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            message: "empty token sequence".to_string(),
            class: EncodeFailureClass::EmptyTokens,
        }
    }
}

/// Classify a helper/parent encode error string into a stable inventory class.
///
/// Helper batch records use prefixes `parse: ` (serde deserialize) and
/// `read: ` (filesystem). Single-file encode wraps those in
/// `Failed to encode …: …`. Classification is intentionally heuristic and
/// pinned to serde's English messages for the workspace serde version.
pub fn classify_encode_failure_message(message: &str) -> EncodeFailureClass {
    let lower = message.to_ascii_lowercase();
    let body = lower
        .strip_prefix("parse: ")
        .or_else(|| lower.strip_prefix("read: "))
        .unwrap_or(lower.as_str());

    if lower.starts_with("read:")
        || body.contains("no such file")
        || body.contains("permission denied")
    {
        return EncodeFailureClass::IoOrParse;
    }

    if body.contains("unknown field") || body.contains("deny_unknown_fields") {
        return EncodeFailureClass::UnknownField;
    }
    if body.contains("missing field") {
        return EncodeFailureClass::MissingField;
    }
    if body.contains("unknown variant") || body.contains("did not match any variant") {
        return EncodeFailureClass::UnknownVariant;
    }
    if body.contains("invalid type")
        || body.contains("invalid value")
        || body.contains("invalid length")
        || (body.contains("expected a")
            && (body.contains("string")
                || body.contains("map")
                || body.contains("sequence")
                || body.contains("boolean")
                || body.contains("integer")
                || body.contains("number")
                || body.contains("null")
                || body.contains("array")
                || body.contains("object")))
    {
        return EncodeFailureClass::TypeMismatch;
    }
    if body.contains("expected value")
        || body.contains("eof while parsing")
        || body.contains("trailing characters")
        || body.contains("key must be a string")
        || body.contains("invalid number")
        || body.contains("control character")
    {
        return EncodeFailureClass::IoOrParse;
    }
    if body.contains("empty token") {
        return EncodeFailureClass::EmptyTokens;
    }

    EncodeFailureClass::Other
}

/// Prepare/corpus inventory of unencodable instances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnencodableReport {
    pub schema_version: String,
    pub total_attempted: usize,
    pub failed: usize,
    pub succeeded: usize,
    pub counts_by_class: BTreeMap<String, usize>,
    /// Capped sample paths per class for human debugging.
    pub samples_by_class: BTreeMap<String, Vec<String>>,
    pub failures: Vec<UnencodableFailureEntry>,
    pub policy_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnencodableFailureEntry {
    pub path: String,
    pub class: String,
    pub message: String,
}

const DEFAULT_SAMPLES_PER_CLASS: usize = 5;

impl UnencodableReport {
    pub fn from_failures(
        total_attempted: usize,
        failures: &[EncodeFailure],
        samples_per_class: usize,
    ) -> Self {
        let failed = failures.len();
        let succeeded = total_attempted.saturating_sub(failed);
        let mut counts_by_class: BTreeMap<String, usize> = BTreeMap::new();
        let mut samples_by_class: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut entries = Vec::with_capacity(failures.len());

        for failure in failures {
            let class_key = failure.class.as_str().to_string();
            *counts_by_class.entry(class_key.clone()).or_insert(0) += 1;
            let samples = samples_by_class.entry(class_key.clone()).or_default();
            if samples.len() < samples_per_class {
                samples.push(failure.path.display().to_string());
            }
            entries.push(UnencodableFailureEntry {
                path: failure.path.display().to_string(),
                class: class_key,
                message: failure.message.clone(),
            });
        }

        Self {
            schema_version: "tct-unencodable-report-v1".to_string(),
            total_attempted,
            failed,
            succeeded,
            counts_by_class,
            samples_by_class,
            failures: entries,
            policy_note: "JSON Schema valid ≠ TCT encodable. Closed objects \
                (additionalProperties: false / deny_unknown_fields) reject \
                undeclared keys; class unknown_field is the common agentic \
                corpus drop. Policy A (strict reject) is the default; strip-extra \
                and open-map paths are opt-in schema shapes, not silent defaults."
                .to_string(),
        }
    }

    pub fn from_failures_default(total_attempted: usize, failures: &[EncodeFailure]) -> Self {
        Self::from_failures(total_attempted, failures, DEFAULT_SAMPLES_PER_CLASS)
    }

    pub fn write_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        std::fs::write(path, json)
    }

    /// Compact one-line summary for stderr: `unencodable: N (unknown_field=…, …)`.
    pub fn summary_line(&self) -> String {
        if self.failed == 0 {
            return "unencodable: 0".to_string();
        }
        let mut parts: Vec<String> = self
            .counts_by_class
            .iter()
            .map(|(class, count)| format!("{class}={count}"))
            .collect();
        parts.sort();
        format!("unencodable: {} ({})", self.failed, parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_unknown_field_from_serde_message() {
        let msg = "parse: unknown field `name`, expected one of `id`, `value` at line 1 column 20";
        assert_eq!(
            classify_encode_failure_message(msg),
            EncodeFailureClass::UnknownField
        );
    }

    #[test]
    fn classifies_missing_field_and_type_mismatch() {
        assert_eq!(
            classify_encode_failure_message("parse: missing field `calls`"),
            EncodeFailureClass::MissingField
        );
        assert_eq!(
            classify_encode_failure_message("parse: invalid type: string \"x\", expected u64"),
            EncodeFailureClass::TypeMismatch
        );
    }

    #[test]
    fn classifies_read_and_json_parse() {
        assert_eq!(
            classify_encode_failure_message("read: No such file or directory (os error 2)"),
            EncodeFailureClass::IoOrParse
        );
        assert_eq!(
            classify_encode_failure_message("parse: expected value at line 1 column 1"),
            EncodeFailureClass::IoOrParse
        );
    }

    #[test]
    fn classifies_unknown_variant() {
        assert_eq!(
            classify_encode_failure_message(
                "parse: unknown variant `basic`, expected `premium` at line 1 column 10"
            ),
            EncodeFailureClass::UnknownVariant
        );
    }

    #[test]
    fn unencodable_report_counts_by_class() {
        let failures = vec![
            EncodeFailure::new("a.json", "parse: unknown field `x`, expected `y`"),
            EncodeFailure::new("b.json", "parse: unknown field `z`, expected `y`"),
            EncodeFailure::new("c.json", "parse: missing field `id`"),
        ];
        let report = UnencodableReport::from_failures_default(10, &failures);
        assert_eq!(report.total_attempted, 10);
        assert_eq!(report.failed, 3);
        assert_eq!(report.succeeded, 7);
        assert_eq!(report.counts_by_class.get("unknown_field"), Some(&2));
        assert_eq!(report.counts_by_class.get("missing_field"), Some(&1));
        assert!(report.summary_line().contains("unknown_field=2"));
        assert!(
            report
                .policy_note
                .contains("JSON Schema valid ≠ TCT encodable")
        );
    }
}
