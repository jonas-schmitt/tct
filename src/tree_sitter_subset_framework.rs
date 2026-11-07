//! Shared tree-sitter construct-subset codec framework (CPU-2 extract).
//!
//! Owns the duplicated pin → subset → StructuralModel → raw SM machinery that
//! both the JSON and C claim-bounded subset codecs previously inlined:
//!
//! - mixed-payload risk inventory helpers
//! - claim-boundary / round-trip report structs
//! - fixture round-trip harness (prefix + complete-stream SM checks)
//! - construct-time multi-alt / alt-token freeze for `child_node_start`
//!
//! Language-specific projection rules, value ASTs, and parse/render stay in
//! `tree_sitter_json_subset_codec` / `tree_sitter_c_subset_codec`. This module
//! is not a production-route registration surface and does not change claim
//! IDs, token layouts, or absolute golden streams.

use crate::grammar_ir::{GrammarIr, GrammarRule, GrammarTerminalKind};
use crate::grammar_ir_structural_model::{
    GrammarFiniteDomain, GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
    lower_grammar_ir_to_structural_model,
};
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxDecodedNode, RawSyntaxError, SyntaxTokenRoles,
};
use crate::syntax_token_layout::ProjectionTokenLayoutSummary;
use crate::tree_sitter_metadata::{TreeSitterGrammarPin, TreeSitterMetadataAdapter};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::Path;

/// One documented full-pin mixed-payload residual that a subset must avoid.
///
/// `payload_kinds` entries are `"node_kind:TerminalVariantName"` strings
/// (e.g. `"number_literal:IntegerPayload"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixedPayloadRisk {
    pub owner_rule: &'static str,
    pub owner_field: &'static str,
    pub payload_kinds: &'static [&'static str],
    pub subset_resolution: &'static str,
}

/// Successful syntax-level subset round-trip report (shared claim-boundary fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSitterSubsetRoundTrip {
    pub source: String,
    pub rendered: String,
    pub raw_token_count: usize,
    pub raw_token_ids: Vec<u32>,
    pub claim_boundary: &'static str,
    pub pin_id: String,
    pub subset_id: &'static str,
}

/// Phase-tagged error for subset inventory / codec / framework work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSitterSubsetError {
    pub phase: &'static str,
    pub message: String,
}

impl TreeSitterSubsetError {
    pub fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            phase,
            message: message.into(),
        }
    }
}

impl fmt::Display for TreeSitterSubsetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} error: {}", self.phase, self.message)
    }
}

impl Error for TreeSitterSubsetError {}

/// Construct-time frozen multi-alt table for child `node_start` selection.
///
/// Built once from `structural_model.finite_domains` so encode never re-scans
/// domains with per-emit string comparisons. Multi-alt Nonterminal choice
/// fields use `node_start_alternative_tokens[node_kind]`; single-alt fields
/// use the plain `node_start` role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenMultiAltNodeStarts {
    /// `(owner_rule, owner_field)` pairs that are multi-alt Nonterminal domains.
    multi_alt_fields: BTreeSet<(String, String)>,
    plain_node_start: u32,
    alt_tokens: BTreeMap<String, u32>,
}

impl FrozenMultiAltNodeStarts {
    /// Freeze multi-alt owner fields and alt-token map at codec construct time.
    pub fn freeze(roles: &SyntaxTokenRoles, finite_domains: &[GrammarFiniteDomain]) -> Self {
        let mut multi_alt_fields = BTreeSet::new();
        for domain in finite_domains {
            let multi = domain.alternatives.len() > 1
                && domain
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal);
            if multi {
                multi_alt_fields.insert((domain.owner_rule.clone(), domain.owner_field.clone()));
            }
        }
        Self {
            multi_alt_fields,
            plain_node_start: roles.node_start,
            alt_tokens: roles.node_start_alternative_tokens.clone(),
        }
    }

    /// Select the child `node_start` token for `owner_rule.owner_field` → `node_kind`.
    pub fn child_node_start(
        &self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
    ) -> Result<u32, TreeSitterSubsetError> {
        if !self.is_multi_alt(owner_rule, owner_field) {
            return Ok(self.plain_node_start);
        }
        self.alt_tokens.get(node_kind).copied().ok_or_else(|| {
            TreeSitterSubsetError::new(
                "encode",
                format!(
                    "missing alternative node_start for {owner_rule}.{owner_field} child {node_kind}"
                ),
            )
        })
    }

    /// True when `(owner_rule, owner_field)` was frozen as multi-alt Nonterminal.
    pub fn is_multi_alt(&self, owner_rule: &str, owner_field: &str) -> bool {
        self.multi_alt_fields
            .iter()
            .any(|(rule, field)| rule == owner_rule && field == owner_field)
    }

    /// Number of frozen multi-alt owner fields (test / inventory aid).
    pub fn multi_alt_field_count(&self) -> usize {
        self.multi_alt_fields.len()
    }
}

