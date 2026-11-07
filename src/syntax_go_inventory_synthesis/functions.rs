use crate::syntax_go_inventory_synthesis::GoAstInventoryDeclarativeTemplateCoverageSpec;
use crate::syntax_go_inventory_synthesis::registry::go_ast_inventory_import_spec_declarative_coverage_spec;

mod array_type;
mod assignment;
mod binary_expr;
mod branch_stmt;
mod case_clause;
mod chan_type;
mod comm_clause;
mod composite_lit;
mod decl_stmt;
mod defer_stmt;
mod ellipsis_array_lit;
mod empty_stmt;
mod func_lit;
mod func_type;
mod function_return;
mod go_stmt;
mod if_stmt;
mod inc_dec_stmt;
mod index_expr;
mod index_list_expr;
mod interface_type;
mod key_value_expr;
mod labeled_stmt;
mod paren_expr;
mod range_stmt;
mod selector_call;
mod send_stmt;
mod slice_expr;
mod star_expr;
mod struct_type;
mod type_assert_expr;
mod type_switch_stmt;
mod unary_expr;
pub use self::array_type::{
    GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryArrayTypeReturnPayloadSpec, GoAstInventoryArrayTypeReturnSynthesis,
    go_ast_inventory_array_type_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_array_type_return_raw_tokens,
    synthesize_go_ast_inventory_array_type_return_raw_tokens_from_inventory,
};
pub use self::assignment::{
    GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID, GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_BOUNDARY,
    GoAstInventoryAssignmentPayloadSpec, GoAstInventoryAssignmentSynthesis,
    go_ast_inventory_assignment_declarative_coverage_spec,
    synthesize_go_ast_inventory_assignment_raw_tokens,
    synthesize_go_ast_inventory_assignment_raw_tokens_from_inventory,
};
pub use self::binary_expr::{
    GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryBinaryExprReturnPayloadSpec, GoAstInventoryBinaryExprReturnSynthesis,
    go_ast_inventory_binary_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_binary_expr_return_raw_tokens,
    synthesize_go_ast_inventory_binary_expr_return_raw_tokens_from_inventory,
};
pub use self::branch_stmt::{
    GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_BOUNDARY,
    GoAstInventoryBranchStmtPayloadSpec, GoAstInventoryBranchStmtSynthesis,
    go_ast_inventory_branch_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_branch_stmt_raw_tokens,
    synthesize_go_ast_inventory_branch_stmt_raw_tokens_from_inventory,
};
pub use self::case_clause::{
    GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID, GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_BOUNDARY,
    GoAstInventoryCaseClausePayloadSpec, GoAstInventoryCaseClauseSynthesis,
    go_ast_inventory_case_clause_declarative_coverage_spec,
    synthesize_go_ast_inventory_case_clause_raw_tokens,
    synthesize_go_ast_inventory_case_clause_raw_tokens_from_inventory,
};
pub use self::chan_type::{
    GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_BOUNDARY, GoAstInventoryChanTypeReturnPayloadSpec,
    GoAstInventoryChanTypeReturnSynthesis,
    go_ast_inventory_chan_type_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_chan_type_return_raw_tokens,
    synthesize_go_ast_inventory_chan_type_return_raw_tokens_from_inventory,
};
pub use self::comm_clause::{
    GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID, GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_BOUNDARY,
    GoAstInventoryCommClausePayloadSpec, GoAstInventoryCommClauseSynthesis,
    go_ast_inventory_comm_clause_declarative_coverage_spec,
    synthesize_go_ast_inventory_comm_clause_raw_tokens,
    synthesize_go_ast_inventory_comm_clause_raw_tokens_from_inventory,
};
pub use self::composite_lit::{
    GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryCompositeLitReturnPayloadSpec, GoAstInventoryCompositeLitReturnSynthesis,
    go_ast_inventory_composite_lit_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_composite_lit_return_raw_tokens,
    synthesize_go_ast_inventory_composite_lit_return_raw_tokens_from_inventory,
};
pub use self::decl_stmt::{
    GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_BOUNDARY,
    GoAstInventoryDeclStmtPayloadSpec, GoAstInventoryDeclStmtSynthesis,
    go_ast_inventory_decl_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_decl_stmt_raw_tokens,
    synthesize_go_ast_inventory_decl_stmt_raw_tokens_from_inventory,
};
pub use self::defer_stmt::{
    GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_BOUNDARY,
    GoAstInventoryDeferStmtPayloadSpec, GoAstInventoryDeferStmtSynthesis,
    go_ast_inventory_defer_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_defer_stmt_raw_tokens,
    synthesize_go_ast_inventory_defer_stmt_raw_tokens_from_inventory,
};
pub use self::ellipsis_array_lit::{
    GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_BOUNDARY,
    GoAstInventoryEllipsisArrayLiteralPayloadSpec, GoAstInventoryEllipsisArrayLiteralSynthesis,
    synthesize_go_ast_inventory_ellipsis_array_literal_raw_tokens,
    synthesize_go_ast_inventory_ellipsis_array_literal_raw_tokens_from_inventory,
};
pub use self::empty_stmt::{
    GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_BOUNDARY,
    GoAstInventoryEmptyStmtPayloadSpec, GoAstInventoryEmptyStmtSynthesis,
    go_ast_inventory_empty_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_empty_stmt_raw_tokens,
    synthesize_go_ast_inventory_empty_stmt_raw_tokens_from_inventory,
};
pub use self::func_lit::{
    GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID, GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_BOUNDARY,
    GoAstInventoryFuncLitPayloadSpec, GoAstInventoryFuncLitSynthesis,
    go_ast_inventory_func_lit_declarative_coverage_spec,
    synthesize_go_ast_inventory_func_lit_raw_tokens,
    synthesize_go_ast_inventory_func_lit_raw_tokens_from_inventory,
};
pub use self::func_type::{
    GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_BOUNDARY, GoAstInventoryFuncTypeResultPayloadSpec,
    GoAstInventoryFuncTypeResultSynthesis, synthesize_go_ast_inventory_func_type_result_raw_tokens,
    synthesize_go_ast_inventory_func_type_result_raw_tokens_from_inventory,
};
pub use self::function_return::{
    GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_BOUNDARY, GoAstInventoryFunctionReturnPayloadSpec,
    GoAstInventoryFunctionReturnSynthesis, synthesize_go_ast_inventory_function_return_raw_tokens,
    synthesize_go_ast_inventory_function_return_raw_tokens_from_inventory,
};
pub use self::go_stmt::{
    GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_GO_STMT_SYNTHESIS_BOUNDARY,
    GoAstInventoryGoStmtPayloadSpec, GoAstInventoryGoStmtSynthesis,
    go_ast_inventory_go_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_go_stmt_raw_tokens,
    synthesize_go_ast_inventory_go_stmt_raw_tokens_from_inventory,
};
pub use self::if_stmt::{
    GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_IF_STMT_SYNTHESIS_BOUNDARY,
    GoAstInventoryIfStmtPayloadSpec, GoAstInventoryIfStmtSynthesis,
    go_ast_inventory_if_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_if_stmt_raw_tokens,
    synthesize_go_ast_inventory_if_stmt_raw_tokens_from_inventory,
};
pub use self::inc_dec_stmt::{
    GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_BOUNDARY, GoAstInventoryIncDecStmtPayloadSpec,
    GoAstInventoryIncDecStmtSynthesis, go_ast_inventory_inc_dec_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_inc_dec_stmt_raw_tokens,
    synthesize_go_ast_inventory_inc_dec_stmt_raw_tokens_from_inventory,
};
pub use self::index_expr::{
    GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryIndexExprReturnPayloadSpec, GoAstInventoryIndexExprReturnSynthesis,
    go_ast_inventory_index_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_index_expr_return_raw_tokens,
    synthesize_go_ast_inventory_index_expr_return_raw_tokens_from_inventory,
};
pub use self::index_list_expr::{
    GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryIndexListExprReturnPayloadSpec, GoAstInventoryIndexListExprReturnSynthesis,
    go_ast_inventory_index_list_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_index_list_expr_return_raw_tokens,
    synthesize_go_ast_inventory_index_list_expr_return_raw_tokens_from_inventory,
};
pub use self::interface_type::{
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_BOUNDARY,
    GoAstInventoryInterfaceTypeVarPayloadSpec, GoAstInventoryInterfaceTypeVarSynthesis,
    go_ast_inventory_interface_type_var_declarative_coverage_spec,
    synthesize_go_ast_inventory_interface_type_var_raw_tokens,
    synthesize_go_ast_inventory_interface_type_var_raw_tokens_from_inventory,
};
pub use self::key_value_expr::{
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryKeyValueExprReturnPayloadSpec, GoAstInventoryKeyValueExprReturnSynthesis,
    go_ast_inventory_key_value_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_key_value_expr_return_raw_tokens,
    synthesize_go_ast_inventory_key_value_expr_return_raw_tokens_from_inventory,
};
pub use self::labeled_stmt::{
    GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_BOUNDARY, GoAstInventoryLabeledStmtPayloadSpec,
    GoAstInventoryLabeledStmtSynthesis, go_ast_inventory_labeled_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_labeled_stmt_raw_tokens,
    synthesize_go_ast_inventory_labeled_stmt_raw_tokens_from_inventory,
};
pub use self::paren_expr::{
    GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryParenExprReturnPayloadSpec, GoAstInventoryParenExprReturnSynthesis,
    go_ast_inventory_paren_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_paren_expr_return_raw_tokens,
    synthesize_go_ast_inventory_paren_expr_return_raw_tokens_from_inventory,
};
pub use self::range_stmt::{
    GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_BOUNDARY,
    GoAstInventoryRangeStmtPayloadSpec, GoAstInventoryRangeStmtSynthesis,
    go_ast_inventory_range_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_range_stmt_raw_tokens,
    synthesize_go_ast_inventory_range_stmt_raw_tokens_from_inventory,
};
pub use self::selector_call::{
    GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_BOUNDARY, GoAstInventorySelectorCallPayloadSpec,
    GoAstInventorySelectorCallSynthesis, go_ast_inventory_selector_call_declarative_coverage_spec,
    synthesize_go_ast_inventory_selector_call_raw_tokens,
    synthesize_go_ast_inventory_selector_call_raw_tokens_from_inventory,
};
pub use self::send_stmt::{
    GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_BOUNDARY,
    GoAstInventorySendStmtPayloadSpec, GoAstInventorySendStmtSynthesis,
    go_ast_inventory_send_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_send_stmt_raw_tokens,
    synthesize_go_ast_inventory_send_stmt_raw_tokens_from_inventory,
};
pub use self::slice_expr::{
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventorySliceExprReturnPayloadSpec, GoAstInventorySliceExprReturnSynthesis,
    go_ast_inventory_slice_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_slice_expr_return_raw_tokens,
    synthesize_go_ast_inventory_slice_expr_return_raw_tokens_from_inventory,
};
pub use self::star_expr::{
    GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_BOUNDARY, GoAstInventoryStarExprReturnPayloadSpec,
    GoAstInventoryStarExprReturnSynthesis,
    go_ast_inventory_star_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_star_expr_return_raw_tokens,
    synthesize_go_ast_inventory_star_expr_return_raw_tokens_from_inventory,
};
pub use self::struct_type::{
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_BOUNDARY, GoAstInventoryStructTypeDeclPayloadSpec,
    GoAstInventoryStructTypeDeclSynthesis,
    go_ast_inventory_struct_type_decl_declarative_coverage_spec,
    synthesize_go_ast_inventory_struct_type_decl_raw_tokens,
    synthesize_go_ast_inventory_struct_type_decl_raw_tokens_from_inventory,
};
pub use self::type_assert_expr::{
    GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryTypeAssertExprReturnPayloadSpec, GoAstInventoryTypeAssertExprReturnSynthesis,
    go_ast_inventory_type_assert_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_type_assert_expr_return_raw_tokens,
    synthesize_go_ast_inventory_type_assert_expr_return_raw_tokens_from_inventory,
};
pub use self::type_switch_stmt::{
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_BOUNDARY, GoAstInventoryTypeSwitchStmtPayloadSpec,
    GoAstInventoryTypeSwitchStmtSynthesis,
    go_ast_inventory_type_switch_stmt_declarative_coverage_spec,
    synthesize_go_ast_inventory_type_switch_stmt_raw_tokens,
    synthesize_go_ast_inventory_type_switch_stmt_raw_tokens_from_inventory,
};
pub use self::unary_expr::{
    GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_BOUNDARY,
    GoAstInventoryUnaryExprReturnPayloadSpec, GoAstInventoryUnaryExprReturnSynthesis,
    go_ast_inventory_unary_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_unary_expr_return_raw_tokens,
    synthesize_go_ast_inventory_unary_expr_return_raw_tokens_from_inventory,
};

