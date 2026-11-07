//! Data-driven terminal classification for tree-sitter metadata pins (CPU-5 T3).
//!
//! Structural composites are always Nonterminal (decided by the adapter from
//! node-types shape). Leaf kinds use the built-in default map plus an optional
//! per-pin `terminal_classes.json` sidecar.

use crate::grammar_ir::GrammarTerminalKind;
use crate::provenance::file_provenance;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::Path;

/// Errors while loading or parsing a terminal-class table / sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalClassError {
    pub phase: &'static str,
    pub node_type: Option<String>,
    pub message: String,
}

impl fmt::Display for TerminalClassError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} error: {}", self.phase, self.message)
    }
}

impl Error for TerminalClassError {}

/// Built-in default terminal-class map. Names match the historical
/// `classify_terminal` allowlist and must not move without a layout migration.
///
/// BooleanPayload is reserved for node kinds that are exactly `true` / `false`.
/// Shared-SM mixed-field compile (`invalid_boolean_identity`) rejects any other
/// BooleanPayload node_kind, so wrapper names such as `boolean` /
/// `boolean_literal` must fall through to Nonterminal (structural leaf rule
/// identity) rather than BooleanPayload. Do not re-add those aliases without a
/// layout migration and a mixed-field compile proof on a held-out pin.
pub fn default_terminal_class_map() -> BTreeMap<&'static str, GrammarTerminalKind> {
    BTreeMap::from([
        ("identifier", GrammarTerminalKind::IdentifierPayload),
        ("string", GrammarTerminalKind::StringPayload),
        ("string_literal", GrammarTerminalKind::StringPayload),
        ("string_content", GrammarTerminalKind::StringPayload),
        ("escape_sequence", GrammarTerminalKind::StringPayload),
        ("number", GrammarTerminalKind::IntegerPayload),
        ("integer", GrammarTerminalKind::IntegerPayload),
        ("number_literal", GrammarTerminalKind::IntegerPayload),
        ("true", GrammarTerminalKind::BooleanPayload),
        ("false", GrammarTerminalKind::BooleanPayload),
    ])
}

/// Leaf terminal-class table: default map plus optional pin overrides.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalClassTable {
    /// Kind → class overrides (and defaults after `with_defaults`).
    classes: BTreeMap<String, GrammarTerminalKind>,
}

impl TerminalClassTable {
    /// Default historical map only — no pin overrides.
    pub fn with_defaults() -> Self {
        let mut table = Self::default();
        for (kind, terminal) in default_terminal_class_map() {
            table.classes.insert(kind.to_string(), terminal);
        }
        table
    }

    /// Merge sidecar overrides on top of the default map.
    pub fn with_defaults_and_overrides(overrides: BTreeMap<String, GrammarTerminalKind>) -> Self {
        let mut table = Self::with_defaults();
        for (kind, terminal) in overrides {
            table.classes.insert(kind, terminal);
        }
        table
    }

    pub fn class_for_leaf(&self, node_kind: &str) -> GrammarTerminalKind {
        self.classes
            .get(node_kind)
            .copied()
            .unwrap_or(GrammarTerminalKind::Nonterminal)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TerminalClassesSidecar {
    version: u32,
    #[serde(default)]
    classes: BTreeMap<String, String>,
}

const TERMINAL_CLASSES_SIDECAR_VERSION: u32 = 1;

fn parse_terminal_class_name(
    kind: &str,
    name: &str,
) -> Result<GrammarTerminalKind, TerminalClassError> {
    match name {
        "nonterminal" => Ok(GrammarTerminalKind::Nonterminal),
        "identifier" => Ok(GrammarTerminalKind::IdentifierPayload),
        "string" => Ok(GrammarTerminalKind::StringPayload),
        "integer" => Ok(GrammarTerminalKind::IntegerPayload),
        "boolean" => Ok(GrammarTerminalKind::BooleanPayload),
        other => Err(TerminalClassError {
            phase: "parse",
            node_type: Some(kind.to_string()),
            message: format!(
                "unknown terminal class `{other}` for kind `{kind}` \
                 (expected nonterminal|identifier|string|integer|boolean)"
            ),
        }),
    }
}

/// Load the default map, optionally merged with a pin-declared sidecar.
pub fn load_terminal_class_table(
    dir: &Path,
    terminal_classes_file: Option<&str>,
    terminal_classes_sha256: Option<&str>,
) -> Result<TerminalClassTable, TerminalClassError> {
    match (terminal_classes_file, terminal_classes_sha256) {
        (None, None) => Ok(TerminalClassTable::with_defaults()),
        (Some(file), Some(expected_sha)) => {
            let path = dir.join(file);
            let contents =
                std::fs::read_to_string(&path).map_err(|error| TerminalClassError {
                    phase: "read",
                    node_type: None,
                    message: format!("failed to read {}: {error}", path.display()),
                })?;
            let file_prov = file_provenance(&path).map_err(|error| TerminalClassError {
                phase: "provenance",
                node_type: None,
                message: error.to_string(),
            })?;
            if file_prov.sha256 != expected_sha {
                return Err(TerminalClassError {
                    phase: "provenance",
                    node_type: None,
                    message: format!(
                        "terminal_classes sha256 mismatch: pin expects {}, file has {}",
                        expected_sha, file_prov.sha256
                    ),
                });
            }
            let sidecar: TerminalClassesSidecar =
                serde_json::from_str(&contents).map_err(|error| TerminalClassError {
                    phase: "parse",
                    node_type: None,
                    message: format!("terminal_classes.json is invalid: {error}"),
                })?;
            if sidecar.version != TERMINAL_CLASSES_SIDECAR_VERSION {
                return Err(TerminalClassError {
                    phase: "parse",
                    node_type: None,
                    message: format!(
                        "unsupported terminal_classes version {} (expected {TERMINAL_CLASSES_SIDECAR_VERSION})",
                        sidecar.version
                    ),
                });
            }
            let mut overrides = BTreeMap::new();
            for (kind, class_name) in sidecar.classes {
                overrides.insert(kind.clone(), parse_terminal_class_name(&kind, &class_name)?);
            }
            Ok(TerminalClassTable::with_defaults_and_overrides(overrides))
        }
        (Some(_), None) | (None, Some(_)) => Err(TerminalClassError {
            phase: "parse",
            node_type: None,
            message: "pin.json must set both terminal_classes_file and terminal_classes_sha256, or neither"
                .to_string(),
        }),
    }
}
