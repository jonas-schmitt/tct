use super::core::{
    GoAstInventoryExpectedFieldOrder, GoAstInventoryFiniteDomainDecision,
    GoAstInventoryFiniteDomainRequirement, GoAstInventoryRuleFieldOrder,
    GoAstInventorySynthesisPlan, validate_go_ast_inventory_synthesis_plan_from_inventory,
};
use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_raw_token_stream::GoRawTokenStreamBuilder;
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

mod declarative;
mod expression;
mod function_signature;
pub use self::declarative::{
    GO_AST_INVENTORY_DECLARATIVE_EXPRESSION_SPEC_KIND,
    GoAstInventoryCheckedDeclarativeExpressionSpec, GoAstInventoryDeclarativeExpressionSpec,
    GoAstInventoryDeclarativeFieldSpec, GoAstInventoryDeclarativeFieldValue,
    GoAstInventoryDeclarativeNodeSpec, GoAstInventoryDeclarativeParentFieldSpec,
    GoAstInventoryDeclarativeTemplateCoverageSpec,
    check_go_ast_inventory_declarative_expression_spec_from_inventory,
};
use self::declarative::{
    emit_declarative_expression_template_value, validate_declarative_expression_spec,
};
pub use self::expression::GoAstInventoryExpressionTemplate;
use self::expression::{emit_expression_template, validate_expression_template};
pub use self::function_signature::{
    GoAstInventoryBlockTemplate, GoAstInventoryFunctionSignatureTemplate,
};

pub const GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-template-synthesis-v1";
pub const GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory template/spec IR + field-order and finite-domain evidence + payload literals -> token-only GoSourceFile raw-token stream";
pub const GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID: &str =
    "go-ast-inventory-function-return-expression-template-v1";
pub const GO_AST_INVENTORY_FUNCTION_STATEMENT_LIST_TEMPLATE_ID: &str =
    "go-ast-inventory-function-statement-list-template-v1";
pub const GO_AST_INVENTORY_PACKAGE_VAR_VALUE_EXPR_TEMPLATE_ID: &str =
    "go-ast-inventory-package-var-value-expression-template-v1";
pub const GO_AST_INVENTORY_PACKAGE_VAR_TYPE_EXPR_TEMPLATE_ID: &str =
    "go-ast-inventory-package-var-type-expression-template-v1";
pub const GO_AST_INVENTORY_PACKAGE_TYPE_EXPR_TEMPLATE_ID: &str =
    "go-ast-inventory-package-type-expression-template-v1";
pub const GO_AST_INVENTORY_PACKAGE_IMPORT_SPEC_TEMPLATE_ID: &str =
    "go-ast-inventory-package-import-spec-template-v1";
const GO_AST_INVENTORY_TEMPLATE_ROOT: &str = "GoSourceFile";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryTemplateSpec {
    pub template_id: String,
    pub synthesis_id: String,
    pub root_rule: String,
    pub expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
    pub finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    pub source_file: GoAstInventoryTemplateSourceFile,
}

