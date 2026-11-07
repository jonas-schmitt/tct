use super::{
    GoAstInventoryCheckedDeclarativeExpressionSpec, GoAstInventoryDeclarativeExpressionSpec,
    GoAstInventoryDeclarativeFieldSpec, GoAstInventoryDeclarativeFieldValue,
    GoAstInventoryDeclarativeNodeSpec, GoAstInventoryDeclarativeParentFieldSpec,
    validate_declarative_expression_spec,
};
use crate::go_ast_schema_inventory::GoAstSchemaInventoryAdapter;
use crate::grammar_ir::{GrammarField, GrammarIr, GrammarTerminalKind};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_raw_token_stream::go_syntax_synthetic_trivia_field_names;

use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryExpectedFieldOrder, GoAstInventoryFiniteDomainRequirement,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn check_go_ast_inventory_declarative_expression_spec_from_inventory(
    inventory_path: &Path,
    parent_field: Option<&GoAstInventoryDeclarativeParentFieldSpec>,
    expression: &GoAstInventoryDeclarativeExpressionSpec,
) -> Result<GoAstInventoryCheckedDeclarativeExpressionSpec, SyntaxBackendBoundaryError> {
    validate_declarative_expression_spec(expression)?;
    let grammar =
        GoAstSchemaInventoryAdapter::lower_inventory_file(inventory_path).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_ast_inventory_declarative_template_spec.inventory",
                format!("failed to lower checked Go AST inventory: {error}"),
            )
        })?;
    let mut collector = DeclarativePlanCollector::default();
    if let Some(parent_field) = parent_field {
        let parent = inventory_field(&grammar, &parent_field.rule_name, &parent_field.field_name)?;
        require_nonterminal_alternative(
            parent,
            expression.root_node_kind(),
            &mut collector,
            &parent_field.rule_name,
            &parent_field.field_name,
        )?;
    }
    let root = normalize_node_spec(&grammar, &expression.root, &mut collector)?;
    Ok(GoAstInventoryCheckedDeclarativeExpressionSpec {
        expression: GoAstInventoryDeclarativeExpressionSpec { root },
        expected_field_orders: collector.expected_field_orders,
        finite_domain_requirements: collector.finite_domain_requirements,
    })
}

fn normalize_node_spec(
    grammar: &GrammarIr,
    node: &GoAstInventoryDeclarativeNodeSpec,
    collector: &mut DeclarativePlanCollector,
) -> Result<GoAstInventoryDeclarativeNodeSpec, SyntaxBackendBoundaryError> {
    let rule = grammar.rule(&node.node_kind).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.node_kind",
            format!("checked inventory is missing node kind {}", node.node_kind),
        )
    })?;
    collector.add_field_order(&rule.node_kind, rule.fields.iter().map(|field| &field.name));

    let mut provided = node
        .fields
        .iter()
        .map(|field| (field.field_name.as_str(), field))
        .collect::<BTreeMap<_, _>>();
    let trivia_fields = go_syntax_synthetic_trivia_field_names(&rule.node_kind);
    let mut normalized_fields = Vec::with_capacity(rule.fields.len());
    for inventory_field in &rule.fields {
        if trivia_fields.contains(&inventory_field.name.as_str()) {
            // Synthesized trailing trivia fields are auto-appended (empty)
            // by the stream builder; declarative specs must not spell them.
            if provided.remove(inventory_field.name.as_str()).is_some() {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_declarative_template_spec.trivia_field",
                    format!(
                        "declarative spec for {} must not spell synthesized trivia field {}; the stream builder appends it empty",
                        rule.node_kind, inventory_field.name
                    ),
                ));
            }
            continue;
        }
        let spec_field = provided
            .remove(inventory_field.name.as_str())
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_declarative_template_spec.missing_field",
                    format!(
                        "declarative spec for {} must spell policy for checked field {}",
                        rule.node_kind, inventory_field.name
                    ),
                )
            })?;
        let normalized_value = normalize_field_value(
            grammar,
            &rule.node_kind,
            inventory_field,
            &spec_field.value,
            collector,
        )?;
        normalized_fields.push(GoAstInventoryDeclarativeFieldSpec {
            field_name: inventory_field.name.clone(),
            value: normalized_value,
        });
    }
    if let Some(extra_field_name) = provided.keys().next() {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.unknown_field",
            format!(
                "declarative spec for {} names unknown checked field {}",
                rule.node_kind, extra_field_name
            ),
        ));
    }

    Ok(GoAstInventoryDeclarativeNodeSpec {
        node_kind: rule.node_kind.clone(),
        fields: normalized_fields,
    })
}

