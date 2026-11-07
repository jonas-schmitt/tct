use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::grammar_ir::GrammarTerminalKind;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_raw_token_stream::GoRawTokenStreamBuilder;
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

mod core;
mod functions;
mod registry;
mod template;

pub use self::core::{
    GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID, GO_AST_INVENTORY_SYNTHESIS_CORE_BOUNDARY,
    GoAstInventoryExpectedFieldOrder, GoAstInventoryFiniteDomainDecision,
    GoAstInventoryFiniteDomainRequirement, GoAstInventoryRuleFieldOrder,
    GoAstInventorySynthesisPlan, validate_go_ast_inventory_synthesis_plan_from_inventory,
};
use self::core::{require_non_empty, require_string};
pub use self::functions::{
    GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID, GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID, GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID, GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID, GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_BOUNDARY, GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_GO_STMT_SYNTHESIS_BOUNDARY, GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_IF_STMT_SYNTHESIS_BOUNDARY, GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_BOUNDARY, GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryArrayTypeReturnPayloadSpec, GoAstInventoryArrayTypeReturnSynthesis,
    GoAstInventoryAssignmentPayloadSpec, GoAstInventoryAssignmentSynthesis,
    GoAstInventoryBinaryExprReturnPayloadSpec, GoAstInventoryBinaryExprReturnSynthesis,
    GoAstInventoryBranchStmtPayloadSpec, GoAstInventoryBranchStmtSynthesis,
    GoAstInventoryCaseClausePayloadSpec, GoAstInventoryCaseClauseSynthesis,
    GoAstInventoryChanTypeReturnPayloadSpec, GoAstInventoryChanTypeReturnSynthesis,
    GoAstInventoryCommClausePayloadSpec, GoAstInventoryCommClauseSynthesis,
    GoAstInventoryCompositeLitReturnPayloadSpec, GoAstInventoryCompositeLitReturnSynthesis,
    GoAstInventoryDeclStmtPayloadSpec, GoAstInventoryDeclStmtSynthesis,
    GoAstInventoryDeferStmtPayloadSpec, GoAstInventoryDeferStmtSynthesis,
    GoAstInventoryEllipsisArrayLiteralPayloadSpec, GoAstInventoryEllipsisArrayLiteralSynthesis,
    GoAstInventoryEmptyStmtPayloadSpec, GoAstInventoryEmptyStmtSynthesis,
    GoAstInventoryFuncLitPayloadSpec, GoAstInventoryFuncLitSynthesis,
    GoAstInventoryFuncTypeResultPayloadSpec, GoAstInventoryFuncTypeResultSynthesis,
    GoAstInventoryFunctionReturnPayloadSpec, GoAstInventoryFunctionReturnSynthesis,
    GoAstInventoryGoStmtPayloadSpec, GoAstInventoryGoStmtSynthesis,
    GoAstInventoryIfStmtPayloadSpec, GoAstInventoryIfStmtSynthesis,
    GoAstInventoryIncDecStmtPayloadSpec, GoAstInventoryIncDecStmtSynthesis,
    GoAstInventoryIndexExprReturnPayloadSpec, GoAstInventoryIndexExprReturnSynthesis,
    GoAstInventoryIndexListExprReturnPayloadSpec, GoAstInventoryIndexListExprReturnSynthesis,
    GoAstInventoryInterfaceTypeVarPayloadSpec, GoAstInventoryInterfaceTypeVarSynthesis,
    GoAstInventoryKeyValueExprReturnPayloadSpec, GoAstInventoryKeyValueExprReturnSynthesis,
    GoAstInventoryLabeledStmtPayloadSpec, GoAstInventoryLabeledStmtSynthesis,
    GoAstInventoryParenExprReturnPayloadSpec, GoAstInventoryParenExprReturnSynthesis,
    GoAstInventoryRangeStmtPayloadSpec, GoAstInventoryRangeStmtSynthesis,
    GoAstInventorySelectorCallPayloadSpec, GoAstInventorySelectorCallSynthesis,
    GoAstInventorySendStmtPayloadSpec, GoAstInventorySendStmtSynthesis,
    GoAstInventorySliceExprReturnPayloadSpec, GoAstInventorySliceExprReturnSynthesis,
    GoAstInventoryStarExprReturnPayloadSpec, GoAstInventoryStarExprReturnSynthesis,
    GoAstInventoryStructTypeDeclPayloadSpec, GoAstInventoryStructTypeDeclSynthesis,
    GoAstInventoryTypeAssertExprReturnPayloadSpec, GoAstInventoryTypeAssertExprReturnSynthesis,
    GoAstInventoryTypeSwitchStmtPayloadSpec, GoAstInventoryTypeSwitchStmtSynthesis,
    GoAstInventoryUnaryExprReturnPayloadSpec, GoAstInventoryUnaryExprReturnSynthesis,
    go_ast_inventory_array_type_return_declarative_coverage_spec,
    go_ast_inventory_assignment_declarative_coverage_spec,
    go_ast_inventory_binary_expr_return_declarative_coverage_spec,
    go_ast_inventory_branch_stmt_declarative_coverage_spec,
    go_ast_inventory_case_clause_declarative_coverage_spec,
    go_ast_inventory_chan_type_return_declarative_coverage_spec,
    go_ast_inventory_comm_clause_declarative_coverage_spec,
    go_ast_inventory_composite_lit_return_declarative_coverage_spec,
    go_ast_inventory_decl_stmt_declarative_coverage_spec,
    go_ast_inventory_declarative_template_coverage_specs,
    go_ast_inventory_defer_stmt_declarative_coverage_spec,
    go_ast_inventory_empty_stmt_declarative_coverage_spec,
    go_ast_inventory_func_lit_declarative_coverage_spec,
    go_ast_inventory_go_stmt_declarative_coverage_spec,
    go_ast_inventory_if_stmt_declarative_coverage_spec,
    go_ast_inventory_inc_dec_stmt_declarative_coverage_spec,
    go_ast_inventory_index_expr_return_declarative_coverage_spec,
    go_ast_inventory_index_list_expr_return_declarative_coverage_spec,
    go_ast_inventory_interface_type_var_declarative_coverage_spec,
    go_ast_inventory_key_value_expr_return_declarative_coverage_spec,
    go_ast_inventory_labeled_stmt_declarative_coverage_spec,
    go_ast_inventory_paren_expr_return_declarative_coverage_spec,
    go_ast_inventory_range_stmt_declarative_coverage_spec,
    go_ast_inventory_selector_call_declarative_coverage_spec,
    go_ast_inventory_send_stmt_declarative_coverage_spec,
    go_ast_inventory_slice_expr_return_declarative_coverage_spec,
    go_ast_inventory_star_expr_return_declarative_coverage_spec,
    go_ast_inventory_struct_type_decl_declarative_coverage_spec,
    go_ast_inventory_type_assert_expr_return_declarative_coverage_spec,
    go_ast_inventory_type_switch_stmt_declarative_coverage_spec,
    go_ast_inventory_unary_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_array_type_return_raw_tokens,
    synthesize_go_ast_inventory_array_type_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_assignment_raw_tokens,
    synthesize_go_ast_inventory_assignment_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_binary_expr_return_raw_tokens,
    synthesize_go_ast_inventory_binary_expr_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_branch_stmt_raw_tokens,
    synthesize_go_ast_inventory_branch_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_case_clause_raw_tokens,
    synthesize_go_ast_inventory_case_clause_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_chan_type_return_raw_tokens,
    synthesize_go_ast_inventory_chan_type_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_comm_clause_raw_tokens,
    synthesize_go_ast_inventory_comm_clause_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_composite_lit_return_raw_tokens,
    synthesize_go_ast_inventory_composite_lit_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_decl_stmt_raw_tokens,
    synthesize_go_ast_inventory_decl_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_defer_stmt_raw_tokens,
    synthesize_go_ast_inventory_defer_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_ellipsis_array_literal_raw_tokens,
    synthesize_go_ast_inventory_ellipsis_array_literal_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_empty_stmt_raw_tokens,
    synthesize_go_ast_inventory_empty_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_func_lit_raw_tokens,
    synthesize_go_ast_inventory_func_lit_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_func_type_result_raw_tokens,
    synthesize_go_ast_inventory_func_type_result_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_function_return_raw_tokens,
    synthesize_go_ast_inventory_function_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_go_stmt_raw_tokens,
    synthesize_go_ast_inventory_go_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_if_stmt_raw_tokens,
    synthesize_go_ast_inventory_if_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_inc_dec_stmt_raw_tokens,
    synthesize_go_ast_inventory_inc_dec_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_index_expr_return_raw_tokens,
    synthesize_go_ast_inventory_index_expr_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_index_list_expr_return_raw_tokens,
    synthesize_go_ast_inventory_index_list_expr_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_interface_type_var_raw_tokens,
    synthesize_go_ast_inventory_interface_type_var_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_key_value_expr_return_raw_tokens,
    synthesize_go_ast_inventory_key_value_expr_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_labeled_stmt_raw_tokens,
    synthesize_go_ast_inventory_labeled_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_paren_expr_return_raw_tokens,
    synthesize_go_ast_inventory_paren_expr_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_range_stmt_raw_tokens,
    synthesize_go_ast_inventory_range_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_selector_call_raw_tokens,
    synthesize_go_ast_inventory_selector_call_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_send_stmt_raw_tokens,
    synthesize_go_ast_inventory_send_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_slice_expr_return_raw_tokens,
    synthesize_go_ast_inventory_slice_expr_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_star_expr_return_raw_tokens,
    synthesize_go_ast_inventory_star_expr_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_struct_type_decl_raw_tokens,
    synthesize_go_ast_inventory_struct_type_decl_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_type_assert_expr_return_raw_tokens,
    synthesize_go_ast_inventory_type_assert_expr_return_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_type_switch_stmt_raw_tokens,
    synthesize_go_ast_inventory_type_switch_stmt_raw_tokens_from_inventory,
    synthesize_go_ast_inventory_unary_expr_return_raw_tokens,
    synthesize_go_ast_inventory_unary_expr_return_raw_tokens_from_inventory,
};
pub use self::registry::{
    GO_AST_INVENTORY_SYNTHESIS_REGISTRY_API_ID, GO_AST_INVENTORY_SYNTHESIS_REGISTRY_BOUNDARY,
    GoAstInventoryRegistryExpectedFieldOrdersFn, GoAstInventoryRegistryExpressionSlot,
    GoAstInventoryRegistryExpressions, GoAstInventoryRegistryExpressionsFn,
    GoAstInventoryRegistryFiniteDomainRequirementsFn, GoAstInventoryRegistryPayload,
    GoAstInventoryRegistryPayloadDefault, GoAstInventoryRegistryPayloadField,
    GoAstInventoryRegistryPayloadRule, GoAstInventoryRegistryPayloadValue,
    GoAstInventoryRegistrySynthesis, GoAstInventorySynthesisRegistry,
    GoAstInventorySynthesisRegistryEntry, GoAstInventorySynthesisShell,
    go_ast_inventory_synthesis_registry, synthesize_go_ast_inventory_registry_raw_tokens,
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory,
};
pub use self::template::{
    GO_AST_INVENTORY_DECLARATIVE_EXPRESSION_SPEC_KIND,
    GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID,
    GO_AST_INVENTORY_FUNCTION_STATEMENT_LIST_TEMPLATE_ID,
    GO_AST_INVENTORY_PACKAGE_IMPORT_SPEC_TEMPLATE_ID,
    GO_AST_INVENTORY_PACKAGE_TYPE_EXPR_TEMPLATE_ID,
    GO_AST_INVENTORY_PACKAGE_VAR_TYPE_EXPR_TEMPLATE_ID,
    GO_AST_INVENTORY_PACKAGE_VAR_VALUE_EXPR_TEMPLATE_ID,
    GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_API_ID, GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_BOUNDARY,
    GoAstInventoryBlockTemplate, GoAstInventoryCheckedDeclarativeExpressionSpec,
    GoAstInventoryDeclarationTemplate, GoAstInventoryDeclarativeExpressionSpec,
    GoAstInventoryDeclarativeFieldSpec, GoAstInventoryDeclarativeFieldValue,
    GoAstInventoryDeclarativeNodeSpec, GoAstInventoryDeclarativeParentFieldSpec,
    GoAstInventoryDeclarativeTemplateCoverageSpec, GoAstInventoryExpressionTemplate,
    GoAstInventoryFunctionSignatureTemplate, GoAstInventoryTemplateSourceFile,
    GoAstInventoryTemplateSpec, GoAstInventoryTemplateSynthesis,
    check_go_ast_inventory_declarative_expression_spec_from_inventory,
    synthesize_go_ast_inventory_template_raw_tokens,
    synthesize_go_ast_inventory_template_raw_tokens_from_inventory,
};

