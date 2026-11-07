use super::core::{
    GoAstInventoryExpectedFieldOrder, GoAstInventoryFiniteDomainDecision,
    GoAstInventoryFiniteDomainRequirement, GoAstInventoryRuleFieldOrder,
    GoAstInventorySynthesisPlan, require_non_empty, require_string,
};
use super::{
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
    GO_AST_INVENTORY_IF_STMT_SYNTHESIS_BOUNDARY, GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_ALIAS,
    GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_GO_VERSION,
    GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PACKAGE_NAME,
    GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_KIND,
    GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_VALUE,
    GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID, GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_BOUNDARY,
    GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID,
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
    GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_BOUNDARY, GoAstInventoryDeclarativeExpressionSpec,
    GoAstInventoryDeclarativeFieldSpec, GoAstInventoryDeclarativeFieldValue,
    GoAstInventoryDeclarativeNodeSpec, GoAstInventoryDeclarativeParentFieldSpec,
    GoAstInventoryDeclarativeTemplateCoverageSpec, GoAstInventoryExpressionTemplate,
    GoAstInventoryTemplateSpec, check_go_ast_inventory_declarative_expression_spec_from_inventory,
    synthesize_go_ast_inventory_template_raw_tokens_from_inventory,
};
use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::grammar_ir::GrammarTerminalKind;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const GO_AST_INVENTORY_SYNTHESIS_REGISTRY_API_ID: &str =
    "go-ast-inventory-synthesis-registry-v1";
pub const GO_AST_INVENTORY_SYNTHESIS_REGISTRY_BOUNDARY: &str = "registered (api id + declarative spec + payload defaults) entry -> one generic checked synthesis pipeline -> token-only GoSourceFile raw-token stream";

const GO_AST_INVENTORY_REGISTRY_ROOT: &str = "GoSourceFile";
const GO_AST_INVENTORY_REGISTRY_PACKAGE_NAME_KEY: &str = "package_name";
const GO_AST_INVENTORY_REGISTRY_GO_VERSION_KEY: &str = "go_version";

pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_LHS_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_OPERATOR: &str = "=";
pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_RETURN_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_BRANCH_TOKEN: &str = "break";
pub(super) const GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_RETURN_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_RETURN_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_RANGE_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_RANGE_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_RANGE_STMT_DEFAULT_KEY_IDENTIFIER: &str = "i";
pub(super) const GO_AST_INVENTORY_RANGE_STMT_DEFAULT_TOKEN: &str = ":=";
pub(super) const GO_AST_INVENTORY_RANGE_STMT_DEFAULT_SOURCE_IDENTIFIER: &str = "xs";
pub(super) const GO_AST_INVENTORY_RANGE_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_TARGET_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_EMPTY_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_EMPTY_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_EMPTY_STMT_DEFAULT_IMPLICIT: bool = false;
pub(super) const GO_AST_INVENTORY_EMPTY_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_TARGET_IDENTIFIER: &str = "i";
pub(super) const GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_TOKEN: &str = "++";
pub(super) const GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_SEND_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_SEND_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_SEND_STMT_DEFAULT_CHANNEL_IDENTIFIER: &str = "ch";
pub(super) const GO_AST_INVENTORY_SEND_STMT_DEFAULT_VALUE_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_SEND_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_LABELED_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_LABELED_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_LABELED_STMT_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_LABELED_STMT_DEFAULT_LABEL_IDENTIFIER: &str = "label";
pub(super) const GO_AST_INVENTORY_LABELED_STMT_DEFAULT_RETURN_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_LABELED_STMT_DEFAULT_RETURN_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_LABELED_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_SWITCH_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_SWITCH_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_CASE_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_CASE_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_RETURN_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_RETURN_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_FILE_COMMENT: &str = "// tct: comm clause";
pub(super) const GO_AST_INVENTORY_DECL_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_DECL_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_DECL_STMT_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_DECL_STMT_DEFAULT_DECL_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_DECL_STMT_DEFAULT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_DECL_STMT_DEFAULT_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_DECL_STMT_DEFAULT_RETURN_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_DECL_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_DEFER_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_DEFER_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_DEFER_STMT_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_DEFER_STMT_DEFAULT_CALL_RECEIVER: &str = "m";
pub(super) const GO_AST_INVENTORY_DEFER_STMT_DEFAULT_CALL_SELECTOR: &str = "N";
pub(super) const GO_AST_INVENTORY_DEFER_STMT_DEFAULT_RETURN_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_DEFER_STMT_DEFAULT_RETURN_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_DEFER_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_GO_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_GO_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_GO_STMT_DEFAULT_CALL_RECEIVER: &str = "m";
pub(super) const GO_AST_INVENTORY_GO_STMT_DEFAULT_CALL_SELECTOR: &str = "N";
pub(super) const GO_AST_INVENTORY_GO_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_OPERATOR: &str = ":=";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_COND_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_BODY_RETURN_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_BODY_RETURN_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_ELSE_RETURN_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_ELSE_RETURN_LITERAL_VALUE: &str = "0";
pub(super) const GO_AST_INVENTORY_IF_STMT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_ELEMENT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_RETURN_IDENTIFIER: &str = "nil";
pub(super) const GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_DIRECTION: &str = "SEND_RECV";
pub(super) const GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_ELEMENT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_RETURN_IDENTIFIER: &str = "nil";
pub(super) const GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_LEFT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_LEFT_LITERAL_VALUE: &str = "2";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_OPERATOR: &str = "+";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_RIGHT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_RIGHT_LITERAL_VALUE: &str = "3";
pub(super) const GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_OPERATOR: &str = "-";
pub(super) const GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER: &str = "xs";
pub(super) const GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_INDEX_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_INDEX_LITERAL_VALUE: &str = "0";
pub(super) const GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER: &str = "g";
pub(super) const GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_FIRST_INDEX_IDENTIFIER: &str =
    "int";
pub(super) const GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_SECOND_INDEX_IDENTIFIER: &str =
    "string";
pub(super) const GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER: &str = "xs";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_VALUE: &str = "2";
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_THREE_INDEX: bool = false;
pub(super) const GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_RECEIVER: &str = "m";
pub(super) const GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_SELECTOR: &str = "N";
pub(super) const GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_ELEMENT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_INCOMPLETE: bool = false;
pub(super) const GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_IDENTIFIER: &str = "xs";
pub(super) const GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_ELEMENT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_INCOMPLETE: bool = false;
pub(super) const GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_TYPE: &str = "string";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_KIND: &str = "STRING";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_VALUE: &str = "\"k\"";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_VALUE: &str = "1";
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_INCOMPLETE: bool = false;
pub(super) const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_FUNC_LIT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_FUNC_LIT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_FUNC_LIT_DEFAULT_SIGNATURE_POLICY: &str = "no_params_no_results";
pub(super) const GO_AST_INVENTORY_FUNC_LIT_DEFAULT_BODY_POLICY: &str = "empty_block";
pub(super) const GO_AST_INVENTORY_FUNC_LIT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_ASSERTED_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_SIGNATURE_POLICY: &str =
    "no_params_no_results";
pub(super) const GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_RETURN_IDENTIFIER: &str = "nil";
pub(super) const GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_IDENTIFIER: &str = "f";
pub(super) const GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_RESULT_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_LITERAL_KIND: &str = "INT";
pub(super) const GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_LITERAL_VALUE: &str = "7";
pub(super) const GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_IDENTIFIER: &str = "x";
pub(super) const GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_METHODS_POLICY: &str =
    "empty_method_set";
pub(super) const GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_INCOMPLETE: bool = false;
pub(super) const GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_GO_VERSION: &str = "go1.24";
pub(super) const GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_PACKAGE_NAME: &str = "p";
pub(super) const GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_TYPE_NAME: &str = "S";
pub(super) const GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELD_NAME: &str = "X";
pub(super) const GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELD_TYPE: &str = "int";
pub(super) const GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELDS_POLICY: &str =
    "single_named_field";
pub(super) const GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_INCOMPLETE: bool = false;
pub(super) const GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_GO_VERSION: &str = "go1.24";

/// One concrete payload value supplied to (or defaulted by) a registry entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoAstInventoryRegistryPayloadValue {
    Str(String),
    Bool(bool),
}

/// Key-value payload for the generic registry synthesis pipeline. Keys not
/// declared by the selected entry are rejected; declared keys missing from the
/// payload are filled from the entry defaults before any rule runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoAstInventoryRegistryPayload {
    values: BTreeMap<String, GoAstInventoryRegistryPayloadValue>,
}

impl GoAstInventoryRegistryPayload {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_str(mut self, key: &str, value: &str) -> Self {
        self.values.insert(
            key.to_string(),
            GoAstInventoryRegistryPayloadValue::Str(value.to_string()),
        );
        self
    }

    pub fn with_bool(mut self, key: &str, value: bool) -> Self {
        self.values.insert(
            key.to_string(),
            GoAstInventoryRegistryPayloadValue::Bool(value),
        );
        self
    }

    pub fn str_value(&self, key: &str) -> Result<&str, SyntaxBackendBoundaryError> {
        match self.values.get(key) {
            Some(GoAstInventoryRegistryPayloadValue::Str(value)) => Ok(value),
            Some(GoAstInventoryRegistryPayloadValue::Bool(_)) => {
                Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_synthesis_registry.payload",
                    format!("payload entry {key} is not a string value"),
                ))
            }
            None => Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_synthesis_registry.payload",
                format!("missing string payload entry {key}"),
            )),
        }
    }

    pub fn bool_value(&self, key: &str) -> Result<bool, SyntaxBackendBoundaryError> {
        match self.values.get(key) {
            Some(GoAstInventoryRegistryPayloadValue::Bool(value)) => Ok(*value),
            Some(GoAstInventoryRegistryPayloadValue::Str(_)) => {
                Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_synthesis_registry.payload",
                    format!("payload entry {key} is not a boolean value"),
                ))
            }
            None => Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_synthesis_registry.payload",
                format!("missing boolean payload entry {key}"),
            )),
        }
    }
}

/// Default value for one declared payload key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoAstInventoryRegistryPayloadDefault {
    Str(&'static str),
    Bool(bool),
}

/// Declarative validation rule applied to one merged payload value, with the
/// per-entry error-context string preserved from the per-construct modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoAstInventoryRegistryPayloadRule {
    /// String value must be non-empty.
    NonEmpty,
    /// String value must equal the pinned v1 literal.
    Exact(&'static str),
    /// Boolean value must be false; the message documents the v1 restriction.
    ForbiddenTrue(&'static str),
}

/// One declared payload key: lookup key, error-context string, default, rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoAstInventoryRegistryPayloadField {
    pub key: &'static str,
    pub error_context: &'static str,
    pub default: GoAstInventoryRegistryPayloadDefault,
    pub rule: GoAstInventoryRegistryPayloadRule,
}

/// The fixed source-file shells shared by every registry entry; maps 1:1 onto
/// the `GoAstInventoryTemplateSpec` shell constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoAstInventorySynthesisShell {
    FunctionReturn,
    FunctionStatementListReturn,
    FunctionStatementList,
    FunctionStatementListNoResult,
    PackageVarValue,
    PackageVarType,
    PackageType,
    PackageImportSpec,
}

/// One expression slot produced by an entry. `Declarative` slots are checked
/// against the inventory (deriving field orders and finite-domain
/// requirements); `Legacy` slots pass an expression template through unchecked
/// during the migration window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoAstInventoryRegistryExpressionSlot {
    Legacy(GoAstInventoryExpressionTemplate),
    Declarative {
        parent_field: Option<GoAstInventoryDeclarativeParentFieldSpec>,
        expression: GoAstInventoryDeclarativeExpressionSpec,
    },
}

/// Shell slot values built from a merged payload by an entry's expressions
/// constructor. The shell decides which slots are required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryRegistryExpressions {
    pub result_type: Option<GoAstInventoryRegistryExpressionSlot>,
    pub statements: Vec<GoAstInventoryRegistryExpressionSlot>,
    pub expression: GoAstInventoryRegistryExpressionSlot,
}

pub type GoAstInventoryRegistryExpressionsFn =
    fn(
        &GoAstInventoryRegistryPayload,
    ) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError>;
pub type GoAstInventoryRegistryExpectedFieldOrdersFn =
    fn(
        &GoAstInventoryRegistryPayload,
    ) -> Result<Vec<GoAstInventoryExpectedFieldOrder>, SyntaxBackendBoundaryError>;
pub type GoAstInventoryRegistryFiniteDomainRequirementsFn =
    fn(
        &GoAstInventoryRegistryPayload,
    ) -> Result<Vec<GoAstInventoryFiniteDomainRequirement>, SyntaxBackendBoundaryError>;

/// One registered construct: adding a construct is adding one of these
/// entries (data plus payload-to-data constructors), not a new module.
#[derive(Debug, Clone, Copy)]
pub struct GoAstInventorySynthesisRegistryEntry {
    pub api_id: &'static str,
    pub api_boundary: &'static str,
    pub shell: GoAstInventorySynthesisShell,
    /// Payload key carrying the shell's declaration identifier (function,
    /// variable, or type name).
    pub declaration_identifier_key: &'static str,
    pub payload_fields: &'static [GoAstInventoryRegistryPayloadField],
    pub expressions: GoAstInventoryRegistryExpressionsFn,
    pub extra_expected_field_orders: GoAstInventoryRegistryExpectedFieldOrdersFn,
    pub extra_finite_domain_requirements: GoAstInventoryRegistryFiniteDomainRequirementsFn,
}

#[derive(Debug, Clone)]
pub struct GoAstInventorySynthesisRegistry {
    entries: Vec<GoAstInventorySynthesisRegistryEntry>,
}