fn normalize_field_value(
    grammar: &GrammarIr,
    rule_name: &str,
    field: &GrammarField,
    value: &GoAstInventoryDeclarativeFieldValue,
    collector: &mut DeclarativePlanCollector,
) -> Result<GoAstInventoryDeclarativeFieldValue, SyntaxBackendBoundaryError> {
    match value {
        GoAstInventoryDeclarativeFieldValue::Missing => {
            if field.required || field.multiple {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_declarative_template_spec.missing_field",
                    format!(
                        "{}.{} cannot use missing policy because it is {}",
                        rule_name,
                        field.name,
                        if field.multiple {
                            "a repeated field"
                        } else {
                            "required"
                        }
                    ),
                ));
            }
            Ok(GoAstInventoryDeclarativeFieldValue::Missing)
        }
        GoAstInventoryDeclarativeFieldValue::TerminalPayload {
            selected_alternative,
            terminal_kind,
            payload,
        } => {
            if field.multiple {
                return Err(repeated_field_policy_error(rule_name, field));
            }
            let selected = selected_terminal_alternative(
                field,
                selected_alternative.as_deref(),
                *terminal_kind,
            )?;
            collector.add_finite_domain_requirement_if_choice(field, rule_name, &selected);
            Ok(GoAstInventoryDeclarativeFieldValue::TerminalPayload {
                selected_alternative: Some(selected),
                terminal_kind: *terminal_kind,
                payload: payload.clone(),
            })
        }
        GoAstInventoryDeclarativeFieldValue::BooleanPayload {
            selected_alternative,
            value,
        } => {
            if field.multiple {
                return Err(repeated_field_policy_error(rule_name, field));
            }
            let selected = selected_terminal_alternative(
                field,
                selected_alternative.as_deref(),
                GrammarTerminalKind::BooleanPayload,
            )?;
            collector.add_finite_domain_requirement_if_choice(field, rule_name, &selected);
            Ok(GoAstInventoryDeclarativeFieldValue::BooleanPayload {
                selected_alternative: Some(selected),
                value: *value,
            })
        }
        GoAstInventoryDeclarativeFieldValue::AlternativeNode(child) => {
            if field.multiple {
                return Err(repeated_field_policy_error(rule_name, field));
            }
            require_nonterminal_alternative(
                field,
                &child.node_kind,
                collector,
                rule_name,
                &field.name,
            )?;
            Ok(GoAstInventoryDeclarativeFieldValue::AlternativeNode(
                Box::new(normalize_node_spec(grammar, child, collector)?),
            ))
        }
        GoAstInventoryDeclarativeFieldValue::GenericNode(child) => {
            if field.multiple {
                return Err(repeated_field_policy_error(rule_name, field));
            }
            if field.alternatives.len() != 1
                || field.alternatives[0].terminal != GrammarTerminalKind::Nonterminal
            {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_declarative_template_spec.generic_field_policy",
                    format!(
                        "{}.{} cannot use generic-node policy because it is not a singleton nonterminal field",
                        rule_name, field.name
                    ),
                ));
            }
            require_nonterminal_alternative(
                field,
                &child.node_kind,
                collector,
                rule_name,
                &field.name,
            )?;
            Ok(GoAstInventoryDeclarativeFieldValue::GenericNode(Box::new(
                normalize_node_spec(grammar, child, collector)?,
            )))
        }
        GoAstInventoryDeclarativeFieldValue::OptionalNode(child) => {
            require_optional_singleton_nonterminal_policy(field, rule_name)?;
            let normalized = if let Some(child) = child {
                require_nonterminal_alternative(
                    field,
                    &child.node_kind,
                    collector,
                    rule_name,
                    &field.name,
                )?;
                Some(Box::new(normalize_node_spec(grammar, child, collector)?))
            } else {
                None
            };
            Ok(GoAstInventoryDeclarativeFieldValue::OptionalNode(
                normalized,
            ))
        }
        GoAstInventoryDeclarativeFieldValue::OptionalAlternativeNode(child) => {
            require_optional_alternative_nonterminal_policy(field, rule_name)?;
            let normalized = if let Some(child) = child {
                require_nonterminal_alternative(
                    field,
                    &child.node_kind,
                    collector,
                    rule_name,
                    &field.name,
                )?;
                Some(Box::new(normalize_node_spec(grammar, child, collector)?))
            } else {
                None
            };
            Ok(GoAstInventoryDeclarativeFieldValue::OptionalAlternativeNode(normalized))
        }
        GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes) => {
            if !field.multiple {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_declarative_template_spec.repeated_field_policy",
                    format!(
                        "{}.{} is not repeated and cannot use repeated-alternative policy",
                        rule_name, field.name
                    ),
                ));
            }
            let mut normalized = Vec::with_capacity(nodes.len());
            for child in nodes {
                require_nonterminal_alternative(
                    field,
                    &child.node_kind,
                    collector,
                    rule_name,
                    &field.name,
                )?;
                normalized.push(normalize_node_spec(grammar, child, collector)?);
            }
            if !normalized.is_empty()
                && field.alternatives.len() == 1
                && field.alternatives[0].terminal == GrammarTerminalKind::Nonterminal
            {
                Ok(GoAstInventoryDeclarativeFieldValue::RepeatedGenericNodes(
                    normalized,
                ))
            } else {
                Ok(GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(normalized))
            }
        }
        GoAstInventoryDeclarativeFieldValue::RepeatedGenericNodes(nodes) => {
            if !field.multiple {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_declarative_template_spec.repeated_field_policy",
                    format!(
                        "{}.{} is not repeated and cannot use repeated-generic policy",
                        rule_name, field.name
                    ),
                ));
            }
            if field.alternatives.len() != 1
                || field.alternatives[0].terminal != GrammarTerminalKind::Nonterminal
            {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_declarative_template_spec.repeated_field_policy",
                    format!(
                        "{}.{} cannot use repeated-generic policy because it is not a singleton nonterminal repeated field",
                        rule_name, field.name
                    ),
                ));
            }
            let mut normalized = Vec::with_capacity(nodes.len());
            for child in nodes {
                require_nonterminal_alternative(
                    field,
                    &child.node_kind,
                    collector,
                    rule_name,
                    &field.name,
                )?;
                normalized.push(normalize_node_spec(grammar, child, collector)?);
            }
            Ok(GoAstInventoryDeclarativeFieldValue::RepeatedGenericNodes(
                normalized,
            ))
        }
    }
}

