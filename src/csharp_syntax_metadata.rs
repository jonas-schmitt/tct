//! C# syntax metadata adapter for the first GrammarIR source-language fixture.
//!
//! This consumes a checked-in Roslyn-style metadata summary rather than running
//! Roslyn during Rust tests. The goal is to prove source-language syntax facts
//! can enter GrammarIR before any production C# parser integration exists.

use crate::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarParserProvenance,
    GrammarRule, GrammarTerminalKind,
};
use crate::provenance::file_provenance;
use serde::Deserialize;
use std::error::Error;
use std::fmt;
use std::path::Path;

pub const CSHARP_SYNTAX_METADATA_ARTIFACT_KIND: &str = "tct_csharp_syntax_metadata_v0";
pub const CSHARP_SYNTAX_METADATA_VERSION: u32 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpSyntaxMetadataAdapter {
    grammar_id: String,
}

impl CSharpSyntaxMetadataAdapter {
    pub fn new(grammar_id: impl Into<String>) -> Self {
        Self {
            grammar_id: grammar_id.into(),
        }
    }

    pub fn lower_metadata_file(&self, path: &Path) -> Result<GrammarIr, CSharpSyntaxMetadataError> {
        let contents = std::fs::read_to_string(path).map_err(|error| {
            CSharpSyntaxMetadataError::new(
                "read",
                None,
                None,
                Some(path.display().to_string()),
                format!("failed to read metadata: {error}"),
            )
        })?;
        let provenance = file_provenance(path).map_err(|error| {
            CSharpSyntaxMetadataError::new(
                "provenance",
                None,
                None,
                Some(path.display().to_string()),
                error.to_string(),
            )
        })?;
        self.lower_metadata_str(
            &contents,
            Some(path.display().to_string()),
            Some(provenance.sha256),
        )
    }

    pub fn lower_metadata_str(
        &self,
        contents: &str,
        metadata_path: Option<String>,
        metadata_sha256: Option<String>,
    ) -> Result<GrammarIr, CSharpSyntaxMetadataError> {
        let metadata: CSharpSyntaxMetadata = serde_json::from_str(contents).map_err(|error| {
            CSharpSyntaxMetadataError::new(
                "parse",
                None,
                None,
                None,
                format!("C# syntax metadata JSON is invalid: {error}"),
            )
        })?;
        validate_metadata(&metadata)?;

        let mut grammar = GrammarIr::new(
            self.grammar_id.clone(),
            GrammarIrProvenance {
                source_kind: "csharp_syntax_metadata".to_string(),
                source_path: Some(metadata.source.path),
                source_sha256: Some(metadata.source.sha256),
                metadata_path,
                metadata_sha256,
                parser: Some(GrammarParserProvenance {
                    tool: metadata.parser.tool,
                    version: metadata.parser.version,
                    package: metadata.parser.package,
                    package_version: metadata.parser.package_version,
                }),
            },
        );
        grammar.rules = metadata
            .rules
            .into_iter()
            .enumerate()
            .map(|(rule_index, rule)| lower_rule(rule_index, rule))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(grammar)
    }
}

fn validate_metadata(metadata: &CSharpSyntaxMetadata) -> Result<(), CSharpSyntaxMetadataError> {
    if metadata.artifact_kind != CSHARP_SYNTAX_METADATA_ARTIFACT_KIND {
        return Err(CSharpSyntaxMetadataError::new(
            "validate",
            None,
            None,
            Some("artifact_kind".to_string()),
            format!(
                "expected artifact kind {CSHARP_SYNTAX_METADATA_ARTIFACT_KIND}, got {}",
                metadata.artifact_kind
            ),
        ));
    }
    if metadata.version != CSHARP_SYNTAX_METADATA_VERSION {
        return Err(CSharpSyntaxMetadataError::new(
            "validate",
            None,
            None,
            Some("version".to_string()),
            format!(
                "expected metadata version {CSHARP_SYNTAX_METADATA_VERSION}, got {}",
                metadata.version
            ),
        ));
    }
    if metadata.source.path.is_empty() {
        return Err(CSharpSyntaxMetadataError::new(
            "validate",
            None,
            None,
            Some("source.path".to_string()),
            "source path must be present",
        ));
    }
    if metadata.source.sha256.len() != 64 {
        return Err(CSharpSyntaxMetadataError::new(
            "validate",
            None,
            None,
            Some("source.sha256".to_string()),
            "source sha256 must be a 64-character hex digest",
        ));
    }
    if metadata.parser.tool.is_empty() || metadata.parser.version.is_empty() {
        return Err(CSharpSyntaxMetadataError::new(
            "validate",
            None,
            None,
            Some("parser".to_string()),
            "parser tool and version must be present",
        ));
    }
    if metadata.rules.is_empty() {
        return Err(CSharpSyntaxMetadataError::new(
            "validate",
            None,
            None,
            Some("rules".to_string()),
            "metadata must contain at least one syntax rule",
        ));
    }
    Ok(())
}