impl GoAstInventorySynthesisRegistry {
    pub fn from_entries(
        entries: Vec<GoAstInventorySynthesisRegistryEntry>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let mut seen_api_ids = BTreeSet::new();
        for entry in &entries {
            require_non_empty("go_ast_inventory_synthesis_registry.api_id", entry.api_id)?;
            require_non_empty(
                "go_ast_inventory_synthesis_registry.api_boundary",
                entry.api_boundary,
            )?;
            if !seen_api_ids.insert(entry.api_id) {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_synthesis_registry.entries",
                    format!(
                        "duplicate synthesis registry entry for api id {}",
                        entry.api_id
                    ),
                ));
            }
            let mut seen_keys = BTreeSet::new();
            for field in entry.payload_fields {
                if !seen_keys.insert(field.key) {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_inventory_synthesis_registry.entries",
                        format!(
                            "registry entry {} declares duplicate payload key {}",
                            entry.api_id, field.key
                        ),
                    ));
                }
                let rule_matches_default_kind = matches!(
                    (field.default, field.rule),
                    (
                        GoAstInventoryRegistryPayloadDefault::Str(_),
                        GoAstInventoryRegistryPayloadRule::NonEmpty
                            | GoAstInventoryRegistryPayloadRule::Exact(_),
                    ) | (
                        GoAstInventoryRegistryPayloadDefault::Bool(_),
                        GoAstInventoryRegistryPayloadRule::ForbiddenTrue(_),
                    )
                );
                if !rule_matches_default_kind {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_inventory_synthesis_registry.entries",
                        format!(
                            "registry entry {} payload key {} pairs rule {:?} with default {:?}; string rules require string defaults and boolean rules require boolean defaults",
                            entry.api_id, field.key, field.rule, field.default
                        ),
                    ));
                }
            }
        }
        Ok(Self { entries })
    }

    pub fn entry(
        &self,
        api_id: &str,
    ) -> Result<&GoAstInventorySynthesisRegistryEntry, SyntaxBackendBoundaryError> {
        self.entries
            .iter()
            .find(|entry| entry.api_id == api_id)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_synthesis_registry.api_id",
                    format!("unknown synthesis registry api id {api_id}"),
                )
            })
    }

    pub fn entries(&self) -> &[GoAstInventorySynthesisRegistryEntry] {
        &self.entries
    }
}

/// The canonical registry. Entries are appended here; one entry per construct.
pub fn go_ast_inventory_synthesis_registry()
-> Result<GoAstInventorySynthesisRegistry, SyntaxBackendBoundaryError> {
    GoAstInventorySynthesisRegistry::from_entries(vec![
        go_ast_inventory_import_spec_registry_entry(),
        go_ast_inventory_assignment_registry_entry(),
        go_ast_inventory_branch_stmt_registry_entry(),
        go_ast_inventory_range_stmt_registry_entry(),
        go_ast_inventory_type_switch_stmt_registry_entry(),
        go_ast_inventory_labeled_stmt_registry_entry(),
        go_ast_inventory_case_clause_registry_entry(),
        go_ast_inventory_comm_clause_registry_entry(),
        go_ast_inventory_decl_stmt_registry_entry(),
        go_ast_inventory_defer_stmt_registry_entry(),
        go_ast_inventory_go_stmt_registry_entry(),
        go_ast_inventory_if_stmt_registry_entry(),
        go_ast_inventory_empty_stmt_registry_entry(),
        go_ast_inventory_inc_dec_stmt_registry_entry(),
        go_ast_inventory_send_stmt_registry_entry(),
        go_ast_inventory_array_type_return_registry_entry(),
        go_ast_inventory_chan_type_return_registry_entry(),
        go_ast_inventory_binary_expr_return_registry_entry(),
        go_ast_inventory_unary_expr_return_registry_entry(),
        go_ast_inventory_paren_expr_return_registry_entry(),
        go_ast_inventory_index_expr_return_registry_entry(),
        go_ast_inventory_index_list_expr_return_registry_entry(),
        go_ast_inventory_interface_type_var_registry_entry(),
        go_ast_inventory_struct_type_decl_registry_entry(),
        go_ast_inventory_slice_expr_return_registry_entry(),
        go_ast_inventory_selector_call_registry_entry(),
        go_ast_inventory_composite_lit_return_registry_entry(),
        go_ast_inventory_ellipsis_array_literal_registry_entry(),
        go_ast_inventory_key_value_expr_return_registry_entry(),
        go_ast_inventory_func_lit_registry_entry(),
        go_ast_inventory_star_expr_return_registry_entry(),
        go_ast_inventory_type_assert_expr_return_registry_entry(),
        go_ast_inventory_function_return_registry_entry(),
        go_ast_inventory_func_type_result_registry_entry(),
    ])
}

/// Generic synthesis result shared by every registry entry; per-construct
/// shims copy these fields into their typed result structs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryRegistrySynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload: GoAstInventoryRegistryPayload,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_registry_raw_tokens(
    roles: &SyntaxTokenRoles,
    api_id: &str,
    payload: GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistrySynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        api_id,
        payload,
    )
}

pub fn synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    api_id: &str,
    payload: GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistrySynthesis, SyntaxBackendBoundaryError> {
    let registry = go_ast_inventory_synthesis_registry()?;
    let entry = registry.entry(api_id)?;
    let payload = merge_payload_with_entry_defaults(entry, payload)?;
    for field in entry.payload_fields {
        apply_payload_rule(field, &payload)?;
    }
    let expressions = (entry.expressions)(&payload)?;

    let mut derived_orders = Vec::new();
    let mut derived_requirements = Vec::new();
    let resolved_result_type = expressions
        .result_type
        .as_ref()
        .map(|slot| {
            resolve_expression_slot(
                inventory_path,
                slot,
                &mut derived_orders,
                &mut derived_requirements,
            )
        })
        .transpose()?;
    let resolved_statements = expressions
        .statements
        .iter()
        .map(|slot| {
            resolve_declarative_statement_slot(
                inventory_path,
                entry.api_id,
                slot,
                &mut derived_orders,
                &mut derived_requirements,
            )
        })
        .collect::<Result<Vec<_>, SyntaxBackendBoundaryError>>()?;
    let resolved_expression = resolve_expression_slot(
        inventory_path,
        &expressions.expression,
        &mut derived_orders,
        &mut derived_requirements,
    )?;
    let resolved_source_file_comments = source_file_comment_slots_for_entry(entry, &payload)?
        .iter()
        .map(|slot| {
            resolve_declarative_source_file_comment_slot(
                inventory_path,
                entry.api_id,
                slot,
                &mut derived_orders,
                &mut derived_requirements,
            )
        })
        .collect::<Result<Vec<_>, SyntaxBackendBoundaryError>>()?;

    let expected_field_orders = combine_field_order_requirements(
        combine_field_order_requirements(
            shell_base_expected_field_orders(entry.shell),
            derived_orders,
        ),
        (entry.extra_expected_field_orders)(&payload)?,
    );
    let finite_domain_requirements = combine_finite_domain_requirements(
        combine_finite_domain_requirements(
            shell_base_finite_domain_requirements(entry.shell),
            derived_requirements,
        ),
        (entry.extra_finite_domain_requirements)(&payload)?,
    );

    let package_name = payload.str_value(GO_AST_INVENTORY_REGISTRY_PACKAGE_NAME_KEY)?;
    let go_version = payload.str_value(GO_AST_INVENTORY_REGISTRY_GO_VERSION_KEY)?;
    let declaration_identifier = payload.str_value(entry.declaration_identifier_key)?;
    let template_spec = build_shell_template_spec(
        entry,
        package_name,
        go_version,
        declaration_identifier,
        resolved_result_type,
        resolved_statements,
        resolved_expression,
        resolved_source_file_comments,
        expected_field_orders,
        finite_domain_requirements,
    )?;
    let template_synthesis = synthesize_go_ast_inventory_template_raw_tokens_from_inventory(
        roles,
        inventory_path,
        template_spec,
    )?;

    Ok(GoAstInventoryRegistrySynthesis {
        api_id: entry.api_id.to_string(),
        api_boundary: entry.api_boundary.to_string(),
        inventory_artifact_path: template_synthesis.inventory_artifact_path.clone(),
        grammar_id: template_synthesis.grammar_id.clone(),
        root_rule: GO_AST_INVENTORY_REGISTRY_ROOT.to_string(),
        validated_field_orders: template_synthesis.validated_field_orders.clone(),
        validated_finite_domain_alternatives: template_synthesis
            .validated_finite_domain_alternatives
            .clone(),
        payload,
        raw_token_ids: template_synthesis.raw_token_ids,
        synthesis_plan: template_synthesis.synthesis_plan,
    })
}

fn merge_payload_with_entry_defaults(
    entry: &GoAstInventorySynthesisRegistryEntry,
    payload: GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryPayload, SyntaxBackendBoundaryError> {
    for key in payload.values.keys() {
        if !entry.payload_fields.iter().any(|field| field.key == key) {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_synthesis_registry.payload",
                format!(
                    "synthesis api id {} does not declare payload entry {key}",
                    entry.api_id
                ),
            ));
        }
    }
    let mut merged = payload;
    for field in entry.payload_fields {
        match (merged.values.get(field.key), field.default) {
            (
                Some(GoAstInventoryRegistryPayloadValue::Str(_)),
                GoAstInventoryRegistryPayloadDefault::Str(_),
            )
            | (
                Some(GoAstInventoryRegistryPayloadValue::Bool(_)),
                GoAstInventoryRegistryPayloadDefault::Bool(_),
            ) => {}
            (Some(_), GoAstInventoryRegistryPayloadDefault::Str(_)) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_synthesis_registry.payload",
                    format!(
                        "payload entry {} for synthesis api id {} expected a string value",
                        field.key, entry.api_id
                    ),
                ));
            }
            (Some(_), GoAstInventoryRegistryPayloadDefault::Bool(_)) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_synthesis_registry.payload",
                    format!(
                        "payload entry {} for synthesis api id {} expected a boolean value",
                        field.key, entry.api_id
                    ),
                ));
            }
            (None, GoAstInventoryRegistryPayloadDefault::Str(default)) => {
                merged.values.insert(
                    field.key.to_string(),
                    GoAstInventoryRegistryPayloadValue::Str(default.to_string()),
                );
            }
            (None, GoAstInventoryRegistryPayloadDefault::Bool(default)) => {
                merged.values.insert(
                    field.key.to_string(),
                    GoAstInventoryRegistryPayloadValue::Bool(default),
                );
            }
        }
    }
    Ok(merged)
}

fn apply_payload_rule(
    field: &GoAstInventoryRegistryPayloadField,
    payload: &GoAstInventoryRegistryPayload,
) -> Result<(), SyntaxBackendBoundaryError> {
    match field.rule {
        GoAstInventoryRegistryPayloadRule::NonEmpty => {
            require_non_empty(field.error_context, payload.str_value(field.key)?)
        }
        GoAstInventoryRegistryPayloadRule::Exact(expected) => {
            require_string(field.error_context, payload.str_value(field.key)?, expected)
        }
        GoAstInventoryRegistryPayloadRule::ForbiddenTrue(message) => {
            if payload.bool_value(field.key)? {
                Err(SyntaxBackendBoundaryError::new(
                    field.error_context,
                    message,
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn resolve_expression_slot(
    inventory_path: &Path,
    slot: &GoAstInventoryRegistryExpressionSlot,
    derived_orders: &mut Vec<GoAstInventoryExpectedFieldOrder>,
    derived_requirements: &mut Vec<GoAstInventoryFiniteDomainRequirement>,
) -> Result<GoAstInventoryExpressionTemplate, SyntaxBackendBoundaryError> {
    match slot {
        GoAstInventoryRegistryExpressionSlot::Legacy(template) => Ok(template.clone()),
        GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field,
            expression,
        } => {
            let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
                inventory_path,
                parent_field.as_ref(),
                expression,
            )?;
            derived_orders.extend(checked.expected_field_orders);
            derived_requirements.extend(checked.finite_domain_requirements);
            Ok(GoAstInventoryExpressionTemplate::Declarative(
                checked.expression,
            ))
        }
    }
}

fn resolve_declarative_statement_slot(
    inventory_path: &Path,
    api_id: &str,
    slot: &GoAstInventoryRegistryExpressionSlot,
    derived_orders: &mut Vec<GoAstInventoryExpectedFieldOrder>,
    derived_requirements: &mut Vec<GoAstInventoryFiniteDomainRequirement>,
) -> Result<GoAstInventoryDeclarativeExpressionSpec, SyntaxBackendBoundaryError> {
    match slot {
        GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field,
            expression,
        } => {
            let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
                inventory_path,
                parent_field.as_ref(),
                expression,
            )?;
            derived_orders.extend(checked.expected_field_orders);
            derived_requirements.extend(checked.finite_domain_requirements);
            Ok(checked.expression)
        }
        GoAstInventoryRegistryExpressionSlot::Legacy(_) => Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_synthesis_registry.expressions",
            format!("synthesis api id {api_id} statement slots must be declarative"),
        )),
    }
}

fn resolve_declarative_source_file_comment_slot(
    inventory_path: &Path,
    api_id: &str,
    slot: &GoAstInventoryRegistryExpressionSlot,
    derived_orders: &mut Vec<GoAstInventoryExpectedFieldOrder>,
    derived_requirements: &mut Vec<GoAstInventoryFiniteDomainRequirement>,
) -> Result<GoAstInventoryDeclarativeExpressionSpec, SyntaxBackendBoundaryError> {
    match slot {
        GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field,
            expression,
        } => {
            let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
                inventory_path,
                parent_field.as_ref(),
                expression,
            )?;
            derived_orders.extend(checked.expected_field_orders);
            derived_requirements.extend(checked.finite_domain_requirements);
            Ok(checked.expression)
        }
        GoAstInventoryRegistryExpressionSlot::Legacy(_) => Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_synthesis_registry.source_file_comments",
            format!("synthesis api id {api_id} source-file comment slots must be declarative"),
        )),
    }
}

fn source_file_comment_slots_for_entry(
    entry: &GoAstInventorySynthesisRegistryEntry,
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryRegistryExpressionSlot>, SyntaxBackendBoundaryError> {
    let comments = match entry.api_id {
        GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID => {
            vec![GoAstInventoryRegistryExpressionSlot::Declarative {
                parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                    "File", "Comments",
                )),
                expression: GoAstInventoryDeclarativeExpressionSpec::new(
                    declarative_comment_group_node(
                        GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_FILE_COMMENT,
                    ),
                ),
            }]
        }
        _ => Vec::new(),
    };
    Ok(comments)
}