/// Compiled subset codec core shared by language modules.
#[derive(Debug, Clone)]
pub struct CompiledSubsetCodecCore {
    pub pin: TreeSitterGrammarPin,
    pub grammar: GrammarIr,
    pub sidecar: GrammarStructuralModelSidecar,
    pub model: CompiledSyntaxModel,
    pub roles: SyntaxTokenRoles,
    /// Construct-time multi-alt freeze; use for all encode `child_node_start` calls.
    pub multi_alt: FrozenMultiAltNodeStarts,
}

impl CompiledSubsetCodecCore {
    /// Lower a pin, project the language subset, compile the raw SM, freeze multi-alt.
    pub fn from_pinned_dir(
        dir: &Path,
        subset_grammar_id: &str,
        project: impl FnOnce(&GrammarIr) -> Result<GrammarIr, TreeSitterSubsetError>,
        layout: GrammarStructuralTokenLayout,
    ) -> Result<Self, TreeSitterSubsetError> {
        let adapter = TreeSitterMetadataAdapter::new(subset_grammar_id);
        let (full_grammar, pin) = adapter
            .lower_pinned_grammar_dir(dir)
            .map_err(|error| TreeSitterSubsetError::new("lower", error.to_string()))?;
        let grammar = project(&full_grammar)?;
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, layout)
            .map_err(|error| TreeSitterSubsetError::new("structural_model", error.to_string()))?;
        if sidecar.structural_model.boundary.core_compiler_path_changed {
            return Err(TreeSitterSubsetError::new(
                "boundary",
                "subset lowering must not mark core_compiler_path_changed",
            ));
        }
        let token_layout =
            ProjectionTokenLayoutSummary::from(&sidecar.structural_model.token_ranges);
        let model = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &token_layout)
            .map_err(|error| TreeSitterSubsetError::new("state_machine", error.to_string()))?;
        let roles = model.token_roles().clone();
        let multi_alt =
            FrozenMultiAltNodeStarts::freeze(&roles, &sidecar.structural_model.finite_domains);
        Ok(Self {
            pin,
            grammar,
            sidecar,
            model,
            roles,
            multi_alt,
        })
    }

    /// Encode a model-conforming decoded-node tree via the generic raw-SM
    /// encoder, then apply the same token and stream checks hand-written emit
    /// paths use today (`validate_encode_tokens` +
    /// `validate_prefix_and_complete_stream`).
    ///
    /// Language modules lower their value AST to [`RawSyntaxDecodedNode`] and
    /// call this (see `docs/generic-codec-layer-contract.md` migration plan).
    /// Does not parse or render; structure walk is entirely generic.
    pub fn encode_decoded_tree(
        &self,
        root: &RawSyntaxDecodedNode,
    ) -> Result<Vec<u32>, TreeSitterSubsetError> {
        let tokens = self
            .model
            .encode_complete_tree(root)
            .map_err(map_raw_syntax_encode_error)?;
        self.validate_encode_tokens(&tokens)?;
        self.validate_prefix_and_complete_stream(&tokens)?;
        Ok(tokens)
    }

    /// Reject encode tokens that collide with PAD or leave the vocabulary.
    pub fn validate_encode_tokens(&self, tokens: &[u32]) -> Result<(), TreeSitterSubsetError> {
        for token in tokens {
            if *token >= self.roles.pad_token {
                return Err(TreeSitterSubsetError::new(
                    "encode",
                    format!("token {token} collides with PAD or exceeds vocabulary"),
                ));
            }
            if *token >= self.roles.vocab_size {
                return Err(TreeSitterSubsetError::new(
                    "encode",
                    format!(
                        "token {token} is outside VOCAB_SIZE {}",
                        self.roles.vocab_size
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Empty-prefix root `node_start` admission + complete-stream no-further-tokens.
    pub fn validate_prefix_and_complete_stream(
        &self,
        tokens: &[u32],
    ) -> Result<(), TreeSitterSubsetError> {
        let empty = self
            .model
            .state_from_prefix(&[])
            .map_err(|error| TreeSitterSubsetError::new("raw_sm", format!("{error:?}")))?;
        if empty.admissible.finite_raw_token_ids != vec![self.roles.node_start] {
            return Err(TreeSitterSubsetError::new(
                "raw_sm",
                format!(
                    "empty prefix admissible tokens {:?}, expected [{:?}]",
                    empty.admissible.finite_raw_token_ids, self.roles.node_start
                ),
            ));
        }
        let complete = self
            .model
            .decode_complete(tokens)
            .map_err(|error| TreeSitterSubsetError::new("raw_sm", format!("{error:?}")))?;
        if !complete.admissible.finite_raw_token_ids.is_empty() {
            return Err(TreeSitterSubsetError::new(
                "raw_sm",
                "complete stream must admit no further raw tokens",
            ));
        }
        Ok(())
    }

    /// Build the shared claim-boundary round-trip report.
    pub fn round_trip_report(
        &self,
        source: &str,
        rendered: String,
        tokens: Vec<u32>,
        claim_boundary: &'static str,
        subset_id: &'static str,
    ) -> TreeSitterSubsetRoundTrip {
        TreeSitterSubsetRoundTrip {
            source: source.to_string(),
            rendered,
            raw_token_count: tokens.len(),
            raw_token_ids: tokens,
            claim_boundary,
            pin_id: self.pin.pin_id.clone(),
            subset_id,
        }
    }

    /// Fixture round-trip harness: parse → encode → decode equality → render →
    /// reparse equality → re-encode equality → raw SM prefix/complete checks.
    #[allow(clippy::too_many_arguments)]
    pub fn run_round_trip_source<T>(
        &self,
        source: &str,
        claim_boundary: &'static str,
        subset_id: &'static str,
        parse: impl Fn(&str) -> Result<T, TreeSitterSubsetError>,
        encode: impl Fn(&T) -> Result<Vec<u32>, TreeSitterSubsetError>,
        decode: impl Fn(&[u32]) -> Result<T, TreeSitterSubsetError>,
        render: impl Fn(&T) -> String,
        value_label: &str,
    ) -> Result<TreeSitterSubsetRoundTrip, TreeSitterSubsetError>
    where
        T: PartialEq + fmt::Debug,
    {
        let value = parse(source)?;
        let tokens = encode(&value)?;
        let decoded = decode(&tokens)?;
        if decoded != value {
            return Err(TreeSitterSubsetError::new(
                "round_trip",
                format!(
                    "decoded {value_label} {decoded:?} diverged from parsed {value_label} {value:?}"
                ),
            ));
        }
        let rendered = render(&decoded);
        let reparsed = parse(&rendered)?;
        if reparsed != value {
            return Err(TreeSitterSubsetError::new(
                "round_trip",
                format!(
                    "reparsed {value_label} {reparsed:?} diverged from original {value_label} {value:?}"
                ),
            ));
        }
        let reencoded = encode(&reparsed)?;
        if reencoded != tokens {
            return Err(TreeSitterSubsetError::new(
                "round_trip",
                "re-encoded tokens diverged from the first encoding",
            ));
        }
        self.validate_prefix_and_complete_stream(&tokens)?;
        Ok(self.round_trip_report(source, rendered, tokens, claim_boundary, subset_id))
    }

    /// Emit an IntegerPayload under the frozen integer range role.
    ///
    /// Uses the shared variable-width escape multi-digit codec on the 256-wide
    /// `u8_integer_payload` band (`0..=253` single token; larger values use
    /// markers 254/255). Values above `16_777_215` fail closed.
    pub fn emit_u8_integer_payload(
        &self,
        tokens: &mut Vec<u32>,
        value: u8,
    ) -> Result<(), TreeSitterSubsetError> {
        self.emit_integer_payload_u32(tokens, u32::from(value))
    }

    /// Emit an integer payload for values up to the multi-digit capacity.
    pub fn emit_integer_payload_u32(
        &self,
        tokens: &mut Vec<u32>,
        value: u32,
    ) -> Result<(), TreeSitterSubsetError> {
        if self.roles.integer_range.role != "u8_integer_payload" {
            return Err(TreeSitterSubsetError::new(
                "encode",
                format!(
                    "subset number encode requires u8_integer_payload layout, got {}",
                    self.roles.integer_range.role
                ),
            ));
        }
        tokens.push(self.roles.terminal);
        let digits = crate::syntax_state_machine::integer_codec_encode_digits(value)
            .map_err(|msg| TreeSitterSubsetError::new("encode", msg))?;
        for digit in digits {
            let token = self
                .roles
                .integer_range
                .start
                .checked_add(u32::from(digit))
                .ok_or_else(|| {
                    TreeSitterSubsetError::new("encode", "integer payload token overflow")
                })?;
            if token >= self.roles.integer_range.end_exclusive {
                return Err(TreeSitterSubsetError::new(
                    "encode",
                    format!("integer payload token {token} outside range"),
                ));
            }
            tokens.push(token);
        }
        tokens.push(self.roles.payload_end);
        Ok(())
    }

    /// Emit UTF-8 byte payload tokens for an ASCII or general string content.
    pub fn emit_utf8_payload(
        &self,
        tokens: &mut Vec<u32>,
        content: &str,
    ) -> Result<(), TreeSitterSubsetError> {
        tokens.push(self.roles.terminal);
        for byte in content.as_bytes() {
            let token = self
                .roles
                .utf8_range
                .start
                .checked_add(u32::from(*byte))
                .ok_or_else(|| {
                    TreeSitterSubsetError::new("encode", "utf8 payload token overflow")
                })?;
            if token >= self.roles.utf8_range.end_exclusive {
                return Err(TreeSitterSubsetError::new(
                    "encode",
                    format!("utf8 payload token {token} outside range"),
                ));
            }
            tokens.push(token);
        }
        tokens.push(self.roles.payload_end);
        Ok(())
    }
}

/// Look up a pin-derived rule; missing rules become a typed subset error.
pub fn require_rule<'a>(
    full: &'a GrammarIr,
    name: &str,
    phase: &'static str,
) -> Result<&'a GrammarRule, TreeSitterSubsetError> {
    full.rule(name).ok_or_else(|| {
        TreeSitterSubsetError::new(
            phase,
            format!("pinned grammar is missing required rule {name}"),
        )
    })
}

/// Summarize `(node_kind, terminal)` alternatives on a named field.
pub fn field_terminal_summary(
    full: &GrammarIr,
    rule: &str,
    field: &str,
) -> Result<Vec<(String, GrammarTerminalKind)>, TreeSitterSubsetError> {
    let rule_ir = require_rule(full, rule, "inventory")?;
    let field_ir = rule_ir
        .fields
        .iter()
        .find(|candidate| candidate.name == field)
        .ok_or_else(|| {
            TreeSitterSubsetError::new("inventory", format!("rule {rule} is missing field {field}"))
        })?;
    Ok(field_ir
        .alternatives
        .iter()
        .map(|alternative| (alternative.node_kind.clone(), alternative.terminal))
        .collect())
}

/// Observe documented mixed-payload risks on a full pin GrammarIR.
///
/// Each risk must still present Nonterminal + payload alternatives on the
/// named field, and each declared `payload_kinds` entry must match.
pub fn observe_mixed_payload_risks(
    full: &GrammarIr,
    risks: &[MixedPayloadRisk],
) -> Result<Vec<String>, TreeSitterSubsetError> {
    let mut observed = Vec::with_capacity(risks.len());
    for risk in risks {
        let terminals = field_terminal_summary(full, risk.owner_rule, risk.owner_field)?;
        let has_nonterminal = terminals
            .iter()
            .any(|(_, terminal)| *terminal == GrammarTerminalKind::Nonterminal);
        let has_payload = terminals.iter().any(|(_, terminal)| {
            matches!(
                terminal,
                GrammarTerminalKind::IntegerPayload
                    | GrammarTerminalKind::IdentifierPayload
                    | GrammarTerminalKind::BooleanPayload
                    | GrammarTerminalKind::StringPayload
            )
        });
        if !(has_nonterminal && has_payload) {
            return Err(TreeSitterSubsetError::new(
                "inventory",
                format!(
                    "expected mixed-payload risk on {}.{} (nonterminal+payload), got {terminals:?}",
                    risk.owner_rule, risk.owner_field
                ),
            ));
        }
        for expected in risk.payload_kinds {
            let (kind, term_name) = expected.split_once(':').ok_or_else(|| {
                TreeSitterSubsetError::new(
                    "inventory",
                    format!("malformed mixed-payload risk entry {expected}"),
                )
            })?;
            let found = terminals.iter().any(|(node_kind, terminal)| {
                node_kind == kind && format!("{terminal:?}").contains(term_name)
            });
            if !found {
                return Err(TreeSitterSubsetError::new(
                    "inventory",
                    format!(
                        "mixed-payload risk {}.{} missing {expected}; got {terminals:?}",
                        risk.owner_rule, risk.owner_field
                    ),
                ));
            }
        }
        observed.push(format!(
            "{}.{} → resolve by {}",
            risk.owner_rule, risk.owner_field, risk.subset_resolution
        ));
    }
    Ok(observed)
}

/// Map a generic-encoder [`RawSyntaxError`] into the framework error type.
///
/// Phase is the encoder's `kind`. The message keeps the encoder text and
/// appends active rule/field (and related) context so rule / field / node-kind
/// detail is never dropped when only the separate fields carried it.
fn map_raw_syntax_encode_error(error: RawSyntaxError) -> TreeSitterSubsetError {
    let mut message = error.message;
    let mut context = Vec::new();
    if let Some(rule) = error.active_rule {
        context.push(format!("active_rule={rule}"));
    }
    if let Some(field) = error.active_field {
        context.push(format!("active_field={field}"));
    }
    if let Some(class) = error.expected_payload_class {
        context.push(format!("expected_payload_class={class}"));
    }
    if error.consumed_raw_tokens > 0 {
        context.push(format!("consumed_raw_tokens={}", error.consumed_raw_tokens));
    }
    if let Some(token) = error.failing_token {
        context.push(format!("failing_token={token}"));
    }
    if !context.is_empty() {
        message = format!("{message} [{}]", context.join(", "));
    }
    TreeSitterSubsetError::new(error.kind, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar_ir::{GrammarIr, GrammarIrProvenance, GrammarRule};
    use crate::grammar_ir_structural_model::{
        GrammarFiniteAlternative, GrammarStructuralTokenLayout,
        lower_grammar_ir_to_structural_model,
    };
    use crate::syntax_token_layout::ProjectionTokenLayoutRangeSummary;
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    fn test_roles(alt_tokens: BTreeMap<String, u32>) -> SyntaxTokenRoles {
        let structural = ProjectionTokenLayoutRangeSummary {
            id: "structural".to_string(),
            role: "structural_decisions".to_string(),
            start: 0,
            end_exclusive: 64,
        };
        let utf8 = ProjectionTokenLayoutRangeSummary {
            id: "utf8".to_string(),
            role: "utf8_byte_payload".to_string(),
            start: 64,
            end_exclusive: 320,
        };
        let integer = ProjectionTokenLayoutRangeSummary {
            id: "u8".to_string(),
            role: "u8_integer_payload".to_string(),
            start: 320,
            end_exclusive: 576,
        };
        SyntaxTokenRoles {
            vocab_size: 577,
            pad_token: 576,
            structural_range: structural,
            utf8_range: utf8,
            integer_range: integer,
            menu_range: None,
            node_start: 1,
            node_end: 2,
            field_start: 3,
            field_end: 4,
            field_missing: 5,
            repeated_empty: 6,
            terminal: 7,
            payload_end: 8,
            boolean_false: 9,
            boolean_true: 10,
            node_start_alternative_tokens: alt_tokens,
            field_choice_table: BTreeMap::new(),
            layout_mode: crate::shared_alphabet_layout::SyntaxTokenLayoutMode::PerGrammar,
        }
    }

    #[test]
    fn frozen_multi_alt_selects_plain_or_alt_token_without_domain_rescan() {
        let mut alt_tokens = BTreeMap::new();
        alt_tokens.insert("function_declarator".to_string(), 20);
        alt_tokens.insert("identifier".to_string(), 21);
        let roles = test_roles(alt_tokens);
        let domains = vec![
            GrammarFiniteDomain {
                id: "d1".to_string(),
                model_kind: "choice".to_string(),
                owner_rule: "_declarator".to_string(),
                owner_field: "subtype".to_string(),
                alternatives: vec![
                    GrammarFiniteAlternative {
                        node_kind: "function_declarator".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFiniteAlternative {
                        node_kind: "identifier".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                ],
            },
            GrammarFiniteDomain {
                id: "d2".to_string(),
                model_kind: "choice".to_string(),
                owner_rule: "statement".to_string(),
                owner_field: "subtype".to_string(),
                alternatives: vec![GrammarFiniteAlternative {
                    node_kind: "return_statement".to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            },
        ];
        let frozen = FrozenMultiAltNodeStarts::freeze(&roles, &domains);
        assert_eq!(frozen.multi_alt_field_count(), 1);
        assert!(frozen.is_multi_alt("_declarator", "subtype"));
        assert!(!frozen.is_multi_alt("statement", "subtype"));
        assert_eq!(
            frozen
                .child_node_start("_declarator", "subtype", "function_declarator")
                .unwrap(),
            20
        );
        assert_eq!(
            frozen
                .child_node_start("_declarator", "subtype", "identifier")
                .unwrap(),
            21
        );
        assert_eq!(
            frozen
                .child_node_start("statement", "subtype", "return_statement")
                .unwrap(),
            1
        );
        assert!(
            frozen
                .child_node_start("_declarator", "subtype", "missing")
                .is_err()
        );
    }

    fn field_less_leaf_core() -> CompiledSubsetCodecCore {
        let mut grammar = GrammarIr::new(
            "subset_framework_encode_decoded_tree_leaf_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_subset_framework".to_string(),
                source_path: Some("src/tree_sitter_subset_framework.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "leaf".to_string(),
            node_kind: "leaf".to_string(),
            fields: vec![],
        });
        let layout = GrammarStructuralTokenLayout::from_toy_assignment(
            &ToyGrammarSpec::assignment_language_v1().layout,
        );
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, layout)
            .expect("synthetic leaf grammar should lower");
        let model = CompiledSyntaxModel::from_sidecar(&sidecar).expect("model should compile");
        let roles = model.token_roles().clone();
        let multi_alt =
            FrozenMultiAltNodeStarts::freeze(&roles, &sidecar.structural_model.finite_domains);
        CompiledSubsetCodecCore {
            pin: TreeSitterGrammarPin {
                pin_id: "synthetic_subset_framework_leaf_v0".to_string(),
                grammar_package: "synthetic".to_string(),
                grammar_version: "0".to_string(),
                license: "test".to_string(),
                license_file: None,
                source_repository: None,
                source_commit: None,
                source_url: None,
                node_types_file: "synthetic".to_string(),
                node_types_sha256: "0".repeat(64),
                terminal_classes_file: None,
                terminal_classes_sha256: None,
                notes: vec![],
            },
            grammar,
            sidecar,
            model,
            roles,
            multi_alt,
        }
    }

    #[test]
    fn encode_decoded_tree_field_less_leaf_validates_stream() {
        let core = field_less_leaf_core();
        let tree = RawSyntaxDecodedNode {
            rule_name: "leaf".to_string(),
            node_kind: "leaf".to_string(),
            fields: vec![],
        };
        let tokens = core
            .encode_decoded_tree(&tree)
            .expect("wrapper encode of field-less leaf");
        assert_eq!(tokens, vec![core.roles.node_start, core.roles.node_end]);

        // Inverse: decode then re-encode through the wrapper.
        let decoded = core
            .model
            .decode_complete_tree(&tokens)
            .expect("decode")
            .decoded_node_tree()
            .expect("tree");
        assert_eq!(decoded, tree);
        let reencoded = core.encode_decoded_tree(&decoded).expect("re-encode");
        assert_eq!(reencoded, tokens);
    }

    #[test]
    fn encode_decoded_tree_maps_raw_syntax_error_phase_and_detail() {
        let core = field_less_leaf_core();
        let err = core
            .encode_decoded_tree(&RawSyntaxDecodedNode {
                rule_name: "other".to_string(),
                node_kind: "leaf".to_string(),
                fields: vec![],
            })
            .expect_err("root mismatch must fail closed");
        assert_eq!(err.phase, "encode_root_mismatch");
        assert!(
            err.message.contains("other"),
            "message must preserve rule detail: {}",
            err.message
        );
        assert!(
            err.message.contains("active_rule=other") || err.message.contains("other"),
            "rule context must not be swallowed: {}",
            err.message
        );
    }

    #[test]
    fn map_raw_syntax_encode_error_preserves_kind_rule_and_field() {
        let mapped = map_raw_syntax_encode_error(RawSyntaxError {
            kind: "encode_unknown_field",
            message: "field z is not part of record r (node_kind k)".to_string(),
            consumed_raw_tokens: 3,
            failing_token: None,
            stack_depth: 0,
            active_rule: Some("r".to_string()),
            active_field: Some("z".to_string()),
            expected_payload_class: None,
        });
        assert_eq!(mapped.phase, "encode_unknown_field");
        assert!(mapped.message.contains("field z"));
        assert!(mapped.message.contains("active_rule=r"));
        assert!(mapped.message.contains("active_field=z"));
        assert!(mapped.message.contains("consumed_raw_tokens=3"));
    }
}