impl GoAstInventoryTemplateSpec {
    #[allow(clippy::too_many_arguments)]
    pub fn function_return_expression(
        synthesis_id: &str,
        package_name: &str,
        go_version: &str,
        function_identifier: &str,
        result_type: GoAstInventoryExpressionTemplate,
        return_expression: GoAstInventoryExpressionTemplate,
        expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
        finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    ) -> Self {
        Self {
            template_id: GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID.to_string(),
            synthesis_id: synthesis_id.to_string(),
            root_rule: GO_AST_INVENTORY_TEMPLATE_ROOT.to_string(),
            expected_field_orders,
            finite_domain_requirements,
            source_file: GoAstInventoryTemplateSourceFile {
                package_name: package_name.to_string(),
                go_version: go_version.to_string(),
                comments: Vec::new(),
                decl: GoAstInventoryDeclarationTemplate::FunctionReturnExpression {
                    function_identifier: function_identifier.to_string(),
                    result_type,
                    return_expression,
                },
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn function_statement_list_return_expression(
        synthesis_id: &str,
        package_name: &str,
        go_version: &str,
        function_identifier: &str,
        result_type: GoAstInventoryExpressionTemplate,
        statements: Vec<GoAstInventoryDeclarativeExpressionSpec>,
        return_expression: GoAstInventoryExpressionTemplate,
        expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
        finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    ) -> Self {
        Self {
            template_id: GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID.to_string(),
            synthesis_id: synthesis_id.to_string(),
            root_rule: GO_AST_INVENTORY_TEMPLATE_ROOT.to_string(),
            expected_field_orders,
            finite_domain_requirements,
            source_file: GoAstInventoryTemplateSourceFile {
                package_name: package_name.to_string(),
                go_version: go_version.to_string(),
                comments: Vec::new(),
                decl: GoAstInventoryDeclarationTemplate::FunctionStatementListReturnExpression {
                    function_identifier: function_identifier.to_string(),
                    result_type,
                    statements,
                    return_expression,
                },
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn function_statement_list(
        synthesis_id: &str,
        package_name: &str,
        go_version: &str,
        function_identifier: &str,
        result_type: GoAstInventoryExpressionTemplate,
        statements: Vec<GoAstInventoryDeclarativeExpressionSpec>,
        expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
        finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    ) -> Self {
        Self {
            template_id: GO_AST_INVENTORY_FUNCTION_STATEMENT_LIST_TEMPLATE_ID.to_string(),
            synthesis_id: synthesis_id.to_string(),
            root_rule: GO_AST_INVENTORY_TEMPLATE_ROOT.to_string(),
            expected_field_orders,
            finite_domain_requirements,
            source_file: GoAstInventoryTemplateSourceFile {
                package_name: package_name.to_string(),
                go_version: go_version.to_string(),
                comments: Vec::new(),
                decl: GoAstInventoryDeclarationTemplate::FunctionStatementList {
                    function_identifier: function_identifier.to_string(),
                    result_type,
                    statements,
                },
            },
        }
    }

    pub fn function_statement_list_no_result(
        synthesis_id: &str,
        package_name: &str,
        go_version: &str,
        function_identifier: &str,
        statements: Vec<GoAstInventoryDeclarativeExpressionSpec>,
        expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
        finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    ) -> Self {
        Self {
            template_id: GO_AST_INVENTORY_FUNCTION_STATEMENT_LIST_TEMPLATE_ID.to_string(),
            synthesis_id: synthesis_id.to_string(),
            root_rule: GO_AST_INVENTORY_TEMPLATE_ROOT.to_string(),
            expected_field_orders,
            finite_domain_requirements,
            source_file: GoAstInventoryTemplateSourceFile {
                package_name: package_name.to_string(),
                go_version: go_version.to_string(),
                comments: Vec::new(),
                decl: GoAstInventoryDeclarationTemplate::FunctionStatementListNoResult {
                    function_identifier: function_identifier.to_string(),
                    statements,
                },
            },
        }
    }

    pub fn package_var_value_expression(
        synthesis_id: &str,
        package_name: &str,
        go_version: &str,
        variable_identifier: &str,
        value_expression: GoAstInventoryExpressionTemplate,
        expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
        finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    ) -> Self {
        Self {
            template_id: GO_AST_INVENTORY_PACKAGE_VAR_VALUE_EXPR_TEMPLATE_ID.to_string(),
            synthesis_id: synthesis_id.to_string(),
            root_rule: GO_AST_INVENTORY_TEMPLATE_ROOT.to_string(),
            expected_field_orders,
            finite_domain_requirements,
            source_file: GoAstInventoryTemplateSourceFile {
                package_name: package_name.to_string(),
                go_version: go_version.to_string(),
                comments: Vec::new(),
                decl: GoAstInventoryDeclarationTemplate::PackageVarValueExpression {
                    variable_identifier: variable_identifier.to_string(),
                    value_expression,
                },
            },
        }
    }

    pub fn package_var_type_expression(
        synthesis_id: &str,
        package_name: &str,
        go_version: &str,
        variable_identifier: &str,
        type_expression: GoAstInventoryExpressionTemplate,
        expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
        finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    ) -> Self {
        Self {
            template_id: GO_AST_INVENTORY_PACKAGE_VAR_TYPE_EXPR_TEMPLATE_ID.to_string(),
            synthesis_id: synthesis_id.to_string(),
            root_rule: GO_AST_INVENTORY_TEMPLATE_ROOT.to_string(),
            expected_field_orders,
            finite_domain_requirements,
            source_file: GoAstInventoryTemplateSourceFile {
                package_name: package_name.to_string(),
                go_version: go_version.to_string(),
                comments: Vec::new(),
                decl: GoAstInventoryDeclarationTemplate::PackageVarTypeExpression {
                    variable_identifier: variable_identifier.to_string(),
                    type_expression,
                },
            },
        }
    }

    pub fn package_type_expression(
        synthesis_id: &str,
        package_name: &str,
        go_version: &str,
        type_identifier: &str,
        type_expression: GoAstInventoryExpressionTemplate,
        expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
        finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    ) -> Self {
        Self {
            template_id: GO_AST_INVENTORY_PACKAGE_TYPE_EXPR_TEMPLATE_ID.to_string(),
            synthesis_id: synthesis_id.to_string(),
            root_rule: GO_AST_INVENTORY_TEMPLATE_ROOT.to_string(),
            expected_field_orders,
            finite_domain_requirements,
            source_file: GoAstInventoryTemplateSourceFile {
                package_name: package_name.to_string(),
                go_version: go_version.to_string(),
                comments: Vec::new(),
                decl: GoAstInventoryDeclarationTemplate::PackageTypeExpression {
                    type_identifier: type_identifier.to_string(),
                    type_expression,
                },
            },
        }
    }

    pub fn package_import_spec(
        synthesis_id: &str,
        package_name: &str,
        go_version: &str,
        import_spec: GoAstInventoryExpressionTemplate,
        expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
        finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
    ) -> Self {
        Self {
            template_id: GO_AST_INVENTORY_PACKAGE_IMPORT_SPEC_TEMPLATE_ID.to_string(),
            synthesis_id: synthesis_id.to_string(),
            root_rule: GO_AST_INVENTORY_TEMPLATE_ROOT.to_string(),
            expected_field_orders,
            finite_domain_requirements,
            source_file: GoAstInventoryTemplateSourceFile {
                package_name: package_name.to_string(),
                go_version: go_version.to_string(),
                comments: Vec::new(),
                decl: GoAstInventoryDeclarationTemplate::PackageImportSpec { import_spec },
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryTemplateSourceFile {
    pub package_name: String,
    pub go_version: String,
    pub comments: Vec<GoAstInventoryDeclarativeExpressionSpec>,
    pub decl: GoAstInventoryDeclarationTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoAstInventoryDeclarationTemplate {
    FunctionReturnExpression {
        function_identifier: String,
        result_type: GoAstInventoryExpressionTemplate,
        return_expression: GoAstInventoryExpressionTemplate,
    },
    FunctionStatementListReturnExpression {
        function_identifier: String,
        result_type: GoAstInventoryExpressionTemplate,
        statements: Vec<GoAstInventoryDeclarativeExpressionSpec>,
        return_expression: GoAstInventoryExpressionTemplate,
    },
    FunctionStatementList {
        function_identifier: String,
        result_type: GoAstInventoryExpressionTemplate,
        statements: Vec<GoAstInventoryDeclarativeExpressionSpec>,
    },
    FunctionStatementListNoResult {
        function_identifier: String,
        statements: Vec<GoAstInventoryDeclarativeExpressionSpec>,
    },
    PackageVarValueExpression {
        variable_identifier: String,
        value_expression: GoAstInventoryExpressionTemplate,
    },
    PackageVarTypeExpression {
        variable_identifier: String,
        type_expression: GoAstInventoryExpressionTemplate,
    },
    PackageTypeExpression {
        type_identifier: String,
        type_expression: GoAstInventoryExpressionTemplate,
    },
    PackageImportSpec {
        import_spec: GoAstInventoryExpressionTemplate,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryTemplateSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub template_id: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub template_spec: GoAstInventoryTemplateSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_template_raw_tokens(
    roles: &SyntaxTokenRoles,
    template_spec: GoAstInventoryTemplateSpec,
) -> Result<GoAstInventoryTemplateSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_template_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        template_spec,
    )
}

pub fn synthesize_go_ast_inventory_template_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    template_spec: GoAstInventoryTemplateSpec,
) -> Result<GoAstInventoryTemplateSynthesis, SyntaxBackendBoundaryError> {
    validate_template_spec(&template_spec)?;
    let synthesis_plan = validate_go_ast_inventory_synthesis_plan_from_inventory(
        inventory_path,
        &template_spec.synthesis_id,
        &template_spec.root_rule,
        &template_spec.expected_field_orders,
        &template_spec.finite_domain_requirements,
    )?;

    let mut stream = GoRawTokenStreamBuilder::new(roles);
    emit_source_file_template(&mut stream, &template_spec.source_file)?;

    Ok(GoAstInventoryTemplateSynthesis {
        api_id: GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_API_ID.to_string(),
        api_boundary: GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_BOUNDARY.to_string(),
        template_id: template_spec.template_id.clone(),
        inventory_artifact_path: synthesis_plan.inventory_artifact_path.clone(),
        grammar_id: synthesis_plan.grammar_id.clone(),
        root_rule: synthesis_plan.root_rule.clone(),
        validated_field_orders: synthesis_plan.validated_field_orders.clone(),
        validated_finite_domain_alternatives: synthesis_plan
            .validated_finite_domain_alternatives
            .clone(),
        raw_token_ids: stream.raw_token_ids,
        template_spec,
        synthesis_plan,
    })
}

fn validate_template_spec(
    template_spec: &GoAstInventoryTemplateSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_non_empty(
        "go_ast_inventory_template_synthesis.template_id",
        &template_spec.template_id,
    )?;
    require_non_empty(
        "go_ast_inventory_template_synthesis.synthesis_id",
        &template_spec.synthesis_id,
    )?;
    require_non_empty(
        "go_ast_inventory_template_synthesis.root_rule",
        &template_spec.root_rule,
    )?;
    if template_spec.expected_field_orders.is_empty() {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_template_synthesis.expected_field_orders",
            "expected at least one checked field-order requirement",
        ));
    }
    require_non_empty(
        "go_ast_inventory_template_synthesis.source_file.package_name",
        &template_spec.source_file.package_name,
    )?;
    require_non_empty(
        "go_ast_inventory_template_synthesis.source_file.go_version",
        &template_spec.source_file.go_version,
    )?;
    for comment in &template_spec.source_file.comments {
        validate_declarative_expression_spec(comment)?;
    }
    match &template_spec.source_file.decl {
        GoAstInventoryDeclarationTemplate::FunctionReturnExpression {
            function_identifier,
            result_type,
            return_expression,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.function_return.function_identifier",
                function_identifier,
            )?;
            validate_expression_template(result_type)?;
            validate_expression_template(return_expression)
        }
        GoAstInventoryDeclarationTemplate::FunctionStatementListReturnExpression {
            function_identifier,
            result_type,
            statements,
            return_expression,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.function_statement_list_return.function_identifier",
                function_identifier,
            )?;
            validate_expression_template(result_type)?;
            if statements.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_synthesis.function_statement_list_return.statements",
                    "expected at least one declarative statement template",
                ));
            }
            for statement in statements {
                validate_declarative_expression_spec(statement)?;
            }
            validate_expression_template(return_expression)
        }
        GoAstInventoryDeclarationTemplate::FunctionStatementList {
            function_identifier,
            result_type,
            statements,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.function_statement_list.function_identifier",
                function_identifier,
            )?;
            validate_expression_template(result_type)?;
            if statements.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_synthesis.function_statement_list.statements",
                    "expected at least one declarative statement template",
                ));
            }
            for statement in statements {
                validate_declarative_expression_spec(statement)?;
            }
            Ok(())
        }
        GoAstInventoryDeclarationTemplate::FunctionStatementListNoResult {
            function_identifier,
            statements,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.function_statement_list_no_result.function_identifier",
                function_identifier,
            )?;
            if statements.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_synthesis.function_statement_list_no_result.statements",
                    "expected at least one declarative statement template",
                ));
            }
            for statement in statements {
                validate_declarative_expression_spec(statement)?;
            }
            Ok(())
        }
        GoAstInventoryDeclarationTemplate::PackageVarValueExpression {
            variable_identifier,
            value_expression,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.package_var.variable_identifier",
                variable_identifier,
            )?;
            validate_expression_template(value_expression)
        }
        GoAstInventoryDeclarationTemplate::PackageVarTypeExpression {
            variable_identifier,
            type_expression,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.package_var_type.variable_identifier",
                variable_identifier,
            )?;
            validate_expression_template(type_expression)
        }
        GoAstInventoryDeclarationTemplate::PackageTypeExpression {
            type_identifier,
            type_expression,
        } => {
            require_non_empty(
                "go_ast_inventory_template_synthesis.package_type.type_identifier",
                type_identifier,
            )?;
            validate_expression_template(type_expression)
        }
        GoAstInventoryDeclarationTemplate::PackageImportSpec { import_spec } => {
            if !matches!(
                import_spec,
                GoAstInventoryExpressionTemplate::Declarative(_)
            ) {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_synthesis.package_import_spec.import_spec",
                    "package import template requires a declarative ImportSpec",
                ));
            }
            validate_expression_template(import_spec)
        }
    }
}