#[allow(clippy::too_many_arguments)]
fn build_shell_template_spec(
    entry: &GoAstInventorySynthesisRegistryEntry,
    package_name: &str,
    go_version: &str,
    declaration_identifier: &str,
    result_type: Option<GoAstInventoryExpressionTemplate>,
    statements: Vec<GoAstInventoryDeclarativeExpressionSpec>,
    expression: GoAstInventoryExpressionTemplate,
    source_file_comments: Vec<GoAstInventoryDeclarativeExpressionSpec>,
    expected_field_orders: Vec<GoAstInventoryExpectedFieldOrder>,
    finite_domain_requirements: Vec<GoAstInventoryFiniteDomainRequirement>,
) -> Result<GoAstInventoryTemplateSpec, SyntaxBackendBoundaryError> {
    let shell_mismatch = |message: String| {
        SyntaxBackendBoundaryError::new("go_ast_inventory_synthesis_registry.expressions", message)
    };
    if !matches!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListReturn
            | GoAstInventorySynthesisShell::FunctionStatementList
            | GoAstInventorySynthesisShell::FunctionStatementListNoResult
    ) && !statements.is_empty()
    {
        return Err(shell_mismatch(format!(
            "synthesis api id {} shell {:?} does not accept statement slots",
            entry.api_id, entry.shell
        )));
    }
    let mut template_spec = match (entry.shell, result_type) {
        (GoAstInventorySynthesisShell::FunctionReturn, Some(result_type)) => {
            Ok(GoAstInventoryTemplateSpec::function_return_expression(
                entry.api_id,
                package_name,
                go_version,
                declaration_identifier,
                result_type,
                expression,
                expected_field_orders,
                finite_domain_requirements,
            ))
        }
        (GoAstInventorySynthesisShell::FunctionStatementListReturn, Some(result_type)) => Ok(
            GoAstInventoryTemplateSpec::function_statement_list_return_expression(
                entry.api_id,
                package_name,
                go_version,
                declaration_identifier,
                result_type,
                statements,
                expression,
                expected_field_orders,
                finite_domain_requirements,
            ),
        ),
        (GoAstInventorySynthesisShell::FunctionStatementList, Some(result_type)) => {
            Ok(GoAstInventoryTemplateSpec::function_statement_list(
                entry.api_id,
                package_name,
                go_version,
                declaration_identifier,
                result_type,
                statements,
                expected_field_orders,
                finite_domain_requirements,
            ))
        }
        (GoAstInventorySynthesisShell::FunctionStatementListNoResult, None) => Ok(
            GoAstInventoryTemplateSpec::function_statement_list_no_result(
                entry.api_id,
                package_name,
                go_version,
                declaration_identifier,
                statements,
                expected_field_orders,
                finite_domain_requirements,
            ),
        ),
        (
            GoAstInventorySynthesisShell::FunctionReturn
            | GoAstInventorySynthesisShell::FunctionStatementListReturn,
            None,
        )
        | (GoAstInventorySynthesisShell::FunctionStatementList, None) => {
            Err(shell_mismatch(format!(
                "synthesis api id {} shell {:?} requires a result_type expression slot",
                entry.api_id, entry.shell
            )))
        }
        (GoAstInventorySynthesisShell::FunctionStatementListNoResult, Some(_)) => {
            Err(shell_mismatch(format!(
                "synthesis api id {} shell {:?} does not accept a result_type expression slot",
                entry.api_id, entry.shell
            )))
        }
        (GoAstInventorySynthesisShell::PackageVarValue, None) => {
            Ok(GoAstInventoryTemplateSpec::package_var_value_expression(
                entry.api_id,
                package_name,
                go_version,
                declaration_identifier,
                expression,
                expected_field_orders,
                finite_domain_requirements,
            ))
        }
        (GoAstInventorySynthesisShell::PackageVarType, None) => {
            Ok(GoAstInventoryTemplateSpec::package_var_type_expression(
                entry.api_id,
                package_name,
                go_version,
                declaration_identifier,
                expression,
                expected_field_orders,
                finite_domain_requirements,
            ))
        }
        (GoAstInventorySynthesisShell::PackageType, None) => {
            Ok(GoAstInventoryTemplateSpec::package_type_expression(
                entry.api_id,
                package_name,
                go_version,
                declaration_identifier,
                expression,
                expected_field_orders,
                finite_domain_requirements,
            ))
        }
        (GoAstInventorySynthesisShell::PackageImportSpec, None) => {
            Ok(GoAstInventoryTemplateSpec::package_import_spec(
                entry.api_id,
                package_name,
                go_version,
                expression,
                expected_field_orders,
                finite_domain_requirements,
            ))
        }
        (
            GoAstInventorySynthesisShell::PackageVarValue
            | GoAstInventorySynthesisShell::PackageVarType
            | GoAstInventorySynthesisShell::PackageType
            | GoAstInventorySynthesisShell::PackageImportSpec,
            Some(_),
        ) => Err(shell_mismatch(format!(
            "synthesis api id {} shell {:?} does not accept a result_type expression slot",
            entry.api_id, entry.shell
        ))),
    }?;
    template_spec.source_file.comments = source_file_comments;
    Ok(template_spec)
}

fn shell_base_expected_field_orders(
    shell: GoAstInventorySynthesisShell,
) -> Vec<GoAstInventoryExpectedFieldOrder> {
    match shell {
        GoAstInventorySynthesisShell::FunctionReturn
        | GoAstInventorySynthesisShell::FunctionStatementListReturn => {
            function_return_base_expected_field_orders()
        }
        GoAstInventorySynthesisShell::FunctionStatementList => {
            function_statement_list_base_expected_field_orders()
        }
        GoAstInventorySynthesisShell::FunctionStatementListNoResult => {
            function_statement_list_no_result_base_expected_field_orders()
        }
        GoAstInventorySynthesisShell::PackageVarValue => {
            package_var_value_base_expected_field_orders()
        }
        GoAstInventorySynthesisShell::PackageVarType => {
            package_var_type_base_expected_field_orders()
        }
        GoAstInventorySynthesisShell::PackageType => package_type_decl_base_expected_field_orders(),
        GoAstInventorySynthesisShell::PackageImportSpec => {
            package_import_spec_base_expected_field_orders()
        }
    }
}

fn shell_base_finite_domain_requirements(
    shell: GoAstInventorySynthesisShell,
) -> Vec<GoAstInventoryFiniteDomainRequirement> {
    match shell {
        GoAstInventorySynthesisShell::FunctionReturn
        | GoAstInventorySynthesisShell::FunctionStatementListReturn
        | GoAstInventorySynthesisShell::FunctionStatementList => {
            function_return_presence_finite_domain_requirements()
        }
        GoAstInventorySynthesisShell::FunctionStatementListNoResult => {
            function_no_result_presence_finite_domain_requirements()
        }
        GoAstInventorySynthesisShell::PackageVarValue => {
            package_var_value_finite_domain_requirements()
        }
        GoAstInventorySynthesisShell::PackageVarType => {
            package_var_type_finite_domain_requirements()
        }
        GoAstInventorySynthesisShell::PackageType => package_type_decl_finite_domain_requirements(),
        GoAstInventorySynthesisShell::PackageImportSpec => {
            package_import_spec_finite_domain_requirements()
        }
    }
}

const GO_AST_INVENTORY_IMPORT_SPEC_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_import_spec_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "alias",
        error_context: "go_ast_inventory_import_spec_synthesis.payload_spec.alias",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_ALIAS,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "declaration_token",
        error_context: "go_ast_inventory_import_spec_synthesis.payload_spec.declaration_token",
        default: GoAstInventoryRegistryPayloadDefault::Str("import"),
        rule: GoAstInventoryRegistryPayloadRule::Exact("import"),
    },
    GoAstInventoryRegistryPayloadField {
        key: "path_literal_kind",
        error_context: "go_ast_inventory_import_spec_synthesis.payload_spec.path_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "path_literal_value",
        error_context: "go_ast_inventory_import_spec_synthesis.payload_spec.path_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_import_spec_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_import_spec_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::PackageImportSpec,
        declaration_identifier_key: "alias",
        payload_fields: GO_AST_INVENTORY_IMPORT_SPEC_REGISTRY_PAYLOAD_FIELDS,
        expressions: import_spec_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn import_spec_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    payload.str_value("declaration_token")?;
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "GenDecl", "Specs",
            )),
            expression: go_ast_inventory_import_spec_expression_spec(
                payload.str_value("alias")?,
                payload.str_value("path_literal_kind")?,
                payload.str_value("path_literal_value")?,
            ),
        },
    })
}

pub(crate) fn go_ast_inventory_import_spec_declarative_coverage_spec()
-> GoAstInventoryDeclarativeTemplateCoverageSpec {
    GoAstInventoryDeclarativeTemplateCoverageSpec {
        synthesis_api_id: GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID.to_string(),
        parent_field: GoAstInventoryDeclarativeParentFieldSpec::new("GenDecl", "Specs"),
        expression: go_ast_inventory_import_spec_expression_spec(
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_ALIAS,
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_KIND,
            GO_AST_INVENTORY_IMPORT_SPEC_DEFAULT_PATH_LITERAL_VALUE,
        ),
    }
}

const GO_AST_INVENTORY_ASSIGNMENT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] =
    &[
        GoAstInventoryRegistryPayloadField {
            key: "package_name",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.package_name",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_PACKAGE_NAME,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "identifier",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_IDENTIFIER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "result_type",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.result_type",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_RESULT_TYPE,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "lhs_identifier",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.lhs_identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_LHS_IDENTIFIER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "operator",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.operator",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_OPERATOR,
            ),
            rule: GoAstInventoryRegistryPayloadRule::Exact(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_OPERATOR,
            ),
        },
        GoAstInventoryRegistryPayloadField {
            key: "literal_kind",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.literal_kind",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_LITERAL_KIND,
            ),
            rule: GoAstInventoryRegistryPayloadRule::Exact(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_LITERAL_KIND,
            ),
        },
        GoAstInventoryRegistryPayloadField {
            key: "literal_value",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.literal_value",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_LITERAL_VALUE,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "return_identifier",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.return_identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_RETURN_IDENTIFIER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "go_version",
            error_context: "go_ast_inventory_assignment_synthesis.payload_spec.go_version",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_GO_VERSION,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
    ];

fn go_ast_inventory_assignment_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_ASSIGNMENT_REGISTRY_PAYLOAD_FIELDS,
        expressions: assignment_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: assignment_registry_finite_domain_requirements,
    }
}

fn assignment_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_assignment_parent_field()),
            expression: go_ast_inventory_assignment_statement_spec(
                payload.str_value("lhs_identifier")?,
                payload.str_value("operator")?,
                payload.str_value("literal_kind")?,
                payload.str_value("literal_value")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("return_identifier")?.to_string(),
            },
        ),
    })
}

fn assignment_registry_finite_domain_requirements(
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryFiniteDomainRequirement>, SyntaxBackendBoundaryError> {
    Ok(vec![GoAstInventoryFiniteDomainRequirement::new(
        "ReturnStmt",
        "Results",
        "Ident",
        GrammarTerminalKind::Nonterminal,
    )])
}

const GO_AST_INVENTORY_BRANCH_STMT_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_branch_stmt_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_branch_stmt_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_branch_stmt_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "branch_token",
        error_context: "go_ast_inventory_branch_stmt_synthesis.payload_spec.branch_token",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_BRANCH_TOKEN,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_BRANCH_TOKEN,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_literal_kind",
        error_context: "go_ast_inventory_branch_stmt_synthesis.payload_spec.return_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_RETURN_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_RETURN_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_literal_value",
        error_context: "go_ast_inventory_branch_stmt_synthesis.payload_spec.return_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_RETURN_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_branch_stmt_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BRANCH_STMT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_branch_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_BRANCH_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: branch_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn branch_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("result_type")?,
            )),
        }),
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_branch_stmt_parent_field()),
            expression: go_ast_inventory_branch_stmt_statement_spec(
                payload.str_value("branch_token")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ReturnStmt",
                "Results",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_basic_lit_node(
                payload.str_value("return_literal_kind")?,
                payload.str_value("return_literal_value")?,
            )),
        },
    })
}

const GO_AST_INVENTORY_RANGE_STMT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] =
    &[
        GoAstInventoryRegistryPayloadField {
            key: "package_name",
            error_context: "go_ast_inventory_range_stmt_synthesis.payload_spec.package_name",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_RANGE_STMT_DEFAULT_PACKAGE_NAME,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "identifier",
            error_context: "go_ast_inventory_range_stmt_synthesis.payload_spec.identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_RANGE_STMT_DEFAULT_IDENTIFIER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "key_identifier",
            error_context: "go_ast_inventory_range_stmt_synthesis.payload_spec.key_identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_RANGE_STMT_DEFAULT_KEY_IDENTIFIER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "token",
            error_context: "go_ast_inventory_range_stmt_synthesis.payload_spec.token",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_RANGE_STMT_DEFAULT_TOKEN,
            ),
            rule: GoAstInventoryRegistryPayloadRule::Exact(
                GO_AST_INVENTORY_RANGE_STMT_DEFAULT_TOKEN,
            ),
        },
        GoAstInventoryRegistryPayloadField {
            key: "source_identifier",
            error_context: "go_ast_inventory_range_stmt_synthesis.payload_spec.source_identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_RANGE_STMT_DEFAULT_SOURCE_IDENTIFIER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "go_version",
            error_context: "go_ast_inventory_range_stmt_synthesis.payload_spec.go_version",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_RANGE_STMT_DEFAULT_GO_VERSION,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
    ];

