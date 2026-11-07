use super::declarative::{
    GoAstInventoryDeclarativeExpressionSpec, emit_declarative_expression_template,
    emit_declarative_expression_template_value, validate_declarative_expression_spec,
};
use super::function_signature::{
    GoAstInventoryBlockTemplate, GoAstInventoryFunctionSignatureTemplate,
    emit_func_lit_template_contents, emit_function_signature_contents, validate_block_template,
    validate_function_signature_template,
};
use super::require_non_empty;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_raw_token_stream::GoRawTokenStreamBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoAstInventoryExpressionTemplate {
    Ident {
        payload: String,
    },
    BasicLit {
        literal_kind: String,
        literal_value: String,
    },
    ArrayTypeIdent {
        element_type: String,
    },
    ArrayTypeEllipsisIdent {
        element_type: String,
    },
    ChanTypeIdent {
        direction: String,
        element_type: String,
    },
    BinaryExpr {
        left: Box<GoAstInventoryExpressionTemplate>,
        operator: String,
        right: Box<GoAstInventoryExpressionTemplate>,
    },
    CompositeLit {
        type_expression: Box<GoAstInventoryExpressionTemplate>,
        elements: Vec<GoAstInventoryExpressionTemplate>,
        incomplete: bool,
    },
    FuncLit {
        signature: GoAstInventoryFunctionSignatureTemplate,
        body: GoAstInventoryBlockTemplate,
    },
    FuncType {
        signature: GoAstInventoryFunctionSignatureTemplate,
    },
    IndexExpr {
        target: Box<GoAstInventoryExpressionTemplate>,
        index: Box<GoAstInventoryExpressionTemplate>,
    },
    Declarative(GoAstInventoryDeclarativeExpressionSpec),
    SelectorCall {
        receiver: String,
        selector: String,
    },
}

pub(super) fn validate_expression_template(
    expression: &GoAstInventoryExpressionTemplate,
) -> Result<(), SyntaxBackendBoundaryError> {
    match expression {
        GoAstInventoryExpressionTemplate::Ident { payload } => {
            require_non_empty("go_ast_inventory_template_synthesis.expr.ident", payload)
        }
        GoAstInventoryExpressionTemplate::BasicLit {
            literal_kind,
            literal_value,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.expr.basic_lit.kind",
                literal_kind,
            )?;
            require_non_empty(
                "go_ast_inventory_template_synthesis.expr.basic_lit.value",
                literal_value,
            )
        }
        GoAstInventoryExpressionTemplate::ArrayTypeIdent { element_type } => require_non_empty(
            "go_ast_inventory_template_synthesis.expr.array_type_ident.element_type",
            element_type,
        ),
        GoAstInventoryExpressionTemplate::ArrayTypeEllipsisIdent { element_type } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.expr.array_type_ellipsis_ident.element_type",
                element_type,
            )
        }
        GoAstInventoryExpressionTemplate::ChanTypeIdent {
            direction,
            element_type,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.expr.chan_type_ident.direction",
                direction,
            )?;
            require_non_empty(
                "go_ast_inventory_template_synthesis.expr.chan_type_ident.element_type",
                element_type,
            )
        }
        GoAstInventoryExpressionTemplate::BinaryExpr {
            left,
            operator,
            right,
        } => {
            validate_expression_template(left)?;
            require_non_empty(
                "go_ast_inventory_template_synthesis.expr.binary_expr.operator",
                operator,
            )?;
            validate_expression_template(right)
        }
        GoAstInventoryExpressionTemplate::CompositeLit {
            type_expression,
            elements,
            incomplete: _,
        } => {
            validate_expression_template(type_expression)?;
            elements.iter().try_for_each(validate_expression_template)
        }
        GoAstInventoryExpressionTemplate::FuncLit { signature, body } => {
            validate_function_signature_template(signature);
            validate_block_template(body);
            Ok(())
        }
        GoAstInventoryExpressionTemplate::FuncType { signature } => {
            validate_function_signature_template(signature);
            Ok(())
        }
        GoAstInventoryExpressionTemplate::IndexExpr { target, index } => {
            validate_expression_template(target)?;
            validate_expression_template(index)
        }
        GoAstInventoryExpressionTemplate::Declarative(spec) => {
            validate_declarative_expression_spec(spec)
        }
        GoAstInventoryExpressionTemplate::SelectorCall { receiver, selector } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.expr.selector_call.receiver",
                receiver,
            )?;
            require_non_empty(
                "go_ast_inventory_template_synthesis.expr.selector_call.selector",
                selector,
            )
        }
    }
}

