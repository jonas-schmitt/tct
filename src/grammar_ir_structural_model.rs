//! Report-only GrammarIR to StructuralModel lowering.
//!
//! This module is intentionally additive evidence. It maps the minimal
//! `GrammarIR` shape into the same StructuralModel v0 categories used by the
//! GoIR sidecar without changing JSON Schema compilation or generated
//! tokenizer code.

use crate::grammar_ir::{GrammarIr, GrammarIrProvenance, GrammarTerminalKind};
use crate::toy_grammar_frontend::ToyTokenLayout;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const TOOL: &str = "grammar_ir_structural_model_lowering";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_grammar_ir_structural_model_sidecar";
pub const MODEL_VERSION: &str = "structural-model-v0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralTokenLayout {
    pub vocab_size: u32,
    pub pad_token: u32,
    pub ranges: Vec<GrammarStructuralTokenRange>,
    pub integer_value_range: Option<GrammarIntegerValueRange>,
}

impl GrammarStructuralTokenLayout {
    pub fn from_toy_assignment(layout: &ToyTokenLayout) -> Self {
        Self {
            vocab_size: layout.vocab_size,
            pad_token: layout.pad_token,
            ranges: vec![
                GrammarStructuralTokenRange {
                    id: "toy.structural_tokens".to_string(),
                    role: "structural_decisions".to_string(),
                    start: layout.structural_tokens.start,
                    end_exclusive: layout.structural_tokens.end_exclusive,
                },
                GrammarStructuralTokenRange {
                    id: "toy.utf8_byte_payload".to_string(),
                    role: "utf8_byte_payload".to_string(),
                    start: layout.utf8_byte_tokens.start,
                    end_exclusive: layout.utf8_byte_tokens.end_exclusive,
                },
                GrammarStructuralTokenRange {
                    id: "toy.u8_integer_payload".to_string(),
                    role: "u8_integer_payload".to_string(),
                    start: layout.u8_integer_tokens.start,
                    end_exclusive: layout.u8_integer_tokens.end_exclusive,
                },
            ],
            integer_value_range: Some(GrammarIntegerValueRange {
                id: "toy.u8_integer_value".to_string(),
                valid_min: 0,
                valid_max: 255,
            }),
        }
    }

    /// Validate PAD/VOCAB bounds, non-empty ranges, and pairwise non-overlap.
    ///
    /// Same checks `lower_grammar_ir_to_structural_model` applies before lowering.
    /// Used by the I5 shared-alphabet layout constructor tests without wiring the
    /// shared mode into the state machine (checklist tranche 2).
    pub fn validate(&self) -> Result<(), GrammarStructuralModelError> {
        validate_token_layout(self)
    }

    pub fn raw_base_vocab_size(&self) -> u32 {
        self.ranges
            .iter()
            .map(|range| range.end_exclusive)
            .max()
            .unwrap_or(0)
    }