fn go_ast_inventory_range_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListNoResult,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_RANGE_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: range_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn range_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_range_stmt_parent_field()),
            expression: go_ast_inventory_range_stmt_statement_spec(
                payload.str_value("key_identifier")?,
                payload.str_value("token")?,
                payload.str_value("source_identifier")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

const GO_AST_INVENTORY_TYPE_SWITCH_STMT_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_type_switch_stmt_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_type_switch_stmt_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "target_identifier",
        error_context: "go_ast_inventory_type_switch_stmt_synthesis.payload_spec.target_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_TARGET_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_type_switch_stmt_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_type_switch_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListNoResult,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_TYPE_SWITCH_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: type_switch_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn type_switch_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_type_switch_stmt_parent_field()),
            expression: go_ast_inventory_type_switch_stmt_statement_spec(
                payload.str_value("target_identifier")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

const GO_AST_INVENTORY_EMPTY_STMT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] =
    &[
        GoAstInventoryRegistryPayloadField {
            key: "package_name",
            error_context: "go_ast_inventory_empty_stmt_synthesis.payload_spec.package_name",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_EMPTY_STMT_DEFAULT_PACKAGE_NAME,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "identifier",
            error_context: "go_ast_inventory_empty_stmt_synthesis.payload_spec.identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_EMPTY_STMT_DEFAULT_IDENTIFIER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "implicit",
            error_context: "go_ast_inventory_empty_stmt_synthesis.payload_spec.implicit",
            default: GoAstInventoryRegistryPayloadDefault::Bool(
                GO_AST_INVENTORY_EMPTY_STMT_DEFAULT_IMPLICIT,
            ),
            rule: GoAstInventoryRegistryPayloadRule::ForbiddenTrue(
                "v1 empty-statement synthesis only emits explicit empty statements",
            ),
        },
        GoAstInventoryRegistryPayloadField {
            key: "go_version",
            error_context: "go_ast_inventory_empty_stmt_synthesis.payload_spec.go_version",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_EMPTY_STMT_DEFAULT_GO_VERSION,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
    ];

fn go_ast_inventory_empty_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListNoResult,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_EMPTY_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: empty_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: empty_stmt_extra_finite_domain_requirements,
    }
}

fn empty_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_empty_stmt_parent_field()),
            expression: go_ast_inventory_empty_stmt_statement_spec(payload.bool_value("implicit")?),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

fn empty_stmt_extra_finite_domain_requirements(
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryFiniteDomainRequirement>, SyntaxBackendBoundaryError> {
    Ok(vec![GoAstInventoryFiniteDomainRequirement::new(
        "EmptyStmt",
        "Implicit",
        "EmptyStmt.Implicit.bool",
        GrammarTerminalKind::BooleanPayload,
    )])
}

const GO_AST_INVENTORY_INC_DEC_STMT_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_inc_dec_stmt_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_inc_dec_stmt_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "target_identifier",
        error_context: "go_ast_inventory_inc_dec_stmt_synthesis.payload_spec.target_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_TARGET_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "token",
        error_context: "go_ast_inventory_inc_dec_stmt_synthesis.payload_spec.token",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_TOKEN,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_TOKEN),
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_inc_dec_stmt_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INC_DEC_STMT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_inc_dec_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListNoResult,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_INC_DEC_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: inc_dec_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn inc_dec_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_inc_dec_stmt_parent_field()),
            expression: go_ast_inventory_inc_dec_stmt_statement_spec(
                payload.str_value("target_identifier")?,
                payload.str_value("token")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

const GO_AST_INVENTORY_SEND_STMT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_send_stmt_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SEND_STMT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_send_stmt_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SEND_STMT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "channel_identifier",
        error_context: "go_ast_inventory_send_stmt_synthesis.payload_spec.channel_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SEND_STMT_DEFAULT_CHANNEL_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "value_identifier",
        error_context: "go_ast_inventory_send_stmt_synthesis.payload_spec.value_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SEND_STMT_DEFAULT_VALUE_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_send_stmt_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SEND_STMT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_send_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListNoResult,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_SEND_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: send_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn send_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_send_stmt_parent_field()),
            expression: go_ast_inventory_send_stmt_statement_spec(
                payload.str_value("channel_identifier")?,
                payload.str_value("value_identifier")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

const GO_AST_INVENTORY_LABELED_STMT_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_labeled_stmt_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_LABELED_STMT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_labeled_stmt_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_LABELED_STMT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_labeled_stmt_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_LABELED_STMT_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "label_identifier",
        error_context: "go_ast_inventory_labeled_stmt_synthesis.payload_spec.label_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_LABELED_STMT_DEFAULT_LABEL_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_literal_kind",
        error_context: "go_ast_inventory_labeled_stmt_synthesis.payload_spec.return_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_LABELED_STMT_DEFAULT_RETURN_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_LABELED_STMT_DEFAULT_RETURN_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_literal_value",
        error_context: "go_ast_inventory_labeled_stmt_synthesis.payload_spec.return_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_LABELED_STMT_DEFAULT_RETURN_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_labeled_stmt_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_LABELED_STMT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_labeled_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementList,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_LABELED_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: labeled_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn labeled_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("result_type")?,
            )),
        }),
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_labeled_stmt_parent_field()),
            expression: go_ast_inventory_labeled_stmt_statement_spec(
                payload.str_value("label_identifier")?,
                payload.str_value("return_literal_kind")?,
                payload.str_value("return_literal_value")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

const GO_AST_INVENTORY_CASE_CLAUSE_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "switch_literal_kind",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.switch_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_SWITCH_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_SWITCH_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "switch_literal_value",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.switch_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_SWITCH_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "case_literal_kind",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.case_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_CASE_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_CASE_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "case_literal_value",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.case_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_CASE_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_literal_kind",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.return_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_RETURN_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_RETURN_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_literal_value",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.return_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_RETURN_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_case_clause_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CASE_CLAUSE_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_case_clause_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_CASE_CLAUSE_REGISTRY_PAYLOAD_FIELDS,
        expressions: case_clause_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn case_clause_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("result_type")?,
            )),
        }),
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_case_clause_parent_field()),
            expression: go_ast_inventory_case_clause_statement_spec(
                payload.str_value("switch_literal_kind")?,
                payload.str_value("switch_literal_value")?,
                payload.str_value("case_literal_kind")?,
                payload.str_value("case_literal_value")?,
                payload.str_value("return_literal_kind")?,
                payload.str_value("return_literal_value")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ReturnStmt",
                "Results",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_basic_lit_node(
                payload.str_value("return_literal_kind")?,
                payload.str_value("return_literal_value")?,
            )),
        },
    })
}

const GO_AST_INVENTORY_COMM_CLAUSE_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_comm_clause_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_comm_clause_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_comm_clause_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_comm_clause_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMM_CLAUSE_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_comm_clause_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementList,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_COMM_CLAUSE_REGISTRY_PAYLOAD_FIELDS,
        expressions: comm_clause_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn comm_clause_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("result_type")?,
            )),
        }),
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_comm_clause_parent_field()),
            expression: go_ast_inventory_comm_clause_statement_spec(),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

const GO_AST_INVENTORY_DECL_STMT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_decl_stmt_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_decl_stmt_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_decl_stmt_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "decl_identifier",
        error_context: "go_ast_inventory_decl_stmt_synthesis.payload_spec.decl_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_DECL_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_kind",
        error_context: "go_ast_inventory_decl_stmt_synthesis.payload_spec.literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_value",
        error_context: "go_ast_inventory_decl_stmt_synthesis.payload_spec.literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_identifier",
        error_context: "go_ast_inventory_decl_stmt_synthesis.payload_spec.return_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_RETURN_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_decl_stmt_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_DECL_STMT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_decl_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_DECL_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: decl_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn decl_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("result_type")?,
            )),
        }),
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_decl_stmt_parent_field()),
            expression: go_ast_inventory_decl_stmt_statement_spec(
                payload.str_value("decl_identifier")?,
                payload.str_value("literal_kind")?,
                payload.str_value("literal_value")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ReturnStmt",
                "Results",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("return_identifier")?,
            )),
        },
    })
}

const GO_AST_INVENTORY_DEFER_STMT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] =
    &[
        GoAstInventoryRegistryPayloadField {
            key: "package_name",
            error_context: "go_ast_inventory_defer_stmt_synthesis.payload_spec.package_name",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_PACKAGE_NAME,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "identifier",
            error_context: "go_ast_inventory_defer_stmt_synthesis.payload_spec.identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_IDENTIFIER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "result_type",
            error_context: "go_ast_inventory_defer_stmt_synthesis.payload_spec.result_type",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_RESULT_TYPE,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "call_receiver",
            error_context: "go_ast_inventory_defer_stmt_synthesis.payload_spec.call_receiver",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_CALL_RECEIVER,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "call_selector",
            error_context: "go_ast_inventory_defer_stmt_synthesis.payload_spec.call_selector",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_CALL_SELECTOR,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "return_literal_kind",
            error_context: "go_ast_inventory_defer_stmt_synthesis.payload_spec.return_literal_kind",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_RETURN_LITERAL_KIND,
            ),
            rule: GoAstInventoryRegistryPayloadRule::Exact(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_RETURN_LITERAL_KIND,
            ),
        },
        GoAstInventoryRegistryPayloadField {
            key: "return_literal_value",
            error_context: "go_ast_inventory_defer_stmt_synthesis.payload_spec.return_literal_value",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_RETURN_LITERAL_VALUE,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
        GoAstInventoryRegistryPayloadField {
            key: "go_version",
            error_context: "go_ast_inventory_defer_stmt_synthesis.payload_spec.go_version",
            default: GoAstInventoryRegistryPayloadDefault::Str(
                GO_AST_INVENTORY_DEFER_STMT_DEFAULT_GO_VERSION,
            ),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        },
    ];

fn go_ast_inventory_defer_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_DEFER_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: defer_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn defer_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("result_type")?,
            )),
        }),
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_defer_stmt_parent_field()),
            expression: go_ast_inventory_defer_stmt_statement_spec(
                payload.str_value("call_receiver")?,
                payload.str_value("call_selector")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ReturnStmt",
                "Results",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_basic_lit_node(
                payload.str_value("return_literal_kind")?,
                payload.str_value("return_literal_value")?,
            )),
        },
    })
}

const GO_AST_INVENTORY_GO_STMT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_go_stmt_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_GO_STMT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_go_stmt_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_GO_STMT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "call_receiver",
        error_context: "go_ast_inventory_go_stmt_synthesis.payload_spec.call_receiver",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_GO_STMT_DEFAULT_CALL_RECEIVER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "call_selector",
        error_context: "go_ast_inventory_go_stmt_synthesis.payload_spec.call_selector",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_GO_STMT_DEFAULT_CALL_SELECTOR,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_go_stmt_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_GO_STMT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_go_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_GO_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementListNoResult,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_GO_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: go_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn go_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_go_stmt_parent_field()),
            expression: go_ast_inventory_go_stmt_statement_spec(
                payload.str_value("call_receiver")?,
                payload.str_value("call_selector")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

const GO_AST_INVENTORY_IF_STMT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "init_identifier",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.init_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "init_operator",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.init_operator",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_OPERATOR,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "init_literal_kind",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.init_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "init_literal_value",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.init_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "cond_identifier",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.cond_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_COND_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "body_return_literal_kind",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.body_return_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_BODY_RETURN_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "body_return_literal_value",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.body_return_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_BODY_RETURN_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "else_return_literal_kind",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.else_return_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_ELSE_RETURN_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "else_return_literal_value",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.else_return_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_ELSE_RETURN_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_if_stmt_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_IF_STMT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_if_stmt_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_IF_STMT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionStatementList,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_IF_STMT_REGISTRY_PAYLOAD_FIELDS,
        expressions: if_stmt_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn if_stmt_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("result_type")?,
            )),
        }),
        statements: vec![GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_if_stmt_parent_field()),
            expression: go_ast_inventory_if_stmt_statement_spec(
                payload.str_value("init_identifier")?,
                payload.str_value("init_operator")?,
                payload.str_value("init_literal_kind")?,
                payload.str_value("init_literal_value")?,
                payload.str_value("cond_identifier")?,
                payload.str_value("body_return_literal_kind")?,
                payload.str_value("body_return_literal_value")?,
                payload.str_value("else_return_literal_kind")?,
                payload.str_value("else_return_literal_value")?,
            ),
        }],
        expression: GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: "unused".to_string(),
            },
        ),
    })
}

const GO_AST_INVENTORY_ARRAY_TYPE_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_array_type_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_array_type_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "element_type",
        error_context: "go_ast_inventory_array_type_return_synthesis.payload_spec.element_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_ELEMENT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_identifier",
        error_context: "go_ast_inventory_array_type_return_synthesis.payload_spec.return_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_RETURN_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_array_type_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ARRAY_TYPE_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_array_type_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_ARRAY_TYPE_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: array_type_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn array_type_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_array_type_return_parent_field()),
            expression: go_ast_inventory_array_type_return_expression_spec(
                payload.str_value("element_type")?,
            ),
        }),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ReturnStmt",
                "Results",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("return_identifier")?,
            )),
        },
    })
}

const GO_AST_INVENTORY_CHAN_TYPE_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_chan_type_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_chan_type_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "direction",
        error_context: "go_ast_inventory_chan_type_return_synthesis.payload_spec.direction",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_DIRECTION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_DIRECTION,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "element_type",
        error_context: "go_ast_inventory_chan_type_return_synthesis.payload_spec.element_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_ELEMENT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_identifier",
        error_context: "go_ast_inventory_chan_type_return_synthesis.payload_spec.return_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_RETURN_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_chan_type_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_CHAN_TYPE_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_chan_type_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_CHAN_TYPE_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: chan_type_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn chan_type_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_chan_type_return_parent_field()),
            expression: go_ast_inventory_chan_type_return_expression_spec(
                payload.str_value("direction")?,
                payload.str_value("element_type")?,
            ),
        }),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ReturnStmt",
                "Results",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("return_identifier")?,
            )),
        },
    })
}

const GO_AST_INVENTORY_BINARY_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "left_literal_kind",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.left_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_LEFT_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_LEFT_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "left_literal_value",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.left_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_LEFT_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "operator",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.operator",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_OPERATOR,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_OPERATOR,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "right_literal_kind",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.right_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_RIGHT_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_RIGHT_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "right_literal_value",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.right_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_RIGHT_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_binary_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_binary_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_BINARY_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: binary_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn binary_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_binary_expr_return_parent_field()),
            expression: go_ast_inventory_binary_expr_return_expression_spec(
                payload.str_value("left_literal_kind")?,
                payload.str_value("left_literal_value")?,
                payload.str_value("operator")?,
                payload.str_value("right_literal_kind")?,
                payload.str_value("right_literal_value")?,
            ),
        },
    })
}

fn no_extra_expected_field_orders(
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryExpectedFieldOrder>, SyntaxBackendBoundaryError> {
    Ok(Vec::new())
}

const GO_AST_INVENTORY_UNARY_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_unary_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_unary_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_unary_expr_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "operator",
        error_context: "go_ast_inventory_unary_expr_return_synthesis.payload_spec.operator",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_OPERATOR,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_OPERATOR,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "target_identifier",
        error_context: "go_ast_inventory_unary_expr_return_synthesis.payload_spec.target_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_unary_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_UNARY_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_unary_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_UNARY_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: unary_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn unary_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_unary_expr_return_parent_field()),
            expression: go_ast_inventory_unary_expr_return_expression_spec(
                payload.str_value("operator")?,
                payload.str_value("target_identifier")?,
            ),
        },
    })
}