fn lower_rule(
    rule_index: usize,
    rule: CSharpSyntaxRule,
) -> Result<GrammarRule, CSharpSyntaxMetadataError> {
    let rule_name = non_empty(
        rule.name,
        "lower",
        None,
        None,
        Some(format!("rules[{rule_index}].name")),
    )?;
    let node_kind = non_empty(
        rule.node_kind,
        "lower",
        Some(rule_name.clone()),
        None,
        Some(format!("rules[{rule_index}].node_kind")),
    )?;
    let fields = rule
        .fields
        .into_iter()
        .enumerate()
        .map(|(field_index, field)| lower_field(&rule_name, rule_index, field_index, field))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(GrammarRule {
        name: rule_name,
        node_kind,
        fields,
    })
}

fn lower_field(
    rule_name: &str,
    rule_index: usize,
    field_index: usize,
    field: CSharpSyntaxField,
) -> Result<GrammarField, CSharpSyntaxMetadataError> {
    let field_name = non_empty(
        field.name,
        "lower",
        Some(rule_name.to_string()),
        None,
        Some(format!("rules[{rule_index}].fields[{field_index}].name")),
    )?;
    if field.alternatives.is_empty() {
        return Err(CSharpSyntaxMetadataError::new(
            "lower",
            Some(rule_name.to_string()),
            Some(field_name),
            Some(format!(
                "rules[{rule_index}].fields[{field_index}].alternatives"
            )),
            "field has no syntax alternatives",
        ));
    }
    let alternatives = field
        .alternatives
        .into_iter()
        .enumerate()
        .map(|(alternative_index, alternative)| {
            let node_kind = non_empty(
                alternative.node_kind,
                "lower",
                Some(rule_name.to_string()),
                Some(field_name.clone()),
                Some(format!(
                    "rules[{rule_index}].fields[{field_index}].alternatives[{alternative_index}].node_kind"
                )),
            )?;
            Ok(GrammarFieldAlternative {
                node_kind,
                terminal: alternative.terminal,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(GrammarField {
        name: field_name,
        required: field.required,
        multiple: field.multiple,
        alternatives,
    })
}

fn non_empty(
    value: String,
    phase: &'static str,
    rule: Option<String>,
    field: Option<String>,
    path: Option<String>,
) -> Result<String, CSharpSyntaxMetadataError> {
    if value.is_empty() {
        Err(CSharpSyntaxMetadataError::new(
            phase,
            rule,
            field,
            path,
            "value must not be empty",
        ))
    } else {
        Ok(value)
    }
}

#[derive(Debug, Deserialize)]
struct CSharpSyntaxMetadata {
    artifact_kind: String,
    version: u32,
    source: CSharpSyntaxSource,
    parser: CSharpSyntaxParser,
    rules: Vec<CSharpSyntaxRule>,
}

#[derive(Debug, Deserialize)]
struct CSharpSyntaxSource {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct CSharpSyntaxParser {
    tool: String,
    version: String,
    #[serde(default)]
    package: Option<String>,
    #[serde(default)]
    package_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CSharpSyntaxRule {
    name: String,
    node_kind: String,
    #[serde(default)]
    fields: Vec<CSharpSyntaxField>,
}

#[derive(Debug, Deserialize)]
struct CSharpSyntaxField {
    name: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    alternatives: Vec<CSharpSyntaxAlternative>,
}

#[derive(Debug, Deserialize)]
struct CSharpSyntaxAlternative {
    node_kind: String,
    terminal: GrammarTerminalKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpSyntaxMetadataError {
    pub phase: &'static str,
    pub rule: Option<String>,
    pub field: Option<String>,
    pub path: Option<String>,
    pub message: String,
}

impl CSharpSyntaxMetadataError {
    fn new(
        phase: &'static str,
        rule: Option<String>,
        field: Option<String>,
        path: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            rule,
            field,
            path,
            message: message.into(),
        }
    }
}

impl fmt::Display for CSharpSyntaxMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "C# syntax metadata {} error", self.phase)?;
        if let Some(rule) = &self.rule {
            write!(formatter, " in rule {rule}")?;
        }
        if let Some(field) = &self.field {
            write!(formatter, " field {field}")?;
        }
        if let Some(path) = &self.path {
            write!(formatter, " at {path}")?;
        }
        write!(formatter, ": {}", self.message)
    }
}

impl Error for CSharpSyntaxMetadataError {}