pub const GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-var-decl-synthesis-v1";
pub const GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + minimal payload fixture spec -> token-only GoSourceFile package var-declaration raw-token stream";
pub const GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-import-spec-synthesis-v1";
pub const GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + minimal payload fixture spec -> token-only GoSourceFile import-alias raw-token stream";
const GO_AST_INVENTORY_SYNTHESIS_ROOT: &str = "GoSourceFile";
const GO_AST_INVENTORY_VAR_DECL_DEFAULT_PACKAGE_NAME: &str = "p";
const GO_AST_INVENTORY_VAR_DECL_DEFAULT_IDENTIFIER: &str = "x";
const GO_AST_INVENTORY_VAR_DECL_DEFAULT_LITERAL_KIND: &str = "INT";
const GO_AST_INVENTORY_VAR_DECL_DEFAULT_LITERAL_VALUE: &str = "0";
const GO_AST_INVENTORY_VAR_DECL_DEFAULT_GO_VERSION: &str = "go1.24";
const GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PACKAGE_NAME: &str = "p";
const GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_ALIAS: &str = "x";
const GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_KIND: &str = "STRING";
const GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_VALUE: &str = "\"m\"";
const GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_GO_VERSION: &str = "go1.24";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryVarDeclPayloadSpec {
    pub package_name: String,
    pub identifier: String,
    pub declaration_token: String,
    pub literal_kind: String,
    pub literal_value: String,
    pub go_version: String,
}

impl GoAstInventoryVarDeclPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_VAR_DECL_DEFAULT_PACKAGE_NAME.to_string(),
            identifier: GO_AST_INVENTORY_VAR_DECL_DEFAULT_IDENTIFIER.to_string(),
            declaration_token: "var".to_string(),
            literal_kind: GO_AST_INVENTORY_VAR_DECL_DEFAULT_LITERAL_KIND.to_string(),
            literal_value: GO_AST_INVENTORY_VAR_DECL_DEFAULT_LITERAL_VALUE.to_string(),
            go_version: GO_AST_INVENTORY_VAR_DECL_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryImportSpecPayloadSpec {
    pub package_name: String,
    pub alias: String,
    pub declaration_token: String,
    pub path_literal_kind: String,
    pub path_literal_value: String,
    pub go_version: String,
}

impl GoAstInventoryImportSpecPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PACKAGE_NAME.to_string(),
            alias: GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_ALIAS.to_string(),
            declaration_token: "import".to_string(),
            path_literal_kind: GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_KIND.to_string(),
            path_literal_value: GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_VALUE.to_string(),
            go_version: GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryVarDeclSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventoryVarDeclPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryImportSpecSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventoryImportSpecPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_var_decl_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventoryVarDeclSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_var_decl_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventoryVarDeclPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_var_decl_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventoryVarDeclPayloadSpec,
) -> Result<GoAstInventoryVarDeclSynthesis, SyntaxBackendBoundaryError> {
    validate_var_decl_payload_spec(&payload_spec)?;
    let synthesis_plan = validate_go_ast_inventory_synthesis_plan_from_inventory(
        inventory_path,
        GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID,
        GO_AST_INVENTORY_SYNTHESIS_ROOT,
        &var_decl_expected_field_orders(),
        &var_decl_finite_domain_requirements(&payload_spec),
    )?;

    let mut stream = GoRawTokenStreamBuilder::new(roles);
    stream.source_file(
        &payload_spec.package_name,
        &payload_spec.go_version,
        |stream| {
            stream.alternative_node_field("GenDecl", |stream| {
                stream.missing_field();
                stream.terminal_field(&payload_spec.declaration_token);
                stream.missing_field();
                stream.alternative_node_field("ValueSpec", |stream| {
                    stream.missing_field();
                    stream.ident_node_field(&payload_spec.identifier)?;
                    stream.missing_field();
                    stream.basic_lit_alternative(
                        &payload_spec.literal_kind,
                        &payload_spec.literal_value,
                    )?;
                    stream.missing_field();
                    Ok(())
                })?;
                stream.missing_field();
                Ok(())
            })
        },
    )?;

    Ok(GoAstInventoryVarDeclSynthesis {
        api_id: GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID.to_string(),
        api_boundary: GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_BOUNDARY.to_string(),
        inventory_artifact_path: synthesis_plan.inventory_artifact_path.clone(),
        grammar_id: synthesis_plan.grammar_id.clone(),
        root_rule: GO_AST_INVENTORY_SYNTHESIS_ROOT.to_string(),
        validated_field_orders: synthesis_plan.validated_field_orders.clone(),
        validated_finite_domain_alternatives: synthesis_plan
            .validated_finite_domain_alternatives
            .clone(),
        payload_spec,
        raw_token_ids: stream.raw_token_ids,
        synthesis_plan,
    })
}