fn emit_source_file_template(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    source_file: &GoAstInventoryTemplateSourceFile,
) -> Result<(), SyntaxBackendBoundaryError> {
    stream.source_file_with_comments(
        &source_file.package_name,
        &source_file.go_version,
        |stream| {
            if source_file.comments.is_empty() {
                stream.repeated_empty_field();
            } else {
                stream.repeated_node_field(|stream| {
                    for comment in &source_file.comments {
                        emit_declarative_expression_template_value(stream, comment)?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        },
        |stream| match &source_file.decl {
            GoAstInventoryDeclarationTemplate::FunctionReturnExpression {
                function_identifier,
                result_type,
                return_expression,
            } => stream.func_decl_with_return_expr(
                function_identifier,
                |stream| emit_expression_template(stream, result_type),
                |stream| emit_expression_template(stream, return_expression),
            ),
            GoAstInventoryDeclarationTemplate::FunctionStatementListReturnExpression {
                function_identifier,
                result_type,
                statements,
                return_expression,
            } => stream.func_decl_with_result_body(
                function_identifier,
                |stream| emit_expression_template(stream, result_type),
                |stream| {
                    stream.repeated_node_field(|stream| {
                        for statement in statements {
                            emit_declarative_expression_template_value(stream, statement)?;
                        }
                        stream.alternative_node_value("ReturnStmt", |stream| {
                            emit_expression_template(stream, return_expression)
                        })
                    })
                },
            ),
            GoAstInventoryDeclarationTemplate::FunctionStatementList {
                function_identifier,
                result_type,
                statements,
            } => stream.func_decl_with_result_body(
                function_identifier,
                |stream| emit_expression_template(stream, result_type),
                |stream| {
                    stream.repeated_node_field(|stream| {
                        for statement in statements {
                            emit_declarative_expression_template_value(stream, statement)?;
                        }
                        Ok(())
                    })
                },
            ),
            GoAstInventoryDeclarationTemplate::FunctionStatementListNoResult {
                function_identifier,
                statements,
            } => stream.func_decl_with_body(function_identifier, |stream| {
                stream.repeated_node_field(|stream| {
                    for statement in statements {
                        emit_declarative_expression_template_value(stream, statement)?;
                    }
                    Ok(())
                })
            }),
            GoAstInventoryDeclarationTemplate::PackageVarValueExpression {
                variable_identifier,
                value_expression,
            } => stream.alternative_node_field("GenDecl", |stream| {
                stream.missing_field();
                stream.terminal_field("var");
                stream.missing_field();
                stream.alternative_node_field("ValueSpec", |stream| {
                    stream.missing_field();
                    stream.ident_node_field(variable_identifier)?;
                    stream.missing_field();
                    emit_expression_template(stream, value_expression)?;
                    stream.missing_field();
                    Ok(())
                })?;
                stream.missing_field();
                Ok(())
            }),
            GoAstInventoryDeclarationTemplate::PackageVarTypeExpression {
                variable_identifier,
                type_expression,
            } => stream.alternative_node_field("GenDecl", |stream| {
                stream.missing_field();
                stream.terminal_field("var");
                stream.missing_field();
                stream.alternative_node_field("ValueSpec", |stream| {
                    stream.missing_field();
                    stream.ident_node_field(variable_identifier)?;
                    emit_expression_template(stream, type_expression)?;
                    stream.repeated_empty_field();
                    stream.missing_field();
                    Ok(())
                })?;
                stream.missing_field();
                Ok(())
            }),
            GoAstInventoryDeclarationTemplate::PackageTypeExpression {
                type_identifier,
                type_expression,
            } => stream.alternative_node_field("GenDecl", |stream| {
                stream.missing_field();
                stream.terminal_field("type");
                stream.missing_field();
                stream.alternative_node_field("TypeSpec", |stream| {
                    stream.missing_field();
                    stream.ident_node_field(type_identifier)?;
                    stream.missing_field();
                    stream.missing_field();
                    emit_expression_template(stream, type_expression)?;
                    stream.missing_field();
                    Ok(())
                })?;
                stream.missing_field();
                Ok(())
            }),
            GoAstInventoryDeclarationTemplate::PackageImportSpec { import_spec } => stream
                .alternative_node_field("GenDecl", |stream| {
                    stream.missing_field();
                    stream.terminal_field("import");
                    stream.missing_field();
                    emit_expression_template(stream, import_spec)?;
                    stream.missing_field();
                    Ok(())
                }),
        },
    )
}

pub(super) fn require_non_empty(
    field: &'static str,
    actual: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected non-empty value",
        ))
    } else {
        Ok(())
    }
}
