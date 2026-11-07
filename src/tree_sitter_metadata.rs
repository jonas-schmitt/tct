//! Tree-sitter-style metadata adapter for GrammarIR spikes.
//!
//! The adapter consumes checked-in `node-types.json` exports (toy fixtures and
//! pinned real grammars). It does not depend on Tree-sitter at runtime.
//! Supported export subset: named nodes, field maps, field-less `children`
//! blocks, `subtypes` supertype lists, and anonymous node kinds that are
//! referenced from named-node alternatives (materialized as leaf rules so
//! multi-grammar pins like C can lower through StructuralModel).
//!
//! Rule identity is keyed on kind name. When the same kind appears as both a
//! named node and an anonymous keyword (JavaScript `class`, `import`), the
//! named node owns the rule and the anonymous form is a terminal literal.
//!
//! Terminal classification is data-driven (CPU-5 T3): see
//! `tree_sitter_terminal_classes` for the default map and optional per-pin
//! `terminal_classes.json` sidecar.

use crate::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarParserProvenance,
    GrammarRule, GrammarTerminalKind,
};
use crate::provenance::file_provenance;
use crate::tree_sitter_terminal_classes::load_terminal_class_table;
pub use crate::tree_sitter_terminal_classes::{TerminalClassTable, default_terminal_class_map};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

/// Synthetic GrammarIR field name for tree-sitter field-less `children` blocks.
pub const CHILDREN_FIELD_NAME: &str = "children";
/// Synthetic GrammarIR field name for tree-sitter `subtypes` (supertype choice).
pub const SUBTYPE_FIELD_NAME: &str = "subtype";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSitterMetadataAdapter {
    grammar_id: String,
}

impl TreeSitterMetadataAdapter {
    pub fn new(grammar_id: impl Into<String>) -> Self {
        Self {
            grammar_id: grammar_id.into(),
        }
    }