pub fn synthesize_go_ast_inventory_import_spec_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventoryImportSpecSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_import_spec_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventoryImportSpecPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_import_spec_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventoryImportSpecPayloadSpec,
) -> Result<GoAstInventoryImportSpecSynthesis, SyntaxBackendBoundaryError> {
    validate_import_spec_payload_spec(&payload_spec)?;
    let registry_synthesis = synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        inventory_path,
        GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new()
            .with_str("package_name", &payload_spec.package_name)
            .with_str("alias", &payload_spec.alias)
            .with_str("declaration_token", &payload_spec.declaration_token)
            .with_str("path_literal_kind", &payload_spec.path_literal_kind)
            .with_str("path_literal_value", &payload_spec.path_literal_value)
            .with_str("go_version", &payload_spec.go_version),
    )?;

    Ok(GoAstInventoryImportSpecSynthesis {
        api_id: GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID.to_string(),
        api_boundary: GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_BOUNDARY.to_string(),
        inventory_artifact_path: registry_synthesis.inventory_artifact_path.clone(),
        grammar_id: registry_synthesis.grammar_id.clone(),
        root_rule: GO_AST_INVENTORY_SYNTHESIS_ROOT.to_string(),
        validated_field_orders: registry_synthesis.validated_field_orders.clone(),
        validated_finite_domain_alternatives: registry_synthesis
            .validated_finite_domain_alternatives
            .clone(),
        payload_spec,
        raw_token_ids: registry_synthesis.raw_token_ids,
        synthesis_plan: registry_synthesis.synthesis_plan,
    })
}

