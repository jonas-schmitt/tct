use crate::grammar_ir::GrammarTerminalKind;
use crate::grammar_ir_structural_model::{
    GrammarStructuralField, GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelFacts {
    pub(super) root_rule: String,
    pub(super) records: BTreeMap<String, RecordSpec>,
    pub(super) references: BTreeSet<(String, String, String)>,
    pub(super) children_by_field: BTreeMap<(String, String), Vec<ChildSpec>>,
    pub(super) payloads_by_field: BTreeMap<(String, String), Vec<PayloadSpec>>,
    pub(super) multi_alternative_nonterminal_fields: BTreeSet<(String, String)>,
    /// Target-neutral per-field alternative bodies derived from StructuralModel.
    ///
    /// One entry per finite-domain alternative, distinguishing child-rule
    /// descent from zero-width boolean, bounded integer, and UTF-8 payload
    /// bodies. Mixed fields (Nonterminal + any payload) are also listed in
    /// [`Self::mixed_payload_fields`].
    pub(super) alternatives_by_field: BTreeMap<(String, String), Vec<DiscriminatedAlternative>>,
    /// Fields whose alternatives mix Nonterminal with at least one payload class.
    pub(super) mixed_payload_fields: BTreeSet<(String, String)>,
}

/// Compile-time body kind for one field alternative after a structural choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AlternativeBodyKind {
    /// Descend into the selected child rule (Nonterminal).
    ChildRuleDescent,
    /// Choice alone completes a BooleanPayload (zero payload tokens).
    BooleanPayload,
    /// Open the integer payload substate (bounded u8 or lexeme).
    IntegerPayload,
    /// Open the UTF-8 string/identifier payload substate.
    Utf8Payload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscriminatedAlternative {
    pub(super) node_kind: String,
    pub(super) terminal: GrammarTerminalKind,
    pub(super) body_kind: AlternativeBodyKind,
    /// Present for Nonterminal alternatives; target record rule name.
    pub(super) target_rule: Option<String>,
    /// Present for payload alternatives; StructuralModel payload_kind string.
    pub(super) payload_kind: Option<String>,
}

impl ModelFacts {
    pub(super) fn from_sidecar(
        sidecar: &GrammarStructuralModelSidecar,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let root_rule = sidecar
            .structural_model
            .records
            .first()
            .map(|record| record.grammar_rule.clone())
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.model.records",
                    "StructuralModel must contain at least one record",
                )
            })?;
        let records = sidecar
            .structural_model
            .records
            .iter()
            .map(|record| {
                (
                    record.grammar_rule.clone(),
                    RecordSpec {
                        rule: record.grammar_rule.clone(),
                        node_kind: record.record_type.clone(),
                        fields: record.fields.iter().map(FieldSpec::from).collect(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let references = sidecar
            .structural_model
            .references
            .iter()
            .map(|reference| {
                (
                    reference.owner_rule.clone(),
                    reference.field.clone(),
                    reference.target_node_kind.clone(),
                )
            })
            .collect::<BTreeSet<_>>();
        let mut children_by_field: BTreeMap<(String, String), Vec<ChildSpec>> = BTreeMap::new();
        for reference in &sidecar.structural_model.references {
            children_by_field
                .entry((reference.owner_rule.clone(), reference.field.clone()))
                .or_default()
                .push(ChildSpec {
                    node_kind: reference.target_node_kind.clone(),
                    target_rule: reference.target_rule.clone(),
                });
        }
        for children in children_by_field.values_mut() {
            children.sort_by(|left, right| left.node_kind.cmp(&right.node_kind));
            children.dedup_by(|left, right| left.node_kind == right.node_kind);
        }

        let mut payloads_by_field: BTreeMap<(String, String), Vec<PayloadSpec>> = BTreeMap::new();
        for payload in &sidecar.structural_model.payload_leaves {
            payloads_by_field
                .entry((payload.owner_rule.clone(), payload.field.clone()))
                .or_default()
                .push(PayloadSpec {
                    node_kind: payload.node_kind.clone(),
                    payload_kind: payload.payload_kind.clone(),
                });
        }
        for payloads in payloads_by_field.values_mut() {
            payloads.sort_by(|left, right| {
                left.node_kind
                    .cmp(&right.node_kind)
                    .then(left.payload_kind.cmp(&right.payload_kind))
            });
            payloads.dedup();
        }

        let multi_alternative_nonterminal_fields = sidecar
            .structural_model
            .finite_domains
            .iter()
            .filter(|domain| domain.alternatives.len() > 1)
            .filter(|domain| {
                domain
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal)
            })
            .map(|domain| (domain.owner_rule.clone(), domain.owner_field.clone()))
            .collect();

        let mut alternatives_by_field: BTreeMap<(String, String), Vec<DiscriminatedAlternative>> =
            BTreeMap::new();
        let mut mixed_payload_fields = BTreeSet::new();

        for domain in &sidecar.structural_model.finite_domains {
            let field_key = (domain.owner_rule.clone(), domain.owner_field.clone());
            let children = children_by_field
                .get(&field_key)
                .cloned()
                .unwrap_or_default();
            let payloads = payloads_by_field
                .get(&field_key)
                .cloned()
                .unwrap_or_default();
            // GrammarTerminalKind is not Ord; track identities via body-kind class
            // strings that still distinguish Nonterminal from each payload class.
            let mut seen_identities: BTreeSet<(String, &'static str)> = BTreeSet::new();
            let mut node_kind_to_class: BTreeMap<String, &'static str> = BTreeMap::new();
            let mut alts = Vec::with_capacity(domain.alternatives.len());

            for alternative in &domain.alternatives {
                let class_name = terminal_class_name(alternative.terminal);
                let identity = (alternative.node_kind.clone(), class_name);
                if !seen_identities.insert(identity) {
                    return Err(ambiguous_alternative_error(
                        &domain.owner_rule,
                        &domain.owner_field,
                        &alternative.node_kind,
                        alternative.terminal,
                        "duplicate alternative identity on the same field",
                    ));
                }
                if let Some(existing) = node_kind_to_class.get(&alternative.node_kind) {
                    if *existing != class_name {
                        return Err(ambiguous_alternative_error(
                            &domain.owner_rule,
                            &domain.owner_field,
                            &alternative.node_kind,
                            alternative.terminal,
                            &format!(
                                "node_kind already bound to terminal class {existing}; mixed rule-identity and payload for the same node_kind is forbidden"
                            ),
                        ));
                    }
                } else {
                    node_kind_to_class.insert(alternative.node_kind.clone(), class_name);
                }

                let body_kind = body_kind_for_terminal(alternative.terminal);
                let (target_rule, payload_kind) = match alternative.terminal {
                    GrammarTerminalKind::Nonterminal => {
                        let child = children.iter().find(|child| {
                            child.node_kind == alternative.node_kind
                        }).ok_or_else(|| {
                            SyntaxBackendBoundaryError::new(
                                "syntax_state_machine.mixed_payload.missing_child",
                                format!(
                                    "owner_rule={} field={} node_kind={} terminal_class={:?}: Nonterminal alternative has no StructuralModel reference",
                                    domain.owner_rule,
                                    domain.owner_field,
                                    alternative.node_kind,
                                    alternative.terminal
                                ),
                            )
                        })?;
                        (Some(child.target_rule.clone()), None)
                    }
                    _ => {
                        let payload = payloads.iter().find(|payload| {
                            payload.node_kind == alternative.node_kind
                        }).ok_or_else(|| {
                            SyntaxBackendBoundaryError::new(
                                "syntax_state_machine.mixed_payload.missing_payload",
                                format!(
                                    "owner_rule={} field={} node_kind={} terminal_class={:?}: payload alternative has no StructuralModel payload leaf",
                                    domain.owner_rule,
                                    domain.owner_field,
                                    alternative.node_kind,
                                    alternative.terminal
                                ),
                            )
                        })?;
                        (None, Some(payload.payload_kind.clone()))
                    }
                };

                alts.push(DiscriminatedAlternative {
                    node_kind: alternative.node_kind.clone(),
                    terminal: alternative.terminal,
                    body_kind,
                    target_rule,
                    payload_kind,
                });
            }

            let has_nonterminal = alts
                .iter()
                .any(|alt| alt.body_kind == AlternativeBodyKind::ChildRuleDescent);
            let has_payload = alts
                .iter()
                .any(|alt| alt.body_kind != AlternativeBodyKind::ChildRuleDescent);
            if has_nonterminal && has_payload {
                // Mixed-field BooleanPayload alternatives must use explicit
                // true/false node-kind identity; arbitrary names (e.g. "flag")
                // are rejected here so decode never invents a default value.
                for alt in &alts {
                    if alt.body_kind == AlternativeBodyKind::BooleanPayload
                        && alt.node_kind != "true"
                        && alt.node_kind != "false"
                    {
                        return Err(invalid_boolean_identity_error(
                            &domain.owner_rule,
                            &domain.owner_field,
                            &alt.node_kind,
                            alt.terminal,
                        ));
                    }
                }
                mixed_payload_fields.insert(field_key.clone());
            }
            alternatives_by_field.insert(field_key, alts);
        }

        Ok(Self {
            root_rule,
            records,
            references,
            children_by_field,
            payloads_by_field,
            multi_alternative_nonterminal_fields,
            alternatives_by_field,
            mixed_payload_fields,
        })
    }

    pub(super) fn root_record(&self) -> Result<&RecordSpec, String> {
        self.records.get(&self.root_rule).ok_or_else(|| {
            format!(
                "root rule {} is missing from StructuralModel records",
                self.root_rule
            )
        })
    }

    pub(super) fn mixed_field_alternative(
        &self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
    ) -> Option<&DiscriminatedAlternative> {
        if self.mixed_payload_fields.is_empty() {
            return None;
        }
        let field_key = (owner_rule.to_string(), owner_field.to_string());
        if !self.mixed_payload_fields.contains(&field_key) {
            return None;
        }
        self.alternatives_by_field
            .get(&field_key)?
            .iter()
            .find(|alt| alt.node_kind == node_kind)
    }

    /// Shared-SM mixed-payload contract §C: bare IntegerPayload on a mixed
    /// field requires a declared `GrammarIntegerValueRange` on the token layout.
    /// Homogeneous integer-only fields and mixed fields without IntegerPayload
    /// are unaffected. Failure names owner rule, field, and node kind.
    pub(super) fn require_mixed_integer_value_range_declared(
        &self,
        token_layout: &GrammarStructuralTokenLayout,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if token_layout.integer_value_range.is_some() {
            return Ok(());
        }
        for (owner_rule, owner_field) in &self.mixed_payload_fields {
            let Some(alts) = self
                .alternatives_by_field
                .get(&(owner_rule.clone(), owner_field.clone()))
            else {
                continue;
            };
            for alt in alts {
                if alt.body_kind == AlternativeBodyKind::IntegerPayload {
                    return Err(SyntaxBackendBoundaryError::new(
                        "syntax_state_machine.mixed_payload.missing_integer_value_range",
                        format!(
                            "owner_rule={owner_rule} field={owner_field} node_kind={}: \
                             mixed-field IntegerPayload requires token layout \
                             GrammarIntegerValueRange (shared-SM mixed-payload contract §C)",
                            alt.node_kind
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn body_kind_for_terminal(terminal: GrammarTerminalKind) -> AlternativeBodyKind {
    match terminal {
        GrammarTerminalKind::Nonterminal => AlternativeBodyKind::ChildRuleDescent,
        GrammarTerminalKind::BooleanPayload => AlternativeBodyKind::BooleanPayload,
        GrammarTerminalKind::IntegerPayload => AlternativeBodyKind::IntegerPayload,
        GrammarTerminalKind::StringPayload | GrammarTerminalKind::IdentifierPayload => {
            AlternativeBodyKind::Utf8Payload
        }
    }
}

pub(super) fn terminal_class_name(terminal: GrammarTerminalKind) -> &'static str {
    match terminal {
        GrammarTerminalKind::Nonterminal => "Nonterminal",
        GrammarTerminalKind::BooleanPayload => "BooleanPayload",
        GrammarTerminalKind::IntegerPayload => "IntegerPayload",
        GrammarTerminalKind::StringPayload => "StringPayload",
        GrammarTerminalKind::IdentifierPayload => "IdentifierPayload",
    }
}

fn ambiguous_alternative_error(
    owner_rule: &str,
    owner_field: &str,
    node_kind: &str,
    terminal: GrammarTerminalKind,
    detail: &str,
) -> SyntaxBackendBoundaryError {
    SyntaxBackendBoundaryError::new(
        "syntax_state_machine.mixed_payload.ambiguous_alternative",
        format!(
            "owner_rule={owner_rule} field={owner_field} node_kind={node_kind} terminal_class={terminal:?}: {detail}"
        ),
    )
}

/// Mixed-field BooleanPayload alternatives must be exactly `true` or `false`.
fn invalid_boolean_identity_error(
    owner_rule: &str,
    owner_field: &str,
    node_kind: &str,
    terminal: GrammarTerminalKind,
) -> SyntaxBackendBoundaryError {
    SyntaxBackendBoundaryError::new(
        "syntax_state_machine.mixed_payload.invalid_boolean_identity",
        format!(
            "owner_rule={owner_rule} field={owner_field} node_kind={node_kind} terminal_class={terminal:?}: \
             mixed-field BooleanPayload alternatives must use node_kind exactly \"true\" or \"false\""
        ),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordSpec {
    pub(super) rule: String,
    pub(super) node_kind: String,
    pub(super) fields: Vec<FieldSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FieldSpec {
    pub(super) name: String,
    pub(super) required: bool,
    pub(super) multiple: bool,
}

impl From<&GrammarStructuralField> for FieldSpec {
    fn from(field: &GrammarStructuralField) -> Self {
        Self {
            name: field.name.clone(),
            required: field.required,
            multiple: field.multiple,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChildSpec {
    pub(super) node_kind: String,
    pub(super) target_rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PayloadSpec {
    pub(super) node_kind: String,
    pub(super) payload_kind: String,
}

/// Minimal synthetic GrammarIR/StructuralModel witness for the shared-SM
/// mixed-payload contract: one required single-valued field mixes Nonterminal,
/// IntegerPayload (bounded u8), StringPayload (utf8 band), and explicit
/// true/false BooleanPayload without language keys.
#[cfg(test)]
pub(crate) mod mixed_payload_witness {
    use super::*;
    use crate::grammar_ir::{
        GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
    };
    use crate::grammar_ir_structural_model::{
        GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
    };
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    pub(crate) const WITNESS_GRAMMAR_ID: &str = "shared_sm_mixed_payload_witness_v0";

    pub(crate) fn mixed_payload_grammar_ir() -> GrammarIr {
        let mut grammar = GrammarIr::new(
            WITNESS_GRAMMAR_ID,
            GrammarIrProvenance {
                source_kind: "synthetic_shared_sm_mixed_payload_witness".to_string(),
                source_path: Some("src/syntax_state_machine/model_facts.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![GrammarField {
                name: "value".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![
                    GrammarFieldAlternative {
                        node_kind: "child".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: "number".to_string(),
                        terminal: GrammarTerminalKind::IntegerPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "text".to_string(),
                        terminal: GrammarTerminalKind::StringPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "true".to_string(),
                        terminal: GrammarTerminalKind::BooleanPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "false".to_string(),
                        terminal: GrammarTerminalKind::BooleanPayload,
                    },
                ],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "child".to_string(),
            node_kind: "child".to_string(),
            fields: vec![],
        });
        grammar
    }

    pub(crate) fn mixed_payload_sidecar() -> GrammarStructuralModelSidecar {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        lower_grammar_ir_to_structural_model(
            &mixed_payload_grammar_ir(),
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("mixed-payload witness should lower to StructuralModel")
    }

    pub(crate) fn ambiguous_boolean_and_wrapper_grammar_ir() -> GrammarIr {
        let mut grammar = mixed_payload_grammar_ir();
        // BooleanPayload true together with Nonterminal true is ambiguous.
        let field = &mut grammar.rules[0].fields[0];
        field.alternatives = vec![
            GrammarFieldAlternative {
                node_kind: "child".to_string(),
                terminal: GrammarTerminalKind::Nonterminal,
            },
            GrammarFieldAlternative {
                node_kind: "true".to_string(),
                terminal: GrammarTerminalKind::BooleanPayload,
            },
            GrammarFieldAlternative {
                node_kind: "true".to_string(),
                terminal: GrammarTerminalKind::Nonterminal,
            },
        ];
        grammar.rules.push(GrammarRule {
            name: "true".to_string(),
            node_kind: "true".to_string(),
            fields: vec![],
        });
        grammar
    }

    /// Mixed field with an arbitrary BooleanPayload node kind (not true/false).
    pub(crate) fn invalid_boolean_identity_grammar_ir() -> GrammarIr {
        let mut grammar = mixed_payload_grammar_ir();
        let field = &mut grammar.rules[0].fields[0];
        field.alternatives = vec![
            GrammarFieldAlternative {
                node_kind: "child".to_string(),
                terminal: GrammarTerminalKind::Nonterminal,
            },
            GrammarFieldAlternative {
                node_kind: "flag".to_string(),
                terminal: GrammarTerminalKind::BooleanPayload,
            },
        ];
        grammar
    }

    /// Sibling of the single-field witness: two mixed fields on the root rule.
    ///
    /// Field `left` and field `right` deliberately share some alternative node
    /// kinds (`number`, `true`, `false`) so free-slot assignment reuses one
    /// structural choice token per shared kind, while field-local kinds
    /// (`left_child` / `right_child`, and `text` only on `right`) get distinct
    /// tokens. Exercises the multi-field loop in
    /// `ensure_mixed_field_choice_tokens` without changing the single-field
    /// witness used by the rest of the mixed-payload contract tests.
    pub(crate) const TWO_MIXED_FIELDS_GRAMMAR_ID: &str = "shared_sm_two_mixed_fields_witness_v0";

    pub(crate) fn two_mixed_fields_grammar_ir() -> GrammarIr {
        let mut grammar = GrammarIr::new(
            TWO_MIXED_FIELDS_GRAMMAR_ID,
            GrammarIrProvenance {
                source_kind: "synthetic_shared_sm_two_mixed_fields_witness".to_string(),
                source_path: Some("src/syntax_state_machine/model_facts.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                GrammarField {
                    name: "left".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "left_child".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "number".to_string(),
                            terminal: GrammarTerminalKind::IntegerPayload,
                        },
                        GrammarFieldAlternative {
                            node_kind: "true".to_string(),
                            terminal: GrammarTerminalKind::BooleanPayload,
                        },
                        GrammarFieldAlternative {
                            node_kind: "false".to_string(),
                            terminal: GrammarTerminalKind::BooleanPayload,
                        },
                    ],
                },
                GrammarField {
                    name: "right".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "right_child".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "number".to_string(),
                            terminal: GrammarTerminalKind::IntegerPayload,
                        },
                        GrammarFieldAlternative {
                            node_kind: "text".to_string(),
                            terminal: GrammarTerminalKind::StringPayload,
                        },
                        GrammarFieldAlternative {
                            node_kind: "true".to_string(),
                            terminal: GrammarTerminalKind::BooleanPayload,
                        },
                        GrammarFieldAlternative {
                            node_kind: "false".to_string(),
                            terminal: GrammarTerminalKind::BooleanPayload,
                        },
                    ],
                },
            ],
        });
        grammar.rules.push(GrammarRule {
            name: "left_child".to_string(),
            node_kind: "left_child".to_string(),
            fields: vec![],
        });
        grammar.rules.push(GrammarRule {
            name: "right_child".to_string(),
            node_kind: "right_child".to_string(),
            fields: vec![],
        });
        grammar
    }

    pub(crate) fn two_mixed_fields_sidecar() -> GrammarStructuralModelSidecar {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        lower_grammar_ir_to_structural_model(
            &two_mixed_fields_grammar_ir(),
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("two-mixed-fields witness should lower to StructuralModel")
    }

    /// Sibling of the single-field witness with `multiple: true` on the mixed
    /// field so the re-admit path after each completed value is reachable.
    pub(crate) const REPEATED_MIXED_FIELD_GRAMMAR_ID: &str =
        "shared_sm_repeated_mixed_field_witness_v0";

    pub(crate) fn repeated_mixed_field_grammar_ir() -> GrammarIr {
        let mut grammar = GrammarIr::new(
            REPEATED_MIXED_FIELD_GRAMMAR_ID,
            GrammarIrProvenance {
                source_kind: "synthetic_shared_sm_repeated_mixed_field_witness".to_string(),
                source_path: Some("src/syntax_state_machine/model_facts.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![GrammarField {
                name: "values".to_string(),
                required: true,
                multiple: true,
                alternatives: vec![
                    GrammarFieldAlternative {
                        node_kind: "child".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: "number".to_string(),
                        terminal: GrammarTerminalKind::IntegerPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "text".to_string(),
                        terminal: GrammarTerminalKind::StringPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "true".to_string(),
                        terminal: GrammarTerminalKind::BooleanPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "false".to_string(),
                        terminal: GrammarTerminalKind::BooleanPayload,
                    },
                ],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "child".to_string(),
            node_kind: "child".to_string(),
            fields: vec![],
        });
        grammar
    }

    pub(crate) fn repeated_mixed_field_sidecar() -> GrammarStructuralModelSidecar {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        lower_grammar_ir_to_structural_model(
            &repeated_mixed_field_grammar_ir(),
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("repeated mixed-field witness should lower to StructuralModel")
    }

    /// Root holds a single Nonterminal child; the child rule itself has the
    /// mixed field. Exercises frame push/pop interaction with mixed-field
    /// state that the root-only witness never reaches.
    pub(crate) const NESTED_MIXED_FIELD_GRAMMAR_ID: &str =
        "shared_sm_nested_mixed_field_witness_v0";

    pub(crate) fn nested_mixed_field_grammar_ir() -> GrammarIr {
        let mut grammar = GrammarIr::new(
            NESTED_MIXED_FIELD_GRAMMAR_ID,
            GrammarIrProvenance {
                source_kind: "synthetic_shared_sm_nested_mixed_field_witness".to_string(),
                source_path: Some("src/syntax_state_machine/model_facts.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![GrammarField {
                name: "container".to_string(),
                required: true,
                multiple: false,
                // Single Nonterminal alternative: homogeneous descent into
                // the child rule (plain node_start, not a mixed choice).
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "container".to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "container".to_string(),
            node_kind: "container".to_string(),
            fields: vec![GrammarField {
                name: "inner".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![
                    GrammarFieldAlternative {
                        node_kind: "leaf".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: "number".to_string(),
                        terminal: GrammarTerminalKind::IntegerPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "text".to_string(),
                        terminal: GrammarTerminalKind::StringPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "true".to_string(),
                        terminal: GrammarTerminalKind::BooleanPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "false".to_string(),
                        terminal: GrammarTerminalKind::BooleanPayload,
                    },
                ],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "leaf".to_string(),
            node_kind: "leaf".to_string(),
            fields: vec![],
        });
        grammar
    }

    pub(crate) fn nested_mixed_field_sidecar() -> GrammarStructuralModelSidecar {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        lower_grammar_ir_to_structural_model(
            &nested_mixed_field_grammar_ir(),
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("nested mixed-field witness should lower to StructuralModel")
    }
}

#[cfg(test)]
mod tests {
    use super::mixed_payload_witness::{
        ambiguous_boolean_and_wrapper_grammar_ir, invalid_boolean_identity_grammar_ir,
        mixed_payload_sidecar,
    };
    use super::*;
    use crate::grammar_ir_structural_model::{
        GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
    };
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    #[test]
    fn model_facts_lower_records_children_and_payloads_from_structural_model() {
        let spec = ToyGrammarSpec::assignment_language_v1();
        let sidecar = lower_grammar_ir_to_structural_model(
            &spec.to_grammar_ir(),
            GrammarStructuralTokenLayout::from_toy_assignment(&spec.layout),
        )
        .expect("toy grammar should lower to StructuralModel");

        let facts = ModelFacts::from_sidecar(&sidecar).expect("facts should lower from sidecar");

        assert_eq!(facts.root_rule, "program");
        let root = facts.root_record().expect("root record should exist");
        assert_eq!(root.rule, "program");
        assert_eq!(root.node_kind, "program");
        assert_eq!(
            root.fields,
            vec![FieldSpec {
                name: "statements".to_string(),
                required: true,
                multiple: true,
            }]
        );
        assert_eq!(
            facts
                .children_by_field
                .get(&("program".to_string(), "statements".to_string())),
            Some(&vec![ChildSpec {
                node_kind: "assignment".to_string(),
                target_rule: "assignment".to_string(),
            }])
        );
        assert_eq!(
            facts
                .payloads_by_field
                .get(&("assignment".to_string(), "value".to_string())),
            Some(&vec![
                PayloadSpec {
                    node_kind: "boolean".to_string(),
                    payload_kind: "boolean".to_string(),
                },
                PayloadSpec {
                    node_kind: "integer".to_string(),
                    payload_kind: "integer".to_string(),
                },
                PayloadSpec {
                    node_kind: "string".to_string(),
                    payload_kind: "string".to_string(),
                },
            ])
        );
        assert!(facts.multi_alternative_nonterminal_fields.is_empty());
        // Homogeneous payload multi-alt is not a mixed field (no Nonterminal).
        // Homogeneous `boolean:BooleanPayload` keeps the legacy finite-token path.
        assert!(
            !facts
                .mixed_payload_fields
                .contains(&("assignment".to_string(), "value".to_string()))
        );
    }

    #[test]
    fn mixed_payload_witness_derives_discriminated_alternatives_and_marks_mixed_field() {
        let sidecar = mixed_payload_sidecar();
        let facts = ModelFacts::from_sidecar(&sidecar).expect("mixed witness facts");

        assert!(
            facts
                .mixed_payload_fields
                .contains(&("wrapper".to_string(), "value".to_string())),
            "wrapper.value must be marked as a mixed-payload field"
        );
        let alts = facts
            .alternatives_by_field
            .get(&("wrapper".to_string(), "value".to_string()))
            .expect("wrapper.value alternatives");
        assert_eq!(alts.len(), 5);
        assert_eq!(alts[0].node_kind, "child");
        assert_eq!(alts[0].body_kind, AlternativeBodyKind::ChildRuleDescent);
        assert_eq!(alts[0].target_rule.as_deref(), Some("child"));
        assert_eq!(alts[1].node_kind, "number");
        assert_eq!(alts[1].body_kind, AlternativeBodyKind::IntegerPayload);
        assert_eq!(alts[1].payload_kind.as_deref(), Some("integer"));
        assert_eq!(alts[2].node_kind, "text");
        assert_eq!(alts[2].body_kind, AlternativeBodyKind::Utf8Payload);
        assert_eq!(alts[2].payload_kind.as_deref(), Some("string"));
        assert_eq!(alts[3].node_kind, "true");
        assert_eq!(alts[3].body_kind, AlternativeBodyKind::BooleanPayload);
        assert_eq!(alts[3].payload_kind.as_deref(), Some("boolean"));
        assert_eq!(alts[4].node_kind, "false");
        assert_eq!(alts[4].body_kind, AlternativeBodyKind::BooleanPayload);
        assert_eq!(alts[4].payload_kind.as_deref(), Some("boolean"));
    }

    #[test]
    fn mixed_payload_rejects_ambiguous_boolean_and_nonterminal_identity() {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            &ambiguous_boolean_and_wrapper_grammar_ir(),
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("ambiguous grammar still lowers; SM construction must reject");
        let err = ModelFacts::from_sidecar(&sidecar).expect_err("ambiguous identities must fail");
        let message = err.to_string();
        assert!(
            message.contains("owner_rule=wrapper"),
            "diagnostic must name owner rule: {message}"
        );
        assert!(
            message.contains("field=value"),
            "diagnostic must name field: {message}"
        );
        assert!(
            message.contains("node_kind=true"),
            "diagnostic must name node kind: {message}"
        );
        assert!(
            message.contains("terminal_class=")
                || message.contains("BooleanPayload")
                || message.contains("Nonterminal"),
            "diagnostic must name terminal class: {message}"
        );
    }

    #[test]
    fn mixed_payload_rejects_arbitrary_boolean_payload_node_kind() {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            &invalid_boolean_identity_grammar_ir(),
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("invalid-identity grammar still lowers; SM construction must reject");
        let err = ModelFacts::from_sidecar(&sidecar)
            .expect_err("arbitrary BooleanPayload kind must fail");
        let message = err.to_string();
        assert!(
            message.contains("invalid_boolean_identity")
                || message.contains("must use node_kind exactly"),
            "diagnostic must identify the boolean identity rule: {message}"
        );
        assert!(
            message.contains("owner_rule=wrapper"),
            "diagnostic must name owner rule: {message}"
        );
        assert!(
            message.contains("field=value"),
            "diagnostic must name field: {message}"
        );
        assert!(
            message.contains("node_kind=flag"),
            "diagnostic must name node kind: {message}"
        );
        assert!(
            message.contains("terminal_class=") || message.contains("BooleanPayload"),
            "diagnostic must name terminal class: {message}"
        );
    }
}