    pub fn lower_node_types_file(&self, path: &Path) -> Result<GrammarIr, TreeSitterMetadataError> {
        let contents = std::fs::read_to_string(path).map_err(|error| TreeSitterMetadataError {
            phase: "read",
            node_type: None,
            field: None,
            message: format!("failed to read {}: {error}", path.display()),
        })?;
        let provenance = file_provenance(path).map_err(|error| TreeSitterMetadataError {
            phase: "provenance",
            node_type: None,
            field: None,
            message: error.to_string(),
        })?;
        self.lower_node_types_str(
            &contents,
            GrammarIrProvenance {
                source_kind: "tree_sitter_node_types".to_string(),
                source_path: Some(path.display().to_string()),
                source_sha256: Some(provenance.sha256),
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        )
    }

    /// Lower a pinned grammar directory containing `node-types.json` and
    /// `pin.json` (package, version, license, expected sha256).
    pub fn lower_pinned_grammar_dir(
        &self,
        dir: &Path,
    ) -> Result<(GrammarIr, TreeSitterGrammarPin), TreeSitterMetadataError> {
        let pin_path = dir.join("pin.json");
        let pin_contents =
            std::fs::read_to_string(&pin_path).map_err(|error| TreeSitterMetadataError {
                phase: "read",
                node_type: None,
                field: None,
                message: format!("failed to read {}: {error}", pin_path.display()),
            })?;
        let pin: TreeSitterGrammarPin =
            serde_json::from_str(&pin_contents).map_err(|error| TreeSitterMetadataError {
                phase: "parse",
                node_type: None,
                field: None,
                message: format!("pin.json is invalid: {error}"),
            })?;

        let node_types_path = dir.join(&pin.node_types_file);
        let contents =
            std::fs::read_to_string(&node_types_path).map_err(|error| TreeSitterMetadataError {
                phase: "read",
                node_type: None,
                field: None,
                message: format!("failed to read {}: {error}", node_types_path.display()),
            })?;
        let file_prov =
            file_provenance(&node_types_path).map_err(|error| TreeSitterMetadataError {
                phase: "provenance",
                node_type: None,
                field: None,
                message: error.to_string(),
            })?;
        if file_prov.sha256 != pin.node_types_sha256 {
            return Err(TreeSitterMetadataError {
                phase: "provenance",
                node_type: None,
                field: None,
                message: format!(
                    "node-types sha256 mismatch: pin expects {}, file has {}",
                    pin.node_types_sha256, file_prov.sha256
                ),
            });
        }

        let pin_prov = file_provenance(&pin_path).map_err(|error| TreeSitterMetadataError {
            phase: "provenance",
            node_type: None,
            field: None,
            message: error.to_string(),
        })?;

        let terminal_classes = load_terminal_class_table(
            dir,
            pin.terminal_classes_file.as_deref(),
            pin.terminal_classes_sha256.as_deref(),
        )
        .map_err(|error| TreeSitterMetadataError {
            phase: error.phase,
            node_type: error.node_type,
            field: None,
            message: error.message,
        })?;

        let grammar = self.lower_node_types_str_with_classes(
            &contents,
            GrammarIrProvenance {
                source_kind: "tree_sitter_node_types".to_string(),
                source_path: Some(node_types_path.display().to_string()),
                source_sha256: Some(file_prov.sha256),
                metadata_path: Some(pin_path.display().to_string()),
                metadata_sha256: Some(pin_prov.sha256),
                parser: Some(GrammarParserProvenance {
                    tool: "tree-sitter".to_string(),
                    version: "node-types-export".to_string(),
                    package: Some(pin.grammar_package.clone()),
                    package_version: Some(pin.grammar_version.clone()),
                }),
            },
            &terminal_classes,
        )?;
        Ok((grammar, pin))
    }

    pub fn lower_node_types_str(
        &self,
        contents: &str,
        provenance: GrammarIrProvenance,
    ) -> Result<GrammarIr, TreeSitterMetadataError> {
        self.lower_node_types_str_with_classes(
            contents,
            provenance,
            &TerminalClassTable::with_defaults(),
        )
    }

    pub fn lower_node_types_str_with_classes(
        &self,
        contents: &str,
        provenance: GrammarIrProvenance,
        terminal_classes: &TerminalClassTable,
    ) -> Result<GrammarIr, TreeSitterMetadataError> {
        let node_types: Vec<TreeSitterNodeType> =
            serde_json::from_str(contents).map_err(|error| TreeSitterMetadataError {
                phase: "parse",
                node_type: None,
                field: None,
                message: format!("node-types JSON is invalid: {error}"),
            })?;

        // Named nodes always become rules. Anonymous nodes are kept only when a
        // named node field/children/subtype alternative references them (e.g.
        // C operators like `*`), so StructuralModel nonterminal refs resolve
        // without pulling every punctuation leaf into JSON-sized pins.
        //
        // Rule identity is keyed on kind name. When the same kind appears as
        // both a named node and an anonymous keyword (JS `class`, `import`),
        // prefer the named node and treat the anonymous form as a terminal
        // literal — never emit two rules for one name.
        let named_kinds: BTreeSet<String> = node_types
            .iter()
            .filter(|node| node.named)
            .map(|node| node.kind.clone())
            .collect();
        let referenced_kinds = referenced_node_kinds(node_types.iter().filter(|node| node.named));
        let emitted_nodes: Vec<TreeSitterNodeType> = node_types
            .into_iter()
            .filter(|node| {
                if node.named {
                    true
                } else if named_kinds.contains(&node.kind) {
                    // Named rule owns this kind; skip the anonymous keyword.
                    false
                } else {
                    referenced_kinds.contains(&node.kind)
                }
            })
            .collect();
        let composite_kinds: BTreeSet<String> = emitted_nodes
            .iter()
            .filter(|node| node.is_composite())
            .map(|node| node.kind.clone())
            .collect();

        let mut grammar = GrammarIr::new(self.grammar_id.clone(), provenance);
        for node in emitted_nodes {
            // Structural leaves (named, no fields/children/subtypes) lower as
            // field-less rules: rule identity carries meaning (CPU-5 T2 /
            // design-note step 4.3). Composites keep their field maps.
            let fields = lower_node_fields(&node, &composite_kinds, terminal_classes)?;
            grammar.rules.push(GrammarRule {
                name: node.kind.clone(),
                node_kind: node.kind,
                fields,
            });
        }
        grammar
            .rules
            .sort_by(|left, right| left.name.cmp(&right.name));
        Ok(grammar)
    }
}

fn referenced_node_kinds<'a, I>(nodes: I) -> BTreeSet<String>
where
    I: IntoIterator<Item = &'a TreeSitterNodeType>,
{
    let mut referenced = BTreeSet::new();
    for node in nodes {
        for field in node.fields.values() {
            for alternative in &field.types {
                referenced.insert(alternative.kind.clone());
            }
        }
        if let Some(children) = &node.children {
            for alternative in &children.types {
                referenced.insert(alternative.kind.clone());
            }
        }
        for alternative in &node.subtypes {
            referenced.insert(alternative.kind.clone());
        }
    }
    referenced
}

fn lower_node_fields(
    node: &TreeSitterNodeType,
    composite_kinds: &BTreeSet<String>,
    terminal_classes: &TerminalClassTable,
) -> Result<Vec<GrammarField>, TreeSitterMetadataError> {
    let mut fields = Vec::new();
    for (name, field) in &node.fields {
        if name == CHILDREN_FIELD_NAME || name == SUBTYPE_FIELD_NAME {
            return Err(TreeSitterMetadataError {
                phase: "lower",
                node_type: Some(node.kind.clone()),
                field: Some(name.clone()),
                message: format!("field name collides with synthetic adapter field `{name}`"),
            });
        }
        fields.push(lower_field(
            &node.kind,
            name.clone(),
            field,
            composite_kinds,
            terminal_classes,
        )?);
    }

    if let Some(children) = &node.children {
        if node.fields.contains_key(CHILDREN_FIELD_NAME) {
            return Err(TreeSitterMetadataError {
                phase: "lower",
                node_type: Some(node.kind.clone()),
                field: Some(CHILDREN_FIELD_NAME.to_string()),
                message: "cannot synthesize children field: named field already present"
                    .to_string(),
            });
        }
        fields.push(lower_child_list(
            &node.kind,
            CHILDREN_FIELD_NAME.to_string(),
            children,
            composite_kinds,
            terminal_classes,
        )?);
    }

    if !node.subtypes.is_empty() {
        if node.fields.contains_key(SUBTYPE_FIELD_NAME)
            || fields.iter().any(|field| field.name == SUBTYPE_FIELD_NAME)
        {
            return Err(TreeSitterMetadataError {
                phase: "lower",
                node_type: Some(node.kind.clone()),
                field: Some(SUBTYPE_FIELD_NAME.to_string()),
                message: "cannot synthesize subtype field: field already present".to_string(),
            });
        }
        let synthetic = TreeSitterField {
            multiple: false,
            required: true,
            types: node.subtypes.clone(),
        };
        fields.push(lower_field(
            &node.kind,
            SUBTYPE_FIELD_NAME.to_string(),
            &synthetic,
            composite_kinds,
            terminal_classes,
        )?);
    }

    Ok(fields)
}

fn lower_field(
    owner_node: &str,
    name: String,
    field: &TreeSitterField,
    composite_kinds: &BTreeSet<String>,
    terminal_classes: &TerminalClassTable,
) -> Result<GrammarField, TreeSitterMetadataError> {
    if field.types.is_empty() {
        return Err(TreeSitterMetadataError {
            phase: "lower",
            node_type: Some(owner_node.to_string()),
            field: Some(name),
            message: "field has no type alternatives".to_string(),
        });
    }
    let alternatives = field
        .types
        .iter()
        .map(|alternative| GrammarFieldAlternative {
            terminal: classify_terminal(&alternative.kind, composite_kinds, terminal_classes),
            node_kind: alternative.kind.clone(),
        })
        .collect();
    Ok(GrammarField {
        name,
        required: field.required,
        multiple: field.multiple,
        alternatives,
    })
}

fn lower_child_list(
    owner_node: &str,
    name: String,
    children: &TreeSitterChildren,
    composite_kinds: &BTreeSet<String>,
    terminal_classes: &TerminalClassTable,
) -> Result<GrammarField, TreeSitterMetadataError> {
    if children.types.is_empty() {
        return Err(TreeSitterMetadataError {
            phase: "lower",
            node_type: Some(owner_node.to_string()),
            field: Some(name),
            message: "children block has no type alternatives".to_string(),
        });
    }
    let alternatives = children
        .types
        .iter()
        .map(|alternative| GrammarFieldAlternative {
            terminal: classify_terminal(&alternative.kind, composite_kinds, terminal_classes),
            node_kind: alternative.kind.clone(),
        })
        .collect();
    Ok(GrammarField {
        name,
        required: children.required,
        multiple: children.multiple,
        alternatives,
    })
}

fn classify_terminal(
    node_kind: &str,
    composite_kinds: &BTreeSet<String>,
    terminal_classes: &TerminalClassTable,
) -> GrammarTerminalKind {
    // (a) Structural composites always lower as nonterminals so nested rules
    // remain linked in StructuralModel references — wins over sidecar labels.
    if composite_kinds.contains(node_kind) {
        return GrammarTerminalKind::Nonterminal;
    }
    // (b) Default map + optional pin sidecar for leaves.
    terminal_classes.class_for_leaf(node_kind)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TreeSitterGrammarPin {
    pub pin_id: String,
    pub grammar_package: String,
    pub grammar_version: String,
    pub license: String,
    #[serde(default)]
    pub license_file: Option<String>,
    #[serde(default)]
    pub source_repository: Option<String>,
    #[serde(default)]
    pub source_commit: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    pub node_types_file: String,
    pub node_types_sha256: String,
    /// Optional declarative leaf terminal-class overrides (CPU-5 T3).
    #[serde(default)]
    pub terminal_classes_file: Option<String>,
    /// SHA-256 of `terminal_classes_file` when present (same provenance policy).
    #[serde(default)]
    pub terminal_classes_sha256: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl TreeSitterGrammarPin {
    /// Pinned tree-sitter-json v0.24.8 metadata fixture (first real grammar pin).
    pub fn fixture_dir() -> PathBuf {
        Self::json_fixture_dir()
    }

    pub fn json_fixture_dir() -> PathBuf {
        PathBuf::from("tests/fixtures/tree_sitter_metadata/json")
    }

    /// Pinned tree-sitter-c v0.24.2 metadata fixture (second real grammar pin).
    ///
    /// Metadata + adapter lower only — not a C codec or Go inventory path.
    pub fn c_fixture_dir() -> PathBuf {
        PathBuf::from("tests/fixtures/tree_sitter_metadata/c")
    }

    /// Pinned tree-sitter-javascript v0.25.0 metadata fixture (third real pin).
    ///
    /// Proves `lower_pinned_grammar_dir` is not JSON/C-only. Metadata + adapter
    /// lower only — not a full JavaScript codec claim.
    pub fn javascript_fixture_dir() -> PathBuf {
        PathBuf::from("tests/fixtures/tree_sitter_metadata/javascript")
    }

    /// Held-out tree-sitter-rust v0.24.0 pin for CPU-5 T5 generalization proof.
    ///
    /// Plumbing accessor only — not classification logic. Metadata lower only;
    /// no Rust codec, corpus, or production-route claim.
    pub fn rust_fixture_dir() -> PathBuf {
        PathBuf::from("tests/fixtures/tree_sitter_metadata/rust")
    }
}

#[derive(Debug, Deserialize)]
struct TreeSitterNodeType {
    #[serde(rename = "type")]
    kind: String,
    named: bool,
    #[serde(default)]
    fields: BTreeMap<String, TreeSitterField>,
    #[serde(default)]
    children: Option<TreeSitterChildren>,
    #[serde(default)]
    subtypes: Vec<TreeSitterTypeRef>,
    #[serde(default)]
    #[allow(dead_code)]
    root: bool,
}

impl TreeSitterNodeType {
    fn is_composite(&self) -> bool {
        !self.fields.is_empty() || self.children.is_some() || !self.subtypes.is_empty()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TreeSitterField {
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    types: Vec<TreeSitterTypeRef>,
}

#[derive(Debug, Clone, Deserialize)]
struct TreeSitterChildren {
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    types: Vec<TreeSitterTypeRef>,
}

#[derive(Debug, Clone, Deserialize)]
struct TreeSitterTypeRef {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    named: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSitterMetadataError {
    pub phase: &'static str,
    pub node_type: Option<String>,
    pub field: Option<String>,
    pub message: String,
}

impl fmt::Display for TreeSitterMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} error", self.phase)?;
        if let Some(node_type) = &self.node_type {
            write!(formatter, " in node {node_type}")?;
        }
        if let Some(field) = &self.field {
            write!(formatter, " field {field}")?;
        }
        write!(formatter, ": {}", self.message)
    }
}

impl Error for TreeSitterMetadataError {}
