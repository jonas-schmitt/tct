//! Language-neutral encode: `RawSyntaxDecodedNode` → expanded raw token stream.
//!
//! Inverse of `CompiledSyntaxModel::decode_complete_tree` → `decoded_node_tree`.
//! Model-faithful: emit exactly what the compiled `StructuralModel` / roles
//! carry. Projection/homogenization stays in language modules.
//!
//! Contract: `docs/generic-codec-layer-contract.md` (`generic_codec_layer_contract_v0`).

use super::factored_menu_codec::factored_digits_for_flat_ordinal;
use super::menu_codec::menu_digit_width;
use super::model_facts::{AlternativeBodyKind, FieldSpec, ModelFacts, RecordSpec};
use super::{
    MenuSiteBindings, MenuSiteKind, RawSyntaxDecodedField, RawSyntaxDecodedNode,
    RawSyntaxDecodedTerminal, RawSyntaxDecodedValue, RawSyntaxError, SyntaxTokenRoles,
};
use crate::syntax_token_layout::ProjectionTokenLayoutRangeSummary;

/// Shared model handles threaded through encode recursion (keeps helper arity low).
struct EncodeModel<'a> {
    facts: &'a ModelFacts,
    roles: &'a SyntaxTokenRoles,
    menu_bindings: &'a MenuSiteBindings,
}

/// Encode a complete model-conforming decoded-node tree to an expanded raw stream.
pub(super) fn encode_complete_tree(
    facts: &ModelFacts,
    roles: &SyntaxTokenRoles,
    menu_bindings: &MenuSiteBindings,
    root: &RawSyntaxDecodedNode,
) -> Result<Vec<u32>, RawSyntaxError> {
    let root_record = facts.root_record().map_err(|message| {
        encode_error(
            "encode_model",
            message,
            0,
            Some(facts.root_rule.clone()),
            None,
        )
    })?;
    if root.rule_name != root_record.rule || root.node_kind != root_record.node_kind {
        return Err(encode_error(
            "encode_root_mismatch",
            format!(
                "root rule/node_kind ({}, {}) does not match model root ({}, {})",
                root.rule_name, root.node_kind, root_record.rule, root_record.node_kind
            ),
            0,
            Some(root.rule_name.clone()),
            None,
        ));
    }

    let mut tokens = Vec::new();
    // Live table scope for column menus; filled as table sites encode (**P5**).
    let mut scope_tables: Vec<u32> = Vec::new();
    let model = EncodeModel {
        facts,
        roles,
        menu_bindings,
    };
    // Root always opens with the plain node_start role (never an alternative choice).
    tokens.push(roles.node_start);
    encode_record_body(model, root_record, root, &mut tokens, &mut scope_tables)?;
    for &token in &tokens {
        if token >= roles.pad_token || token >= roles.vocab_size {
            return Err(encode_error(
                "encode_token_out_of_range",
                format!(
                    "emitted token {token} collides with PAD or leaves VOCAB_SIZE {}",
                    roles.vocab_size
                ),
                tokens.len(),
                Some(root.rule_name.clone()),
                None,
            ));
        }
    }
    Ok(tokens)
}

/// Encode fields of an already-opened record, then `node_end`.
///
/// Scope-root records open a **fresh** encode-side `scope_tables` vector so a
/// nested uncorrelated subquery does not inherit or leak tables from the outer
/// scope — matching replay, which stores scope on the nearest scope-root
/// `FrameState` (**P5**).
fn encode_record_body(
    model: EncodeModel<'_>,
    record: &RecordSpec,
    node: &RawSyntaxDecodedNode,
    tokens: &mut Vec<u32>,
    scope_tables: &mut Vec<u32>,
) -> Result<(), RawSyntaxError> {
    if model.menu_bindings.is_scope_root(&record.rule) {
        let mut nested_scope = Vec::new();
        return encode_record_body_in_scope(model, record, node, tokens, &mut nested_scope);
    }
    encode_record_body_in_scope(model, record, node, tokens, scope_tables)
}

fn encode_record_body_in_scope(
    model: EncodeModel<'_>,
    record: &RecordSpec,
    node: &RawSyntaxDecodedNode,
    tokens: &mut Vec<u32>,
    scope_tables: &mut Vec<u32>,
) -> Result<(), RawSyntaxError> {
    let EncodeModel {
        facts,
        roles,
        menu_bindings,
    } = model;
    if node.rule_name != record.rule || node.node_kind != record.node_kind {
        return Err(encode_error(
            "encode_node_mismatch",
            format!(
                "node rule/node_kind ({}, {}) does not match record ({}, {})",
                node.rule_name, node.node_kind, record.rule, record.node_kind
            ),
            tokens.len(),
            Some(node.rule_name.clone()),
            None,
        ));
    }

    // Reject unknown fields on the tree (model-faithful; no silent drop).
    for field in &node.fields {
        if !record.fields.iter().any(|spec| spec.name == field.name) {
            return Err(encode_error(
                "encode_unknown_field",
                format!(
                    "field {} is not part of record {} (node_kind {})",
                    field.name, record.rule, record.node_kind
                ),
                tokens.len(),
                Some(record.rule.clone()),
                Some(field.name.clone()),
            ));
        }
    }

    // Walk model field order, not tree field order.
    for field_spec in &record.fields {
        let tree_field = node.field(&field_spec.name);
        encode_field(
            EncodeModel {
                facts,
                roles,
                menu_bindings,
            },
            &record.rule,
            field_spec,
            tree_field,
            tokens,
            scope_tables,
        )?;
    }

    tokens.push(roles.node_end);
    Ok(())
}