pub(super) fn emit_expression_template(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    expression: &GoAstInventoryExpressionTemplate,
) -> Result<(), SyntaxBackendBoundaryError> {
    match expression {
        GoAstInventoryExpressionTemplate::Ident { payload } => {
            stream.ident_alternative_node_field(payload)
        }
        GoAstInventoryExpressionTemplate::BasicLit {
            literal_kind,
            literal_value,
        } => stream.basic_lit_alternative(literal_kind, literal_value),
        GoAstInventoryExpressionTemplate::ArrayTypeIdent { element_type } => {
            stream.array_type_ident_alternative(element_type)
        }
        GoAstInventoryExpressionTemplate::ArrayTypeEllipsisIdent { element_type } => {
            stream.array_type_ellipsis_ident_alternative(element_type)
        }
        GoAstInventoryExpressionTemplate::ChanTypeIdent {
            direction,
            element_type,
        } => stream.chan_type_ident_alternative(direction, element_type),
        GoAstInventoryExpressionTemplate::BinaryExpr {
            left,
            operator,
            right,
        } => stream.alternative_node_field("BinaryExpr", |stream| {
            emit_expression_template(stream, left)?;
            stream.terminal_field(operator);
            emit_expression_template(stream, right)
        }),
        GoAstInventoryExpressionTemplate::CompositeLit {
            type_expression,
            elements,
            incomplete,
        } => stream.alternative_node_field("CompositeLit", |stream| {
            emit_expression_template(stream, type_expression)?;
            stream.repeated_node_field(|stream| {
                for element in elements {
                    emit_expression_template_value(stream, element)?;
                }
                Ok(())
            })?;
            stream.boolean_terminal_field(*incomplete);
            Ok(())
        }),
        GoAstInventoryExpressionTemplate::FuncLit { signature, body } => stream
            .alternative_node_field("FuncLit", |stream| {
                emit_func_lit_template_contents(stream, signature, body)
            }),
        GoAstInventoryExpressionTemplate::FuncType { signature } => stream
            .alternative_node_field("FuncType", |stream| {
                emit_function_signature_contents(stream, signature)
            }),
        GoAstInventoryExpressionTemplate::IndexExpr { target, index } => stream
            .alternative_node_field("IndexExpr", |stream| {
                emit_expression_template(stream, target)?;
                emit_expression_template(stream, index)
            }),
        GoAstInventoryExpressionTemplate::Declarative(spec) => {
            emit_declarative_expression_template(stream, spec)
        }
        GoAstInventoryExpressionTemplate::SelectorCall { receiver, selector } => {
            stream.selector_call_alternative(receiver, selector)
        }
    }
}

fn emit_expression_template_value(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    expression: &GoAstInventoryExpressionTemplate,
) -> Result<(), SyntaxBackendBoundaryError> {
    match expression {
        GoAstInventoryExpressionTemplate::Ident { payload } => {
            stream.alternative_node_value("Ident", |stream| {
                stream.terminal_field(payload);
                Ok(())
            })
        }
        GoAstInventoryExpressionTemplate::BasicLit {
            literal_kind,
            literal_value,
        } => stream.alternative_node_value("BasicLit", |stream| {
            stream.terminal_field(literal_kind);
            stream.terminal_field(literal_value);
            Ok(())
        }),
        GoAstInventoryExpressionTemplate::ArrayTypeIdent { element_type } => stream
            .alternative_node_value("ArrayType", |stream| {
                stream.missing_field();
                stream.ident_alternative_node_field(element_type)
            }),
        GoAstInventoryExpressionTemplate::ArrayTypeEllipsisIdent { element_type } => stream
            .alternative_node_value("ArrayType", |stream| {
                stream.alternative_node_field("Ellipsis", |stream| {
                    stream.missing_field();
                    Ok(())
                })?;
                stream.ident_alternative_node_field(element_type)
            }),
        GoAstInventoryExpressionTemplate::ChanTypeIdent {
            direction,
            element_type,
        } => stream.alternative_node_value("ChanType", |stream| {
            stream.terminal_field(direction);
            stream.ident_alternative_node_field(element_type)
        }),
        GoAstInventoryExpressionTemplate::BinaryExpr {
            left,
            operator,
            right,
        } => stream.alternative_node_value("BinaryExpr", |stream| {
            emit_expression_template(stream, left)?;
            stream.terminal_field(operator);
            emit_expression_template(stream, right)
        }),
        GoAstInventoryExpressionTemplate::CompositeLit {
            type_expression,
            elements,
            incomplete,
        } => stream.alternative_node_value("CompositeLit", |stream| {
            emit_expression_template(stream, type_expression)?;
            stream.repeated_node_field(|stream| {
                for element in elements {
                    emit_expression_template_value(stream, element)?;
                }
                Ok(())
            })?;
            stream.boolean_terminal_field(*incomplete);
            Ok(())
        }),
        GoAstInventoryExpressionTemplate::FuncLit { signature, body } => stream
            .alternative_node_value("FuncLit", |stream| {
                emit_func_lit_template_contents(stream, signature, body)
            }),
        GoAstInventoryExpressionTemplate::FuncType { signature } => stream
            .alternative_node_value("FuncType", |stream| {
                emit_function_signature_contents(stream, signature)
            }),
        GoAstInventoryExpressionTemplate::IndexExpr { target, index } => stream
            .alternative_node_value("IndexExpr", |stream| {
                emit_expression_template(stream, target)?;
                emit_expression_template(stream, index)
            }),
        GoAstInventoryExpressionTemplate::Declarative(spec) => {
            emit_declarative_expression_template_value(stream, spec)
        }
        GoAstInventoryExpressionTemplate::SelectorCall { receiver, selector } => stream
            .alternative_node_value("CallExpr", |stream| {
                stream.alternative_node_field("SelectorExpr", |stream| {
                    stream.ident_alternative_node_field(receiver)?;
                    stream.ident_node_field(selector)?;
                    Ok(())
                })?;
                stream.repeated_empty_field();
                stream.boolean_terminal_field(false);
                Ok(())
            }),
    }
}