fn validate_var_decl_payload_spec(
    payload_spec: &GoAstInventoryVarDeclPayloadSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_non_empty(
        "go_ast_inventory_var_decl_synthesis.payload_spec.package_name",
        &payload_spec.package_name,
    )?;
    require_non_empty(
        "go_ast_inventory_var_decl_synthesis.payload_spec.identifier",
        &payload_spec.identifier,
    )?;
    require_string(
        "go_ast_inventory_var_decl_synthesis.payload_spec.declaration_token",
        &payload_spec.declaration_token,
        "var",
    )?;
    require_string(
        "go_ast_inventory_var_decl_synthesis.payload_spec.literal_kind",
        &payload_spec.literal_kind,
        "INT",
    )?;
    require_non_empty(
        "go_ast_inventory_var_decl_synthesis.payload_spec.literal_value",
        &payload_spec.literal_value,
    )?;
    require_non_empty(
        "go_ast_inventory_var_decl_synthesis.payload_spec.go_version",
        &payload_spec.go_version,
    )
}

fn validate_import_spec_payload_spec(
    payload_spec: &GoAstInventoryImportSpecPayloadSpec,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_non_empty(
        "go_ast_inventory_import_spec_synthesis.payload_spec.package_name",
        &payload_spec.package_name,
    )?;
    require_non_empty(
        "go_ast_inventory_import_spec_synthesis.payload_spec.alias",
        &payload_spec.alias,
    )?;
    require_string(
        "go_ast_inventory_import_spec_synthesis.payload_spec.declaration_token",
        &payload_spec.declaration_token,
        "import",
    )?;
    require_string(
        "go_ast_inventory_import_spec_synthesis.payload_spec.path_literal_kind",
        &payload_spec.path_literal_kind,
        "STRING",
    )?;
    require_non_empty(
        "go_ast_inventory_import_spec_synthesis.payload_spec.path_literal_value",
        &payload_spec.path_literal_value,
    )?;
    require_non_empty(
        "go_ast_inventory_import_spec_synthesis.payload_spec.go_version",
        &payload_spec.go_version,
    )
}

fn var_decl_expected_field_orders() -> Vec<GoAstInventoryExpectedFieldOrder> {
    vec![
        GoAstInventoryExpectedFieldOrder::new(
            "File",
            &[
                "Doc",
                "Name",
                "Decls",
                "Comments",
                "GoVersion",
                "CommentBefore",
                "NoBlankLineBefore",
            ][..],
        ),
        GoAstInventoryExpectedFieldOrder::new(
            "GenDecl",
            &["Doc", "Tok", "Lparen", "Specs", "Rparen"][..],
        ),
        GoAstInventoryExpectedFieldOrder::new(
            "ValueSpec",
            &["Doc", "Names", "Type", "Values", "Comment"][..],
        ),
        GoAstInventoryExpectedFieldOrder::new("Ident", &["Name"][..]),
        GoAstInventoryExpectedFieldOrder::new("BasicLit", &["Kind", "Value"][..]),
    ]
}

fn var_decl_finite_domain_requirements(
    payload_spec: &GoAstInventoryVarDeclPayloadSpec,
) -> Vec<GoAstInventoryFiniteDomainRequirement> {
    vec![
        GoAstInventoryFiniteDomainRequirement::new(
            "GenDecl",
            "Tok",
            "go_token_declaration_tokens.VAR",
            GrammarTerminalKind::IdentifierPayload,
        ),
        GoAstInventoryFiniteDomainRequirement::new(
            "BasicLit",
            "Kind",
            &format!("go_token_basic_literal_kinds.{}", payload_spec.literal_kind),
            GrammarTerminalKind::IdentifierPayload,
        ),
    ]
}