fn no_extra_finite_domain_requirements(
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryFiniteDomainRequirement>, SyntaxBackendBoundaryError> {
    Ok(Vec::new())
}

const GO_AST_INVENTORY_PAREN_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_paren_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_paren_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_paren_expr_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_kind",
        error_context: "go_ast_inventory_paren_expr_return_synthesis.payload_spec.literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_value",
        error_context: "go_ast_inventory_paren_expr_return_synthesis.payload_spec.literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_paren_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_PAREN_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_paren_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_PAREN_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: paren_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn paren_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_paren_expr_return_parent_field()),
            expression: go_ast_inventory_paren_expr_return_expression_spec(
                payload.str_value("literal_kind")?,
                payload.str_value("literal_value")?,
            ),
        },
    })
}

const GO_AST_INVENTORY_INDEX_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_index_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_index_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_index_expr_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "target_identifier",
        error_context: "go_ast_inventory_index_expr_return_synthesis.payload_spec.target_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "index_literal_kind",
        error_context: "go_ast_inventory_index_expr_return_synthesis.payload_spec.index_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_INDEX_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_INDEX_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "index_literal_value",
        error_context: "go_ast_inventory_index_expr_return_synthesis.payload_spec.index_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_INDEX_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_index_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_index_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_INDEX_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: index_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn index_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_index_expr_return_parent_field()),
            expression: go_ast_inventory_index_expr_return_expression_spec(
                payload.str_value("target_identifier")?,
                payload.str_value("index_literal_kind")?,
                payload.str_value("index_literal_value")?,
            ),
        },
    })
}

const GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "target_identifier",
        error_context: "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.target_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "first_index_identifier",
        error_context: "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.first_index_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_FIRST_INDEX_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "second_index_identifier",
        error_context: "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.second_index_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_SECOND_INDEX_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_index_list_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry
{
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: index_list_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn index_list_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_index_list_expr_return_parent_field()),
            expression: go_ast_inventory_index_list_expr_return_expression_spec(
                payload.str_value("target_identifier")?,
                payload.str_value("first_index_identifier")?,
                payload.str_value("second_index_identifier")?,
            ),
        },
    })
}

const GO_AST_INVENTORY_INTERFACE_TYPE_VAR_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_interface_type_var_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_interface_type_var_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "methods_policy",
        error_context: "go_ast_inventory_interface_type_var_synthesis.payload_spec.methods_policy",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_METHODS_POLICY,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_METHODS_POLICY,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "incomplete",
        error_context: "go_ast_inventory_interface_type_var_synthesis.payload_spec.incomplete",
        default: GoAstInventoryRegistryPayloadDefault::Bool(
            GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_INCOMPLETE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::ForbiddenTrue(
            "v1 interface-type synthesis only emits parser-valid complete interface types",
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_interface_type_var_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_interface_type_var_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::PackageVarType,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_REGISTRY_PAYLOAD_FIELDS,
        expressions: interface_type_var_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn interface_type_var_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_interface_type_var_parent_field()),
            expression: go_ast_inventory_interface_type_var_expression_spec(
                payload.bool_value("incomplete")?,
            ),
        },
    })
}

const GO_AST_INVENTORY_STRUCT_TYPE_DECL_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_struct_type_decl_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "type_name",
        error_context: "go_ast_inventory_struct_type_decl_synthesis.payload_spec.type_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_TYPE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "field_name",
        error_context: "go_ast_inventory_struct_type_decl_synthesis.payload_spec.field_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELD_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "field_type",
        error_context: "go_ast_inventory_struct_type_decl_synthesis.payload_spec.field_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELD_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "fields_policy",
        error_context: "go_ast_inventory_struct_type_decl_synthesis.payload_spec.fields_policy",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELDS_POLICY,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELDS_POLICY,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "incomplete",
        error_context: "go_ast_inventory_struct_type_decl_synthesis.payload_spec.incomplete",
        default: GoAstInventoryRegistryPayloadDefault::Bool(
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_INCOMPLETE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::ForbiddenTrue(
            "v1 struct-type synthesis only emits parser-valid complete struct types",
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_struct_type_decl_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_struct_type_decl_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::PackageType,
        declaration_identifier_key: "type_name",
        payload_fields: GO_AST_INVENTORY_STRUCT_TYPE_DECL_REGISTRY_PAYLOAD_FIELDS,
        expressions: struct_type_decl_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn struct_type_decl_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_struct_type_decl_parent_field()),
            expression: go_ast_inventory_struct_type_decl_expression_spec(
                payload.str_value("field_name")?,
                payload.str_value("field_type")?,
                payload.bool_value("incomplete")?,
            ),
        },
    })
}

const GO_AST_INVENTORY_SLICE_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "target_identifier",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.target_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "low_literal_kind",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.low_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "low_literal_value",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.low_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "high_literal_kind",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.high_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "high_literal_value",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.high_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "three_index",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.three_index",
        default: GoAstInventoryRegistryPayloadDefault::Bool(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_THREE_INDEX,
        ),
        rule: GoAstInventoryRegistryPayloadRule::ForbiddenTrue(
            "v1 slice-expression synthesis only emits two-index slices",
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_slice_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_slice_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_SLICE_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: slice_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: slice_expr_return_extra_finite_domain_requirements,
    }
}

fn slice_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_slice_expr_return_parent_field()),
            expression: go_ast_inventory_slice_expr_return_expression_spec(
                payload.str_value("target_identifier")?,
                payload.str_value("low_literal_kind")?,
                payload.str_value("low_literal_value")?,
                payload.str_value("high_literal_kind")?,
                payload.str_value("high_literal_value")?,
                payload.bool_value("three_index")?,
            ),
        },
    })
}

fn slice_expr_return_extra_finite_domain_requirements(
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryFiniteDomainRequirement>, SyntaxBackendBoundaryError> {
    Ok(vec![GoAstInventoryFiniteDomainRequirement::new(
        "SliceExpr",
        "Slice3",
        "SliceExpr.Slice3.bool",
        GrammarTerminalKind::BooleanPayload,
    )])
}

const GO_AST_INVENTORY_SELECTOR_CALL_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_selector_call_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_selector_call_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_selector_call_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "receiver",
        error_context: "go_ast_inventory_selector_call_synthesis.payload_spec.receiver",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_RECEIVER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "selector",
        error_context: "go_ast_inventory_selector_call_synthesis.payload_spec.selector",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_SELECTOR,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_selector_call_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_SELECTOR_CALL_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_selector_call_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_SELECTOR_CALL_REGISTRY_PAYLOAD_FIELDS,
        expressions: selector_call_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn selector_call_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_selector_call_parent_field()),
            expression: go_ast_inventory_selector_call_expression_spec(
                payload.str_value("receiver")?,
                payload.str_value("selector")?,
            ),
        },
    })
}

const GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_composite_lit_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_composite_lit_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "element_type",
        error_context: "go_ast_inventory_composite_lit_return_synthesis.payload_spec.element_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_ELEMENT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_kind",
        error_context: "go_ast_inventory_composite_lit_return_synthesis.payload_spec.literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_value",
        error_context: "go_ast_inventory_composite_lit_return_synthesis.payload_spec.literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "incomplete",
        error_context: "go_ast_inventory_composite_lit_return_synthesis.payload_spec.incomplete",
        default: GoAstInventoryRegistryPayloadDefault::Bool(
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_INCOMPLETE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::ForbiddenTrue(
            "v1 composite-literal synthesis only emits complete literals",
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_composite_lit_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_composite_lit_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: composite_lit_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: composite_lit_return_extra_finite_domain_requirements,
    }
}

fn composite_lit_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_array_type_return_parent_field()),
            expression: go_ast_inventory_array_type_return_expression_spec(
                payload.str_value("element_type")?,
            ),
        }),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_composite_lit_return_parent_field()),
            expression: go_ast_inventory_composite_lit_return_expression_spec(
                payload.str_value("element_type")?,
                payload.str_value("literal_kind")?,
                payload.str_value("literal_value")?,
                payload.bool_value("incomplete")?,
            ),
        },
    })
}

fn composite_lit_return_extra_finite_domain_requirements(
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryFiniteDomainRequirement>, SyntaxBackendBoundaryError> {
    Ok(vec![GoAstInventoryFiniteDomainRequirement::new(
        "CompositeLit",
        "Incomplete",
        "CompositeLit.Incomplete.bool",
        GrammarTerminalKind::BooleanPayload,
    )])
}

const GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "element_type",
        error_context: "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.element_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_ELEMENT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_kind",
        error_context: "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_value",
        error_context: "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "incomplete",
        error_context: "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.incomplete",
        default: GoAstInventoryRegistryPayloadDefault::Bool(
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_INCOMPLETE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::ForbiddenTrue(
            "v1 ellipsis array-literal synthesis only emits complete composite literals",
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_ellipsis_array_literal_registry_entry() -> GoAstInventorySynthesisRegistryEntry
{
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::PackageVarValue,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_REGISTRY_PAYLOAD_FIELDS,
        expressions: ellipsis_array_literal_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: ellipsis_array_literal_extra_finite_domain_requirements,
    }
}

fn ellipsis_array_literal_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ValueSpec",
                "Values",
            )),
            expression: go_ast_inventory_ellipsis_array_literal_expression_spec(
                payload.str_value("element_type")?,
                payload.str_value("literal_kind")?,
                payload.str_value("literal_value")?,
                payload.bool_value("incomplete")?,
            ),
        },
    })
}

fn ellipsis_array_literal_extra_finite_domain_requirements(
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryFiniteDomainRequirement>, SyntaxBackendBoundaryError> {
    Ok(vec![GoAstInventoryFiniteDomainRequirement::new(
        "CompositeLit",
        "Incomplete",
        "CompositeLit.Incomplete.bool",
        GrammarTerminalKind::BooleanPayload,
    )])
}

const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "key_type",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.key_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "value_type",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.value_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "key_literal_kind",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.key_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "key_literal_value",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.key_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "value_literal_kind",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.value_literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "value_literal_value",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.value_literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "incomplete",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.incomplete",
        default: GoAstInventoryRegistryPayloadDefault::Bool(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_INCOMPLETE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::ForbiddenTrue(
            "v1 key-value-expression synthesis only emits complete composite literals",
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_key_value_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: key_value_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: key_value_expr_return_extra_finite_domain_requirements,
    }
}

fn key_value_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: go_ast_inventory_key_value_expr_return_map_type_spec(
                payload.str_value("key_type")?,
                payload.str_value("value_type")?,
            ),
        }),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_key_value_expr_return_parent_field()),
            expression: go_ast_inventory_key_value_expr_return_expression_spec(
                payload.str_value("key_type")?,
                payload.str_value("value_type")?,
                payload.str_value("key_literal_kind")?,
                payload.str_value("key_literal_value")?,
                payload.str_value("value_literal_kind")?,
                payload.str_value("value_literal_value")?,
                payload.bool_value("incomplete")?,
            ),
        },
    })
}

fn key_value_expr_return_extra_finite_domain_requirements(
    _payload: &GoAstInventoryRegistryPayload,
) -> Result<Vec<GoAstInventoryFiniteDomainRequirement>, SyntaxBackendBoundaryError> {
    Ok(vec![GoAstInventoryFiniteDomainRequirement::new(
        "CompositeLit",
        "Incomplete",
        "CompositeLit.Incomplete.bool",
        GrammarTerminalKind::BooleanPayload,
    )])
}

const GO_AST_INVENTORY_FUNC_LIT_REGISTRY_PAYLOAD_FIELDS: &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_func_lit_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_LIT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_func_lit_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_LIT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "signature_policy",
        error_context: "go_ast_inventory_func_lit_synthesis.payload_spec.signature_policy",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_LIT_DEFAULT_SIGNATURE_POLICY,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_FUNC_LIT_DEFAULT_SIGNATURE_POLICY,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "body_policy",
        error_context: "go_ast_inventory_func_lit_synthesis.payload_spec.body_policy",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_LIT_DEFAULT_BODY_POLICY,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_FUNC_LIT_DEFAULT_BODY_POLICY,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_func_lit_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_LIT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_func_lit_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::PackageVarValue,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_FUNC_LIT_REGISTRY_PAYLOAD_FIELDS,
        expressions: func_lit_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn func_lit_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    payload.str_value("signature_policy")?;
    payload.str_value("body_policy")?;
    Ok(GoAstInventoryRegistryExpressions {
        result_type: None,
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_func_lit_parent_field()),
            expression: go_ast_inventory_func_lit_expression_spec(),
        },
    })
}

const GO_AST_INVENTORY_STAR_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_star_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_star_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_star_expr_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "target_identifier",
        error_context: "go_ast_inventory_star_expr_return_synthesis.payload_spec.target_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_star_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_STAR_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_star_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_STAR_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: star_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn star_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_star_expr_return_parent_field()),
            expression: go_ast_inventory_star_expr_return_expression_spec(
                payload.str_value("target_identifier")?,
            ),
        },
    })
}

const GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_type_assert_expr_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_type_assert_expr_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_type_assert_expr_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "target_identifier",
        error_context: "go_ast_inventory_type_assert_expr_return_synthesis.payload_spec.target_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "asserted_type",
        error_context: "go_ast_inventory_type_assert_expr_return_synthesis.payload_spec.asserted_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_ASSERTED_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_type_assert_expr_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_type_assert_expr_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry
{
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: type_assert_expr_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn type_assert_expr_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Legacy(
            GoAstInventoryExpressionTemplate::Ident {
                payload: payload.str_value("result_type")?.to_string(),
            },
        )),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_type_assert_expr_return_parent_field()),
            expression: go_ast_inventory_type_assert_expr_return_expression_spec(
                payload.str_value("target_identifier")?,
                payload.str_value("asserted_type")?,
            ),
        },
    })
}

