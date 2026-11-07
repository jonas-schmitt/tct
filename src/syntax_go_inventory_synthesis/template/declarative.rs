use super::require_non_empty;
use crate::grammar_ir::GrammarTerminalKind;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryExpectedFieldOrder, GoAstInventoryFiniteDomainRequirement,
};
use crate::syntax_go_raw_token_stream::GoRawTokenStreamBuilder;
use std::collections::BTreeSet;

mod checks;
pub use self::checks::check_go_ast_inventory_declarative_expression_spec_from_inventory;

pub const GO_AST_INVENTORY_DECLARATIVE_EXPRESSION_SPEC_KIND: &str = "DeclarativeExpressionSpec";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryDeclarativeExpressionSpec {
    pub root: GoAstInventoryDeclarativeNodeSpec,
}

impl GoAstInventoryDeclarativeExpressionSpec {
    pub fn new(root: GoAstInventoryDeclarativeNodeSpec) -> Self {
        Self { root }
    }

    pub fn root_node_kind(&self) -> &str {
        &self.root.node_kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryDeclarativeNodeSpec {
    pub node_kind: String,
    pub fields: Vec<GoAstInventoryDeclarativeFieldSpec>,
}

impl GoAstInventoryDeclarativeNodeSpec {
    pub fn new(node_kind: &str, fields: Vec<GoAstInventoryDeclarativeFieldSpec>) -> Self {
        Self {
            node_kind: node_kind.to_string(),
            fields,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryDeclarativeFieldSpec {
    pub field_name: String,
    pub value: GoAstInventoryDeclarativeFieldValue,
}

impl GoAstInventoryDeclarativeFieldSpec {
    pub fn new(field_name: &str, value: GoAstInventoryDeclarativeFieldValue) -> Self {
        Self {
            field_name: field_name.to_string(),
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoAstInventoryDeclarativeFieldValue {
    Missing,
    TerminalPayload {
        selected_alternative: Option<String>,
        terminal_kind: GrammarTerminalKind,
        payload: String,
    },
    BooleanPayload {
        selected_alternative: Option<String>,
        value: bool,
    },
    AlternativeNode(Box<GoAstInventoryDeclarativeNodeSpec>),
    GenericNode(Box<GoAstInventoryDeclarativeNodeSpec>),
    OptionalNode(Option<Box<GoAstInventoryDeclarativeNodeSpec>>),
    OptionalAlternativeNode(Option<Box<GoAstInventoryDeclarativeNodeSpec>>),
    RepeatedAlternativeNodes(Vec<GoAstInventoryDeclarativeNodeSpec>),
    RepeatedGenericNodes(Vec<GoAstInventoryDeclarativeNodeSpec>),
}

impl GoAstInventoryDeclarativeFieldValue {
    pub fn terminal_payload(payload: &str) -> Self {
        Self::terminal_payload_with_kind(GrammarTerminalKind::IdentifierPayload, payload)
    }

    pub fn terminal_payload_with_kind(terminal_kind: GrammarTerminalKind, payload: &str) -> Self {
        Self::TerminalPayload {
            selected_alternative: None,
            terminal_kind,
            payload: payload.to_string(),
        }
    }

    pub fn selected_terminal_payload(selected_alternative: &str, payload: &str) -> Self {
        Self::selected_terminal_payload_with_kind(
            selected_alternative,
            GrammarTerminalKind::IdentifierPayload,
            payload,
        )
    }

    pub fn selected_terminal_payload_with_kind(
        selected_alternative: &str,
        terminal_kind: GrammarTerminalKind,
        payload: &str,
    ) -> Self {
        Self::TerminalPayload {
            selected_alternative: Some(selected_alternative.to_string()),
            terminal_kind,
            payload: payload.to_string(),
        }
    }

    pub fn boolean_payload(value: bool) -> Self {
        Self::BooleanPayload {
            selected_alternative: None,
            value,
        }
    }

    pub fn selected_boolean_payload(selected_alternative: &str, value: bool) -> Self {
        Self::BooleanPayload {
            selected_alternative: Some(selected_alternative.to_string()),
            value,
        }
    }

    pub fn alternative_node(node: GoAstInventoryDeclarativeNodeSpec) -> Self {
        Self::AlternativeNode(Box::new(node))
    }

    pub fn generic_node(node: GoAstInventoryDeclarativeNodeSpec) -> Self {
        Self::GenericNode(Box::new(node))
    }

    pub fn optional_node_absent() -> Self {
        Self::OptionalNode(None)
    }

    pub fn optional_node_present(node: GoAstInventoryDeclarativeNodeSpec) -> Self {
        Self::OptionalNode(Some(Box::new(node)))
    }

    pub fn optional_alternative_node_present(node: GoAstInventoryDeclarativeNodeSpec) -> Self {
        Self::OptionalAlternativeNode(Some(Box::new(node)))
    }

    pub fn repeated_alternative_nodes(nodes: Vec<GoAstInventoryDeclarativeNodeSpec>) -> Self {
        Self::RepeatedAlternativeNodes(nodes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryDeclarativeParentFieldSpec {
    pub rule_name: String,
    pub field_name: String,
}

impl GoAstInventoryDeclarativeParentFieldSpec {
    pub fn new(rule_name: &str, field_name: &str) -> Self {
        Self {
            rule_name: rule_name.to_string(),
            field_name: field_name.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryCheckedDeclarativeExpressionSpec {
    pub expression: GoAstInventoryDeclarativeExpressionSpec,
    pub expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
    pub finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryDeclarativeTemplateCoverageSpec {
    pub synthesis_api_id: String,
    pub parent_field: GoAstInventoryDeclarativeParentFieldSpec,
    pub expression: GoAstInventoryDeclarativeExpressionSpec,
}

impl GoAstInventoryDeclarativeTemplateCoverageSpec {
    pub fn root_node_kind(&self) -> &str {
        self.expression.root_node_kind()
    }
}

pub(super) fn validate_declarative_expression_spec(
    expression: &GoAstInventoryDeclarativeExpressionSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    validate_declarative_node_spec(&expression.root)
}

pub(super) fn emit_declarative_expression_template(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    expression: &GoAstInventoryDeclarativeExpressionSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    emit_declarative_node_field(stream, &expression.root)
}

pub(super) fn emit_declarative_expression_template_value(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    expression: &GoAstInventoryDeclarativeExpressionSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    emit_declarative_node_value(stream, &expression.root)
}

fn validate_declarative_node_spec(
    node: &GoAstInventoryDeclarativeNodeSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_non_empty(
        "go_ast_inventory_declarative_template_spec.node_kind",
        &node.node_kind,
    )?;
    let mut seen = BTreeSet::new();
    for field in &node.fields {
        require_non_empty(
            "go_ast_inventory_declarative_template_spec.field_name",
            &field.field_name,
        )?;
        if !seen.insert(field.field_name.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_declarative_template_spec.field_name",
                format!(
                    "duplicate declarative field {}.{}",
                    node.node_kind, field.field_name
                ),
            ));
        }
        validate_declarative_field_value(&field.value)?;
    }
    Ok(())
}

fn validate_declarative_field_value(
    value: &GoAstInventoryDeclarativeFieldValue,
) -> Result<(), SyntaxBackendBoundaryError> {
    match value {
        GoAstInventoryDeclarativeFieldValue::Missing => Ok(()),
        GoAstInventoryDeclarativeFieldValue::TerminalPayload {
            selected_alternative,
            terminal_kind: _,
            payload,
        } => {
            if let Some(selected_alternative) = selected_alternative {
                require_non_empty(
                    "go_ast_inventory_declarative_template_spec.terminal.selected_alternative",
                    selected_alternative,
                )?;
            }
            require_non_empty(
                "go_ast_inventory_declarative_template_spec.terminal.payload",
                payload,
            )
        }
        GoAstInventoryDeclarativeFieldValue::BooleanPayload {
            selected_alternative,
            value: _,
        } => {
            if let Some(selected_alternative) = selected_alternative {
                require_non_empty(
                    "go_ast_inventory_declarative_template_spec.boolean.selected_alternative",
                    selected_alternative,
                )?;
            }
            Ok(())
        }
        GoAstInventoryDeclarativeFieldValue::AlternativeNode(node) => {
            validate_declarative_node_spec(node)
        }
        GoAstInventoryDeclarativeFieldValue::GenericNode(node) => {
            validate_declarative_node_spec(node)
        }
        GoAstInventoryDeclarativeFieldValue::OptionalNode(node) => {
            if let Some(node) = node {
                validate_declarative_node_spec(node)?;
            }
            Ok(())
        }
        GoAstInventoryDeclarativeFieldValue::OptionalAlternativeNode(node) => {
            if let Some(node) = node {
                validate_declarative_node_spec(node)?;
            }
            Ok(())
        }
        GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes) => {
            for node in nodes {
                validate_declarative_node_spec(node)?;
            }
            Ok(())
        }
        GoAstInventoryDeclarativeFieldValue::RepeatedGenericNodes(nodes) => {
            for node in nodes {
                validate_declarative_node_spec(node)?;
            }
            Ok(())
        }
    }
}

fn emit_declarative_node_field(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    node: &GoAstInventoryDeclarativeNodeSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    if stream.has_alternative_node_start_token(&node.node_kind) {
        stream.alternative_node_field(&node.node_kind, |stream| {
            emit_declarative_node_fields(stream, node)
        })
    } else {
        stream.generic_node_field(|stream| {
            emit_declarative_node_fields(stream, node)?;
            stream.synthetic_trivia_fields(&node.node_kind);
            Ok(())
        })
    }
}

fn emit_declarative_node_value(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    node: &GoAstInventoryDeclarativeNodeSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    if stream.has_alternative_node_start_token(&node.node_kind) {
        stream.alternative_node_value(&node.node_kind, |stream| {
            emit_declarative_node_fields(stream, node)
        })
    } else {
        stream.generic_node_value(|stream| {
            emit_declarative_node_fields(stream, node)?;
            stream.synthetic_trivia_fields(&node.node_kind);
            Ok(())
        })
    }
}

fn emit_declarative_node_fields(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    node: &GoAstInventoryDeclarativeNodeSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    for field in &node.fields {
        match &field.value {
            GoAstInventoryDeclarativeFieldValue::Missing => stream.missing_field(),
            GoAstInventoryDeclarativeFieldValue::TerminalPayload {
                selected_alternative: _,
                terminal_kind: _,
                payload,
            } => stream.terminal_field(payload),
            GoAstInventoryDeclarativeFieldValue::BooleanPayload {
                selected_alternative: _,
                value,
            } => stream.boolean_terminal_field(*value),
            GoAstInventoryDeclarativeFieldValue::AlternativeNode(node) => {
                emit_declarative_node_field(stream, node)?;
            }
            GoAstInventoryDeclarativeFieldValue::GenericNode(node) => {
                emit_declarative_generic_node_field(stream, node)?;
            }
            GoAstInventoryDeclarativeFieldValue::OptionalNode(node) => {
                if let Some(node) = node {
                    emit_declarative_generic_node_field(stream, node)?;
                } else {
                    stream.missing_field();
                }
            }
            GoAstInventoryDeclarativeFieldValue::OptionalAlternativeNode(node) => {
                if let Some(node) = node {
                    emit_declarative_node_field(stream, node)?;
                } else {
                    stream.missing_field();
                }
            }
            GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes) => {
                if nodes.is_empty() {
                    stream.repeated_empty_field();
                } else {
                    stream.repeated_node_field(|stream| {
                        for node in nodes {
                            emit_declarative_node_value(stream, node)?;
                        }
                        Ok(())
                    })?;
                }
            }
            GoAstInventoryDeclarativeFieldValue::RepeatedGenericNodes(nodes) => {
                if nodes.is_empty() {
                    stream.repeated_empty_field();
                } else {
                    stream.repeated_node_field(|stream| {
                        for node in nodes {
                            emit_declarative_generic_node_value(stream, node)?;
                        }
                        Ok(())
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn emit_declarative_generic_node_field(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    node: &GoAstInventoryDeclarativeNodeSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    stream.generic_node_field(|stream| {
        emit_declarative_node_fields(stream, node)?;
        stream.synthetic_trivia_fields(&node.node_kind);
        Ok(())
    })
}

fn emit_declarative_generic_node_value(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    node: &GoAstInventoryDeclarativeNodeSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    stream.generic_node_value(|stream| {
        emit_declarative_node_fields(stream, node)?;
        stream.synthetic_trivia_fields(&node.node_kind);
        Ok(())
    })
}