fn encode_field(
    model: EncodeModel<'_>,
    owner_rule: &str,
    field_spec: &FieldSpec,
    tree_field: Option<&RawSyntaxDecodedField>,
    tokens: &mut Vec<u32>,
    scope_tables: &mut Vec<u32>,
) -> Result<(), RawSyntaxError> {
    let EncodeModel {
        facts,
        roles,
        menu_bindings,
    } = model;
    tokens.push(roles.field_start);

    let field_key = (owner_rule.to_string(), field_spec.name.clone());
    let is_mixed = facts.mixed_payload_fields.contains(&field_key);

    match tree_field {
        None => {
            if field_spec.required {
                return Err(encode_error(
                    "encode_missing_field",
                    format!(
                        "required field {}.{} is absent from the tree",
                        owner_rule, field_spec.name
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(field_spec.name.clone()),
                ));
            }
            // Optional field omitted from the tree → field_missing.
            tokens.push(roles.field_missing);
        }
        Some(field) if field.missing => {
            if field_spec.required {
                // Decode rejects field_missing on required fields
                // (raw_replay::missing_required_field). Fail closed here so
                // encode never emits an SM-invalid stream for this shape.
                return Err(encode_error(
                    "encode_missing_field",
                    format!(
                        "required field {}.{} cannot be encoded as missing",
                        owner_rule, field_spec.name
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(field_spec.name.clone()),
                ));
            }
            if !field.values.is_empty() || field.repeated_empty {
                return Err(encode_error(
                    "encode_nonconforming_field",
                    format!(
                        "field {}.{} has missing=true but also carries values or repeated_empty",
                        owner_rule, field_spec.name
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(field_spec.name.clone()),
                ));
            }
            tokens.push(roles.field_missing);
        }
        Some(field) if field.repeated_empty => {
            // Decode admits repeated_empty only when multiple && !required &&
            // value_count == 0 (see raw_replay::apply_field_repeated_empty).
            if !field_spec.multiple || field_spec.required {
                return Err(encode_error(
                    "encode_nonconforming_field",
                    format!(
                        "field {}.{} cannot use repeated_empty (requires optional repeated field)",
                        owner_rule, field_spec.name
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(field_spec.name.clone()),
                ));
            }
            if !field.values.is_empty() || field.missing {
                return Err(encode_error(
                    "encode_nonconforming_field",
                    format!(
                        "field {}.{} has repeated_empty=true but also carries values or missing",
                        owner_rule, field_spec.name
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(field_spec.name.clone()),
                ));
            }
            tokens.push(roles.repeated_empty);
        }
        Some(field) if field.values.is_empty() => {
            return Err(encode_error(
                "encode_empty_field",
                format!(
                    "field {}.{} has no values and neither missing nor repeated_empty markers",
                    owner_rule, field_spec.name
                ),
                tokens.len(),
                Some(owner_rule.to_string()),
                Some(field_spec.name.clone()),
            ));
        }
        Some(field) => {
            if !field_spec.multiple && field.values.len() > 1 {
                return Err(encode_error(
                    "encode_cardinality",
                    format!(
                        "field {}.{} is single-valued but tree carries {} values",
                        owner_rule,
                        field_spec.name,
                        field.values.len()
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(field_spec.name.clone()),
                ));
            }
            for value in &field.values {
                if is_mixed {
                    encode_mixed_value(
                        EncodeModel {
                            facts,
                            roles,
                            menu_bindings,
                        },
                        owner_rule,
                        &field_spec.name,
                        value,
                        tokens,
                        scope_tables,
                    )?;
                } else {
                    encode_homogeneous_value(
                        EncodeModel {
                            facts,
                            roles,
                            menu_bindings,
                        },
                        owner_rule,
                        &field_spec.name,
                        value,
                        tokens,
                        scope_tables,
                    )?;
                }
            }
        }
    }

    tokens.push(roles.field_end);
    Ok(())
}

fn encode_homogeneous_value(
    model: EncodeModel<'_>,
    owner_rule: &str,
    owner_field: &str,
    value: &RawSyntaxDecodedValue,
    tokens: &mut Vec<u32>,
    scope_tables: &mut Vec<u32>,
) -> Result<(), RawSyntaxError> {
    match value {
        RawSyntaxDecodedValue::Node(child) => encode_homogeneous_nonterminal_child(
            model,
            owner_rule,
            owner_field,
            child,
            tokens,
            scope_tables,
        ),
        RawSyntaxDecodedValue::Terminal(terminal) => encode_homogeneous_payload(
            model,
            owner_rule,
            owner_field,
            terminal,
            tokens,
            scope_tables,
        ),
    }
}

fn encode_homogeneous_nonterminal_child(
    model: EncodeModel<'_>,
    owner_rule: &str,
    owner_field: &str,
    child: &RawSyntaxDecodedNode,
    tokens: &mut Vec<u32>,
    scope_tables: &mut Vec<u32>,
) -> Result<(), RawSyntaxError> {
    let EncodeModel {
        facts,
        roles,
        menu_bindings,
    } = model;
    let children = facts
        .children_by_field
        .get(&(owner_rule.to_string(), owner_field.to_string()))
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let child_spec = children
        .iter()
        .find(|c| c.node_kind == child.node_kind)
        .ok_or_else(|| {
            encode_error(
                "encode_unknown_alternative",
                format!(
                    "node_kind {} is not a Nonterminal alternative of field {}.{}",
                    child.node_kind, owner_rule, owner_field
                ),
                tokens.len(),
                Some(owner_rule.to_string()),
                Some(owner_field.to_string()),
            )
        })?;
    if child.rule_name != child_spec.target_rule {
        return Err(encode_error(
            "encode_node_mismatch",
            format!(
                "child rule {} does not match StructuralModel target_rule {} for node_kind {} on {}.{}",
                child.rule_name, child_spec.target_rule, child.node_kind, owner_rule, owner_field
            ),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    }

    let open = child_node_start_token(facts, roles, owner_rule, owner_field, &child.node_kind)?;
    tokens.push(open);

    let record = facts.records.get(&child.rule_name).ok_or_else(|| {
        encode_error(
            "encode_unknown_rule",
            format!(
                "child rule {} (node_kind {}) is not a StructuralModel record",
                child.rule_name, child.node_kind
            ),
            tokens.len(),
            Some(child.rule_name.clone()),
            Some(owner_field.to_string()),
        )
    })?;
    encode_record_body(
        EncodeModel {
            facts,
            roles,
            menu_bindings,
        },
        record,
        child,
        tokens,
        scope_tables,
    )
}

fn child_node_start_token(
    facts: &ModelFacts,
    roles: &SyntaxTokenRoles,
    owner_rule: &str,
    owner_field: &str,
    node_kind: &str,
) -> Result<u32, RawSyntaxError> {
    let multi = facts
        .multi_alternative_nonterminal_fields
        .contains(&(owner_rule.to_string(), owner_field.to_string()));
    if !multi {
        return Ok(roles.node_start);
    }
    // SharedAlphabet: field-relative CHOICE(n,k); PerGrammar: global map.
    roles
        .alternative_token_for_field(owner_rule, owner_field, node_kind)
        .ok_or_else(|| {
            encode_error(
                "encode_missing_alt_token",
                format!(
                    "missing alternative node_start for {owner_rule}.{owner_field} child {node_kind}"
                ),
                0,
                Some(owner_rule.to_string()),
                Some(owner_field.to_string()),
            )
        })
}

fn encode_homogeneous_payload(
    model: EncodeModel<'_>,
    owner_rule: &str,
    owner_field: &str,
    terminal: &RawSyntaxDecodedTerminal,
    tokens: &mut Vec<u32>,
    scope_tables: &mut Vec<u32>,
) -> Result<(), RawSyntaxError> {
    let EncodeModel {
        facts,
        roles,
        menu_bindings,
    } = model;
    let payloads = facts
        .payloads_by_field
        .get(&(owner_rule.to_string(), owner_field.to_string()))
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if payloads.is_empty() {
        return Err(encode_error(
            "encode_terminal_on_nonterminal_field",
            format!(
                "field {}.{} cannot accept a Terminal (node_kind {}); only Nonterminal children are legal",
                owner_rule, owner_field, terminal.node_kind
            ),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    }

    // Prefer an exact node_kind match; fall back to sole payload when decode
    // materializes a joined multi-kind label (homogeneous multi-payload rare).
    let payload_spec = payloads
        .iter()
        .find(|p| p.node_kind == terminal.node_kind)
        .or_else(|| {
            if payloads.len() == 1 {
                Some(&payloads[0])
            } else {
                None
            }
        })
        .ok_or_else(|| {
            encode_error(
                "encode_unknown_payload",
                format!(
                    "terminal node_kind {} is not a payload alternative of field {}.{}",
                    terminal.node_kind, owner_rule, owner_field
                ),
                tokens.len(),
                Some(owner_rule.to_string()),
                Some(owner_field.to_string()),
            )
        })?;

    let kind = payload_spec.payload_kind.as_str();
    if is_boolean_payload_kind(kind) {
        // Homogeneous boolean: terminal + finite boolean role (no payload_end).
        tokens.push(roles.terminal);
        let bool_token = match terminal.payload.as_str() {
            "true" => roles.boolean_true,
            "false" => roles.boolean_false,
            other => {
                return Err(encode_error(
                    "encode_invalid_boolean",
                    format!(
                        "homogeneous BooleanPayload on {}.{} has payload {other:?}; expected \"true\" or \"false\"",
                        owner_rule, owner_field
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                ));
            }
        };
        tokens.push(bool_token);
        return Ok(());
    }

    if is_integer_payload_kind(kind) {
        tokens.push(roles.terminal);
        // Menu-bound IntegerPayload sites emit one menu-band ordinal (**P4**).
        // Empty binding keeps the u8 integer path (**P10**). Column sites use
        // live scope cardinality (**P5**).
        if let Some(cardinality) =
            menu_bindings.resolve_cardinality(owner_rule, owner_field, scope_tables)
        {
            if cardinality == 0 {
                return Err(encode_error(
                    "encode_menu_scope_empty",
                    format!("menu site {owner_rule}.{owner_field} has empty scope during encode"),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                ));
            }
            let ordinal = emit_menu_index_payload(
                roles,
                &terminal.payload,
                MenuEmitSite {
                    kind: menu_bindings.site_kind(owner_rule, owner_field),
                    scope_tables,
                    cardinality,
                    max_menu_digits: menu_bindings.max_menu_digits,
                    owner_rule,
                    owner_field,
                },
                tokens,
            )?;
            if matches!(
                menu_bindings.site_kind(owner_rule, owner_field),
                Some(MenuSiteKind::Fixed {
                    pushes_scope: true,
                    ..
                })
            ) {
                push_scope_table_encode(
                    scope_tables,
                    ordinal,
                    tokens.len(),
                    owner_rule,
                    owner_field,
                )?;
            }
        } else {
            emit_u8_integer_payload_bytes(
                roles,
                &terminal.payload,
                tokens,
                owner_rule,
                owner_field,
            )?;
        }
        tokens.push(roles.payload_end);
        return Ok(());
    }

    if is_utf8_payload_kind(kind) {
        tokens.push(roles.terminal);
        emit_utf8_payload_bytes(roles, &terminal.payload, tokens, owner_rule, owner_field)?;
        tokens.push(roles.payload_end);
        return Ok(());
    }

    Err(encode_error(
        "encode_unsupported_payload",
        format!(
            "field {}.{} payload_kind {} is not supported by generic encode",
            owner_rule, owner_field, kind
        ),
        tokens.len(),
        Some(owner_rule.to_string()),
        Some(owner_field.to_string()),
    ))
}

fn encode_mixed_value(
    model: EncodeModel<'_>,
    owner_rule: &str,
    owner_field: &str,
    value: &RawSyntaxDecodedValue,
    tokens: &mut Vec<u32>,
    scope_tables: &mut Vec<u32>,
) -> Result<(), RawSyntaxError> {
    let EncodeModel {
        facts,
        roles,
        menu_bindings,
    } = model;
    let (node_kind, alt) = match value {
        RawSyntaxDecodedValue::Node(child) => {
            let alt = facts
                .mixed_field_alternative(owner_rule, owner_field, &child.node_kind)
                .ok_or_else(|| {
                    encode_error(
                        "encode_unknown_alternative",
                        format!(
                            "node_kind {} is not a mixed alternative of field {}.{}",
                            child.node_kind, owner_rule, owner_field
                        ),
                        tokens.len(),
                        Some(owner_rule.to_string()),
                        Some(owner_field.to_string()),
                    )
                })?;
            if alt.body_kind != AlternativeBodyKind::ChildRuleDescent {
                return Err(encode_error(
                    "encode_value_kind_mismatch",
                    format!(
                        "mixed field {}.{} alternative {} expects a payload Terminal, got Nonterminal Node",
                        owner_rule, owner_field, child.node_kind
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                ));
            }
            (child.node_kind.as_str(), alt)
        }
        RawSyntaxDecodedValue::Terminal(terminal) => {
            let alt = facts
                .mixed_field_alternative(owner_rule, owner_field, &terminal.node_kind)
                .ok_or_else(|| {
                    encode_error(
                        "encode_unknown_alternative",
                        format!(
                            "terminal node_kind {} is not a mixed alternative of field {}.{}",
                            terminal.node_kind, owner_rule, owner_field
                        ),
                        tokens.len(),
                        Some(owner_rule.to_string()),
                        Some(owner_field.to_string()),
                    )
                })?;
            if alt.body_kind == AlternativeBodyKind::ChildRuleDescent {
                return Err(encode_error(
                    "encode_value_kind_mismatch",
                    format!(
                        "mixed field {}.{} alternative {} expects a Nonterminal Node, got Terminal",
                        owner_rule, owner_field, terminal.node_kind
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                ));
            }
            (terminal.node_kind.as_str(), alt)
        }
    };

    // SharedAlphabet: field-relative CHOICE(n,k); PerGrammar: global map.
    let choice = roles
        .alternative_token_for_field(owner_rule, owner_field, node_kind)
        .ok_or_else(|| {
            encode_error(
                "encode_missing_alt_token",
                format!(
                    "missing mixed-field choice token for {owner_rule}.{owner_field} alternative {node_kind}"
                ),
                tokens.len(),
                Some(owner_rule.to_string()),
                Some(owner_field.to_string()),
            )
        })?;
    tokens.push(choice);

    match alt.body_kind {
        AlternativeBodyKind::ChildRuleDescent => {
            let RawSyntaxDecodedValue::Node(child) = value else {
                unreachable!("ChildRuleDescent checked against Node above");
            };
            let target_rule = alt.target_rule.as_deref().ok_or_else(|| {
                encode_error(
                    "encode_missing_target_rule",
                    format!(
                        "mixed Nonterminal alternative {node_kind} on {owner_rule}.{owner_field} has no target_rule"
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                )
            })?;
            if child.rule_name != target_rule {
                return Err(encode_error(
                    "encode_node_mismatch",
                    format!(
                        "child rule {} does not match mixed alternative target_rule {target_rule} for {node_kind}",
                        child.rule_name
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                ));
            }
            let record = facts.records.get(target_rule).ok_or_else(|| {
                encode_error(
                    "encode_unknown_rule",
                    format!("mixed alternative target_rule {target_rule} is not a record"),
                    tokens.len(),
                    Some(target_rule.to_string()),
                    Some(owner_field.to_string()),
                )
            })?;
            // Choice token *is* the child node_start; only the record body remains.
            encode_record_body(
                EncodeModel {
                    facts,
                    roles,
                    menu_bindings,
                },
                record,
                child,
                tokens,
                scope_tables,
            )
        }
        AlternativeBodyKind::BooleanPayload => {
            // Choice alone completes; zero payload bytes. Explicit true/false only.
            if node_kind != "true" && node_kind != "false" {
                return Err(encode_error(
                    "encode_invalid_boolean",
                    format!(
                        "mixed BooleanPayload node_kind {node_kind:?} on {owner_rule}.{owner_field} is not an explicit true/false identity"
                    ),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                ));
            }
            Ok(())
        }
        AlternativeBodyKind::IntegerPayload => {
            let RawSyntaxDecodedValue::Terminal(terminal) = value else {
                unreachable!("IntegerPayload checked against Terminal above");
            };
            // No generic terminal: choice already opened the payload substate.
            if let Some(cardinality) =
                menu_bindings.resolve_cardinality(owner_rule, owner_field, scope_tables)
            {
                if cardinality == 0 {
                    return Err(encode_error(
                        "encode_menu_scope_empty",
                        format!(
                            "menu site {owner_rule}.{owner_field} has empty scope during encode"
                        ),
                        tokens.len(),
                        Some(owner_rule.to_string()),
                        Some(owner_field.to_string()),
                    ));
                }
                let ordinal = emit_menu_index_payload(
                    roles,
                    &terminal.payload,
                    MenuEmitSite {
                        kind: menu_bindings.site_kind(owner_rule, owner_field),
                        scope_tables,
                        cardinality,
                        max_menu_digits: menu_bindings.max_menu_digits,
                        owner_rule,
                        owner_field,
                    },
                    tokens,
                )?;
                if matches!(
                    menu_bindings.site_kind(owner_rule, owner_field),
                    Some(MenuSiteKind::Fixed {
                        pushes_scope: true,
                        ..
                    })
                ) {
                    push_scope_table_encode(
                        scope_tables,
                        ordinal,
                        tokens.len(),
                        owner_rule,
                        owner_field,
                    )?;
                }
            } else {
                emit_u8_integer_payload_bytes(
                    roles,
                    &terminal.payload,
                    tokens,
                    owner_rule,
                    owner_field,
                )?;
            }
            tokens.push(roles.payload_end);
            Ok(())
        }
        AlternativeBodyKind::Utf8Payload => {
            let RawSyntaxDecodedValue::Terminal(terminal) = value else {
                unreachable!("Utf8Payload checked against Terminal above");
            };
            emit_utf8_payload_bytes(roles, &terminal.payload, tokens, owner_rule, owner_field)?;
            tokens.push(roles.payload_end);
            Ok(())
        }
    }
}

/// Push a table menu ordinal onto the encode-side scope, rejecting duplicates.
///
/// Pre-M6 there is no occurrence alias identity; repeated table ordinals make
/// rendered qualifiers non-injective and break **P8** render fixpoint.
fn push_scope_table_encode(
    scope_tables: &mut Vec<u32>,
    ordinal: u32,
    token_index: usize,
    owner_rule: &str,
    owner_field: &str,
) -> Result<(), RawSyntaxError> {
    if scope_tables.contains(&ordinal) {
        return Err(encode_error(
            "encode_duplicate_scope_table",
            format!(
                "table menu ordinal {ordinal} is already in scope during encode of {owner_rule}.{owner_field}; repeated table ordinals are rejected until occurrence aliases (M6)"
            ),
            token_index,
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    }
    scope_tables.push(ordinal);
    Ok(())
}

/// Menu-site context threaded into menu emission (keeps helper arity low).
struct MenuEmitSite<'a> {
    /// Bound site kind; `FactoredScopeColumns` selects the two-digit path.
    kind: Option<&'a MenuSiteKind>,
    /// Live table scope, needed to decompose a factored column ordinal.
    scope_tables: &'a [u32],
    /// Live exclusive cardinality of the flat ordinal space.
    cardinality: u32,
    /// Per-binding radix digit budget.
    max_menu_digits: u32,
    owner_rule: &'a str,
    owner_field: &'a str,
}

/// Emit fixed-width big-endian menu-band digit tokens for a bound menu site
/// (**P4**). Width is `menu_digit_width(cardinality, band_width)` so N ≤ base
/// stays a single token (bit-identical to historical emission) and N > base
/// emits multiple digits when bind-time `max_menu_digits` permitted the site.
///
/// Fail-closed when `menu_digit_width > max_menu_digits`: live cardinality must
/// not need more digits than the binding budget. Bind time
/// (`CompiledSyntaxModel::with_menu_bindings`) already rejects sites whose
/// band limit exceeds `band_width.pow(max_menu_digits)`; this check mirrors that
/// capacity message so an invariant slip yields a clear capacity error rather
/// than a confusing incomplete-ordinal decode failure.
/// Returns the accepted ordinal so callers can push table sites onto scope.
fn emit_menu_index_payload(
    roles: &SyntaxTokenRoles,
    payload_text: &str,
    site: MenuEmitSite<'_>,
    tokens: &mut Vec<u32>,
) -> Result<u32, RawSyntaxError> {
    let MenuEmitSite {
        kind: site_kind,
        scope_tables,
        cardinality,
        max_menu_digits,
        owner_rule,
        owner_field,
    } = site;
    let Some(menu_range) = roles.menu_range.as_ref() else {
        return Err(encode_error(
            "encode_menu_layout",
            format!("menu site {owner_rule}.{owner_field} requires menu_index_payload layout role"),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    };
    if let Some(MenuSiteKind::FactoredScopeColumns { columns_per_table }) = site_kind {
        return emit_factored_menu_index_payload(
            menu_range,
            payload_text,
            columns_per_table,
            scope_tables,
            tokens,
            owner_rule,
            owner_field,
        );
    }
    let ordinal: u32 = payload_text.parse().map_err(|_| {
        encode_error(
            "encode_invalid_menu_index",
            format!(
                "menu site {owner_rule}.{owner_field} payload {payload_text:?} is not a decimal ordinal"
            ),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        )
    })?;
    if ordinal >= cardinality {
        return Err(encode_error(
            "encode_menu_out_of_range",
            format!(
                "menu index {ordinal} is out of bound menu [0, {cardinality}) for {owner_rule}.{owner_field}"
            ),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    }
    let base = menu_range.end_exclusive.saturating_sub(menu_range.start);
    if base == 0 {
        return Err(encode_error(
            "encode_menu_band_overflow",
            format!("menu band has zero width for {owner_rule}.{owner_field}"),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    }
    let width = menu_digit_width(cardinality, base);
    // Mirror bind-time capacity: capacity = band_width.pow(max_menu_digits).
    // Default max_menu_digits=1 keeps the historical single-token ceiling.
    let cap = max_menu_digits.max(1);
    if width > cap {
        let capacity = (base as u64).saturating_pow(cap);
        return Err(encode_error(
            "encode_menu_capacity",
            format!(
                "menu site {owner_rule}.{owner_field} cardinality {cardinality} exceeds menu capacity {capacity} (band width {base}, max_menu_digits {cap})"
            ),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    }
    // Big-endian base-B digits; at W=1 this is a single token (ordinal).
    for k in 0..width {
        let place = (base as u64).pow(width - 1 - k);
        let digit = ((ordinal as u64) / place) % (base as u64);
        let digit_u32 = digit as u32;
        let token = menu_range.start.saturating_add(digit_u32);
        if token >= menu_range.end_exclusive {
            return Err(encode_error(
                "encode_menu_band_overflow",
                format!(
                    "menu digit {digit_u32} of ordinal {ordinal} falls outside menu band for {owner_rule}.{owner_field}"
                ),
                tokens.len(),
                Some(owner_rule.to_string()),
                Some(owner_field.to_string()),
            ));
        }
        tokens.push(token);
    }
    Ok(ordinal)
}

/// Emit a factored column address: the catalog table digit, then the column
/// digit inside that table.
///
/// The AST still carries the **flat scope-relative ordinal**, so encode
/// decomposes it against the live scope with the same helper the decoder uses
/// to recompose it. Fails closed when the ordinal is not addressable in the
/// live scope or when either digit does not fit the menu band, rather than
/// emitting a pair the mask would reject. Returns the flat ordinal so the
/// caller's accounting is unchanged.
fn emit_factored_menu_index_payload(
    menu_range: &ProjectionTokenLayoutRangeSummary,
    payload_text: &str,
    columns_per_table: &[u32],
    scope_tables: &[u32],
    tokens: &mut Vec<u32>,
    owner_rule: &str,
    owner_field: &str,
) -> Result<u32, RawSyntaxError> {
    let ordinal: u32 = payload_text.parse().map_err(|_| {
        encode_error(
            "encode_invalid_menu_index",
            format!(
                "menu site {owner_rule}.{owner_field} payload {payload_text:?} is not a decimal ordinal"
            ),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        )
    })?;
    let Some((table, column)) =
        factored_digits_for_flat_ordinal(ordinal, scope_tables, columns_per_table)
    else {
        return Err(encode_error(
            "encode_menu_out_of_range",
            format!(
                "factored column menu index {ordinal} is not addressable in scope {scope_tables:?} for {owner_rule}.{owner_field}"
            ),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    };
    for digit in [table, column] {
        let token = menu_range.start.saturating_add(digit);
        if token >= menu_range.end_exclusive {
            return Err(encode_error(
                "encode_menu_band_overflow",
                format!(
                    "factored column digit {digit} of ordinal {ordinal} falls outside menu band for {owner_rule}.{owner_field}"
                ),
                tokens.len(),
                Some(owner_rule.to_string()),
                Some(owner_field.to_string()),
            ));
        }
        tokens.push(token);
    }
    Ok(ordinal)
}

/// Emit the integer value token(s) for a u8 integer payload band (no bracketing
/// terminal / payload_end). Variable-width escape multi-digit on the existing
/// 256-wide band — see [`super::integer_codec`].
fn emit_u8_integer_payload_bytes(
    roles: &SyntaxTokenRoles,
    payload_text: &str,
    tokens: &mut Vec<u32>,
    owner_rule: &str,
    owner_field: &str,
) -> Result<(), RawSyntaxError> {
    if roles.integer_range.role != "u8_integer_payload" {
        return Err(encode_error(
            "encode_integer_layout",
            format!(
                "integer encode on {owner_rule}.{owner_field} requires u8_integer_payload layout, got {}",
                roles.integer_range.role
            ),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        ));
    }
    let value = super::integer_codec::parse_integer_payload_text(payload_text).map_err(|msg| {
        encode_error(
            "encode_invalid_integer",
            format!("{msg} on {owner_rule}.{owner_field}"),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        )
    })?;
    let digits = super::integer_codec::encode_integer_digits(value).map_err(|msg| {
        encode_error(
            "encode_integer_capacity",
            format!("{msg} on {owner_rule}.{owner_field}"),
            tokens.len(),
            Some(owner_rule.to_string()),
            Some(owner_field.to_string()),
        )
    })?;
    for digit in digits {
        let token = roles
            .integer_range
            .start
            .checked_add(u32::from(digit))
            .ok_or_else(|| {
                encode_error(
                    "encode_integer_overflow",
                    format!("integer payload token overflow on {owner_rule}.{owner_field}"),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                )
            })?;
        if token >= roles.integer_range.end_exclusive {
            return Err(encode_error(
                "encode_integer_out_of_range",
                format!(
                    "integer payload token {token} outside range on {owner_rule}.{owner_field}"
                ),
                tokens.len(),
                Some(owner_rule.to_string()),
                Some(owner_field.to_string()),
            ));
        }
        tokens.push(token);
    }
    Ok(())
}

/// Emit UTF-8 byte tokens (no bracketing terminal / payload_end). Same algorithm
/// as `CompiledSubsetCodecCore::emit_utf8_payload` without the terminal push.
fn emit_utf8_payload_bytes(
    roles: &SyntaxTokenRoles,
    content: &str,
    tokens: &mut Vec<u32>,
    owner_rule: &str,
    owner_field: &str,
) -> Result<(), RawSyntaxError> {
    for byte in content.as_bytes() {
        let token = roles
            .utf8_range
            .start
            .checked_add(u32::from(*byte))
            .ok_or_else(|| {
                encode_error(
                    "encode_utf8_overflow",
                    format!("utf8 payload token overflow on {owner_rule}.{owner_field}"),
                    tokens.len(),
                    Some(owner_rule.to_string()),
                    Some(owner_field.to_string()),
                )
            })?;
        if token >= roles.utf8_range.end_exclusive {
            return Err(encode_error(
                "encode_utf8_out_of_range",
                format!("utf8 payload token {token} outside range on {owner_rule}.{owner_field}"),
                tokens.len(),
                Some(owner_rule.to_string()),
                Some(owner_field.to_string()),
            ));
        }
        tokens.push(token);
    }
    Ok(())
}

fn is_boolean_payload_kind(payload_kind: &str) -> bool {
    matches!(payload_kind, "boolean" | "boolean_payload")
        || payload_kind.contains(":boolean_payload")
}

fn is_integer_payload_kind(payload_kind: &str) -> bool {
    matches!(payload_kind, "integer" | "integer_payload")
        || payload_kind.contains(":integer_payload")
}

fn is_utf8_payload_kind(payload_kind: &str) -> bool {
    matches!(
        payload_kind,
        "string" | "string_payload" | "identifier" | "identifier_payload"
    ) || payload_kind.contains(":string_payload")
        || payload_kind.contains(":identifier_payload")
}

fn encode_error(
    kind: &'static str,
    message: impl Into<String>,
    consumed_raw_tokens: usize,
    active_rule: Option<String>,
    active_field: Option<String>,
) -> RawSyntaxError {
    RawSyntaxError {
        kind,
        message: message.into(),
        consumed_raw_tokens,
        failing_token: None,
        stack_depth: 0,
        active_rule,
        active_field,
        expected_payload_class: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::grammar_ir::{
        GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
        GrammarTerminalKind,
    };
    use crate::grammar_ir_structural_model::{
        GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
    };
    use crate::syntax_state_machine::{
        CompiledSyntaxModel, RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedTerminal,
        RawSyntaxDecodedValue,
    };
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    fn toy_layout() -> GrammarStructuralTokenLayout {
        GrammarStructuralTokenLayout::from_toy_assignment(
            &ToyGrammarSpec::assignment_language_v1().layout,
        )
    }

    fn compile_grammar(grammar: &GrammarIr) -> CompiledSyntaxModel {
        let sidecar = lower_grammar_ir_to_structural_model(grammar, toy_layout())
            .expect("grammar should lower to StructuralModel");
        CompiledSyntaxModel::from_sidecar(&sidecar).expect("model should compile")
    }

    fn field_less_leaf_model() -> CompiledSyntaxModel {
        let mut grammar = GrammarIr::new(
            "generic_encode_field_less_leaf_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
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
        compile_grammar(&grammar)
    }

    fn assert_stream_tree_inverses(model: &CompiledSyntaxModel, tree: &RawSyntaxDecodedNode) {
        let tokens = model
            .encode_complete_tree(tree)
            .expect("encode should succeed");
        let decoded = model
            .decode_complete_tree(&tokens)
            .expect("decode should accept encoded stream");
        let tree2 = decoded
            .decoded_node_tree()
            .expect("decoded node tree should materialize");
        assert_eq!(&tree2, tree, "decode(encode(tree)) must equal tree");

        let reencoded = model
            .encode_complete_tree(&tree2)
            .expect("re-encode should succeed");
        assert_eq!(
            reencoded, tokens,
            "encode(decode(tokens)) must equal tokens"
        );
    }

    #[test]
    fn encode_field_less_leaf_round_trips_stream_and_tree() {
        let model = field_less_leaf_model();
        let roles = model.token_roles();
        let expected_tokens = vec![roles.node_start, roles.node_end];

        let tree = RawSyntaxDecodedNode {
            rule_name: "leaf".to_string(),
            node_kind: "leaf".to_string(),
            fields: vec![],
        };
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("field-less leaf encode");
        assert_eq!(tokens, expected_tokens);

        let decoded = model
            .decode_complete_tree(&tokens)
            .expect("decode field-less stream");
        let tree2 = decoded.decoded_node_tree().expect("tree");
        assert_eq!(tree2, tree);

        // Stream → tree → stream identity from a hand-built complete stream.
        let from_stream = model
            .decode_complete_tree(&expected_tokens)
            .expect("hand stream")
            .decoded_node_tree()
            .expect("tree from stream");
        let reencoded = model.encode_complete_tree(&from_stream).expect("re-encode");
        assert_eq!(reencoded, expected_tokens);

        assert_stream_tree_inverses(&model, &tree);
    }

    #[test]
    fn encode_rejects_root_rule_mismatch() {
        let model = field_less_leaf_model();
        let err = model
            .encode_complete_tree(&RawSyntaxDecodedNode {
                rule_name: "other".to_string(),
                node_kind: "leaf".to_string(),
                fields: vec![],
            })
            .expect_err("root rule mismatch must fail closed");
        assert_eq!(err.kind, "encode_root_mismatch");
        assert!(err.message.contains("other") || err.active_rule.as_deref() == Some("other"));
    }

    fn single_alt_child_model() -> CompiledSyntaxModel {
        let mut grammar = GrammarIr::new(
            "generic_encode_single_alt_child_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
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
                name: "body".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "leaf".to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "leaf".to_string(),
            node_kind: "leaf".to_string(),
            fields: vec![],
        });
        compile_grammar(&grammar)
    }

    #[test]
    fn encode_single_alt_nonterminal_child_with_field_brackets() {
        let model = single_alt_child_model();
        let roles = model.token_roles();
        let tree = RawSyntaxDecodedNode {
            rule_name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "body".to_string(),
                values: vec![RawSyntaxDecodedValue::Node(Box::new(
                    RawSyntaxDecodedNode {
                        rule_name: "leaf".to_string(),
                        node_kind: "leaf".to_string(),
                        fields: vec![],
                    },
                ))],
                missing: false,
                repeated_empty: false,
            }],
        };

        let tokens = model
            .encode_complete_tree(&tree)
            .expect("encode single-alt");
        // wrapper open, field_start, plain node_start for leaf, leaf end, field_end, wrapper end.
        assert_eq!(
            tokens,
            vec![
                roles.node_start,
                roles.field_start,
                roles.node_start, // single-alt child uses plain node_start
                roles.node_end,
                roles.field_end,
                roles.node_end,
            ]
        );
        assert_stream_tree_inverses(&model, &tree);
    }

    fn multi_alt_child_model() -> CompiledSyntaxModel {
        let mut grammar = GrammarIr::new(
            "generic_encode_multi_alt_child_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
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
                name: "body".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![
                    GrammarFieldAlternative {
                        node_kind: "alpha".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: "beta".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                ],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "alpha".to_string(),
            node_kind: "alpha".to_string(),
            fields: vec![],
        });
        grammar.rules.push(GrammarRule {
            name: "beta".to_string(),
            node_kind: "beta".to_string(),
            fields: vec![],
        });
        compile_grammar(&grammar)
    }

    #[test]
    fn encode_multi_alt_nonterminal_uses_alternative_node_start_tokens() {
        let model = multi_alt_child_model();
        let roles = model.token_roles();
        let alpha_token = *roles
            .node_start_alternative_tokens
            .get("alpha")
            .expect("alpha alt token");
        let beta_token = *roles
            .node_start_alternative_tokens
            .get("beta")
            .expect("beta alt token");
        assert_ne!(alpha_token, beta_token);
        assert_ne!(alpha_token, roles.node_start);
        assert_ne!(beta_token, roles.node_start);

        for (kind, alt_token) in [("alpha", alpha_token), ("beta", beta_token)] {
            let tree = RawSyntaxDecodedNode {
                rule_name: "wrapper".to_string(),
                node_kind: "wrapper".to_string(),
                fields: vec![RawSyntaxDecodedField {
                    name: "body".to_string(),
                    values: vec![RawSyntaxDecodedValue::Node(Box::new(
                        RawSyntaxDecodedNode {
                            rule_name: kind.to_string(),
                            node_kind: kind.to_string(),
                            fields: vec![],
                        },
                    ))],
                    missing: false,
                    repeated_empty: false,
                }],
            };
            let tokens = model
                .encode_complete_tree(&tree)
                .unwrap_or_else(|e| panic!("encode multi-alt {kind}: {e:?}"));
            assert_eq!(
                tokens,
                vec![
                    roles.node_start,
                    roles.field_start,
                    alt_token,
                    roles.node_end,
                    roles.field_end,
                    roles.node_end,
                ],
                "multi-alt child {kind} must open with alt-specific node_start"
            );
            assert_stream_tree_inverses(&model, &tree);
        }
    }

    fn integer_payload_model() -> CompiledSyntaxModel {
        let mut grammar = GrammarIr::new(
            "generic_encode_integer_payload_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "number".to_string(),
            node_kind: "number".to_string(),
            fields: vec![GrammarField {
                name: "value".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "int".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                }],
            }],
        });
        compile_grammar(&grammar)
    }

    fn utf8_payload_model() -> CompiledSyntaxModel {
        let mut grammar = GrammarIr::new(
            "generic_encode_utf8_payload_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "string".to_string(),
            node_kind: "string".to_string(),
            fields: vec![GrammarField {
                name: "content".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "text".to_string(),
                    terminal: GrammarTerminalKind::StringPayload,
                }],
            }],
        });
        compile_grammar(&grammar)
    }

    #[test]
    fn encode_homogeneous_integer_payload_round_trips() {
        let model = integer_payload_model();
        let roles = model.token_roles();
        assert_eq!(roles.integer_range.role, "u8_integer_payload");
        let tree = RawSyntaxDecodedNode {
            rule_name: "number".to_string(),
            node_kind: "number".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "value".to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "int".to_string(),
                    terminal: "integer".to_string(),
                    payload: "42".to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        let tokens = model.encode_complete_tree(&tree).expect("encode integer");
        assert_eq!(
            tokens,
            vec![
                roles.node_start,
                roles.field_start,
                roles.terminal,
                roles.integer_range.start + 42,
                roles.payload_end,
                roles.field_end,
                roles.node_end,
            ]
        );
        assert_stream_tree_inverses(&model, &tree);
    }

    #[test]
    fn encode_homogeneous_utf8_payload_including_empty() {
        let model = utf8_payload_model();
        let roles = model.token_roles();

        for content in ["", "hi", "π"] {
            // Decode materializes terminal.terminal from payload_kind ("string").
            // Round-trip through decode first to get the canonical terminal label.
            let mut hand = vec![roles.node_start, roles.field_start, roles.terminal];
            for byte in content.as_bytes() {
                hand.push(roles.utf8_range.start + u32::from(*byte));
            }
            hand.push(roles.payload_end);
            hand.push(roles.field_end);
            hand.push(roles.node_end);
            let tree = model
                .decode_complete_tree(&hand)
                .expect("hand utf8 stream")
                .decoded_node_tree()
                .expect("tree");
            let tokens = model
                .encode_complete_tree(&tree)
                .unwrap_or_else(|e| panic!("encode utf8 {content:?}: {e:?}"));
            assert_eq!(tokens, hand, "utf8 content {content:?}");
            assert_stream_tree_inverses(&model, &tree);
        }
    }

    fn optional_and_repeated_model() -> CompiledSyntaxModel {
        let mut grammar = GrammarIr::new(
            "generic_encode_optional_repeated_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
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
                    name: "opt".to_string(),
                    required: false,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: "leaf".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
                // repeated_empty is only legal on optional repeated fields
                // (raw_replay rejects it when field.required).
                GrammarField {
                    name: "items".to_string(),
                    required: false,
                    multiple: true,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: "leaf".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
            ],
        });
        grammar.rules.push(GrammarRule {
            name: "leaf".to_string(),
            node_kind: "leaf".to_string(),
            fields: vec![],
        });
        compile_grammar(&grammar)
    }

    #[test]
    fn encode_optional_field_missing_and_repeated_empty() {
        let model = optional_and_repeated_model();
        let roles = model.token_roles();
        let tree = RawSyntaxDecodedNode {
            rule_name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                RawSyntaxDecodedField {
                    name: "opt".to_string(),
                    values: vec![],
                    missing: true,
                    repeated_empty: false,
                },
                RawSyntaxDecodedField {
                    name: "items".to_string(),
                    values: vec![],
                    missing: false,
                    repeated_empty: true,
                },
            ],
        };
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("encode missing/empty");
        assert_eq!(
            tokens,
            vec![
                roles.node_start,
                roles.field_start,
                roles.field_missing,
                roles.field_end,
                roles.field_start,
                roles.repeated_empty,
                roles.field_end,
                roles.node_end,
            ]
        );
        assert_stream_tree_inverses(&model, &tree);

        // Optional field omitted entirely from the tree is also field_missing.
        let tree_omitted = RawSyntaxDecodedNode {
            rule_name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "items".to_string(),
                values: vec![],
                missing: false,
                repeated_empty: true,
            }],
        };
        let tokens_omitted = model
            .encode_complete_tree(&tree_omitted)
            .expect("omitted optional");
        assert_eq!(tokens_omitted, tokens);
    }

    #[test]
    fn encode_rejects_missing_true_on_required_field() {
        // Required field with missing=true must fail closed (decode rejects
        // field_missing on required fields). optional_and_repeated_model has
        // only optional fields; build a one-required-field model.
        let mut grammar = GrammarIr::new(
            "generic_encode_required_missing_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
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
                name: "child".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "leaf".to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "leaf".to_string(),
            node_kind: "leaf".to_string(),
            fields: vec![],
        });
        let model = compile_grammar(&grammar);
        let tree = RawSyntaxDecodedNode {
            rule_name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "child".to_string(),
                values: vec![],
                missing: true,
                repeated_empty: false,
            }],
        };
        let err = model
            .encode_complete_tree(&tree)
            .expect_err("required field with missing=true must not encode");
        assert_eq!(err.kind, "encode_missing_field");
        assert!(
            err.message.contains("required field wrapper.child"),
            "diagnostic must name rule and field: {}",
            err.message
        );
    }

    #[test]
    fn encode_mixed_payload_witness_all_body_kinds() {
        use super::super::model_facts::mixed_payload_witness::mixed_payload_sidecar;

        let model = CompiledSyntaxModel::from_sidecar(&mixed_payload_sidecar())
            .expect("mixed witness should compile");
        let roles = model.token_roles();

        let choice = |kind: &str| {
            *roles
                .node_start_alternative_tokens
                .get(kind)
                .unwrap_or_else(|| panic!("missing choice for {kind}"))
        };

        // Boolean true: choice only.
        let bool_stream = vec![
            roles.node_start,
            roles.field_start,
            choice("true"),
            roles.field_end,
            roles.node_end,
        ];
        assert!(
            !bool_stream.contains(&roles.terminal),
            "mixed boolean must never emit generic terminal"
        );
        let bool_tree = model
            .decode_complete_tree(&bool_stream)
            .expect("bool stream")
            .decoded_node_tree()
            .expect("bool tree");
        let encoded_bool = model.encode_complete_tree(&bool_tree).expect("encode bool");
        assert_eq!(encoded_bool, bool_stream);
        assert!(!encoded_bool.contains(&roles.terminal));
        assert_stream_tree_inverses(&model, &bool_tree);

        // Boolean false.
        let false_stream = vec![
            roles.node_start,
            roles.field_start,
            choice("false"),
            roles.field_end,
            roles.node_end,
        ];
        let false_tree = model
            .decode_complete_tree(&false_stream)
            .expect("false stream")
            .decoded_node_tree()
            .expect("false tree");
        assert_eq!(
            model
                .encode_complete_tree(&false_tree)
                .expect("encode false"),
            false_stream
        );

        // Integer: choice + value + payload_end (no terminal).
        let int_stream = vec![
            roles.node_start,
            roles.field_start,
            choice("number"),
            roles.integer_range.start + 7,
            roles.payload_end,
            roles.field_end,
            roles.node_end,
        ];
        assert!(!int_stream.contains(&roles.terminal));
        let int_tree = model
            .decode_complete_tree(&int_stream)
            .expect("int stream")
            .decoded_node_tree()
            .expect("int tree");
        let encoded_int = model.encode_complete_tree(&int_tree).expect("encode int");
        assert_eq!(encoded_int, int_stream);
        assert!(!encoded_int.contains(&roles.terminal));
        assert_stream_tree_inverses(&model, &int_tree);

        // Utf8: choice + bytes + payload_end (no terminal).
        let text = "ab";
        let mut utf8_stream = vec![roles.node_start, roles.field_start, choice("text")];
        for byte in text.as_bytes() {
            utf8_stream.push(roles.utf8_range.start + u32::from(*byte));
        }
        utf8_stream.push(roles.payload_end);
        utf8_stream.push(roles.field_end);
        utf8_stream.push(roles.node_end);
        assert!(!utf8_stream.contains(&roles.terminal));
        let utf8_tree = model
            .decode_complete_tree(&utf8_stream)
            .expect("utf8 stream")
            .decoded_node_tree()
            .expect("utf8 tree");
        let encoded_utf8 = model.encode_complete_tree(&utf8_tree).expect("encode utf8");
        assert_eq!(encoded_utf8, utf8_stream);
        assert!(!encoded_utf8.contains(&roles.terminal));
        assert_stream_tree_inverses(&model, &utf8_tree);

        // Nonterminal descent: choice (= child node_start) + child body.
        let child_stream = vec![
            roles.node_start,
            roles.field_start,
            choice("child"),
            roles.node_end, // empty child
            roles.field_end,
            roles.node_end,
        ];
        assert!(!child_stream.contains(&roles.terminal));
        let child_tree = model
            .decode_complete_tree(&child_stream)
            .expect("child stream")
            .decoded_node_tree()
            .expect("child tree");
        let encoded_child = model
            .encode_complete_tree(&child_tree)
            .expect("encode child");
        assert_eq!(encoded_child, child_stream);
        assert_stream_tree_inverses(&model, &child_tree);
    }

    /// Encode fail-closes with a capacity-shaped error when live cardinality
    /// needs more digits than `max_menu_digits`. Bind time already rejects this
    /// for attached models; this unit path exercises the encode guard directly
    /// so a future invariant slip cannot emit more digits than decode accepts.
    #[test]
    fn emit_menu_index_payload_fails_closed_when_width_exceeds_max_menu_digits() {
        let sidecar = crate::sql_schema_linking::grammar::lower_sql_schema_linking_grammar()
            .expect("sql grammar lowers");
        let model = CompiledSyntaxModel::from_sidecar(&sidecar).expect("sql model compiles");
        let roles = model.token_roles();
        let menu = roles
            .menu_range
            .as_ref()
            .expect("SQL layout declares menu_index_payload");
        let base = menu.end_exclusive.saturating_sub(menu.start);
        assert_eq!(base, 256, "SQL menu band width is 256");

        // N=257 needs W=2 over base 256; budget 1 must reject before emission.
        let mut tokens = Vec::new();
        let err = super::emit_menu_index_payload(
            roles,
            "0",
            super::MenuEmitSite {
                kind: None,
                scope_tables: &[],
                cardinality: 257,
                max_menu_digits: 1,
                owner_rule: "table_ref",
                owner_field: "menu_index",
            },
            &mut tokens,
        )
        .expect_err("cardinality 257 under max_menu_digits=1 must fail closed");
        assert_eq!(err.kind, "encode_menu_capacity");
        assert!(
            err.message.contains("exceeds menu capacity")
                && err.message.contains("band width 256")
                && err.message.contains("max_menu_digits 1")
                && err.message.contains("257"),
            "capacity message must name band width and max_menu_digits: {}",
            err.message
        );
        assert!(
            tokens.is_empty(),
            "failed capacity check must not emit digit tokens"
        );

        // Same cardinality with budget 2 emits W=2 (and does not change W=1
        // historical paths; this only unlocks multi-digit when permitted).
        let mut tokens_ok = Vec::new();
        let ordinal = super::emit_menu_index_payload(
            roles,
            "0",
            super::MenuEmitSite {
                kind: None,
                scope_tables: &[],
                cardinality: 257,
                max_menu_digits: 2,
                owner_rule: "table_ref",
                owner_field: "menu_index",
            },
            &mut tokens_ok,
        )
        .expect("cardinality 257 under max_menu_digits=2 must encode");
        assert_eq!(ordinal, 0);
        assert_eq!(tokens_ok.len(), 2, "W=2 emits two digit tokens");

        // N ≤ base stays single-token under budget 1 (historical path).
        let mut tokens_single = Vec::new();
        let ordinal_single = super::emit_menu_index_payload(
            roles,
            "42",
            super::MenuEmitSite {
                kind: None,
                scope_tables: &[],
                cardinality: 256,
                max_menu_digits: 1,
                owner_rule: "table_ref",
                owner_field: "menu_index",
            },
            &mut tokens_single,
        )
        .expect("cardinality 256 under max_menu_digits=1 is historical single-token");
        assert_eq!(ordinal_single, 42);
        assert_eq!(tokens_single.len(), 1);
        assert_eq!(tokens_single[0], menu.start + 42);
    }
}

/// I5 checklist tranche 6 — generic encode under SharedAlphabet.
///
/// Claim boundary: `compiler_shared_alphabet_layout_only`.
///
/// Confirms `encode_complete_tree` is mode-agnostic over roles + ModelFacts
/// (no language keys): only SharedAlphabet role assignment changes absolute
/// choice IDs. Synthetic inverses: field-less leaf, multi-alt NT, mixed
/// payload witness. Default PerGrammar pins are not exercised here.
#[cfg(test)]
mod shared_alphabet_generic_encode_inverse_tests {
    use super::super::model_facts::mixed_payload_witness::mixed_payload_sidecar;
    use crate::grammar_ir::{
        GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
        GrammarTerminalKind,
    };
    use crate::grammar_ir_structural_model::{
        GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
    };
    use crate::shared_alphabet_layout::{
        SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
        shared_alphabet_choice_token_id, shared_alphabet_token_layout,
    };
    use crate::syntax_state_machine::{
        CompiledSyntaxModel, RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedTerminal,
        RawSyntaxDecodedValue,
    };
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    fn toy_layout() -> GrammarStructuralTokenLayout {
        GrammarStructuralTokenLayout::from_toy_assignment(
            &ToyGrammarSpec::assignment_language_v1().layout,
        )
    }

    fn compile_shared(grammar: &GrammarIr) -> CompiledSyntaxModel {
        let sidecar = lower_grammar_ir_to_structural_model(grammar, toy_layout())
            .expect("grammar should lower to StructuralModel");
        CompiledSyntaxModel::from_sidecar_with_mode(&sidecar, SyntaxTokenLayoutMode::SharedAlphabet)
            .expect("SharedAlphabet compile")
    }

    fn assert_stream_tree_inverses(model: &CompiledSyntaxModel, tree: &RawSyntaxDecodedNode) {
        assert_eq!(model.layout_mode(), SyntaxTokenLayoutMode::SharedAlphabet);
        let tokens = model
            .encode_complete_tree(tree)
            .expect("encode should succeed under SharedAlphabet");
        for &token in &tokens {
            assert!(
                token < model.token_roles().pad_token && token < model.token_roles().vocab_size,
                "token {token} must stay below PAD/VOCAB under shared layout"
            );
        }
        let decoded = model
            .decode_complete_tree(&tokens)
            .expect("decode should accept SharedAlphabet encoded stream");
        let tree2 = decoded
            .decoded_node_tree()
            .expect("decoded node tree should materialize");
        assert_eq!(
            &tree2, tree,
            "decode(encode(tree)) must equal tree under SharedAlphabet"
        );

        let reencoded = model
            .encode_complete_tree(&tree2)
            .expect("re-encode should succeed");
        assert_eq!(
            reencoded, tokens,
            "encode(decode(tokens)) must equal tokens under SharedAlphabet"
        );
    }

    fn field_choice(model: &CompiledSyntaxModel, rule: &str, field: &str, node_kind: &str) -> u32 {
        model
            .token_roles()
            .alternative_token_for_field(rule, field, node_kind)
            .unwrap_or_else(|| {
                panic!("SharedAlphabet field choice missing for {rule}.{field} / {node_kind}")
            })
    }

    /// Field-less leaf: only plain brackets; shared layout ranges differ from
    /// toy per-grammar packing, but stream shape is node_start + node_end.
    #[test]
    fn shared_alphabet_field_less_leaf_inverse() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );

        let mut grammar = GrammarIr::new(
            "i5_shared_alphabet_encode_field_less_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
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

        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let shared = shared_alphabet_token_layout();
        assert_eq!(roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
        assert_eq!(roles.vocab_size, shared.vocab_size);
        assert_eq!(roles.pad_token, shared.pad_token);
        assert_eq!(roles.layout_mode, SyntaxTokenLayoutMode::SharedAlphabet);

        let tree = RawSyntaxDecodedNode {
            rule_name: "leaf".to_string(),
            node_kind: "leaf".to_string(),
            fields: vec![],
        };
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("field-less leaf encode under SharedAlphabet");
        assert_eq!(tokens, vec![roles.node_start, roles.node_end]);
        assert_stream_tree_inverses(&model, &tree);
    }

    /// Homogeneous multi-alt: encode emits field-local CHOICE(n,k); inverse
    /// holds without language-keyed encode branches.
    #[test]
    fn shared_alphabet_multi_alt_nonterminal_inverse() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );

        let mut grammar = GrammarIr::new(
            "i5_shared_alphabet_encode_multi_alt_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_generic_encode".to_string(),
                source_path: Some("src/syntax_state_machine/encode.rs".to_string()),
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
                name: "body".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![
                    GrammarFieldAlternative {
                        node_kind: "alpha".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: "beta".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: "gamma".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                ],
            }],
        });
        for kind in ["alpha", "beta", "gamma"] {
            grammar.rules.push(GrammarRule {
                name: kind.to_string(),
                node_kind: kind.to_string(),
                fields: vec![],
            });
        }

        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        // Domain-stable: alpha, beta, gamma → CHOICE(3, 0..2)
        let alpha = shared_alphabet_choice_token_id(0, 3, 0).unwrap();
        let beta = shared_alphabet_choice_token_id(0, 3, 1).unwrap();
        let gamma = shared_alphabet_choice_token_id(0, 3, 2).unwrap();
        assert_eq!(field_choice(&model, "wrapper", "body", "alpha"), alpha);
        assert_eq!(field_choice(&model, "wrapper", "body", "beta"), beta);
        assert_eq!(field_choice(&model, "wrapper", "body", "gamma"), gamma);

        for (kind, choice) in [("alpha", alpha), ("beta", beta), ("gamma", gamma)] {
            let tree = RawSyntaxDecodedNode {
                rule_name: "wrapper".to_string(),
                node_kind: "wrapper".to_string(),
                fields: vec![RawSyntaxDecodedField {
                    name: "body".to_string(),
                    values: vec![RawSyntaxDecodedValue::Node(Box::new(
                        RawSyntaxDecodedNode {
                            rule_name: kind.to_string(),
                            node_kind: kind.to_string(),
                            fields: vec![],
                        },
                    ))],
                    missing: false,
                    repeated_empty: false,
                }],
            };
            let tokens = model
                .encode_complete_tree(&tree)
                .unwrap_or_else(|e| panic!("encode multi-alt {kind}: {e:?}"));
            assert_eq!(
                tokens,
                vec![
                    roles.node_start,
                    roles.field_start,
                    choice,
                    roles.node_end,
                    roles.field_end,
                    roles.node_end,
                ],
                "multi-alt {kind} must open with CHOICE(3,k) under SharedAlphabet"
            );
            assert_stream_tree_inverses(&model, &tree);
        }
    }

    /// Mixed-payload witness under SharedAlphabet: all body kinds inverse.
    /// Choice tokens are domain-stable CHOICE(5,k), not free-slot ids.
    #[test]
    fn shared_alphabet_mixed_payload_witness_inverse() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );

        let model = CompiledSyntaxModel::from_sidecar_with_mode(
            &mixed_payload_sidecar(),
            SyntaxTokenLayoutMode::SharedAlphabet,
        )
        .expect("mixed witness SharedAlphabet compile");
        let roles = model.token_roles();
        assert_eq!(roles.layout_mode, SyntaxTokenLayoutMode::SharedAlphabet);
        assert_eq!(roles.vocab_size, shared_alphabet_token_layout().vocab_size);

        // Domain-stable sort on (node_kind, terminal_class):
        // child/Nonterminal, false/Boolean, number/Integer, text/String, true/Boolean
        let child = shared_alphabet_choice_token_id(0, 5, 0).unwrap();
        let false_tok = shared_alphabet_choice_token_id(0, 5, 1).unwrap();
        let number = shared_alphabet_choice_token_id(0, 5, 2).unwrap();
        let text_tok = shared_alphabet_choice_token_id(0, 5, 3).unwrap();
        let true_tok = shared_alphabet_choice_token_id(0, 5, 4).unwrap();
        assert_eq!(field_choice(&model, "wrapper", "value", "child"), child);
        assert_eq!(field_choice(&model, "wrapper", "value", "false"), false_tok);
        assert_eq!(field_choice(&model, "wrapper", "value", "number"), number);
        assert_eq!(field_choice(&model, "wrapper", "value", "text"), text_tok);
        assert_eq!(field_choice(&model, "wrapper", "value", "true"), true_tok);

        // Boolean true: CHOICE only (no generic terminal).
        let bool_stream = vec![
            roles.node_start,
            roles.field_start,
            true_tok,
            roles.field_end,
            roles.node_end,
        ];
        assert!(!bool_stream.contains(&roles.terminal));
        let bool_tree = model
            .decode_complete_tree(&bool_stream)
            .expect("bool stream under SharedAlphabet")
            .decoded_node_tree()
            .expect("bool tree");
        assert_eq!(
            model.encode_complete_tree(&bool_tree).expect("encode bool"),
            bool_stream
        );
        assert_stream_tree_inverses(&model, &bool_tree);

        // Boolean false.
        let false_stream = vec![
            roles.node_start,
            roles.field_start,
            false_tok,
            roles.field_end,
            roles.node_end,
        ];
        let false_tree = model
            .decode_complete_tree(&false_stream)
            .expect("false stream")
            .decoded_node_tree()
            .expect("false tree");
        assert_eq!(
            model
                .encode_complete_tree(&false_tree)
                .expect("encode false"),
            false_stream
        );
        assert_stream_tree_inverses(&model, &false_tree);

        // Integer: CHOICE + value + payload_end (no terminal).
        let int_stream = vec![
            roles.node_start,
            roles.field_start,
            number,
            roles.integer_range.start + 7,
            roles.payload_end,
            roles.field_end,
            roles.node_end,
        ];
        assert!(!int_stream.contains(&roles.terminal));
        let int_tree = model
            .decode_complete_tree(&int_stream)
            .expect("int stream")
            .decoded_node_tree()
            .expect("int tree");
        assert_eq!(
            model.encode_complete_tree(&int_tree).expect("encode int"),
            int_stream
        );
        assert_stream_tree_inverses(&model, &int_tree);

        // Utf8: CHOICE + bytes + payload_end.
        let text = "ab";
        let mut utf8_stream = vec![roles.node_start, roles.field_start, text_tok];
        for byte in text.as_bytes() {
            utf8_stream.push(roles.utf8_range.start + u32::from(*byte));
        }
        utf8_stream.push(roles.payload_end);
        utf8_stream.push(roles.field_end);
        utf8_stream.push(roles.node_end);
        assert!(!utf8_stream.contains(&roles.terminal));
        let utf8_tree = model
            .decode_complete_tree(&utf8_stream)
            .expect("utf8 stream")
            .decoded_node_tree()
            .expect("utf8 tree");
        assert_eq!(
            model.encode_complete_tree(&utf8_tree).expect("encode utf8"),
            utf8_stream
        );
        assert_stream_tree_inverses(&model, &utf8_tree);

        // Nonterminal: CHOICE (= child open) + child body.
        let child_stream = vec![
            roles.node_start,
            roles.field_start,
            child,
            roles.node_end,
            roles.field_end,
            roles.node_end,
        ];
        let child_tree = model
            .decode_complete_tree(&child_stream)
            .expect("child stream")
            .decoded_node_tree()
            .expect("child tree");
        assert_eq!(
            model
                .encode_complete_tree(&child_tree)
                .expect("encode child"),
            child_stream
        );
        assert_stream_tree_inverses(&model, &child_tree);

        // Constructed mixed trees without hand streams also invert.
        let constructed_int = RawSyntaxDecodedNode {
            rule_name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "value".to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "number".to_string(),
                    terminal: "integer".to_string(),
                    payload: "42".to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        assert_stream_tree_inverses(&model, &constructed_int);
    }
}