const GO_AST_INVENTORY_FUNCTION_RETURN_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_function_return_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_function_return_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "result_type",
        error_context: "go_ast_inventory_function_return_synthesis.payload_spec.result_type",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_RESULT_TYPE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_kind",
        error_context: "go_ast_inventory_function_return_synthesis.payload_spec.literal_kind",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_LITERAL_KIND,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_LITERAL_KIND,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "literal_value",
        error_context: "go_ast_inventory_function_return_synthesis.payload_spec.literal_value",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_LITERAL_VALUE,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_function_return_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNCTION_RETURN_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_function_return_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_FUNCTION_RETURN_REGISTRY_PAYLOAD_FIELDS,
        expressions: function_return_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn function_return_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "Field", "Type",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("result_type")?,
            )),
        }),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ReturnStmt",
                "Results",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_basic_lit_node(
                payload.str_value("literal_kind")?,
                payload.str_value("literal_value")?,
            )),
        },
    })
}

const GO_AST_INVENTORY_FUNC_TYPE_RESULT_REGISTRY_PAYLOAD_FIELDS:
    &[GoAstInventoryRegistryPayloadField] = &[
    GoAstInventoryRegistryPayloadField {
        key: "package_name",
        error_context: "go_ast_inventory_func_type_result_synthesis.payload_spec.package_name",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_PACKAGE_NAME,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "identifier",
        error_context: "go_ast_inventory_func_type_result_synthesis.payload_spec.identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "signature_policy",
        error_context: "go_ast_inventory_func_type_result_synthesis.payload_spec.signature_policy",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_SIGNATURE_POLICY,
        ),
        rule: GoAstInventoryRegistryPayloadRule::Exact(
            GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_SIGNATURE_POLICY,
        ),
    },
    GoAstInventoryRegistryPayloadField {
        key: "return_identifier",
        error_context: "go_ast_inventory_func_type_result_synthesis.payload_spec.return_identifier",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_RETURN_IDENTIFIER,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
    GoAstInventoryRegistryPayloadField {
        key: "go_version",
        error_context: "go_ast_inventory_func_type_result_synthesis.payload_spec.go_version",
        default: GoAstInventoryRegistryPayloadDefault::Str(
            GO_AST_INVENTORY_FUNC_TYPE_RESULT_DEFAULT_GO_VERSION,
        ),
        rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
    },
];

fn go_ast_inventory_func_type_result_registry_entry() -> GoAstInventorySynthesisRegistryEntry {
    GoAstInventorySynthesisRegistryEntry {
        api_id: GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID,
        api_boundary: GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_BOUNDARY,
        shell: GoAstInventorySynthesisShell::FunctionReturn,
        declaration_identifier_key: "identifier",
        payload_fields: GO_AST_INVENTORY_FUNC_TYPE_RESULT_REGISTRY_PAYLOAD_FIELDS,
        expressions: func_type_result_registry_expressions,
        extra_expected_field_orders: no_extra_expected_field_orders,
        extra_finite_domain_requirements: no_extra_finite_domain_requirements,
    }
}

fn func_type_result_registry_expressions(
    payload: &GoAstInventoryRegistryPayload,
) -> Result<GoAstInventoryRegistryExpressions, SyntaxBackendBoundaryError> {
    payload.str_value("signature_policy")?;
    Ok(GoAstInventoryRegistryExpressions {
        result_type: Some(GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(go_ast_inventory_func_type_result_parent_field()),
            expression: go_ast_inventory_func_type_result_expression_spec(),
        }),
        statements: Vec::new(),
        expression: GoAstInventoryRegistryExpressionSlot::Declarative {
            parent_field: Some(GoAstInventoryDeclarativeParentFieldSpec::new(
                "ReturnStmt",
                "Results",
            )),
            expression: GoAstInventoryDeclarativeExpressionSpec::new(declarative_ident_node(
                payload.str_value("return_identifier")?,
            )),
        },
    })
}

pub(super) fn go_ast_inventory_assignment_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_array_type_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("Field", "Type")
}

pub(super) fn go_ast_inventory_array_type_return_expression_spec(
    element_type: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "ArrayType",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Len",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Elt",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    element_type,
                )),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_chan_type_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("Field", "Type")
}

pub(super) fn go_ast_inventory_chan_type_return_expression_spec(
    direction: &str,
    element_type: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "ChanType",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Dir",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    &format!("go_ast_channel_directions.{direction}"),
                    direction,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Value",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    element_type,
                )),
            ),
        ],
    ))
}

fn go_ast_inventory_func_type_result_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("Field", "Type")
}

fn go_ast_inventory_func_type_result_expression_spec() -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(declarative_func_type_no_params_no_results_node())
}

fn declarative_func_type_no_params_no_results_node() -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "FuncType",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Func",
                GoAstInventoryDeclarativeFieldValue::selected_boolean_payload(
                    "go_syntax_presence_func_type_func.PRESENT",
                    true,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "TypeParams",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Params",
                GoAstInventoryDeclarativeFieldValue::generic_node(
                    declarative_empty_field_list_node(),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Results",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
        ],
    )
}

fn declarative_empty_field_list_node() -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "FieldList",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "List",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(Vec::new()),
        )],
    )
}

pub(super) fn go_ast_inventory_assignment_statement_spec(
    lhs_identifier: &str,
    operator: &str,
    literal_kind: &str,
    literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "AssignStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Lhs",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_ident_node(lhs_identifier),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Tok",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    declarative_assignment_operator_token(operator),
                    operator,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Rhs",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_basic_lit_node(literal_kind, literal_value),
                ]),
            ),
        ],
    ))
}

fn declarative_assignment_operator_token(operator: &str) -> &'static str {
    match operator {
        ":=" => "go_token_assignment_tokens.DEFINE",
        _ => "go_token_assignment_tokens.ASSIGN",
    }
}

pub(super) fn go_ast_inventory_branch_stmt_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_branch_stmt_statement_spec(
    branch_token: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "ForStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Init",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Cond",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Post",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Body",
                GoAstInventoryDeclarativeFieldValue::generic_node(
                    declarative_single_statement_block_node(declarative_branch_stmt_node(
                        branch_token,
                    )),
                ),
            ),
        ],
    ))
}

fn declarative_single_statement_block_node(
    statement: GoAstInventoryDeclarativeNodeSpec,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "BlockStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "List",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![statement]),
        )],
    )
}

fn declarative_branch_stmt_node(branch_token: &str) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "BranchStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Tok",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    "go_token_branch_tokens.BREAK",
                    branch_token,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Label",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
        ],
    )
}

pub(super) fn go_ast_inventory_range_stmt_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_range_stmt_statement_spec(
    key_identifier: &str,
    token: &str,
    source_identifier: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "RangeStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Key",
                GoAstInventoryDeclarativeFieldValue::optional_alternative_node_present(
                    declarative_ident_node(key_identifier),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Value",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Tok",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    declarative_range_assignment_token(token),
                    token,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    source_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Body",
                GoAstInventoryDeclarativeFieldValue::generic_node(declarative_empty_block_node()),
            ),
        ],
    ))
}

fn declarative_range_assignment_token(token: &str) -> &'static str {
    match token {
        ":=" => "go_token_range_assignment_tokens.DEFINE",
        "=" => "go_token_range_assignment_tokens.ASSIGN",
        _ => "go_token_range_assignment_tokens.ILLEGAL",
    }
}

pub(super) fn go_ast_inventory_type_switch_stmt_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_type_switch_stmt_statement_spec(
    target_identifier: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "TypeSwitchStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Init",
                GoAstInventoryDeclarativeFieldValue::OptionalAlternativeNode(None),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Assign",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    declarative_type_switch_expr_stmt_node(target_identifier),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Body",
                GoAstInventoryDeclarativeFieldValue::generic_node(declarative_empty_block_node()),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_empty_stmt_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_empty_stmt_statement_spec(
    implicit: bool,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "EmptyStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Implicit",
            GoAstInventoryDeclarativeFieldValue::boolean_payload(implicit),
        )],
    ))
}

fn declarative_type_switch_expr_stmt_node(
    target_identifier: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "ExprStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "X",
            GoAstInventoryDeclarativeFieldValue::alternative_node(
                declarative_type_switch_assert_expr_node(target_identifier),
            ),
        )],
    )
}

fn declarative_type_switch_assert_expr_node(
    target_identifier: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "TypeAssertExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    target_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Type",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
        ],
    )
}

fn declarative_empty_block_node() -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "BlockStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "List",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(Vec::new()),
        )],
    )
}

pub(super) fn go_ast_inventory_labeled_stmt_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_labeled_stmt_statement_spec(
    label_identifier: &str,
    return_literal_kind: &str,
    return_literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "LabeledStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Label",
                GoAstInventoryDeclarativeFieldValue::optional_node_present(declarative_ident_node(
                    label_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Stmt",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    declarative_return_basic_lit_node(return_literal_kind, return_literal_value),
                ),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_case_clause_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_case_clause_statement_spec(
    switch_literal_kind: &str,
    switch_literal_value: &str,
    case_literal_kind: &str,
    case_literal_value: &str,
    return_literal_kind: &str,
    return_literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "SwitchStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Init",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Tag",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                    switch_literal_kind,
                    switch_literal_value,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Body",
                GoAstInventoryDeclarativeFieldValue::generic_node(
                    go_ast_inventory_case_clause_block_node(
                        case_literal_kind,
                        case_literal_value,
                        return_literal_kind,
                        return_literal_value,
                    ),
                ),
            ),
        ],
    ))
}

fn go_ast_inventory_case_clause_block_node(
    case_literal_kind: &str,
    case_literal_value: &str,
    return_literal_kind: &str,
    return_literal_value: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "BlockStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "List",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                go_ast_inventory_case_clause_node(
                    case_literal_kind,
                    case_literal_value,
                    return_literal_kind,
                    return_literal_value,
                ),
            ]),
        )],
    )
}

fn go_ast_inventory_case_clause_node(
    case_literal_kind: &str,
    case_literal_value: &str,
    return_literal_kind: &str,
    return_literal_value: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "CaseClause",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "List",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_basic_lit_node(case_literal_kind, case_literal_value),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Body",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_return_basic_lit_node(return_literal_kind, return_literal_value),
                ]),
            ),
        ],
    )
}

pub(super) fn go_ast_inventory_comm_clause_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_comm_clause_statement_spec()
-> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "SelectStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Body",
            GoAstInventoryDeclarativeFieldValue::generic_node(
                go_ast_inventory_comm_clause_block_node(),
            ),
        )],
    ))
}

fn go_ast_inventory_comm_clause_block_node() -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "BlockStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "List",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                go_ast_inventory_comm_clause_node(),
            ]),
        )],
    )
}

fn go_ast_inventory_comm_clause_node() -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "CommClause",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Comm",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Body",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(Vec::new()),
            ),
        ],
    )
}

pub(super) fn go_ast_inventory_decl_stmt_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_decl_stmt_statement_spec(
    decl_identifier: &str,
    literal_kind: &str,
    literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "DeclStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Decl",
            GoAstInventoryDeclarativeFieldValue::alternative_node(
                go_ast_inventory_var_gen_decl_node(decl_identifier, literal_kind, literal_value),
            ),
        )],
    ))
}

fn go_ast_inventory_var_gen_decl_node(
    identifier: &str,
    literal_kind: &str,
    literal_value: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "GenDecl",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Doc",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Tok",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    "go_token_declaration_tokens.VAR",
                    "var",
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Lparen",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Specs",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    go_ast_inventory_var_value_spec_node(identifier, literal_kind, literal_value),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Rparen",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
        ],
    )
}

fn go_ast_inventory_var_value_spec_node(
    identifier: &str,
    literal_kind: &str,
    literal_value: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "ValueSpec",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Doc",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Names",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_ident_node(identifier),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Type",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Values",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_basic_lit_node(literal_kind, literal_value),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Comment",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
        ],
    )
}

fn declarative_return_basic_lit_node(
    literal_kind: &str,
    literal_value: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "ReturnStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Results",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                declarative_basic_lit_node(literal_kind, literal_value),
            ]),
        )],
    )
}

pub(super) fn go_ast_inventory_unary_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_unary_expr_return_expression_spec(
    operator: &str,
    target_identifier: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "UnaryExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Op",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    "go_token_unary_operator_tokens.SUB",
                    operator,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    target_identifier,
                )),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_paren_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_paren_expr_return_expression_spec(
    literal_kind: &str,
    literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "ParenExpr",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "X",
            GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                literal_kind,
                literal_value,
            )),
        )],
    ))
}

pub(super) fn go_ast_inventory_binary_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_binary_expr_return_expression_spec(
    left_literal_kind: &str,
    left_literal_value: &str,
    operator: &str,
    right_literal_kind: &str,
    right_literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "BinaryExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                    left_literal_kind,
                    left_literal_value,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Op",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    "go_token_binary_operator_tokens.ADD",
                    operator,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Y",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                    right_literal_kind,
                    right_literal_value,
                )),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_index_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_index_expr_return_expression_spec(
    target_identifier: &str,
    index_literal_kind: &str,
    index_literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "IndexExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    target_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Index",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                    index_literal_kind,
                    index_literal_value,
                )),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_index_list_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_index_list_expr_return_expression_spec(
    target_identifier: &str,
    first_index_identifier: &str,
    second_index_identifier: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "IndexListExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    target_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Indices",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_ident_node(first_index_identifier),
                    declarative_ident_node(second_index_identifier),
                ]),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_interface_type_var_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ValueSpec", "Type")
}

pub(super) fn go_ast_inventory_interface_type_var_expression_spec(
    incomplete: bool,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "InterfaceType",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Methods",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    GoAstInventoryDeclarativeNodeSpec::new(
                        "FieldList",
                        vec![GoAstInventoryDeclarativeFieldSpec::new(
                            "List",
                            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![]),
                        )],
                    ),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Incomplete",
                GoAstInventoryDeclarativeFieldValue::boolean_payload(incomplete),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_struct_type_decl_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("TypeSpec", "Type")
}

pub(super) fn go_ast_inventory_struct_type_decl_expression_spec(
    field_name: &str,
    field_type: &str,
    incomplete: bool,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "StructType",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Fields",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    go_ast_inventory_struct_type_field_list_node(field_name, field_type),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Incomplete",
                GoAstInventoryDeclarativeFieldValue::boolean_payload(incomplete),
            ),
        ],
    ))
}

fn go_ast_inventory_struct_type_field_list_node(
    field_name: &str,
    field_type: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "FieldList",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "List",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                go_ast_inventory_struct_type_field_node(field_name, field_type),
            ]),
        )],
    )
}