fn require_optional_singleton_nonterminal_policy(
    field: &GrammarField,
    rule_name: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if field.required || field.multiple {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.optional_field_policy",
            format!(
                "{}.{} cannot use optional-node policy because it is {}",
                rule_name,
                field.name,
                if field.multiple {
                    "a repeated field"
                } else {
                    "required"
                }
            ),
        ));
    }
    if field.alternatives.len() != 1
        || field.alternatives[0].terminal != GrammarTerminalKind::Nonterminal
    {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.optional_field_policy",
            format!(
                "{}.{} cannot use optional-node policy because it is not a singleton nonterminal field",
                rule_name, field.name
            ),
        ));
    }
    Ok(())
}

fn require_optional_alternative_nonterminal_policy(
    field: &GrammarField,
    rule_name: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if field.required || field.multiple {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.optional_field_policy",
            format!(
                "{}.{} cannot use optional-alternative policy because it is {}",
                rule_name,
                field.name,
                if field.multiple {
                    "a repeated field"
                } else {
                    "required"
                }
            ),
        ));
    }
    let nonterminal_alternative_count = field
        .alternatives
        .iter()
        .filter(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal)
        .count();
    if nonterminal_alternative_count <= 1 {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.optional_field_policy",
            format!(
                "{}.{} cannot use optional-alternative policy because it is not a multi-alternative nonterminal field",
                rule_name, field.name
            ),
        ));
    }
    Ok(())
}