pub fn go_ast_inventory_declarative_template_coverage_specs()
-> Vec<GoAstInventoryDeclarativeTemplateCoverageSpec> {
    vec![
        go_ast_inventory_import_spec_declarative_coverage_spec(),
        go_ast_inventory_array_type_return_declarative_coverage_spec(),
        go_ast_inventory_chan_type_return_declarative_coverage_spec(),
        go_ast_inventory_binary_expr_return_declarative_coverage_spec(),
        go_ast_inventory_branch_stmt_declarative_coverage_spec(),
        go_ast_inventory_case_clause_declarative_coverage_spec(),
        go_ast_inventory_comm_clause_declarative_coverage_spec(),
        go_ast_inventory_decl_stmt_declarative_coverage_spec(),
        go_ast_inventory_index_list_expr_return_declarative_coverage_spec(),
        go_ast_inventory_index_expr_return_declarative_coverage_spec(),
        go_ast_inventory_interface_type_var_declarative_coverage_spec(),
        go_ast_inventory_composite_lit_return_declarative_coverage_spec(),
        go_ast_inventory_func_lit_declarative_coverage_spec(),
        go_ast_inventory_key_value_expr_return_declarative_coverage_spec(),
        go_ast_inventory_labeled_stmt_declarative_coverage_spec(),
        go_ast_inventory_range_stmt_declarative_coverage_spec(),
        go_ast_inventory_paren_expr_return_declarative_coverage_spec(),
        go_ast_inventory_selector_call_declarative_coverage_spec(),
        go_ast_inventory_slice_expr_return_declarative_coverage_spec(),
        go_ast_inventory_star_expr_return_declarative_coverage_spec(),
        go_ast_inventory_struct_type_decl_declarative_coverage_spec(),
        go_ast_inventory_type_assert_expr_return_declarative_coverage_spec(),
        go_ast_inventory_type_switch_stmt_declarative_coverage_spec(),
        go_ast_inventory_unary_expr_return_declarative_coverage_spec(),
        go_ast_inventory_assignment_declarative_coverage_spec(),
        go_ast_inventory_defer_stmt_declarative_coverage_spec(),
        go_ast_inventory_go_stmt_declarative_coverage_spec(),
        go_ast_inventory_if_stmt_declarative_coverage_spec(),
        go_ast_inventory_empty_stmt_declarative_coverage_spec(),
        go_ast_inventory_inc_dec_stmt_declarative_coverage_spec(),
        go_ast_inventory_send_stmt_declarative_coverage_spec(),
    ]
}
