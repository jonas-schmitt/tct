//! Minimal normalized grammar IR for frontend metadata probes.
//!
//! This is intentionally not part of the production JSON Schema compiler path.
//! It gives toy and metadata adapters a typed target before any broader
//! Tree-sitter, ANTLR, or language-specific frontend refactor.

use serde::{Deserialize, Serialize};

pub const GRAMMAR_IR_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarIr {
    pub version: u32,
    pub grammar_id: String,
    pub provenance: GrammarIrProvenance,
    pub rules: Vec<GrammarRule>,
}

impl GrammarIr {
    pub fn new(grammar_id: impl Into<String>, provenance: GrammarIrProvenance) -> Self {
        Self {
            version: GRAMMAR_IR_VERSION,
            grammar_id: grammar_id.into(),
            provenance,
            rules: Vec::new(),
        }
    }

    pub fn rule(&self, name: &str) -> Option<&GrammarRule> {
        self.rules.iter().find(|rule| rule.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarIrProvenance {
    pub source_kind: String,
    pub source_path: Option<String>,
    pub source_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parser: Option<GrammarParserProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarParserProvenance {
    pub tool: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarRule {
    pub name: String,
    pub node_kind: String,
    pub fields: Vec<GrammarField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarField {
    pub name: String,
    pub required: bool,
    pub multiple: bool,
    pub alternatives: Vec<GrammarFieldAlternative>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarFieldAlternative {
    pub node_kind: String,
    pub terminal: GrammarTerminalKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrammarTerminalKind {
    Nonterminal,
    IdentifierPayload,
    StringPayload,
    IntegerPayload,
    BooleanPayload,
}