    pub fn reserve_bpe_boundary(
        &self,
        bpe_merges: u32,
    ) -> Result<GrammarStructuralBpeBoundary, GrammarStructuralModelError> {
        let raw_base_vocab_size = self.raw_base_vocab_size();
        if raw_base_vocab_size > self.vocab_size {
            return Err(GrammarStructuralModelError::new(
                "token_layout",
                format!(
                    "raw base vocab {raw_base_vocab_size} exceeds VOCAB_SIZE {}",
                    self.vocab_size
                ),
            ));
        }
        if self.pad_token < raw_base_vocab_size {
            return Err(GrammarStructuralModelError::new(
                "token_layout",
                format!(
                    "PAD_TOKEN {} collides with raw base vocab {}",
                    self.pad_token, raw_base_vocab_size
                ),
            ));
        }
        for range in &self.ranges {
            if (range.start..range.end_exclusive).contains(&self.pad_token) {
                return Err(GrammarStructuralModelError::new(
                    "token_layout",
                    format!(
                        "PAD_TOKEN {} collides with range {}",
                        self.pad_token, range.id
                    ),
                ));
            }
        }

        let bpe_start = raw_base_vocab_size;
        let bpe_end_exclusive = bpe_start.checked_add(bpe_merges).ok_or_else(|| {
            GrammarStructuralModelError::new("token_layout", "BPE range overflows u32")
        })?;
        let current_layout_hosts_bpe =
            bpe_end_exclusive <= self.pad_token && self.pad_token < self.vocab_size;
        let pad_token = if current_layout_hosts_bpe {
            self.pad_token
        } else {
            bpe_end_exclusive
        };
        let vocab_size = if current_layout_hosts_bpe {
            self.vocab_size
        } else {
            pad_token.checked_add(1).ok_or_else(|| {
                GrammarStructuralModelError::new("token_layout", "BPE VOCAB_SIZE overflows u32")
            })?
        };

        Ok(GrammarStructuralBpeBoundary {
            raw_base_vocab_size,
            bpe_start,
            bpe_end_exclusive,
            bpe_merges,
            pad_token,
            vocab_size,
            current_layout_hosts_bpe,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralTokenRange {
    pub id: String,
    pub role: String,
    pub start: u32,
    pub end_exclusive: u32,
}

impl GrammarStructuralTokenRange {
    fn valid_for(&self, vocab_size: u32, pad_token: u32) -> bool {
        self.start < self.end_exclusive
            && self.end_exclusive <= vocab_size
            && !(self.start..self.end_exclusive).contains(&pad_token)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarIntegerValueRange {
    pub id: String,
    pub valid_min: i64,
    pub valid_max: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralBpeBoundary {
    pub raw_base_vocab_size: u32,
    pub bpe_start: u32,
    pub bpe_end_exclusive: u32,
    pub bpe_merges: u32,
    pub pad_token: u32,
    pub vocab_size: u32,
    pub current_layout_hosts_bpe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralModelSidecar {
    pub tool: String,
    pub version: u32,
    pub artifact_kind: String,
    pub model_version: String,
    pub input: GrammarStructuralModelInput,
    pub structural_model: GrammarStructuralModel,
    pub proof: GrammarStructuralModelProof,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralModelInput {
    pub grammar_id: String,
    pub grammar_version: u32,
    pub provenance: GrammarIrProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralModel {
    pub model_id: String,
    pub frontend_id: String,
    pub boundary: GrammarStructuralBoundary,
    pub records: Vec<GrammarStructuralRecord>,
    pub finite_domains: Vec<GrammarFiniteDomain>,
    pub lists: Vec<GrammarStructuralList>,
    pub optionals: Vec<GrammarStructuralOptional>,
    pub bounded_integers: Vec<GrammarStructuralBoundedInteger>,
    pub references: Vec<GrammarStructuralReference>,
    pub payload_leaves: Vec<GrammarStructuralPayload>,
    pub decision_graph: GrammarDecisionGraph,
    pub token_ranges: GrammarStructuralTokenLayout,
    pub renderer_metadata: GrammarRendererMetadata,
    pub validation_hooks: Vec<GrammarValidationHook>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralBoundary {
    pub role: String,
    pub generated_json_schema_output_changed: bool,
    pub core_compiler_path_changed: bool,
    pub production_grammar_compiler_path_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralRecord {
    pub id: String,
    pub model_kind: String,
    pub record_type: String,
    pub grammar_rule: String,
    pub required_fields: Vec<String>,
    pub fields: Vec<GrammarStructuralField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralField {
    pub name: String,
    pub field_kind: String,
    pub required: bool,
    pub multiple: bool,
    pub domain: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarFiniteDomain {
    pub id: String,
    pub model_kind: String,
    pub owner_rule: String,
    pub owner_field: String,
    pub alternatives: Vec<GrammarFiniteAlternative>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarFiniteAlternative {
    pub node_kind: String,
    pub terminal: GrammarTerminalKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralList {
    pub id: String,
    pub model_kind: String,
    pub owner_rule: String,
    pub field: String,
    pub min_items: u32,
    pub item_domain: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralOptional {
    pub id: String,
    pub model_kind: String,
    pub owner_rule: String,
    pub field: String,
    pub default_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralBoundedInteger {
    pub id: String,
    pub model_kind: String,
    pub owner_rule: String,
    pub field: String,
    pub valid_min: i64,
    pub valid_max: i64,
    pub codec: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralReference {
    pub id: String,
    pub model_kind: String,
    pub owner_rule: String,
    pub field: String,
    pub target_rule: String,
    pub target_node_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralPayload {
    pub id: String,
    pub model_kind: String,
    pub owner_rule: String,
    pub field: String,
    pub node_kind: String,
    pub payload_kind: String,
    pub codec: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarDecisionGraph {
    pub nodes: Vec<GrammarDecisionNode>,
    pub edges: Vec<GrammarDecisionEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarDecisionNode {
    pub id: String,
    pub node_kind: String,
    pub owner_rule: String,
    pub owner_field: Option<String>,
    pub decision_kind: String,
    pub terminal: Option<GrammarTerminalKind>,
    pub token_range: Option<String>,
    pub payload_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarDecisionEdge {
    pub source: String,
    pub target: String,
    pub label: String,
    pub crosses_payload_boundary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarRendererMetadata {
    pub id: String,
    pub output_kind: String,
    pub source_text_round_trip_claimed: bool,
    pub external_checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarValidationHook {
    pub id: String,
    pub owner: String,
    pub runs_in_generated_hot_path: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralModelProof {
    pub grammar_shape: GrammarShape,
    pub structural_model_counts: GrammarStructuralCounts,
    pub checks: BTreeMap<String, bool>,
    pub tokenizable_decisions_match: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarShape {
    pub rules: usize,
    pub fields: usize,
    pub alternatives: usize,
    pub required_fields: usize,
    pub repeated_fields: usize,
    pub payload_terminals: usize,
    pub nonterminal_references: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarStructuralCounts {
    pub records: usize,
    pub finite_domains: usize,
    pub lists: usize,
    pub optionals: usize,
    pub bounded_integers: usize,
    pub references: usize,
    pub payload_leaves: usize,
    pub decision_graph_nodes: usize,
    pub decision_graph_edges: usize,
    pub token_ranges: usize,
    pub renderer_metadata: usize,
    pub validation_hooks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarStructuralModelError {
    phase: &'static str,
    message: String,
}

impl GrammarStructuralModelError {
    fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            phase,
            message: message.into(),
        }
    }
}

impl fmt::Display for GrammarStructuralModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} error: {}", self.phase, self.message)
    }
}

impl Error for GrammarStructuralModelError {}

pub fn lower_grammar_ir_to_structural_model(
    grammar: &GrammarIr,
    token_layout: GrammarStructuralTokenLayout,
) -> Result<GrammarStructuralModelSidecar, GrammarStructuralModelError> {
    validate_grammar(grammar)?;
    validate_token_layout(&token_layout)?;
    let known_rules: BTreeMap<_, _> = grammar
        .rules
        .iter()
        .map(|rule| (rule.node_kind.as_str(), rule.name.as_str()))
        .collect();

    let records = collect_records(grammar);
    let finite_domains = collect_finite_domains(grammar);
    let lists = collect_lists(grammar);
    let optionals = collect_optionals(grammar);
    let bounded_integers = collect_bounded_integers(grammar, &token_layout);
    let references = collect_references(grammar, &known_rules)?;
    let payload_leaves = collect_payload_leaves(grammar, &token_layout);
    let decision_graph = build_decision_graph(grammar, &token_layout);
    let renderer_metadata = GrammarRendererMetadata {
        id: format!("{}.renderer_metadata.v0", grammar.grammar_id),
        output_kind: "canonical_toy_assignment_source".to_string(),
        source_text_round_trip_claimed: false,
        external_checks: vec![
            "toy_grammar_frontend parse/render stability stays in tests".to_string(),
            "no external parser, formatter, or toolchain runs in generated hot paths".to_string(),
        ],
    };
    let validation_hooks = vec![
        GrammarValidationHook {
            id: "grammar_ir.required_fields".to_string(),
            owner: "src/grammar_ir.rs".to_string(),
            runs_in_generated_hot_path: false,
        },
        GrammarValidationHook {
            id: "toy_grammar_frontend.strict_decode".to_string(),
            owner: "src/toy_grammar_frontend.rs".to_string(),
            runs_in_generated_hot_path: false,
        },
        GrammarValidationHook {
            id: "grammar_structural_token_ranges".to_string(),
            owner: "src/grammar_ir_structural_model.rs".to_string(),
            runs_in_generated_hot_path: false,
        },
    ];
    let proof = build_proof(ProofInput {
        grammar,
        records: &records,
        finite_domains: &finite_domains,
        lists: &lists,
        optionals: &optionals,
        bounded_integers: &bounded_integers,
        references: &references,
        payload_leaves: &payload_leaves,
        decision_graph: &decision_graph,
        token_layout: &token_layout,
        renderer_metadata: &renderer_metadata,
        validation_hooks: &validation_hooks,
    });
    Ok(GrammarStructuralModelSidecar {
        tool: TOOL.to_string(),
        version: VERSION,
        artifact_kind: ARTIFACT_KIND.to_string(),
        model_version: MODEL_VERSION.to_string(),
        input: GrammarStructuralModelInput {
            grammar_id: grammar.grammar_id.clone(),
            grammar_version: grammar.version,
            provenance: grammar.provenance.clone(),
        },
        structural_model: GrammarStructuralModel {
            model_id: format!("grammar-ir:{}:structural-model-v0", grammar.grammar_id),
            frontend_id: grammar.grammar_id.clone(),
            boundary: GrammarStructuralBoundary {
                role: "sidecar_evidence_only".to_string(),
                generated_json_schema_output_changed: false,
                core_compiler_path_changed: false,
                production_grammar_compiler_path_changed: false,
            },
            records,
            finite_domains,
            lists,
            optionals,
            bounded_integers,
            references,
            payload_leaves,
            decision_graph,
            token_ranges: token_layout,
            renderer_metadata,
            validation_hooks,
        },
        proof,
    })
}

fn validate_grammar(grammar: &GrammarIr) -> Result<(), GrammarStructuralModelError> {
    if grammar.version != crate::grammar_ir::GRAMMAR_IR_VERSION {
        return Err(GrammarStructuralModelError::new(
            "grammar",
            format!("unsupported GrammarIR version {}", grammar.version),
        ));
    }
    if grammar.rules.is_empty() {
        return Err(GrammarStructuralModelError::new(
            "grammar",
            "GrammarIR must contain at least one rule",
        ));
    }
    let mut rule_names = BTreeSet::new();
    let mut node_kinds = BTreeSet::new();
    for rule in &grammar.rules {
        if !rule_names.insert(rule.name.as_str()) {
            return Err(GrammarStructuralModelError::new(
                "grammar",
                format!("duplicate rule name {}", rule.name),
            ));
        }
        if !node_kinds.insert(rule.node_kind.as_str()) {
            return Err(GrammarStructuralModelError::new(
                "grammar",
                format!("duplicate rule node kind {}", rule.node_kind),
            ));
        }
        let mut field_names = BTreeSet::new();
        for field in &rule.fields {
            if !field_names.insert(field.name.as_str()) {
                // GrammarIR shape invariant: one name per field within a rule.
                // Without this, StructuralModel would emit two finite_domains
                // (and related tables) under the same (owner_rule, owner_field),
                // and ModelFacts::alternatives_by_field would silently overwrite
                // the first alternative table and its admissible-token mask.
                return Err(GrammarStructuralModelError::new(
                    "grammar",
                    format!("duplicate field name {} in rule {}", field.name, rule.name),
                ));
            }
            if field.alternatives.is_empty() {
                return Err(GrammarStructuralModelError::new(
                    "grammar",
                    format!(
                        "rule {} field {} has no alternatives",
                        rule.name, field.name
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_token_layout(
    token_layout: &GrammarStructuralTokenLayout,
) -> Result<(), GrammarStructuralModelError> {
    if token_layout.pad_token >= token_layout.vocab_size {
        return Err(GrammarStructuralModelError::new(
            "token_layout",
            "PAD token must be below VOCAB_SIZE",
        ));
    }
    if token_layout.ranges.is_empty() {
        return Err(GrammarStructuralModelError::new(
            "token_layout",
            "at least one token range is required",
        ));
    }
    if let Some(range) = token_layout
        .ranges
        .iter()
        .find(|range| !range.valid_for(token_layout.vocab_size, token_layout.pad_token))
    {
        return Err(GrammarStructuralModelError::new(
            "token_layout",
            format!(
                "range {} is outside VOCAB_SIZE or collides with PAD",
                range.id
            ),
        ));
    }
    let mut range_ids = BTreeSet::new();
    for range in &token_layout.ranges {
        if !range_ids.insert(range.id.as_str()) {
            return Err(GrammarStructuralModelError::new(
                "token_layout",
                format!("duplicate token range id {}", range.id),
            ));
        }
    }
    for (left_index, left) in token_layout.ranges.iter().enumerate() {
        for right in token_layout.ranges.iter().skip(left_index + 1) {
            // Exact coincidence (identical bounds) is intentional multi-role
            // shared payload pooling (SQL structural+shared-pool layout). Only
            // partial overlaps are rejected.
            if ranges_partially_overlap(left, right) {
                return Err(GrammarStructuralModelError::new(
                    "token_layout",
                    format!("token range {} overlaps {}", left.id, right.id),
                ));
            }
        }
    }
    Ok(())
}

fn collect_records(grammar: &GrammarIr) -> Vec<GrammarStructuralRecord> {
    grammar
        .rules
        .iter()
        .map(|rule| {
            let required_fields = rule
                .fields
                .iter()
                .filter(|field| field.required)
                .map(|field| field.name.clone())
                .collect();
            let fields = rule
                .fields
                .iter()
                .map(|field| {
                    let field_kind = if field.multiple {
                        "List"
                    } else if !field.required {
                        "Optional"
                    } else if field.alternatives.len() > 1 {
                        "Variant"
                    } else {
                        field.alternatives[0].terminal.structural_field_kind()
                    };
                    GrammarStructuralField {
                        name: field.name.clone(),
                        field_kind: field_kind.to_string(),
                        required: field.required,
                        multiple: field.multiple,
                        domain: Some(field_domain_id(&rule.name, &field.name)),
                        target: field
                            .alternatives
                            .iter()
                            .find(|alternative| {
                                alternative.terminal == GrammarTerminalKind::Nonterminal
                            })
                            .map(|alternative| format!("grammar.rule.{}", alternative.node_kind)),
                    }
                })
                .collect();
            GrammarStructuralRecord {
                id: format!("grammar.rule.{}", rule.name),
                model_kind: "Record".to_string(),
                record_type: rule.node_kind.clone(),
                grammar_rule: rule.name.clone(),
                required_fields,
                fields,
            }
        })
        .collect()
}

fn collect_finite_domains(grammar: &GrammarIr) -> Vec<GrammarFiniteDomain> {
    grammar
        .rules
        .iter()
        .flat_map(|rule| {
            rule.fields.iter().map(|field| GrammarFiniteDomain {
                id: field_domain_id(&rule.name, &field.name),
                model_kind: if field.alternatives.len() > 1 {
                    "Variant"
                } else {
                    "FiniteDomain"
                }
                .to_string(),
                owner_rule: rule.name.clone(),
                owner_field: field.name.clone(),
                alternatives: field
                    .alternatives
                    .iter()
                    .map(|alternative| GrammarFiniteAlternative {
                        node_kind: alternative.node_kind.clone(),
                        terminal: alternative.terminal,
                    })
                    .collect(),
            })
        })
        .collect()
}

fn collect_lists(grammar: &GrammarIr) -> Vec<GrammarStructuralList> {
    grammar
        .rules
        .iter()
        .flat_map(|rule| {
            rule.fields
                .iter()
                .filter(|field| field.multiple)
                .map(|field| GrammarStructuralList {
                    id: format!("grammar.rule.{}.field.{}.list", rule.name, field.name),
                    model_kind: "List".to_string(),
                    owner_rule: rule.name.clone(),
                    field: field.name.clone(),
                    min_items: u32::from(field.required),
                    item_domain: field_domain_id(&rule.name, &field.name),
                })
        })
        .collect()
}

fn collect_optionals(grammar: &GrammarIr) -> Vec<GrammarStructuralOptional> {
    grammar
        .rules
        .iter()
        .flat_map(|rule| {
            rule.fields
                .iter()
                .filter(|field| !field.required)
                .map(|field| GrammarStructuralOptional {
                    id: format!("grammar.rule.{}.field.{}.optional", rule.name, field.name),
                    model_kind: "Optional".to_string(),
                    owner_rule: rule.name.clone(),
                    field: field.name.clone(),
                    default_policy: "absent_field_omitted_from_render".to_string(),
                })
        })
        .collect()
}

fn collect_bounded_integers(
    grammar: &GrammarIr,
    token_layout: &GrammarStructuralTokenLayout,
) -> Vec<GrammarStructuralBoundedInteger> {
    let Some(range) = &token_layout.integer_value_range else {
        return Vec::new();
    };
    grammar
        .rules
        .iter()
        .flat_map(|rule| {
            rule.fields.iter().flat_map(move |field| {
                field
                    .alternatives
                    .iter()
                    .filter(|alternative| {
                        alternative.terminal == GrammarTerminalKind::IntegerPayload
                    })
                    .map(move |_| GrammarStructuralBoundedInteger {
                        id: format!("grammar.rule.{}.field.{}.integer", rule.name, field.name),
                        model_kind: "BoundedInteger".to_string(),
                        owner_rule: rule.name.clone(),
                        field: field.name.clone(),
                        valid_min: range.valid_min,
                        valid_max: range.valid_max,
                        codec: range.id.clone(),
                    })
            })
        })
        .collect()
}

fn collect_references(
    grammar: &GrammarIr,
    known_rules: &BTreeMap<&str, &str>,
) -> Result<Vec<GrammarStructuralReference>, GrammarStructuralModelError> {
    let mut references = Vec::new();
    for rule in &grammar.rules {
        for field in &rule.fields {
            for alternative in &field.alternatives {
                if alternative.terminal != GrammarTerminalKind::Nonterminal {
                    continue;
                }
                let target_rule =
                    known_rules
                        .get(alternative.node_kind.as_str())
                        .ok_or_else(|| {
                            GrammarStructuralModelError::new(
                                "grammar",
                                format!(
                                    "rule {} field {} references unknown node kind {}",
                                    rule.name, field.name, alternative.node_kind
                                ),
                            )
                        })?;
                references.push(GrammarStructuralReference {
                    id: format!(
                        "grammar.rule.{}.field.{}.ref.{}",
                        rule.name, field.name, alternative.node_kind
                    ),
                    model_kind: "Reference".to_string(),
                    owner_rule: rule.name.clone(),
                    field: field.name.clone(),
                    target_rule: (*target_rule).to_string(),
                    target_node_kind: alternative.node_kind.clone(),
                });
            }
        }
    }
    Ok(references)
}

fn collect_payload_leaves(
    grammar: &GrammarIr,
    token_layout: &GrammarStructuralTokenLayout,
) -> Vec<GrammarStructuralPayload> {
    grammar
        .rules
        .iter()
        .flat_map(|rule| {
            rule.fields.iter().flat_map(|field| {
                field
                    .alternatives
                    .iter()
                    .filter(|alternative| alternative.terminal.is_payload())
                    .map(|alternative| GrammarStructuralPayload {
                        id: format!(
                            "grammar.rule.{}.field.{}.payload.{}",
                            rule.name, field.name, alternative.node_kind
                        ),
                        model_kind: "Payload".to_string(),
                        owner_rule: rule.name.clone(),
                        field: field.name.clone(),
                        node_kind: alternative.node_kind.clone(),
                        payload_kind: alternative.terminal.payload_kind().to_string(),
                        codec: payload_codec_for_terminal(token_layout, alternative.terminal)
                            .to_string(),
                    })
            })
        })
        .collect()
}

fn build_decision_graph(
    grammar: &GrammarIr,
    token_layout: &GrammarStructuralTokenLayout,
) -> GrammarDecisionGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for rule in &grammar.rules {
        let rule_id = decision_rule_id(&rule.name);
        nodes.push(GrammarDecisionNode {
            id: rule_id.clone(),
            node_kind: "rule".to_string(),
            owner_rule: rule.name.clone(),
            owner_field: None,
            decision_kind: "record".to_string(),
            terminal: None,
            token_range: token_range_id_for_role(token_layout, "structural_decisions"),
            payload_kind: None,
        });

        for field in &rule.fields {
            let field_id = decision_field_id(&rule.name, &field.name);
            nodes.push(GrammarDecisionNode {
                id: field_id.clone(),
                node_kind: "field".to_string(),
                owner_rule: rule.name.clone(),
                owner_field: Some(field.name.clone()),
                decision_kind: field_decision_kind(field).to_string(),
                terminal: None,
                token_range: token_range_id_for_role(token_layout, "structural_decisions"),
                payload_kind: None,
            });
            edges.push(GrammarDecisionEdge {
                source: rule_id.clone(),
                target: field_id.clone(),
                label: field.name.clone(),
                crosses_payload_boundary: false,
            });

            for alternative in &field.alternatives {
                let alternative_id =
                    decision_alternative_id(&rule.name, &field.name, &alternative.node_kind);
                let is_payload = alternative.terminal.is_payload();
                nodes.push(GrammarDecisionNode {
                    id: alternative_id.clone(),
                    node_kind: if is_payload {
                        "alternative_payload"
                    } else {
                        "alternative_reference"
                    }
                    .to_string(),
                    owner_rule: rule.name.clone(),
                    owner_field: Some(field.name.clone()),
                    decision_kind: if is_payload { "payload" } else { "reference" }.to_string(),
                    terminal: Some(alternative.terminal),
                    token_range: token_range_id_for_terminal(token_layout, alternative.terminal),
                    payload_kind: is_payload
                        .then(|| alternative.terminal.payload_kind().to_string()),
                });
                edges.push(GrammarDecisionEdge {
                    source: field_id.clone(),
                    target: alternative_id,
                    label: alternative.node_kind.clone(),
                    crosses_payload_boundary: is_payload,
                });
            }
        }
    }

    GrammarDecisionGraph { nodes, edges }
}

struct ProofInput<'a> {
    grammar: &'a GrammarIr,
    records: &'a [GrammarStructuralRecord],
    finite_domains: &'a [GrammarFiniteDomain],
    lists: &'a [GrammarStructuralList],
    optionals: &'a [GrammarStructuralOptional],
    bounded_integers: &'a [GrammarStructuralBoundedInteger],
    references: &'a [GrammarStructuralReference],
    payload_leaves: &'a [GrammarStructuralPayload],
    decision_graph: &'a GrammarDecisionGraph,
    token_layout: &'a GrammarStructuralTokenLayout,
    renderer_metadata: &'a GrammarRendererMetadata,
    validation_hooks: &'a [GrammarValidationHook],
}

fn build_proof(input: ProofInput<'_>) -> GrammarStructuralModelProof {
    let grammar_shape = grammar_shape(input.grammar);
    let counts = GrammarStructuralCounts {
        records: input.records.len(),
        finite_domains: input.finite_domains.len(),
        lists: input.lists.len(),
        optionals: input.optionals.len(),
        bounded_integers: input.bounded_integers.len(),
        references: input.references.len(),
        payload_leaves: input.payload_leaves.len(),
        decision_graph_nodes: input.decision_graph.nodes.len(),
        decision_graph_edges: input.decision_graph.edges.len(),
        token_ranges: input.token_layout.ranges.len(),
        renderer_metadata: usize::from(!input.renderer_metadata.id.is_empty()),
        validation_hooks: input.validation_hooks.len(),
    };
    let mut checks = BTreeMap::new();
    checks.insert(
        "record_count_matches_rules".to_string(),
        counts.records == grammar_shape.rules,
    );
    checks.insert(
        "field_domain_count_matches_fields".to_string(),
        counts.finite_domains == grammar_shape.fields,
    );
    checks.insert(
        "list_count_matches_repeated_fields".to_string(),
        counts.lists == grammar_shape.repeated_fields,
    );
    checks.insert(
        "optional_count_matches_optional_fields".to_string(),
        counts.optionals == grammar_shape.fields - grammar_shape.required_fields,
    );
    checks.insert(
        "payload_count_matches_payload_terminals".to_string(),
        counts.payload_leaves == grammar_shape.payload_terminals,
    );
    checks.insert(
        "reference_count_matches_nonterminals".to_string(),
        counts.references == grammar_shape.nonterminal_references,
    );
    checks.insert(
        "decision_graph_rules_match_records".to_string(),
        count_decision_nodes(input.decision_graph, "rule") == grammar_shape.rules,
    );
    checks.insert(
        "decision_graph_fields_match_fields".to_string(),
        count_decision_nodes(input.decision_graph, "field") == grammar_shape.fields,
    );
    checks.insert(
        "decision_graph_payloads_match_payload_terminals".to_string(),
        count_decision_nodes(input.decision_graph, "alternative_payload")
            == grammar_shape.payload_terminals,
    );
    checks.insert(
        "decision_graph_edges_resolve".to_string(),
        decision_graph_edges_resolve(input.decision_graph),
    );
    checks.insert(
        "decision_graph_token_ranges_known".to_string(),
        decision_graph_token_ranges_known(input.decision_graph, input.token_layout),
    );
    checks.insert(
        "integer_domains_match_integer_payloads".to_string(),
        counts.bounded_integers
            == count_terminals(input.grammar, GrammarTerminalKind::IntegerPayload),
    );
    checks.insert(
        "token_ranges_below_vocab_size".to_string(),
        input.token_layout.ranges.iter().all(|range| {
            range.valid_for(input.token_layout.vocab_size, input.token_layout.pad_token)
        }),
    );
    checks.insert(
        "token_ranges_do_not_overlap".to_string(),
        ranges_do_not_overlap(&input.token_layout.ranges),
    );
    checks.insert(
        "pad_token_below_vocab_size".to_string(),
        input.token_layout.pad_token < input.token_layout.vocab_size,
    );
    checks.insert(
        "renderer_metadata_present".to_string(),
        counts.renderer_metadata == 1,
    );
    checks.insert(
        "validation_hooks_present".to_string(),
        counts.validation_hooks >= 3,
    );
    GrammarStructuralModelProof {
        grammar_shape,
        structural_model_counts: counts,
        tokenizable_decisions_match: checks.values().all(|value| *value),
        checks,
    }
}

fn grammar_shape(grammar: &GrammarIr) -> GrammarShape {
    let fields = grammar.rules.iter().map(|rule| rule.fields.len()).sum();
    let alternatives = grammar
        .rules
        .iter()
        .flat_map(|rule| &rule.fields)
        .map(|field| field.alternatives.len())
        .sum();
    let required_fields = grammar
        .rules
        .iter()
        .flat_map(|rule| &rule.fields)
        .filter(|field| field.required)
        .count();
    let repeated_fields = grammar
        .rules
        .iter()
        .flat_map(|rule| &rule.fields)
        .filter(|field| field.multiple)
        .count();
    GrammarShape {
        rules: grammar.rules.len(),
        fields,
        alternatives,
        required_fields,
        repeated_fields,
        payload_terminals: grammar
            .rules
            .iter()
            .flat_map(|rule| &rule.fields)
            .flat_map(|field| &field.alternatives)
            .filter(|alternative| alternative.terminal.is_payload())
            .count(),
        nonterminal_references: count_terminals(grammar, GrammarTerminalKind::Nonterminal),
    }
}

fn count_terminals(grammar: &GrammarIr, terminal: GrammarTerminalKind) -> usize {
    grammar
        .rules
        .iter()
        .flat_map(|rule| &rule.fields)
        .flat_map(|field| &field.alternatives)
        .filter(|alternative| alternative.terminal == terminal)
        .count()
}

fn field_domain_id(rule_name: &str, field_name: &str) -> String {
    format!("grammar.rule.{rule_name}.field.{field_name}.alternatives")
}

fn decision_rule_id(rule_name: &str) -> String {
    format!("decision.rule.{rule_name}")
}

fn decision_field_id(rule_name: &str, field_name: &str) -> String {
    format!("decision.rule.{rule_name}.field.{field_name}")
}

fn decision_alternative_id(rule_name: &str, field_name: &str, node_kind: &str) -> String {
    format!("decision.rule.{rule_name}.field.{field_name}.alternative.{node_kind}")
}

fn field_decision_kind(field: &crate::grammar_ir::GrammarField) -> &'static str {
    if field.multiple {
        "repetition"
    } else if !field.required {
        "optional"
    } else if field.alternatives.len() > 1 {
        "choice"
    } else if field.alternatives[0].terminal.is_payload() {
        "payload"
    } else {
        "reference"
    }
}

fn token_range_id_for_terminal(
    token_layout: &GrammarStructuralTokenLayout,
    terminal: GrammarTerminalKind,
) -> Option<String> {
    match terminal {
        GrammarTerminalKind::Nonterminal | GrammarTerminalKind::BooleanPayload => {
            token_range_id_for_role(token_layout, "structural_decisions")
        }
        GrammarTerminalKind::IdentifierPayload | GrammarTerminalKind::StringPayload => {
            token_range_id_for_role(token_layout, "utf8_byte_payload")
        }
        GrammarTerminalKind::IntegerPayload => {
            token_range_id_for_role(token_layout, "integer_lexeme_payload")
                .or_else(|| token_range_id_for_role(token_layout, "u8_integer_payload"))
        }
    }
}

fn token_range_id_for_role(
    token_layout: &GrammarStructuralTokenLayout,
    role: &str,
) -> Option<String> {
    token_layout
        .ranges
        .iter()
        .find(|range| range.role == role)
        .map(|range| range.id.clone())
}

fn count_decision_nodes(decision_graph: &GrammarDecisionGraph, node_kind: &str) -> usize {
    decision_graph
        .nodes
        .iter()
        .filter(|node| node.node_kind == node_kind)
        .count()
}

fn decision_graph_edges_resolve(decision_graph: &GrammarDecisionGraph) -> bool {
    let node_ids: BTreeSet<_> = decision_graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect();
    decision_graph.edges.iter().all(|edge| {
        node_ids.contains(edge.source.as_str()) && node_ids.contains(edge.target.as_str())
    })
}

fn decision_graph_token_ranges_known(
    decision_graph: &GrammarDecisionGraph,
    token_layout: &GrammarStructuralTokenLayout,
) -> bool {
    let range_ids: BTreeSet<_> = token_layout
        .ranges
        .iter()
        .map(|range| range.id.as_str())
        .collect();
    decision_graph
        .nodes
        .iter()
        .filter_map(|node| node.token_range.as_deref())
        .all(|range_id| range_ids.contains(range_id))
}

fn ranges_do_not_overlap(ranges: &[GrammarStructuralTokenRange]) -> bool {
    ranges.iter().enumerate().all(|(left_index, left)| {
        ranges
            .iter()
            .skip(left_index + 1)
            .all(|right| !ranges_partially_overlap(left, right))
    })
}

/// True when two ranges partially occupy the same absolute token ids.
///
/// Exact coincidence (`start` and `end_exclusive` identical) is **not** a
/// partial overlap: multiple roles may intentionally share one payload pool
/// (SQL shared payload pool; see `sql_syntax_token_layout`).
fn ranges_partially_overlap(
    left: &GrammarStructuralTokenRange,
    right: &GrammarStructuralTokenRange,
) -> bool {
    if left.start == right.start && left.end_exclusive == right.end_exclusive {
        return false;
    }
    left.start < right.end_exclusive && right.start < left.end_exclusive
}

trait GrammarTerminalKindExt {
    fn is_payload(&self) -> bool;
    fn payload_kind(&self) -> &'static str;
    fn structural_field_kind(&self) -> &'static str;
}

impl GrammarTerminalKindExt for GrammarTerminalKind {
    fn is_payload(&self) -> bool {
        !matches!(self, GrammarTerminalKind::Nonterminal)
    }

    fn payload_kind(&self) -> &'static str {
        match self {
            GrammarTerminalKind::Nonterminal => "nonterminal",
            GrammarTerminalKind::IdentifierPayload => "identifier",
            GrammarTerminalKind::StringPayload => "string",
            GrammarTerminalKind::IntegerPayload => "integer",
            GrammarTerminalKind::BooleanPayload => "boolean",
        }
    }

    fn structural_field_kind(&self) -> &'static str {
        match self {
            GrammarTerminalKind::Nonterminal => "Reference",
            GrammarTerminalKind::IdentifierPayload
            | GrammarTerminalKind::StringPayload
            | GrammarTerminalKind::IntegerPayload
            | GrammarTerminalKind::BooleanPayload => "Payload",
        }
    }
}

fn payload_codec_for_terminal(
    token_layout: &GrammarStructuralTokenLayout,
    terminal: GrammarTerminalKind,
) -> &'static str {
    match terminal {
        GrammarTerminalKind::Nonterminal => "structural_reference",
        GrammarTerminalKind::IdentifierPayload => "utf8_identifier",
        GrammarTerminalKind::StringPayload => "utf8_string",
        GrammarTerminalKind::IntegerPayload => {
            if token_range_id_for_role(token_layout, "integer_lexeme_payload").is_some() {
                "utf8_integer_lexeme"
            } else {
                "u8_integer"
            }
        }
        GrammarTerminalKind::BooleanPayload => "finite_boolean",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar_ir::{GrammarField, GrammarFieldAlternative, GrammarRule};
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    #[test]
    fn validate_grammar_rejects_duplicate_field_name_within_rule() {
        let mut grammar = ToyGrammarSpec::assignment_language_v1().to_grammar_ir();
        // Clone the first field of assignment and re-append under the same name
        // so lowering would otherwise emit two finite_domains for one key.
        let assignment = grammar
            .rules
            .iter_mut()
            .find(|rule| rule.name == "assignment")
            .expect("assignment rule");
        let first = assignment.fields[0].clone();
        assignment.fields.push(GrammarField {
            name: first.name.clone(),
            required: first.required,
            multiple: first.multiple,
            alternatives: vec![GrammarFieldAlternative {
                node_kind: "string".to_string(),
                terminal: GrammarTerminalKind::StringPayload,
            }],
        });

        let layout = GrammarStructuralTokenLayout::from_toy_assignment(
            &ToyGrammarSpec::assignment_language_v1().layout,
        );
        let err = lower_grammar_ir_to_structural_model(&grammar, layout)
            .expect_err("duplicate field names within a rule must fail lowering");
        let message = err.to_string();
        assert!(
            message.contains("duplicate field name"),
            "diagnostic must identify the duplicate-field invariant: {message}"
        );
        assert!(
            message.contains(&first.name) || message.contains("name"),
            "diagnostic must name the field: {message}"
        );
        assert!(
            message.contains("assignment"),
            "diagnostic must name the owner rule: {message}"
        );
    }

    #[test]
    fn validate_grammar_accepts_same_field_name_on_distinct_rules() {
        // Shared field names across rules are legal (e.g. many rules own "name").
        let mut grammar = GrammarIr::new(
            "duplicate_field_cross_rule_ok_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_duplicate_field_probe".to_string(),
                source_path: Some("src/grammar_ir_structural_model.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "left".to_string(),
            node_kind: "left".to_string(),
            fields: vec![GrammarField {
                name: "value".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "integer".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                }],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "right".to_string(),
            node_kind: "right".to_string(),
            fields: vec![GrammarField {
                name: "value".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "integer".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                }],
            }],
        });
        let layout = GrammarStructuralTokenLayout::from_toy_assignment(
            &ToyGrammarSpec::assignment_language_v1().layout,
        );
        lower_grammar_ir_to_structural_model(&grammar, layout)
            .expect("same field name on distinct rules must lower");
    }
}