fn go_ast_inventory_struct_type_field_node(
    field_name: &str,
    field_type: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "Field",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Doc",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Names",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_ident_node(field_name),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Type",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    field_type,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Tag",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Comment",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
        ],
    )
}

pub(super) fn go_ast_inventory_slice_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_slice_expr_return_expression_spec(
    target_identifier: &str,
    low_literal_kind: &str,
    low_literal_value: &str,
    high_literal_kind: &str,
    high_literal_value: &str,
    three_index: bool,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "SliceExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    target_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Low",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                    low_literal_kind,
                    low_literal_value,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "High",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                    high_literal_kind,
                    high_literal_value,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Max",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Slice3",
                GoAstInventoryDeclarativeFieldValue::boolean_payload(three_index),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_selector_call_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_defer_stmt_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_go_stmt_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_if_stmt_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_inc_dec_stmt_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_send_stmt_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec
{
    GoAstInventoryDeclarativeParentFieldSpec::new("BlockStmt", "List")
}

pub(super) fn go_ast_inventory_defer_stmt_statement_spec(
    receiver: &str,
    selector: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "DeferStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Call",
            GoAstInventoryDeclarativeFieldValue::generic_node(
                go_ast_inventory_selector_call_expression_spec(receiver, selector).root,
            ),
        )],
    ))
}

pub(super) fn go_ast_inventory_go_stmt_statement_spec(
    receiver: &str,
    selector: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "GoStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Call",
            GoAstInventoryDeclarativeFieldValue::generic_node(
                go_ast_inventory_selector_call_expression_spec(receiver, selector).root,
            ),
        )],
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn go_ast_inventory_if_stmt_statement_spec(
    init_identifier: &str,
    init_operator: &str,
    init_literal_kind: &str,
    init_literal_value: &str,
    cond_identifier: &str,
    body_return_literal_kind: &str,
    body_return_literal_value: &str,
    else_return_literal_kind: &str,
    else_return_literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "IfStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Init",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    go_ast_inventory_assignment_statement_spec(
                        init_identifier,
                        init_operator,
                        init_literal_kind,
                        init_literal_value,
                    )
                    .root,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Cond",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    cond_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Body",
                GoAstInventoryDeclarativeFieldValue::generic_node(
                    declarative_single_statement_block_node(declarative_return_basic_lit_node(
                        body_return_literal_kind,
                        body_return_literal_value,
                    )),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Else",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    declarative_single_statement_block_node(declarative_return_basic_lit_node(
                        else_return_literal_kind,
                        else_return_literal_value,
                    )),
                ),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_inc_dec_stmt_statement_spec(
    target_identifier: &str,
    token: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "IncDecStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    target_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Tok",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    "go_token_inc_dec_tokens.INC",
                    token,
                ),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_send_stmt_statement_spec(
    channel_identifier: &str,
    value_identifier: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "SendStmt",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Chan",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    channel_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Value",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    value_identifier,
                )),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_selector_call_expression_spec(
    receiver: &str,
    selector: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "CallExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Fun",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    GoAstInventoryDeclarativeNodeSpec::new(
                        "SelectorExpr",
                        vec![
                            GoAstInventoryDeclarativeFieldSpec::new(
                                "X",
                                GoAstInventoryDeclarativeFieldValue::alternative_node(
                                    declarative_ident_node(receiver),
                                ),
                            ),
                            GoAstInventoryDeclarativeFieldSpec::new(
                                "Sel",
                                GoAstInventoryDeclarativeFieldValue::generic_node(
                                    declarative_ident_node(selector),
                                ),
                            ),
                        ],
                    ),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Args",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(Vec::new()),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Ellipsis",
                GoAstInventoryDeclarativeFieldValue::selected_boolean_payload(
                    "go_syntax_presence_call_expr_ellipsis.ABSENT",
                    false,
                ),
            ),
        ],
    ))
}

pub(super) fn go_ast_inventory_composite_lit_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_composite_lit_return_expression_spec(
    element_type: &str,
    literal_kind: &str,
    literal_value: &str,
    incomplete: bool,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "CompositeLit",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Type",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    go_ast_inventory_array_type_return_expression_spec(element_type).root,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Elts",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_basic_lit_node(literal_kind, literal_value),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Incomplete",
                GoAstInventoryDeclarativeFieldValue::boolean_payload(incomplete),
            ),
        ],
    ))
}

fn go_ast_inventory_ellipsis_array_literal_expression_spec(
    element_type: &str,
    literal_kind: &str,
    literal_value: &str,
    incomplete: bool,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "CompositeLit",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Type",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    declarative_ellipsis_array_type_node(element_type),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Elts",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    declarative_basic_lit_node(literal_kind, literal_value),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Incomplete",
                GoAstInventoryDeclarativeFieldValue::boolean_payload(incomplete),
            ),
        ],
    ))
}

fn declarative_ellipsis_array_type_node(element_type: &str) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "ArrayType",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Len",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    declarative_empty_ellipsis_node(),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Elt",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    element_type,
                )),
            ),
        ],
    )
}

fn declarative_empty_ellipsis_node() -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "Ellipsis",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Elt",
            GoAstInventoryDeclarativeFieldValue::Missing,
        )],
    )
}

pub(super) fn go_ast_inventory_key_value_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_key_value_expr_return_expression_spec(
    key_type: &str,
    value_type: &str,
    key_literal_kind: &str,
    key_literal_value: &str,
    value_literal_kind: &str,
    value_literal_value: &str,
    incomplete: bool,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "CompositeLit",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Type",
                GoAstInventoryDeclarativeFieldValue::alternative_node(
                    go_ast_inventory_key_value_expr_return_map_type_node(key_type, value_type),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Elts",
                GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                    go_ast_inventory_key_value_expr_return_key_value_node(
                        key_literal_kind,
                        key_literal_value,
                        value_literal_kind,
                        value_literal_value,
                    ),
                ]),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Incomplete",
                GoAstInventoryDeclarativeFieldValue::boolean_payload(incomplete),
            ),
        ],
    ))
}

fn go_ast_inventory_key_value_expr_return_map_type_spec(
    key_type: &str,
    value_type: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(
        go_ast_inventory_key_value_expr_return_map_type_node(key_type, value_type),
    )
}

fn go_ast_inventory_key_value_expr_return_map_type_node(
    key_type: &str,
    value_type: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "MapType",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Key",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    key_type,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Value",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    value_type,
                )),
            ),
        ],
    )
}

fn go_ast_inventory_key_value_expr_return_key_value_node(
    key_literal_kind: &str,
    key_literal_value: &str,
    value_literal_kind: &str,
    value_literal_value: &str,
) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "KeyValueExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Key",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                    key_literal_kind,
                    key_literal_value,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Value",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_basic_lit_node(
                    value_literal_kind,
                    value_literal_value,
                )),
            ),
        ],
    )
}

pub(super) fn go_ast_inventory_func_lit_parent_field() -> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ValueSpec", "Values")
}

pub(super) fn go_ast_inventory_func_lit_expression_spec() -> GoAstInventoryDeclarativeExpressionSpec
{
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "FuncLit",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Type",
                GoAstInventoryDeclarativeFieldValue::generic_node(
                    declarative_func_type_no_params_no_results_node(),
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Body",
                GoAstInventoryDeclarativeFieldValue::generic_node(
                    declarative_empty_block_stmt_node(),
                ),
            ),
        ],
    ))
}

fn declarative_empty_block_stmt_node() -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "BlockStmt",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "List",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(Vec::new()),
        )],
    )
}

pub(super) fn go_ast_inventory_star_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_star_expr_return_expression_spec(
    target_identifier: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "StarExpr",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "X",
            GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                target_identifier,
            )),
        )],
    ))
}

pub(super) fn go_ast_inventory_type_assert_expr_return_parent_field()
-> GoAstInventoryDeclarativeParentFieldSpec {
    GoAstInventoryDeclarativeParentFieldSpec::new("ReturnStmt", "Results")
}

pub(super) fn go_ast_inventory_type_assert_expr_return_expression_spec(
    target_identifier: &str,
    asserted_type: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "TypeAssertExpr",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "X",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    target_identifier,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Type",
                GoAstInventoryDeclarativeFieldValue::alternative_node(declarative_ident_node(
                    asserted_type,
                )),
            ),
        ],
    ))
}

fn declarative_ident_node(payload: &str) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "Ident",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Name",
            GoAstInventoryDeclarativeFieldValue::terminal_payload(payload),
        )],
    )
}

fn go_ast_inventory_import_spec_expression_spec(
    alias: &str,
    path_literal_kind: &str,
    path_literal_value: &str,
) -> GoAstInventoryDeclarativeExpressionSpec {
    GoAstInventoryDeclarativeExpressionSpec::new(GoAstInventoryDeclarativeNodeSpec::new(
        "ImportSpec",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Doc",
                GoAstInventoryDeclarativeFieldValue::optional_node_absent(),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Name",
                GoAstInventoryDeclarativeFieldValue::generic_node(declarative_ident_node(alias)),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Path",
                GoAstInventoryDeclarativeFieldValue::generic_node(declarative_basic_lit_node(
                    path_literal_kind,
                    path_literal_value,
                )),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Comment",
                GoAstInventoryDeclarativeFieldValue::Missing,
            ),
        ],
    ))
}

fn declarative_comment_group_node(comment_text: &str) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "CommentGroup",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "List",
            GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(vec![
                declarative_comment_node(comment_text),
            ]),
        )],
    )
}

fn declarative_comment_node(comment_text: &str) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "Comment",
        vec![GoAstInventoryDeclarativeFieldSpec::new(
            "Text",
            GoAstInventoryDeclarativeFieldValue::selected_terminal_payload_with_kind(
                "Comment.Text.string",
                GrammarTerminalKind::StringPayload,
                comment_text,
            ),
        )],
    )
}

fn declarative_basic_lit_node(kind: &str, value: &str) -> GoAstInventoryDeclarativeNodeSpec {
    GoAstInventoryDeclarativeNodeSpec::new(
        "BasicLit",
        vec![
            GoAstInventoryDeclarativeFieldSpec::new(
                "Kind",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload(
                    &format!("go_token_basic_literal_kinds.{kind}"),
                    kind,
                ),
            ),
            GoAstInventoryDeclarativeFieldSpec::new(
                "Value",
                GoAstInventoryDeclarativeFieldValue::selected_terminal_payload_with_kind(
                    "BasicLit.Value.string",
                    GrammarTerminalKind::StringPayload,
                    value,
                ),
            ),
        ],
    )
}

pub(super) fn function_return_base_expected_field_orders() -> Vec<GoAstInventoryExpectedFieldOrder>
{
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
            "FuncDecl",
            &["Doc", "Recv", "Name", "Type", "Body"][..],
        ),
        GoAstInventoryExpectedFieldOrder::new(
            "FuncType",
            &["Func", "TypeParams", "Params", "Results"][..],
        ),
        GoAstInventoryExpectedFieldOrder::new("FieldList", &["List", "BlankLineBefore"][..]),
        GoAstInventoryExpectedFieldOrder::new(
            "Field",
            &["Doc", "Names", "Type", "Tag", "Comment"][..],
        ),
        GoAstInventoryExpectedFieldOrder::new("Ident", &["Name"][..]),
        GoAstInventoryExpectedFieldOrder::new(
            "BlockStmt",
            &[
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ][..],
        ),
        GoAstInventoryExpectedFieldOrder::new("ReturnStmt", &["Results"][..]),
    ]
}

pub(super) fn function_statement_list_base_expected_field_orders()
-> Vec<GoAstInventoryExpectedFieldOrder> {
    function_return_base_expected_field_orders()
        .into_iter()
        .filter(|order| order.rule_name != "ReturnStmt")
        .collect()
}

pub(super) fn function_statement_list_no_result_base_expected_field_orders()
-> Vec<GoAstInventoryExpectedFieldOrder> {
    function_statement_list_base_expected_field_orders()
        .into_iter()
        .filter(|order| order.rule_name != "Field")
        .collect()
}

pub(super) fn package_var_type_base_expected_field_orders() -> Vec<GoAstInventoryExpectedFieldOrder>
{
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
    ]
}

pub(super) fn package_var_value_base_expected_field_orders() -> Vec<GoAstInventoryExpectedFieldOrder>
{
    package_var_type_base_expected_field_orders()
}

pub(super) fn function_return_presence_finite_domain_requirements()
-> Vec<GoAstInventoryFiniteDomainRequirement> {
    vec![GoAstInventoryFiniteDomainRequirement::new(
        "FuncType",
        "Func",
        "go_syntax_presence_func_type_func.PRESENT",
        GrammarTerminalKind::BooleanPayload,
    )]
}

pub(super) fn function_no_result_presence_finite_domain_requirements()
-> Vec<GoAstInventoryFiniteDomainRequirement> {
    function_return_presence_finite_domain_requirements()
}

pub(super) fn package_var_type_finite_domain_requirements()
-> Vec<GoAstInventoryFiniteDomainRequirement> {
    vec![
        GoAstInventoryFiniteDomainRequirement::new(
            "GenDecl",
            "Tok",
            "go_token_declaration_tokens.VAR",
            GrammarTerminalKind::IdentifierPayload,
        ),
        GoAstInventoryFiniteDomainRequirement::new(
            "GenDecl",
            "Specs",
            "ValueSpec",
            GrammarTerminalKind::Nonterminal,
        ),
    ]
}

pub(super) fn package_var_value_finite_domain_requirements()
-> Vec<GoAstInventoryFiniteDomainRequirement> {
    package_var_type_finite_domain_requirements()
}

pub(super) fn package_type_decl_base_expected_field_orders() -> Vec<GoAstInventoryExpectedFieldOrder>
{
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
            "TypeSpec",
            &["Doc", "Name", "TypeParams", "Assign", "Type", "Comment"][..],
        ),
        GoAstInventoryExpectedFieldOrder::new("Ident", &["Name"][..]),
    ]
}

pub(super) fn package_type_decl_finite_domain_requirements()
-> Vec<GoAstInventoryFiniteDomainRequirement> {
    vec![
        GoAstInventoryFiniteDomainRequirement::new(
            "GenDecl",
            "Tok",
            "go_token_declaration_tokens.TYPE",
            GrammarTerminalKind::IdentifierPayload,
        ),
        GoAstInventoryFiniteDomainRequirement::new(
            "GenDecl",
            "Specs",
            "TypeSpec",
            GrammarTerminalKind::Nonterminal,
        ),
    ]
}