fn repeated_field_policy_error(
    rule_name: &str,
    field: &GrammarField,
) -> SyntaxBackendBoundaryError {
    SyntaxBackendBoundaryError::new(
        "go_ast_inventory_declarative_template_spec.repeated_field_policy",
        format!(
            "{}.{} is repeated and requires repeated-alternative policy",
            rule_name, field.name
        ),
    )
}

fn selected_terminal_alternative(
    field: &GrammarField,
    selected_alternative: Option<&str>,
    terminal_kind: GrammarTerminalKind,
) -> Result<String, SyntaxBackendBoundaryError> {
    if let Some(selected_alternative) = selected_alternative {
        if field.alternatives.iter().any(|alternative| {
            alternative.node_kind == selected_alternative && alternative.terminal == terminal_kind
        }) {
            return Ok(selected_alternative.to_string());
        }
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.terminal.selected_alternative",
            format!(
                "field {} missing terminal alternative {selected_alternative}/{terminal_kind:?}",
                field.name
            ),
        ));
    }
    let matches = field
        .alternatives
        .iter()
        .filter(|alternative| alternative.terminal == terminal_kind)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [alternative] => Ok(alternative.node_kind.clone()),
        [] => Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.terminal.selected_alternative",
            format!(
                "field {} has no {terminal_kind:?} terminal alternative",
                field.name
            ),
        )),
        _ => Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.terminal.selected_alternative",
            format!(
                "field {} has multiple {terminal_kind:?} alternatives and requires an explicit selected alternative",
                field.name
            ),
        )),
    }
}

fn require_nonterminal_alternative(
    field: &GrammarField,
    node_kind: &str,
    collector: &mut DeclarativePlanCollector,
    rule_name: &str,
    field_name: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if field.alternatives.iter().any(|alternative| {
        alternative.node_kind == node_kind
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }) {
        collector.add_finite_domain_requirement_if_choice(field, rule_name, node_kind);
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_declarative_template_spec.alternative",
            format!("field {rule_name}.{field_name} missing nonterminal alternative {node_kind}"),
        ))
    }
}

fn inventory_field<'a>(
    grammar: &'a GrammarIr,
    rule_name: &str,
    field_name: &str,
) -> Result<&'a GrammarField, SyntaxBackendBoundaryError> {
    grammar
        .rule(rule_name)
        .and_then(|rule| rule.fields.iter().find(|field| field.name == field_name))
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_inventory_declarative_template_spec.parent_field",
                format!("checked inventory is missing parent field {rule_name}.{field_name}"),
            )
        })
}

#[derive(Default)]
struct DeclarativePlanCollector {
    expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
    seen_field_orders: BTreeSet<String>,
    finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    seen_finite_domain_requirements: BTreeSet<String>,
}

impl DeclarativePlanCollector {
    fn add_field_order<'a>(&mut self, rule_name: &str, fields: impl Iterator<Item = &'a String>) {
        if self.seen_field_orders.insert(rule_name.to_string()) {
            self.expected_field_orders
                .push(GoAstInventoryExpectedFieldOrder {
                    rule_name: rule_name.to_string(),
                    fields: fields.cloned().collect(),
                });
        }
    }

    fn add_finite_domain_requirement_if_choice(
        &mut self,
        field: &GrammarField,
        rule_name: &str,
        selected_alternative: &str,
    ) {
        if field.alternatives.len() <= 1 {
            return;
        }
        let Some(alternative) = field
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == selected_alternative)
        else {
            return;
        };
        let key = format!("{rule_name}.{}={selected_alternative}", field.name);
        if self.seen_finite_domain_requirements.insert(key) {
            self.finite_domain_requirements
                .push(GoAstInventoryFiniteDomainRequirement {
                    rule_name: rule_name.to_string(),
                    field_name: field.name.clone(),
                    selected_alternative: selected_alternative.to_string(),
                    terminal: alternative.terminal,
                });
        }
    }
}