pub(super) fn package_import_spec_base_expected_field_orders()
-> Vec<GoAstInventoryExpectedFieldOrder> {
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
            "ImportSpec",
            &["Doc", "Name", "Path", "Comment"][..],
        ),
        GoAstInventoryExpectedFieldOrder::new("Ident", &["Name"][..]),
        GoAstInventoryExpectedFieldOrder::new("BasicLit", &["Kind", "Value"][..]),
    ]
}

pub(super) fn package_import_spec_finite_domain_requirements()
-> Vec<GoAstInventoryFiniteDomainRequirement> {
    vec![GoAstInventoryFiniteDomainRequirement::new(
        "GenDecl",
        "Tok",
        "go_token_declaration_tokens.IMPORT",
        GrammarTerminalKind::IdentifierPayload,
    )]
}

pub(super) fn combine_field_order_requirements(
    mut base: Vec<GoAstInventoryExpectedFieldOrder>,
    derived: Vec<GoAstInventoryExpectedFieldOrder>,
) -> Vec<GoAstInventoryExpectedFieldOrder> {
    for order in derived {
        if !base
            .iter()
            .any(|existing| existing.rule_name == order.rule_name)
        {
            base.push(order);
        }
    }
    base
}

pub(super) fn combine_finite_domain_requirements(
    mut base: Vec<GoAstInventoryFiniteDomainRequirement>,
    derived: Vec<GoAstInventoryFiniteDomainRequirement>,
) -> Vec<GoAstInventoryFiniteDomainRequirement> {
    for requirement in derived {
        if !base.iter().any(|existing| {
            existing.rule_name == requirement.rule_name
                && existing.field_name == requirement.field_name
                && existing.selected_alternative == requirement.selected_alternative
                && existing.terminal == requirement.terminal
        }) {
            base.push(requirement);
        }
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_construction_registers_entries() {
        let registry = go_ast_inventory_synthesis_registry()
            .expect("canonical synthesis registry should construct");
        let assignment_entry = registry
            .entry(GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID)
            .expect("AssignStmt entry should be registered");
        assert_eq!(
            assignment_entry.api_id,
            GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID
        );
        assert_eq!(
            assignment_entry.api_boundary,
            GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            assignment_entry.shell,
            GoAstInventorySynthesisShell::FunctionStatementListReturn
        );
        let import_spec_entry = registry
            .entry(GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID)
            .expect("ImportSpec entry should be registered");
        assert_eq!(
            import_spec_entry.api_boundary,
            GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            import_spec_entry.shell,
            GoAstInventorySynthesisShell::PackageImportSpec
        );
        let branch_stmt_entry = registry
            .entry(GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID)
            .expect("BranchStmt entry should be registered");
        assert_eq!(
            branch_stmt_entry.api_boundary,
            GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            branch_stmt_entry.shell,
            GoAstInventorySynthesisShell::FunctionStatementListReturn
        );
        let comm_clause_entry = registry
            .entry(GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID)
            .expect("CommClause entry should be registered");
        assert_eq!(
            comm_clause_entry.api_boundary,
            GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            comm_clause_entry.shell,
            GoAstInventorySynthesisShell::FunctionStatementList
        );
        let decl_stmt_entry = registry
            .entry(GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID)
            .expect("DeclStmt entry should be registered");
        assert_eq!(
            decl_stmt_entry.api_boundary,
            GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            decl_stmt_entry.shell,
            GoAstInventorySynthesisShell::FunctionStatementListReturn
        );
        let go_stmt_entry = registry
            .entry(GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID)
            .expect("GoStmt entry should be registered");
        assert_eq!(
            go_stmt_entry.api_boundary,
            GO_AST_INVENTORY_GO_STMT_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            go_stmt_entry.shell,
            GoAstInventorySynthesisShell::FunctionStatementListNoResult
        );
        let if_stmt_entry = registry
            .entry(GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID)
            .expect("IfStmt entry should be registered");
        assert_eq!(
            if_stmt_entry.api_boundary,
            GO_AST_INVENTORY_IF_STMT_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            if_stmt_entry.shell,
            GoAstInventorySynthesisShell::FunctionStatementList
        );
        let array_type_entry = registry
            .entry(GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID)
            .expect("ArrayType return entry should be registered");
        assert_eq!(
            array_type_entry.api_boundary,
            GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            array_type_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let chan_type_entry = registry
            .entry(GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID)
            .expect("ChanType return entry should be registered");
        assert_eq!(
            chan_type_entry.api_boundary,
            GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            chan_type_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let binary_entry = registry
            .entry(GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("BinaryExpr return entry should be registered");
        assert_eq!(
            binary_entry.api_boundary,
            GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            binary_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let unary_entry = registry
            .entry(GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("UnaryExpr return entry should be registered");
        assert_eq!(
            unary_entry.api_boundary,
            GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            unary_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let paren_entry = registry
            .entry(GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("ParenExpr return entry should be registered");
        assert_eq!(
            paren_entry.api_boundary,
            GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            paren_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let index_entry = registry
            .entry(GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("IndexExpr return entry should be registered");
        assert_eq!(
            index_entry.api_boundary,
            GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            index_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let index_list_entry = registry
            .entry(GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("IndexListExpr return entry should be registered");
        assert_eq!(
            index_list_entry.api_boundary,
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            index_list_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let slice_expr_entry = registry
            .entry(GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("SliceExpr return entry should be registered");
        assert_eq!(
            slice_expr_entry.api_boundary,
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            slice_expr_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let interface_type_entry = registry
            .entry(GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID)
            .expect("InterfaceType var entry should be registered");
        assert_eq!(
            interface_type_entry.api_boundary,
            GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            interface_type_entry.shell,
            GoAstInventorySynthesisShell::PackageVarType
        );
        let struct_type_entry = registry
            .entry(GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID)
            .expect("StructType declaration entry should be registered");
        assert_eq!(
            struct_type_entry.api_boundary,
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            struct_type_entry.shell,
            GoAstInventorySynthesisShell::PackageType
        );
        let selector_call_entry = registry
            .entry(GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID)
            .expect("SelectorCall entry should be registered");
        assert_eq!(
            selector_call_entry.api_boundary,
            GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            selector_call_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let composite_lit_entry = registry
            .entry(GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID)
            .expect("CompositeLit return entry should be registered");
        assert_eq!(
            composite_lit_entry.api_boundary,
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            composite_lit_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let ellipsis_array_lit_entry = registry
            .entry(GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID)
            .expect("EllipsisArrayLiteral entry should be registered");
        assert_eq!(
            ellipsis_array_lit_entry.api_boundary,
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            ellipsis_array_lit_entry.shell,
            GoAstInventorySynthesisShell::PackageVarValue
        );
        let key_value_entry = registry
            .entry(GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("KeyValueExpr return entry should be registered");
        assert_eq!(
            key_value_entry.api_boundary,
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            key_value_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let func_lit_entry = registry
            .entry(GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID)
            .expect("FuncLit entry should be registered");
        assert_eq!(
            func_lit_entry.api_boundary,
            GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            func_lit_entry.shell,
            GoAstInventorySynthesisShell::PackageVarValue
        );
        let star_entry = registry
            .entry(GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("StarExpr return entry should be registered");
        assert_eq!(
            star_entry.api_boundary,
            GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            star_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let type_assert_entry = registry
            .entry(GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID)
            .expect("TypeAssertExpr return entry should be registered");
        assert_eq!(
            type_assert_entry.api_boundary,
            GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            type_assert_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let function_return_entry = registry
            .entry(GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID)
            .expect("FunctionReturn entry should be registered");
        assert_eq!(
            function_return_entry.api_boundary,
            GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            function_return_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let func_type_entry = registry
            .entry(GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID)
            .expect("FuncTypeResult entry should be registered");
        assert_eq!(
            func_type_entry.api_boundary,
            GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            func_type_entry.shell,
            GoAstInventorySynthesisShell::FunctionReturn
        );
        let send_stmt_entry = registry
            .entry(GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID)
            .expect("SendStmt entry should be registered");
        assert_eq!(
            send_stmt_entry.api_boundary,
            GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_BOUNDARY
        );
        assert_eq!(
            send_stmt_entry.shell,
            GoAstInventorySynthesisShell::FunctionStatementListNoResult
        );
        assert_eq!(registry.entries().len(), 34);
    }

    #[test]
    fn registry_rejects_unknown_api_id_with_typed_error() {
        let registry = go_ast_inventory_synthesis_registry()
            .expect("canonical synthesis registry should construct");
        let error = registry
            .entry("go-ast-inventory-unknown-synthesis-v1")
            .expect_err("unknown api id should be rejected");
        assert_eq!(error.field(), "go_ast_inventory_synthesis_registry.api_id");
        assert!(
            format!("{error}").contains("go-ast-inventory-unknown-synthesis-v1"),
            "unknown-api error should name the requested api id: {error}"
        );
    }

    #[test]
    fn registry_rejects_duplicate_entries_at_construction() {
        let error = GoAstInventorySynthesisRegistry::from_entries(vec![
            go_ast_inventory_assignment_registry_entry(),
            go_ast_inventory_assignment_registry_entry(),
        ])
        .expect_err("duplicate api ids should be rejected at construction");
        assert_eq!(error.field(), "go_ast_inventory_synthesis_registry.entries");
        assert!(
            format!("{error}").contains(GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID),
            "duplicate-entry error should name the colliding api id: {error}"
        );
    }

    #[test]
    fn registry_rejects_rule_default_kind_mismatch_at_construction() {
        const MISMATCHED_FIELDS: &[GoAstInventoryRegistryPayloadField] =
            &[GoAstInventoryRegistryPayloadField {
                key: "incomplete",
                error_context: "go_ast_inventory_test_synthesis.payload_spec.incomplete",
                default: GoAstInventoryRegistryPayloadDefault::Bool(false),
                rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
            }];
        let mut entry = go_ast_inventory_assignment_registry_entry();
        entry.payload_fields = MISMATCHED_FIELDS;
        let error = GoAstInventorySynthesisRegistry::from_entries(vec![entry])
            .expect_err("string rule on boolean default should be rejected at construction");
        assert_eq!(error.field(), "go_ast_inventory_synthesis_registry.entries");
        assert!(
            format!("{error}").contains("incomplete"),
            "kind-mismatch error should name the payload key: {error}"
        );
    }

    #[test]
    fn payload_merge_rejects_undeclared_keys_and_type_mismatches() {
        let entry = go_ast_inventory_assignment_registry_entry();
        let unknown_key_error = merge_payload_with_entry_defaults(
            &entry,
            GoAstInventoryRegistryPayload::new().with_str("unknown_key", "x"),
        )
        .expect_err("undeclared payload keys should be rejected");
        assert_eq!(
            unknown_key_error.field(),
            "go_ast_inventory_synthesis_registry.payload"
        );
        assert!(
            format!("{unknown_key_error}").contains("unknown_key"),
            "unknown-key error should name the key: {unknown_key_error}"
        );

        let type_mismatch_error = merge_payload_with_entry_defaults(
            &entry,
            GoAstInventoryRegistryPayload::new().with_bool("operator", true),
        )
        .expect_err("boolean value for a string payload key should be rejected");
        assert_eq!(
            type_mismatch_error.field(),
            "go_ast_inventory_synthesis_registry.payload"
        );

        let merged =
            merge_payload_with_entry_defaults(&entry, GoAstInventoryRegistryPayload::new())
                .expect("empty payload should merge to entry defaults");
        assert_eq!(
            merged
                .str_value("operator")
                .expect("merged payload should carry operator"),
            GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_OPERATOR
        );
        assert_eq!(
            merged
                .str_value("package_name")
                .expect("merged payload should carry package_name"),
            GO_AST_INVENTORY_ASSIGNMENT_DEFAULT_PACKAGE_NAME
        );
    }

    #[test]
    fn payload_rules_enforce_non_empty_exact_and_forbidden_true() {
        let non_empty_field = GoAstInventoryRegistryPayloadField {
            key: "identifier",
            error_context: "go_ast_inventory_test_synthesis.payload_spec.identifier",
            default: GoAstInventoryRegistryPayloadDefault::Str("f"),
            rule: GoAstInventoryRegistryPayloadRule::NonEmpty,
        };
        let error = apply_payload_rule(
            &non_empty_field,
            &GoAstInventoryRegistryPayload::new().with_str("identifier", ""),
        )
        .expect_err("empty string should violate NonEmpty");
        assert_eq!(error.field(), non_empty_field.error_context);

        let exact_field = GoAstInventoryRegistryPayloadField {
            key: "operator",
            error_context: "go_ast_inventory_test_synthesis.payload_spec.operator",
            default: GoAstInventoryRegistryPayloadDefault::Str("="),
            rule: GoAstInventoryRegistryPayloadRule::Exact("="),
        };
        let error = apply_payload_rule(
            &exact_field,
            &GoAstInventoryRegistryPayload::new().with_str("operator", "+="),
        )
        .expect_err("non-pinned operator should violate Exact");
        assert_eq!(error.field(), exact_field.error_context);
        apply_payload_rule(
            &exact_field,
            &GoAstInventoryRegistryPayload::new().with_str("operator", "="),
        )
        .expect("pinned operator should satisfy Exact");

        let forbidden_true_field = GoAstInventoryRegistryPayloadField {
            key: "incomplete",
            error_context: "go_ast_inventory_test_synthesis.payload_spec.incomplete",
            default: GoAstInventoryRegistryPayloadDefault::Bool(false),
            rule: GoAstInventoryRegistryPayloadRule::ForbiddenTrue(
                "incomplete composite literals are not part of the v1 fixture contract",
            ),
        };
        let error = apply_payload_rule(
            &forbidden_true_field,
            &GoAstInventoryRegistryPayload::new().with_bool("incomplete", true),
        )
        .expect_err("true should violate ForbiddenTrue");
        assert_eq!(error.field(), forbidden_true_field.error_context);
        apply_payload_rule(
            &forbidden_true_field,
            &GoAstInventoryRegistryPayload::new().with_bool("incomplete", false),
        )
        .expect("false should satisfy ForbiddenTrue");
    }
}
