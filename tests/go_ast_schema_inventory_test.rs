use std::collections::{BTreeSet, hash_map::DefaultHasher};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;

use tct::go_ast_schema_inventory::{
    GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_NEXT_READY_TASK,
    GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY,
    GO_AST_SCHEMA_GRAMMAR_COVERAGE_NEXT_READY_TASK, GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY,
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION,
    GoAstRendererPositionPolicy, GoAstSchemaInventoryAdapter,
};
use tct::grammar_ir::GrammarTerminalKind;
use tct::grammar_ir_structural_model::lower_grammar_ir_to_structural_model;
use tct::syntax_generated_codec::go_syntax_token_layout;
use tct::syntax_go_inventory_synthesis::{
    GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID,
    GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID, GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID, GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID, GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID,
    GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID, GoAstInventoryDeclarativeFieldValue,
    GoAstInventoryExpectedFieldOrder, GoAstInventoryExpressionTemplate,
    GoAstInventoryFiniteDomainRequirement, GoAstInventoryRegistryPayload,
    GoAstInventorySynthesisShell, GoAstInventoryTemplateSpec,
    check_go_ast_inventory_declarative_expression_spec_from_inventory,
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
    go_ast_inventory_slice_expr_return_declarative_coverage_spec,
    go_ast_inventory_star_expr_return_declarative_coverage_spec,
    go_ast_inventory_struct_type_decl_declarative_coverage_spec,
    go_ast_inventory_synthesis_registry,
    go_ast_inventory_type_assert_expr_return_declarative_coverage_spec,
    go_ast_inventory_type_switch_stmt_declarative_coverage_spec,
    go_ast_inventory_unary_expr_return_declarative_coverage_spec,
    synthesize_go_ast_inventory_array_type_return_raw_tokens,
    synthesize_go_ast_inventory_assignment_raw_tokens,
    synthesize_go_ast_inventory_binary_expr_return_raw_tokens,
    synthesize_go_ast_inventory_branch_stmt_raw_tokens,
    synthesize_go_ast_inventory_case_clause_raw_tokens,
    synthesize_go_ast_inventory_chan_type_return_raw_tokens,
    synthesize_go_ast_inventory_comm_clause_raw_tokens,
    synthesize_go_ast_inventory_composite_lit_return_raw_tokens,
    synthesize_go_ast_inventory_decl_stmt_raw_tokens,
    synthesize_go_ast_inventory_defer_stmt_raw_tokens,
    synthesize_go_ast_inventory_ellipsis_array_literal_raw_tokens,
    synthesize_go_ast_inventory_empty_stmt_raw_tokens,
    synthesize_go_ast_inventory_func_lit_raw_tokens,
    synthesize_go_ast_inventory_func_type_result_raw_tokens,
    synthesize_go_ast_inventory_function_return_raw_tokens,
    synthesize_go_ast_inventory_go_stmt_raw_tokens, synthesize_go_ast_inventory_if_stmt_raw_tokens,
    synthesize_go_ast_inventory_import_spec_raw_tokens,
    synthesize_go_ast_inventory_inc_dec_stmt_raw_tokens,
    synthesize_go_ast_inventory_index_expr_return_raw_tokens,
    synthesize_go_ast_inventory_index_list_expr_return_raw_tokens,
    synthesize_go_ast_inventory_interface_type_var_raw_tokens,
    synthesize_go_ast_inventory_key_value_expr_return_raw_tokens,
    synthesize_go_ast_inventory_labeled_stmt_raw_tokens,
    synthesize_go_ast_inventory_paren_expr_return_raw_tokens,
    synthesize_go_ast_inventory_range_stmt_raw_tokens,
    synthesize_go_ast_inventory_registry_raw_tokens,
    synthesize_go_ast_inventory_selector_call_raw_tokens,
    synthesize_go_ast_inventory_send_stmt_raw_tokens,
    synthesize_go_ast_inventory_slice_expr_return_raw_tokens,
    synthesize_go_ast_inventory_star_expr_return_raw_tokens,
    synthesize_go_ast_inventory_struct_type_decl_raw_tokens,
    synthesize_go_ast_inventory_template_raw_tokens,
    synthesize_go_ast_inventory_type_assert_expr_return_raw_tokens,
    synthesize_go_ast_inventory_type_switch_stmt_raw_tokens,
    synthesize_go_ast_inventory_unary_expr_return_raw_tokens,
    synthesize_go_ast_inventory_var_decl_raw_tokens,
};
use tct::syntax_go_raw_tree_renderer::{
    GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_ARTIFACT_KIND,
    GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_STATUS_READY,
    GO_RAW_TREE_RENDERER_INVENTORY_SKELETON_SLICE_ID,
    GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND,
    GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY,
    GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_ID,
    go_raw_tree_renderer_generated_skeleton_dispatch_guard,
    go_raw_tree_renderer_generated_skeleton_slice_for_nodes,
    go_raw_tree_renderer_inventory_skeleton_slice, go_raw_tree_renderer_skeleton_dispatch_audit,
    go_raw_tree_renderer_skeleton_generation_contract,
    go_raw_tree_renderer_source_file_decl_skeleton_slice,
    go_raw_tree_renderer_source_file_decl_spec_generated_dispatch_guard,
    render_go_source_from_raw_tree,
};
use tct::syntax_go_scratch_seed::{
    GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_ELEMENT_TYPE,
    GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_RETURN_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_ROOT, GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_GO_VERSION, GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LHS_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_OPERATOR, GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RETURN_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_ROOT, GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_LEFT_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_LEFT_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_OPERATOR,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RIGHT_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RIGHT_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_ROOT, GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_ROOT, GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_TOKEN,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CASE_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CASE_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CONTRACT, GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_ROOT,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SWITCH_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SWITCH_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_DIRECTION,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ELEMENT_TYPE,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RETURN_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ROOT, GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RESULT_TYPE, GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_ROOT,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ELEMENT_TYPE,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ROOT, GO_AST_INVENTORY_SCRATCH_DECL_STMT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_DECL_STMT_DECL_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_DECL_STMT_GO_VERSION, GO_AST_INVENTORY_SCRATCH_DECL_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_DECL_STMT_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_DECL_STMT_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_DECL_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_DECL_STMT_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_DECL_STMT_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_DECL_STMT_RETURN_IDENTIFIER, GO_AST_INVENTORY_SCRATCH_DECL_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CALL_RECEIVER,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CALL_SELECTOR,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CONTRACT, GO_AST_INVENTORY_SCRATCH_DEFER_STMT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_DEFER_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ELEMENT_TYPE,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ROOT,
    GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_CONTRACT, GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IDENTIFIER, GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IMPLICIT,
    GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_RENDERED_SOURCE, GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CALL_RECEIVER,
    GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CALL_SELECTOR, GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_EXPR_STMT_GO_VERSION, GO_AST_INVENTORY_SCRATCH_EXPR_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_EXPR_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_EXPR_STMT_RENDERED_SOURCE, GO_AST_INVENTORY_SCRATCH_EXPR_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_FUNC_LIT_CONTRACT, GO_AST_INVENTORY_SCRATCH_FUNC_LIT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_FUNC_LIT_IDENTIFIER, GO_AST_INVENTORY_SCRATCH_FUNC_LIT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_FUNC_LIT_RENDERED_SOURCE, GO_AST_INVENTORY_SCRATCH_FUNC_LIT_ROOT,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_ROOT, GO_AST_INVENTORY_SCRATCH_GO_STMT_CALL_RECEIVER,
    GO_AST_INVENTORY_SCRATCH_GO_STMT_CALL_SELECTOR, GO_AST_INVENTORY_SCRATCH_GO_STMT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_GO_STMT_GO_VERSION, GO_AST_INVENTORY_SCRATCH_GO_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_GO_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_GO_STMT_RENDERED_SOURCE, GO_AST_INVENTORY_SCRATCH_GO_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_BODY_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_BODY_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_COND_IDENTIFIER, GO_AST_INVENTORY_SCRATCH_IF_STMT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_ELSE_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_ELSE_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_GO_VERSION, GO_AST_INVENTORY_SCRATCH_IF_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_OPERATOR, GO_AST_INVENTORY_SCRATCH_IF_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_RENDERED_SOURCE, GO_AST_INVENTORY_SCRATCH_IF_STMT_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_IF_STMT_ROOT, GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ALIAS,
    GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_CONTRACT, GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PATH_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PATH_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ROOT, GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_TOKEN, GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_GO_VERSION, GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_INDEX_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_INDEX_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RESULT_TYPE, GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_ROOT,
    GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_FIRST_INDEX_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_ROOT,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_SECOND_INDEX_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_ROOT, GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_TYPE,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_ROOT,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_TYPE,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_LABEL_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_LABELED_STMT_ROOT, GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_GO_VERSION, GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RESULT_TYPE, GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_ROOT,
    GO_AST_INVENTORY_SCRATCH_RANGE_STMT_CONTRACT, GO_AST_INVENTORY_SCRATCH_RANGE_STMT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_RANGE_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_RANGE_STMT_KEY_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_RANGE_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_RANGE_STMT_RENDERED_SOURCE, GO_AST_INVENTORY_SCRATCH_RANGE_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_RANGE_STMT_SOURCE_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_RANGE_STMT_TOKEN, GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RECEIVER,
    GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_ROOT, GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_SELECTOR,
    GO_AST_INVENTORY_SCRATCH_SEND_STMT_CHANNEL_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_SEND_STMT_CONTRACT, GO_AST_INVENTORY_SCRATCH_SEND_STMT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_SEND_STMT_IDENTIFIER, GO_AST_INVENTORY_SCRATCH_SEND_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_SEND_STMT_RENDERED_SOURCE, GO_AST_INVENTORY_SCRATCH_SEND_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_SEND_STMT_VALUE_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_CONTRACT, GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RESULT_TYPE, GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_ROOT,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_THREE_INDEX, GO_AST_INVENTORY_SCRATCH_STAR_EXPR_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_STAR_EXPR_GO_VERSION, GO_AST_INVENTORY_SCRATCH_STAR_EXPR_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_STAR_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RESULT_TYPE, GO_AST_INVENTORY_SCRATCH_STAR_EXPR_ROOT,
    GO_AST_INVENTORY_SCRATCH_STAR_EXPR_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_CONTRACT, GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_NAME,
    GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_TYPE,
    GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_ROOT, GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_TYPE_NAME,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ASSERTED_TYPE,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RESULT_TYPE,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ROOT,
    GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_CONTRACT,
    GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_ROOT,
    GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_CONTRACT, GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_IDENTIFIER, GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_OPERATOR,
    GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RENDERED_SOURCE,
    GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RESULT_TYPE, GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_ROOT,
    GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SCRATCH_VAR_DECL_CONTRACT, GO_AST_INVENTORY_SCRATCH_VAR_DECL_GO_VERSION,
    GO_AST_INVENTORY_SCRATCH_VAR_DECL_IDENTIFIER, GO_AST_INVENTORY_SCRATCH_VAR_DECL_LITERAL_KIND,
    GO_AST_INVENTORY_SCRATCH_VAR_DECL_LITERAL_VALUE,
    GO_AST_INVENTORY_SCRATCH_VAR_DECL_PACKAGE_NAME,
    GO_AST_INVENTORY_SCRATCH_VAR_DECL_RENDERED_SOURCE, GO_AST_INVENTORY_SCRATCH_VAR_DECL_ROOT,
    go_ast_inventory_scratch_array_type_return_raw_tokens,
    go_ast_inventory_scratch_array_type_return_seed,
    go_ast_inventory_scratch_assignment_raw_tokens, go_ast_inventory_scratch_assignment_seed,
    go_ast_inventory_scratch_binary_expr_raw_tokens, go_ast_inventory_scratch_binary_expr_seed,
    go_ast_inventory_scratch_branch_stmt_raw_tokens, go_ast_inventory_scratch_branch_stmt_seed,
    go_ast_inventory_scratch_case_clause_raw_tokens, go_ast_inventory_scratch_case_clause_seed,
    go_ast_inventory_scratch_chan_type_return_raw_tokens,
    go_ast_inventory_scratch_chan_type_return_seed,
    go_ast_inventory_scratch_comm_clause_raw_tokens, go_ast_inventory_scratch_comm_clause_seed,
    go_ast_inventory_scratch_composite_lit_raw_tokens, go_ast_inventory_scratch_composite_lit_seed,
    go_ast_inventory_scratch_decl_stmt_raw_tokens, go_ast_inventory_scratch_decl_stmt_seed,
    go_ast_inventory_scratch_defer_stmt_raw_tokens, go_ast_inventory_scratch_defer_stmt_seed,
    go_ast_inventory_scratch_ellipsis_array_literal_raw_tokens,
    go_ast_inventory_scratch_ellipsis_array_literal_seed,
    go_ast_inventory_scratch_empty_stmt_raw_tokens, go_ast_inventory_scratch_empty_stmt_seed,
    go_ast_inventory_scratch_expr_stmt_seed, go_ast_inventory_scratch_func_lit_raw_tokens,
    go_ast_inventory_scratch_func_lit_seed, go_ast_inventory_scratch_func_type_result_raw_tokens,
    go_ast_inventory_scratch_function_return_raw_tokens,
    go_ast_inventory_scratch_function_return_seed, go_ast_inventory_scratch_go_stmt_raw_tokens,
    go_ast_inventory_scratch_go_stmt_seed, go_ast_inventory_scratch_if_stmt_raw_tokens,
    go_ast_inventory_scratch_if_stmt_seed, go_ast_inventory_scratch_import_spec_raw_tokens,
    go_ast_inventory_scratch_import_spec_seed, go_ast_inventory_scratch_inc_dec_stmt_raw_tokens,
    go_ast_inventory_scratch_inc_dec_stmt_seed, go_ast_inventory_scratch_index_expr_raw_tokens,
    go_ast_inventory_scratch_index_expr_seed, go_ast_inventory_scratch_index_list_expr_raw_tokens,
    go_ast_inventory_scratch_index_list_expr_seed,
    go_ast_inventory_scratch_interface_type_raw_tokens,
    go_ast_inventory_scratch_interface_type_seed,
    go_ast_inventory_scratch_key_value_expr_raw_tokens,
    go_ast_inventory_scratch_key_value_expr_seed, go_ast_inventory_scratch_labeled_stmt_raw_tokens,
    go_ast_inventory_scratch_labeled_stmt_seed, go_ast_inventory_scratch_paren_expr_raw_tokens,
    go_ast_inventory_scratch_paren_expr_seed, go_ast_inventory_scratch_range_stmt_raw_tokens,
    go_ast_inventory_scratch_range_stmt_seed, go_ast_inventory_scratch_seeds,
    go_ast_inventory_scratch_selector_call_raw_tokens, go_ast_inventory_scratch_selector_call_seed,
    go_ast_inventory_scratch_send_stmt_raw_tokens, go_ast_inventory_scratch_send_stmt_seed,
    go_ast_inventory_scratch_slice_expr_raw_tokens, go_ast_inventory_scratch_slice_expr_seed,
    go_ast_inventory_scratch_star_expr_raw_tokens, go_ast_inventory_scratch_star_expr_seed,
    go_ast_inventory_scratch_struct_type_raw_tokens, go_ast_inventory_scratch_struct_type_seed,
    go_ast_inventory_scratch_type_assert_expr_raw_tokens,
    go_ast_inventory_scratch_type_assert_expr_seed,
    go_ast_inventory_scratch_type_switch_stmt_raw_tokens,
    go_ast_inventory_scratch_type_switch_stmt_seed, go_ast_inventory_scratch_unary_expr_raw_tokens,
    go_ast_inventory_scratch_unary_expr_seed, go_ast_inventory_scratch_var_decl_raw_tokens,
    go_ast_inventory_scratch_var_decl_seed, go_ast_inventory_scratch_witness_specs,
};
use tct::syntax_state_machine::{
    CompiledSyntaxModel, RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
    RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID, RawSyntaxDecodedValue, RawSyntaxStateMachineCore,
    RawSyntaxStatus,
};

const STALE_SOURCE_PROGRAM_SUCCESSOR_TASK: &str =
    "syntax backend source-program claim handoff gate";

#[test]
fn checked_go_ast_schema_inventory_lowers_to_structural_model() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");

    assert_eq!(grammar.provenance.source_kind, "go_ast_schema_inventory");
    assert_eq!(
        grammar.provenance.parser.as_ref().unwrap().version,
        GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION
    );

    let rule_names = grammar
        .rules
        .iter()
        .map(|rule| rule.name.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        grammar.rules.first().map(|rule| rule.name.as_str()),
        Some("File"),
        "GoSourceFile must be the lowered GrammarIR root for scratch generation"
    );
    for expected in [
        "File",
        "FuncDecl",
        "GenDecl",
        "BasicLit",
        "BinaryExpr",
        "AssignStmt",
        "RangeStmt",
        "ChanType",
        "CallExpr",
        "FuncType",
    ] {
        assert!(rule_names.contains(expected), "missing {expected} rule");
    }
    assert!(
        !rule_names.contains("BadExpr"),
        "parser-error placeholders must stay out of the valid-Go lowering slice"
    );
    for (rule, field) in [
        ("GenDecl", "Lparen"),
        ("GenDecl", "Rparen"),
        ("TypeSpec", "Assign"),
        ("CallExpr", "Ellipsis"),
        ("FuncType", "Func"),
    ] {
        let grammar_rule = grammar.rule(rule).expect("expected inventory rule");
        let grammar_field = grammar_rule
            .fields
            .iter()
            .find(|candidate| candidate.name == field)
            .unwrap_or_else(|| panic!("missing {rule}.{field} syntax-presence field"));
        assert!(
            !grammar_field.required,
            "{rule}.{field} should be optional syntax presence"
        );
        assert_eq!(grammar_field.alternatives.len(), 2);
    }

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.proof.grammar_shape.rules >= 40);
    assert!(sidecar.proof.grammar_shape.repeated_fields >= 10);
    assert!(sidecar.proof.grammar_shape.payload_terminals >= 20);

    let finite_domains = sidecar
        .structural_model
        .finite_domains
        .iter()
        .map(|domain| domain.id.as_str())
        .collect::<BTreeSet<_>>();
    for expected in [
        "grammar.rule.File.field.Decls.alternatives",
        "grammar.rule.GenDecl.field.Tok.alternatives",
        "grammar.rule.GenDecl.field.Lparen.alternatives",
        "grammar.rule.GenDecl.field.Rparen.alternatives",
        "grammar.rule.BasicLit.field.Kind.alternatives",
        "grammar.rule.BinaryExpr.field.Op.alternatives",
        "grammar.rule.ChanType.field.Dir.alternatives",
        "grammar.rule.TypeSpec.field.Assign.alternatives",
        "grammar.rule.CallExpr.field.Ellipsis.alternatives",
        "grammar.rule.FuncType.field.Func.alternatives",
    ] {
        assert!(
            finite_domains.contains(expected),
            "missing finite domain {expected}"
        );
    }

    let optional_fields = sidecar
        .structural_model
        .optionals
        .iter()
        .map(|optional| format!("{}.{}", optional.owner_rule, optional.field))
        .collect::<BTreeSet<_>>();
    for expected in [
        "GenDecl.Lparen",
        "GenDecl.Rparen",
        "TypeSpec.Assign",
        "CallExpr.Ellipsis",
        "FuncType.Func",
    ] {
        assert!(
            optional_fields.contains(expected),
            "missing optional syntax decision {expected}"
        );
    }

    let references = sidecar
        .structural_model
        .references
        .iter()
        .map(|reference| reference.target_node_kind.as_str())
        .collect::<BTreeSet<_>>();
    for expected in ["FuncDecl", "GenDecl", "FuncType", "BlockStmt", "Ident"] {
        assert!(
            references.contains(expected),
            "missing reference {expected}"
        );
    }
}

#[test]
#[ignore = "known failure: expects 13 synthesized trivia fields, the checked go1.24.3 inventory now carries 15 (File CommentBefore/NoBlankLineBefore); see docs/frontends-development.md"]
fn checked_go_ast_schema_inventory_covers_every_valid_ast_rule_and_field() {
    let inventory: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json"
    ))
    .expect("checked inventory fixture should parse");
    let node_structs = inventory["node_structs"]
        .as_array()
        .expect("checked inventory should contain node_structs");
    let grammar_rules = inventory["grammar_ir_rules"]
        .as_array()
        .expect("checked inventory should contain grammar_ir_rules");

    let tokenizable_nodes = node_structs
        .iter()
        .filter(|node| node["tokenizable"].as_bool() == Some(true))
        .map(|node| {
            node["name"]
                .as_str()
                .expect("tokenizable node should carry name")
        })
        .collect::<BTreeSet<_>>();
    let non_tokenizable_nodes = node_structs
        .iter()
        .filter(|node| node["tokenizable"].as_bool() != Some(true))
        .map(|node| {
            node["name"]
                .as_str()
                .expect("non-tokenizable node should carry name")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        non_tokenizable_nodes,
        ["BadDecl", "BadExpr", "BadStmt"]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        "only parser-error placeholders may stay out of the valid-Go grammar slice"
    );

    let rule_names = grammar_rules
        .iter()
        .map(|rule| {
            rule["name"]
                .as_str()
                .expect("grammar rule should carry name")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        rule_names, tokenizable_nodes,
        "every valid tokenizable go/ast node must have exactly one GrammarIR rule"
    );

    let tokenizable_field_count = node_structs
        .iter()
        .map(|node| {
            node["tokenizable_fields"]
                .as_array()
                .map_or(0, |fields| fields.len())
        })
        .sum::<usize>();
    // Grammar rules carry the go/ast tokenizable fields plus the synthesized
    // trailing comment/layout trivia metadata fields appended by the
    // inventory generator (see goAstSchemaSyntheticTriviaFields).
    let synthesized_trivia_field_count = grammar_rules
        .iter()
        .map(|rule| {
            rule["fields"]
                .as_array()
                .expect("grammar rule should carry fields")
                .iter()
                .filter(|field| {
                    field["policy"].as_str() == Some("synthesized_comment_trivia_metadata")
                })
                .count()
        })
        .sum::<usize>();
    assert_eq!(
        synthesized_trivia_field_count, 13,
        "the inventory grammar must carry the 13 synthesized trivia metadata fields"
    );
    let grammar_field_count = grammar_rules
        .iter()
        .map(|rule| {
            rule["fields"]
                .as_array()
                .expect("grammar rule should carry fields")
                .len()
        })
        .sum::<usize>();
    assert_eq!(
        grammar_field_count,
        tokenizable_field_count + synthesized_trivia_field_count,
        "the inventory-backed GrammarIR field surface must match tokenizable go/ast fields plus synthesized trivia metadata"
    );
    assert_eq!(
        inventory["coverage"]["tokenizable_node_count"].as_u64(),
        Some(tokenizable_nodes.len() as u64)
    );
    assert_eq!(
        inventory["coverage"]["grammar_ir_rule_count"].as_u64(),
        Some(rule_names.len() as u64)
    );
    assert_eq!(
        inventory["coverage"]["tokenizable_field_count"].as_u64(),
        Some(tokenizable_field_count as u64)
    );
}

#[test]
fn checked_go_ast_schema_inventory_exposes_renderer_position_policies() {
    let fields = GoAstSchemaInventoryAdapter::renderer_position_fields_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should expose renderer-position policies");

    assert_eq!(
        fields.len(),
        64,
        "checked Go 1.24.3 inventory should retain every token.Pos renderer-position field"
    );
    assert!(
        fields
            .iter()
            .all(|field| field.go_type.as_str() == "token.Pos"),
        "renderer-position metadata must remain token.Pos-only"
    );

    let categories = fields
        .iter()
        .map(|field| field.policy)
        .collect::<BTreeSet<_>>();
    for expected in [
        GoAstRendererPositionPolicy::BadNodeSourceRangeUnsupported,
        GoAstRendererPositionPolicy::CommentTriviaPositionSidecar,
        GoAstRendererPositionPolicy::ParserSourcePositionSidecar,
        GoAstRendererPositionPolicy::FixedSyntaxTokenRendererMetadata,
        GoAstRendererPositionPolicy::FiniteDomainTokenRendererMetadata,
        GoAstRendererPositionPolicy::SyntaxPresenceRendererMetadata,
    ] {
        assert!(
            categories.contains(&expected),
            "missing renderer-position policy category {}",
            expected.as_str()
        );
    }

    for (node, field, expected) in [
        (
            "ArrayType",
            "Lbrack",
            GoAstRendererPositionPolicy::FixedSyntaxTokenRendererMetadata,
        ),
        (
            "BlockStmt",
            "Lbrace",
            GoAstRendererPositionPolicy::FixedSyntaxTokenRendererMetadata,
        ),
        (
            "BranchStmt",
            "TokPos",
            GoAstRendererPositionPolicy::FiniteDomainTokenRendererMetadata,
        ),
        (
            "CaseClause",
            "Case",
            GoAstRendererPositionPolicy::SyntaxPresenceRendererMetadata,
        ),
        (
            "FieldList",
            "Opening",
            GoAstRendererPositionPolicy::SyntaxPresenceRendererMetadata,
        ),
        (
            "Ident",
            "NamePos",
            GoAstRendererPositionPolicy::ParserSourcePositionSidecar,
        ),
        (
            "ImportSpec",
            "EndPos",
            GoAstRendererPositionPolicy::ParserSourcePositionSidecar,
        ),
        (
            "ParenExpr",
            "Lparen",
            GoAstRendererPositionPolicy::FixedSyntaxTokenRendererMetadata,
        ),
        (
            "UnaryExpr",
            "OpPos",
            GoAstRendererPositionPolicy::FiniteDomainTokenRendererMetadata,
        ),
    ] {
        let actual = renderer_position_policy(&fields, node, field);
        assert_eq!(actual, expected, "unexpected policy for {node}.{field}");
    }
}

#[test]
fn checked_go_ast_schema_inventory_reaches_raw_state_machine_coverage() {
    let coverage = GoAstSchemaInventoryAdapter::grammar_coverage_file(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        go_syntax_token_layout(),
    )
    .expect("checked Go AST schema inventory should produce a grammar coverage report");

    coverage
        .validate()
        .expect("grammar coverage report should validate");
    assert_eq!(coverage.status, GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY);
    assert!(coverage.inventory_surface_checked);
    assert_eq!(
        coverage.parser_error_placeholders_excluded,
        vec![
            "BadDecl".to_string(),
            "BadExpr".to_string(),
            "BadStmt".to_string()
        ]
    );
    assert_eq!(
        coverage.tokenizable_node_count,
        coverage.grammar_ir_rule_count
    );
    assert_eq!(
        coverage.grammar_ir_rule_count,
        coverage.structural_model_record_count
    );
    assert_eq!(
        coverage.tokenizable_field_count,
        coverage.structural_model_field_count
    );
    assert!(coverage.structural_model_lowered);
    assert_eq!(
        coverage.renderer_position_policy_field_count,
        tct::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT
    );
    assert!(
        coverage
            .token_layout
            .multi_alternative_nonterminal_node_kind_count
            <= coverage.token_layout.alternative_node_start_capacity,
        "the current Go token layout should host the full AST-wide node-start surface"
    );
    assert!(coverage.raw_state_machine.raw_state_machine_compile_ready);
    assert!(coverage.raw_state_machine.initial_state_ready);
    assert!(
        coverage
            .raw_state_machine
            .initial_state_admissible_raw_token_count
            > 0
    );
    assert!(coverage.raw_state_machine.initial_state_contains_node_start);
    assert!(coverage.first_blocker.is_none());
    assert_eq!(
        coverage.next_ready_task,
        GO_AST_SCHEMA_GRAMMAR_COVERAGE_NEXT_READY_TASK
    );
    assert_ne!(
        coverage.next_ready_task,
        STALE_SOURCE_PROGRAM_SUCCESSOR_TASK
    );
}

#[test]
fn go_ast_inventory_empty_prefix_scratch_token_space_gate() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let core: &dyn RawSyntaxStateMachineCore = &model;
    assert_eq!(
        core.boundary_id(),
        RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID
    );
    assert_eq!(
        core.boundary_contract(),
        RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY
    );

    let roles = core.token_roles();
    let empty = core
        .state_after_raw_prefix(&[])
        .expect("empty GoSourceFile raw prefix should expose the initial state");
    assert_eq!(empty.status, RawSyntaxStatus::Partial);
    assert_eq!(empty.consumed_raw_tokens, 0);
    assert_eq!(empty.stack_depth, 0);
    assert!(!empty.eof_valid);
    assert!(empty.active_rule.is_none());
    assert!(empty.active_field.is_none());
    assert_eq!(
        empty.admissible.finite_raw_token_ids,
        vec![roles.node_start]
    );
    assert!(empty.admissible.raw_token_ranges.is_empty());
    assert!(!empty.admissible.payload_end_admissible);

    let root_transition = core.candidate_raw_token_transition(&[], roles.node_start);
    assert!(
        root_transition.accepted,
        "root node_start should be accepted from the empty GoSourceFile prefix: {:?}",
        root_transition.error
    );
    let root_state = root_transition
        .state
        .as_ref()
        .expect("accepted root transition should return a state");
    assert_eq!(root_state.status, RawSyntaxStatus::Partial);
    assert_eq!(root_state.consumed_raw_tokens, 1);
    assert_eq!(root_state.stack_depth, 1);
    assert_eq!(root_state.active_rule.as_deref(), Some("File"));
    assert!(root_state.active_field.is_none());
    assert!(!root_state.eof_valid);

    let premature_eof = core
        .strict_decode_complete(&[])
        .expect_err("strict scratch decode must reject an empty token stream");
    assert_eq!(premature_eof.kind, "premature_eof");
    assert_eq!(premature_eof.consumed_raw_tokens, 0);
    assert_eq!(premature_eof.failing_token, None);

    for (label, token, expected_kind) in [
        (
            "node_end before root",
            roles.node_end,
            "unexpected_structural_token",
        ),
        (
            "PAD_TOKEN in raw stream",
            roles.pad_token,
            "pad_token_in_raw_stream",
        ),
        (
            "token id at VOCAB_SIZE",
            roles.vocab_size,
            "token_id_out_of_vocab",
        ),
    ] {
        let rejected = core.candidate_raw_token_transition(&[], token);
        assert!(
            !rejected.accepted,
            "{label} should be rejected from the empty GoSourceFile prefix"
        );
        let error = rejected
            .error
            .as_ref()
            .expect("rejected transition should carry an error");
        assert_eq!(error.kind, expected_kind, "{label} error kind");
        assert_eq!(error.consumed_raw_tokens, 0, "{label} consumed count");
        assert_eq!(error.failing_token, Some(token), "{label} failing token");
    }
}

#[test]
fn go_ast_inventory_scratch_witness_registry_records_are_unique_and_stable() {
    let specs = go_ast_inventory_scratch_witness_specs();
    assert_eq!(specs.len(), 36);

    let expected_records = [
        (
            "go_ast_inventory_scratch_var_decl_seed",
            "package_var_declaration",
        ),
        (
            "go_ast_inventory_scratch_import_spec_seed",
            "import_spec_alias",
        ),
        (
            "go_ast_inventory_scratch_function_return_seed",
            "function_return_integer_literal",
        ),
        (
            "go_ast_inventory_scratch_func_type_result_seed",
            "function_type_result",
        ),
        (
            "go_ast_inventory_scratch_array_type_return_seed",
            "function_return_array_type",
        ),
        (
            "go_ast_inventory_scratch_chan_type_return_seed",
            "function_return_channel_type",
        ),
        (
            "go_ast_inventory_scratch_selector_call_seed",
            "selector_call_member_access_expression",
        ),
        (
            "go_ast_inventory_scratch_index_expr_seed",
            "index_expression",
        ),
        (
            "go_ast_inventory_scratch_index_list_expr_seed",
            "index_list_expression",
        ),
        (
            "go_ast_inventory_scratch_slice_expr_seed",
            "slice_expression",
        ),
        ("go_ast_inventory_scratch_star_expr_seed", "star_expression"),
        ("go_ast_inventory_scratch_struct_type_seed", "struct_type"),
        (
            "go_ast_inventory_scratch_type_assert_expr_seed",
            "type_assertion_expression",
        ),
        (
            "go_ast_inventory_scratch_interface_type_seed",
            "interface_type",
        ),
        (
            "go_ast_inventory_scratch_key_value_expr_seed",
            "key_value_expression",
        ),
        (
            "go_ast_inventory_scratch_assignment_seed",
            "assignment_statement",
        ),
        (
            "go_ast_inventory_scratch_binary_expr_seed",
            "binary_expression",
        ),
        (
            "go_ast_inventory_scratch_unary_expr_seed",
            "unary_expression",
        ),
        (
            "go_ast_inventory_scratch_paren_expr_seed",
            "parenthesized_expression",
        ),
        (
            "go_ast_inventory_scratch_composite_lit_seed",
            "composite_literal",
        ),
        (
            "go_ast_inventory_scratch_ellipsis_array_literal_seed",
            "array_literal_ellipsis_type",
        ),
        (
            "go_ast_inventory_scratch_func_lit_seed",
            "function_literal_expression",
        ),
        (
            "go_ast_inventory_scratch_decl_stmt_seed",
            "declaration_statement",
        ),
        (
            "go_ast_inventory_scratch_defer_stmt_seed",
            "defer_statement",
        ),
        ("go_ast_inventory_scratch_go_stmt_seed", "go_statement"),
        ("go_ast_inventory_scratch_if_stmt_seed", "if_statement"),
        (
            "go_ast_inventory_scratch_empty_stmt_seed",
            "empty_statement",
        ),
        (
            "go_ast_inventory_scratch_expr_stmt_seed",
            "expression_statement",
        ),
        (
            "go_ast_inventory_scratch_inc_dec_stmt_seed",
            "increment_decrement_statement",
        ),
        (
            "go_ast_inventory_scratch_branch_stmt_seed",
            "branch_statement",
        ),
        (
            "go_ast_inventory_scratch_range_stmt_seed",
            "range_statement",
        ),
        ("go_ast_inventory_scratch_send_stmt_seed", "send_statement"),
        (
            "go_ast_inventory_scratch_labeled_stmt_seed",
            "labeled_statement",
        ),
        ("go_ast_inventory_scratch_case_clause_seed", "case_clause"),
        (
            "go_ast_inventory_scratch_type_switch_stmt_seed",
            "type_switch_statement",
        ),
        ("go_ast_inventory_scratch_comm_clause_seed", "comm_clause"),
    ];
    let actual_records = specs
        .iter()
        .map(|spec| (spec.seed_id, spec.grammar_category))
        .collect::<Vec<_>>();
    assert_eq!(actual_records.as_slice(), expected_records.as_slice());

    let mut seed_ids = BTreeSet::new();
    let mut category_records = BTreeSet::new();
    for spec in specs {
        assert!(
            seed_ids.insert(spec.seed_id),
            "duplicate Go scratch witness id {}",
            spec.seed_id
        );
        assert!(
            category_records.insert((spec.grammar_category, spec.root)),
            "duplicate Go scratch witness category/root record {} {}",
            spec.grammar_category,
            spec.root
        );
    }
    let inventory_synthesis_specs = specs
        .iter()
        .filter(|spec| spec.inventory_synthesis_api_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(inventory_synthesis_specs.len(), 35);
    assert_eq!(
        inventory_synthesis_specs
            .iter()
            .map(|spec| spec.seed_id)
            .collect::<Vec<_>>(),
        vec![
            "go_ast_inventory_scratch_var_decl_seed",
            "go_ast_inventory_scratch_import_spec_seed",
            "go_ast_inventory_scratch_function_return_seed",
            "go_ast_inventory_scratch_func_type_result_seed",
            "go_ast_inventory_scratch_array_type_return_seed",
            "go_ast_inventory_scratch_chan_type_return_seed",
            "go_ast_inventory_scratch_selector_call_seed",
            "go_ast_inventory_scratch_index_expr_seed",
            "go_ast_inventory_scratch_index_list_expr_seed",
            "go_ast_inventory_scratch_slice_expr_seed",
            "go_ast_inventory_scratch_star_expr_seed",
            "go_ast_inventory_scratch_struct_type_seed",
            "go_ast_inventory_scratch_type_assert_expr_seed",
            "go_ast_inventory_scratch_interface_type_seed",
            "go_ast_inventory_scratch_key_value_expr_seed",
            "go_ast_inventory_scratch_assignment_seed",
            "go_ast_inventory_scratch_binary_expr_seed",
            "go_ast_inventory_scratch_unary_expr_seed",
            "go_ast_inventory_scratch_paren_expr_seed",
            "go_ast_inventory_scratch_composite_lit_seed",
            "go_ast_inventory_scratch_ellipsis_array_literal_seed",
            "go_ast_inventory_scratch_func_lit_seed",
            "go_ast_inventory_scratch_decl_stmt_seed",
            "go_ast_inventory_scratch_defer_stmt_seed",
            "go_ast_inventory_scratch_go_stmt_seed",
            "go_ast_inventory_scratch_if_stmt_seed",
            "go_ast_inventory_scratch_empty_stmt_seed",
            "go_ast_inventory_scratch_inc_dec_stmt_seed",
            "go_ast_inventory_scratch_branch_stmt_seed",
            "go_ast_inventory_scratch_range_stmt_seed",
            "go_ast_inventory_scratch_send_stmt_seed",
            "go_ast_inventory_scratch_labeled_stmt_seed",
            "go_ast_inventory_scratch_case_clause_seed",
            "go_ast_inventory_scratch_type_switch_stmt_seed",
            "go_ast_inventory_scratch_comm_clause_seed",
        ]
    );
    assert!(inventory_synthesis_specs.iter().all(|spec| {
        spec.inventory_synthesis_core_api_id == Some(GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID)
    }));
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_function_return_seed")
            .expect("function-return witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_func_type_result_seed")
            .expect("function-type result witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_array_type_return_seed")
            .expect("array-type return witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_chan_type_return_seed")
            .expect("channel-type return witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_selector_call_seed")
            .expect("selector-call witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_binary_expr_seed")
            .expect("binary-expression witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_composite_lit_seed")
            .expect("composite-literal witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_ellipsis_array_literal_seed")
            .expect("ellipsis array-literal witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_func_lit_seed")
            .expect("function-literal witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_index_expr_seed")
            .expect("index-expression witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_index_list_expr_seed")
            .expect("index-list-expression witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_interface_type_seed")
            .expect("interface-type witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_slice_expr_seed")
            .expect("slice-expression witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_star_expr_seed")
            .expect("star-expression witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_struct_type_seed")
            .expect("struct-type witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_type_assert_expr_seed")
            .expect("type-assert-expression witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_unary_expr_seed")
            .expect("unary-expression witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_assignment_seed")
            .expect("assignment witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_branch_stmt_seed")
            .expect("BranchStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_range_stmt_seed")
            .expect("RangeStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_send_stmt_seed")
            .expect("SendStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_labeled_stmt_seed")
            .expect("LabeledStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_case_clause_seed")
            .expect("CaseClause witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_type_switch_stmt_seed")
            .expect("TypeSwitchStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_comm_clause_seed")
            .expect("CommClause witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_defer_stmt_seed")
            .expect("DeferStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_go_stmt_seed")
            .expect("GoStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_empty_stmt_seed")
            .expect("EmptyStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.seed_id == "go_ast_inventory_scratch_inc_dec_stmt_seed")
            .expect("IncDecStmt witness remains registered")
            .inventory_synthesis_api_id,
        Some(GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        specs
            .iter()
            .find(|spec| spec.inventory_synthesis_api_id.is_none())
            .expect("registry should still record the next synthesis frontier")
            .seed_id,
        "go_ast_inventory_scratch_expr_stmt_seed"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_reachability_stays_metadata_driven() {
    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let registry_api_ids = registry
        .entries()
        .iter()
        .map(|entry| entry.api_id.to_string())
        .collect::<BTreeSet<_>>();

    let scratch_specs = go_ast_inventory_scratch_witness_specs();
    let scratch_registry_api_ids = scratch_specs
        .iter()
        .filter_map(|spec| spec.inventory_synthesis_api_id)
        .filter(|api_id| registry_api_ids.contains(*api_id))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let missing_registry_api_ids = registry_api_ids
        .difference(&scratch_registry_api_ids)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        missing_registry_api_ids.is_empty(),
        "first synthesis registry entry without a checked scratch seed: {:?}",
        missing_registry_api_ids.first()
    );

    let non_registry_inventory_scratch_records = scratch_specs
        .iter()
        .filter_map(|spec| {
            spec.inventory_synthesis_api_id
                .map(|api_id| (spec.seed_id, api_id))
        })
        .filter(|(_, api_id)| !registry_api_ids.contains(*api_id))
        .map(|(seed_id, api_id)| format!("{seed_id}:{api_id}"))
        .collect::<Vec<_>>();
    let expected_non_registry_inventory_scratch_records = vec![format!(
        "go_ast_inventory_scratch_var_decl_seed:{}",
        GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID
    )];
    assert_eq!(
        non_registry_inventory_scratch_records,
        expected_non_registry_inventory_scratch_records,
        "first inventory-backed scratch seed outside the synthesis registry: {:?}",
        non_registry_inventory_scratch_records.first()
    );

    let non_declarative_scratch_seed_ids = scratch_specs
        .iter()
        .filter(|spec| spec.inventory_synthesis_api_id.is_none())
        .map(|spec| spec.seed_id)
        .collect::<Vec<_>>();
    assert_eq!(
        non_declarative_scratch_seed_ids,
        vec!["go_ast_inventory_scratch_expr_stmt_seed"],
        "first scratch seed still outside inventory synthesis: {:?}",
        non_declarative_scratch_seed_ids.first()
    );

    let declarative_specs = go_ast_inventory_declarative_template_coverage_specs();
    let mut declarative_api_ids = BTreeSet::new();
    for spec in &declarative_specs {
        assert!(
            registry_api_ids.contains(spec.synthesis_api_id.as_str()),
            "declarative coverage spec names unregistered synthesis API {}",
            spec.synthesis_api_id
        );
        assert!(
            declarative_api_ids.insert(spec.synthesis_api_id.clone()),
            "duplicate declarative coverage spec for synthesis API {}",
            spec.synthesis_api_id
        );
    }
    let non_declarative_registry_api_ids = registry_api_ids
        .difference(&declarative_api_ids)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        non_declarative_registry_api_ids,
        vec![
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID.to_string(),
            GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID.to_string(),
            GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID.to_string(),
        ],
        "first registered synthesis API without declarative coverage metadata: {:?}",
        non_declarative_registry_api_ids.first()
    );
}

#[test]
#[ignore = "known failure: expected File field order lacks CommentBefore/NoBlankLineBefore, which the checked go1.24.3 inventory now carries; see docs/frontends-development.md"]
fn go_ast_inventory_var_decl_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_var_decl_raw_tokens(roles)
        .expect("inventory-driven var-decl synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_var_decl_raw_tokens(roles)
        .expect("registered var-decl witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_IDENTIFIER
    );
    assert_eq!(synthesized.payload_spec.declaration_token, "var");
    assert_eq!(
        synthesized.payload_spec.literal_kind,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.literal_value,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_LITERAL_VALUE
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "File")
            .expect("File field order should be checked")
            .fields,
        vec![
            "Doc".to_string(),
            "Name".to_string(),
            "Decls".to_string(),
            "Comments".to_string(),
            "GoVersion".to_string(),
        ]
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| decision.selected_alternative == "go_token_declaration_tokens.VAR")
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| decision.selected_alternative == "go_token_basic_literal_kinds.INT")
    );
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered var-decl witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_assignment_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_assignment_raw_tokens(roles)
        .expect("inventory-driven assignment synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_assignment_raw_tokens(roles)
        .expect("registered assignment witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.lhs_identifier,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LHS_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.operator,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_OPERATOR
    );
    assert_eq!(
        synthesized.payload_spec.literal_kind,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.literal_value,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.return_identifier,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RETURN_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
        ("AssignStmt", vec!["Lhs", "Tok", "Rhs"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("BlockStmt", "List", "AssignStmt"),
        ("AssignStmt", "Lhs", "Ident"),
        ("AssignStmt", "Tok", "go_token_assignment_tokens.ASSIGN"),
        ("AssignStmt", "Rhs", "BasicLit"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
        ("ReturnStmt", "Results", "Ident"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "assignment synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("assignment synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("assignment synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("AssignStmt"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("BasicLit"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("ReturnStmt"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 5);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("assignment synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered assignment witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_empty_stmt_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_empty_stmt_raw_tokens(roles)
        .expect("inventory-driven EmptyStmt synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_empty_stmt_raw_tokens(roles)
        .expect("registered EmptyStmt witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.implicit,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IMPLICIT
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("EmptyStmt", vec!["Implicit"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        ),
        (
            "BlockStmt",
            "List",
            "EmptyStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "EmptyStmt",
            "Implicit",
            "EmptyStmt.Implicit.bool",
            GrammarTerminalKind::BooleanPayload,
        ),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                        && decision.terminal == terminal_kind
                }),
            "EmptyStmt synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("EmptyStmt synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("EmptyStmt synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("EmptyStmt"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("EmptyStmt synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered EmptyStmt witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_inc_dec_stmt_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_inc_dec_stmt_raw_tokens(roles)
        .expect("inventory-driven IncDecStmt synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_inc_dec_stmt_raw_tokens(roles)
        .expect("registered IncDecStmt witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.target_identifier,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_TARGET_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.token,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_TOKEN
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("IncDecStmt", vec!["X", "Tok"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        ),
        (
            "BlockStmt",
            "List",
            "IncDecStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        ("IncDecStmt", "X", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "IncDecStmt",
            "Tok",
            "go_token_inc_dec_tokens.INC",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                        && decision.terminal == terminal_kind
                }),
            "IncDecStmt synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("IncDecStmt synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("IncDecStmt synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("IncDecStmt"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("IncDecStmt synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered IncDecStmt witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_assignment() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID)
        .expect("AssignStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListReturn
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_assignment_raw_tokens(roles)
        .expect("per-construct assignment shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_assignment_raw_tokens(roles)
        .expect("registered assignment witness should emit raw tokens through synthesizer");

    assert_eq!(generic.api_id, GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID);
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("operator")
            .expect("merged registry payload should carry the operator default"),
        "="
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let operator_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("operator", "+="),
    )
    .expect_err("registry payload rules should reject a non-v1 assignment operator");
    assert_eq!(
        operator_error.field(),
        "go_ast_inventory_assignment_synthesis.payload_spec.operator"
    );

    let unknown_api_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        "go-ast-inventory-unknown-synthesis-v1",
        GoAstInventoryRegistryPayload::new(),
    )
    .expect_err("registry should reject an unknown synthesis api id");
    assert_eq!(
        unknown_api_error.field(),
        "go_ast_inventory_synthesis_registry.api_id"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_branch_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID)
        .expect("BranchStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListReturn
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit BranchStmt raw tokens");
    let shim = synthesize_go_ast_inventory_branch_stmt_raw_tokens(roles)
        .expect("per-construct BranchStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_branch_stmt_raw_tokens(roles)
        .expect("registered BranchStmt witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("branch_token")
            .expect("merged registry payload should carry the branch token default"),
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_TOKEN
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct BranchStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered BranchStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    assert!(
        generic
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| decision.rule_name == "BranchStmt"
                && decision.field_name == "Tok"
                && decision.selected_alternative == "go_token_branch_tokens.BREAK"
                && decision.terminal == GrammarTerminalKind::IdentifierPayload),
        "BranchStmt synthesis should validate the pinned break token alternative"
    );

    let token_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("branch_token", "continue"),
    )
    .expect_err("registry payload rules should reject a non-v1 branch token");
    assert_eq!(
        token_error.field(),
        "go_ast_inventory_branch_stmt_synthesis.payload_spec.branch_token"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_range_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID)
        .expect("RangeStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListNoResult
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit RangeStmt raw tokens");
    let shim = synthesize_go_ast_inventory_range_stmt_raw_tokens(roles)
        .expect("per-construct RangeStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_range_stmt_raw_tokens(roles)
        .expect("registered RangeStmt witness should emit raw tokens through synthesizer");

    assert_eq!(generic.api_id, GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("key_identifier")
            .expect("merged registry payload should carry the key default"),
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_KEY_IDENTIFIER
    );
    assert_eq!(
        generic
            .payload
            .str_value("token")
            .expect("merged registry payload should carry the range token default"),
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_TOKEN
    );
    assert_eq!(
        generic
            .payload
            .str_value("source_identifier")
            .expect("merged registry payload should carry the range source default"),
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_SOURCE_IDENTIFIER
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct RangeStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered RangeStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "RangeStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "RangeStmt",
            "Key",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "RangeStmt",
            "Tok",
            "go_token_range_assignment_tokens.DEFINE",
            GrammarTerminalKind::IdentifierPayload,
        ),
        ("RangeStmt", "X", "Ident", GrammarTerminalKind::Nonterminal),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == terminal_kind),
            "RangeStmt synthesis should validate finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let token_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("token", "="),
    )
    .expect_err("registry payload rules should reject a non-v1 range token");
    assert_eq!(
        token_error.field(),
        "go_ast_inventory_range_stmt_synthesis.payload_spec.token"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_type_switch_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID)
        .expect("TypeSwitchStmt registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListNoResult
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit TypeSwitchStmt raw tokens");
    let shim = synthesize_go_ast_inventory_type_switch_stmt_raw_tokens(roles)
        .expect("per-construct TypeSwitchStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_type_switch_stmt_raw_tokens(roles)
        .expect("registered TypeSwitchStmt witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("target_identifier")
            .expect("merged registry payload should carry the target default"),
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_TARGET_IDENTIFIER
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct TypeSwitchStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered TypeSwitchStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "TypeSwitchStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "TypeSwitchStmt",
            "Assign",
            "ExprStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ExprStmt",
            "X",
            "TypeAssertExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "TypeAssertExpr",
            "X",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == terminal_kind),
            "TypeSwitchStmt synthesis should validate finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let target_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("target_identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty type-switch target");
    assert_eq!(
        target_error.field(),
        "go_ast_inventory_type_switch_stmt_synthesis.payload_spec.target_identifier"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_send_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID)
        .expect("SendStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListNoResult
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit SendStmt raw tokens");
    let shim = synthesize_go_ast_inventory_send_stmt_raw_tokens(roles)
        .expect("per-construct SendStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_send_stmt_raw_tokens(roles)
        .expect("registered SendStmt witness should emit raw tokens through synthesizer");

    assert_eq!(generic.api_id, GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("channel_identifier")
            .expect("merged registry payload should carry the channel default"),
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_CHANNEL_IDENTIFIER
    );
    assert_eq!(
        generic
            .payload
            .str_value("value_identifier")
            .expect("merged registry payload should carry the value default"),
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_VALUE_IDENTIFIER
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct SendStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered SendStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    assert!(
        generic
            .validated_field_orders
            .iter()
            .any(|order| order.rule_name == "SendStmt"
                && order.fields == vec!["Chan".to_string(), "Value".to_string()]),
        "SendStmt synthesis should validate the checked Chan/Value field order"
    );
    for (rule_name, field_name, selected_alternative) in [
        ("BlockStmt", "List", "SendStmt"),
        ("SendStmt", "Chan", "Ident"),
        ("SendStmt", "Value", "Ident"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == GrammarTerminalKind::Nonterminal),
            "SendStmt synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let channel_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("channel_identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty SendStmt channel identifier");
    assert_eq!(
        channel_error.field(),
        "go_ast_inventory_send_stmt_synthesis.payload_spec.channel_identifier"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_labeled_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID)
        .expect("LabeledStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementList
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit LabeledStmt raw tokens");
    let shim = synthesize_go_ast_inventory_labeled_stmt_raw_tokens(roles)
        .expect("per-construct LabeledStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_labeled_stmt_raw_tokens(roles)
        .expect("registered LabeledStmt witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("label_identifier")
            .expect("merged registry payload should carry the label default"),
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_LABEL_IDENTIFIER
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct LabeledStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered LabeledStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    assert!(
        generic
            .validated_field_orders
            .iter()
            .any(|order| order.rule_name == "LabeledStmt"
                && order.fields == vec!["Label".to_string(), "Stmt".to_string()]),
        "LabeledStmt synthesis should validate the checked Label/Stmt field order"
    );
    assert!(
        generic
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| decision.rule_name == "LabeledStmt"
                && decision.field_name == "Stmt"
                && decision.selected_alternative == "ReturnStmt"
                && decision.terminal == GrammarTerminalKind::Nonterminal),
        "LabeledStmt synthesis should validate the pinned ReturnStmt body alternative"
    );

    let label_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("label_identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty LabeledStmt label");
    assert_eq!(
        label_error.field(),
        "go_ast_inventory_labeled_stmt_synthesis.payload_spec.label_identifier"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_case_clause() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID)
        .expect("CaseClause registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListReturn
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit CaseClause raw tokens");
    let shim = synthesize_go_ast_inventory_case_clause_raw_tokens(roles)
        .expect("per-construct CaseClause shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_case_clause_raw_tokens(roles)
        .expect("registered CaseClause witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID
    );
    for (key, expected) in [
        (
            "switch_literal_kind",
            GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SWITCH_LITERAL_KIND,
        ),
        (
            "case_literal_kind",
            GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CASE_LITERAL_KIND,
        ),
        (
            "return_literal_kind",
            GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RETURN_LITERAL_KIND,
        ),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(key)
                .expect("merged registry payload should carry CaseClause literal defaults"),
            expected
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct CaseClause shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered CaseClause scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    for (rule_name, field_name, selected_alternative) in [
        ("BlockStmt", "List", "SwitchStmt"),
        ("SwitchStmt", "Tag", "BasicLit"),
        ("CaseClause", "List", "BasicLit"),
        ("CaseClause", "Body", "ReturnStmt"),
        ("ReturnStmt", "Results", "BasicLit"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == GrammarTerminalKind::Nonterminal),
            "CaseClause synthesis should validate finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let literal_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("case_literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject a non-v1 case literal kind");
    assert_eq!(
        literal_error.field(),
        "go_ast_inventory_case_clause_synthesis.payload_spec.case_literal_kind"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_comm_clause() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID)
        .expect("CommClause registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementList
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit CommClause raw tokens");
    let shim = synthesize_go_ast_inventory_comm_clause_raw_tokens(roles)
        .expect("per-construct CommClause shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_comm_clause_raw_tokens(roles)
        .expect("registered CommClause witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID
    );
    for (key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_PACKAGE_NAME,
        ),
        (
            "identifier",
            GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_IDENTIFIER,
        ),
        (
            "result_type",
            GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RESULT_TYPE,
        ),
        (
            "go_version",
            GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_GO_VERSION,
        ),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {key}")),
            expected
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct CommClause shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered CommClause scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    for (rule_name, fields) in [
        ("SelectStmt", vec!["Body"]),
        ("CommentGroup", vec!["List"]),
        ("Comment", vec!["Text"]),
        (
            "CommClause",
            vec![
                "Comm",
                "Body",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        ("Field", "Type", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "BlockStmt",
            "List",
            "SelectStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BlockStmt",
            "List",
            "CommClause",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == terminal_kind),
            "CommClause synthesis should validate finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic CommClause registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic CommClause registry path should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("SelectStmt", 1),
        ("CommClause", 1),
        ("CommentGroup", 1),
        ("Comment", 1),
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(node_kind),
            count,
            "CommClause registry path should contain {count} {node_kind} node(s)"
        );
    }
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic CommClause registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);

    let identifier_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty CommClause identifier");
    assert_eq!(
        identifier_error.field(),
        "go_ast_inventory_comm_clause_synthesis.payload_spec.identifier"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_decl_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID)
        .expect("DeclStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListReturn
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit DeclStmt raw tokens");
    let shim = synthesize_go_ast_inventory_decl_stmt_raw_tokens(roles)
        .expect("per-construct DeclStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_decl_stmt_raw_tokens(roles)
        .expect("registered DeclStmt witness should emit raw tokens through synthesizer");

    assert_eq!(generic.api_id, GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID
    );
    for (key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_DECL_STMT_PACKAGE_NAME,
        ),
        ("identifier", GO_AST_INVENTORY_SCRATCH_DECL_STMT_IDENTIFIER),
        (
            "result_type",
            GO_AST_INVENTORY_SCRATCH_DECL_STMT_RESULT_TYPE,
        ),
        (
            "decl_identifier",
            GO_AST_INVENTORY_SCRATCH_DECL_STMT_DECL_IDENTIFIER,
        ),
        (
            "literal_kind",
            GO_AST_INVENTORY_SCRATCH_DECL_STMT_LITERAL_KIND,
        ),
        (
            "literal_value",
            GO_AST_INVENTORY_SCRATCH_DECL_STMT_LITERAL_VALUE,
        ),
        (
            "return_identifier",
            GO_AST_INVENTORY_SCRATCH_DECL_STMT_RETURN_IDENTIFIER,
        ),
        ("go_version", GO_AST_INVENTORY_SCRATCH_DECL_STMT_GO_VERSION),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {key}")),
            expected
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct DeclStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered DeclStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    for (rule_name, fields) in [
        ("DeclStmt", vec!["Decl"]),
        ("GenDecl", vec!["Doc", "Tok", "Lparen", "Specs", "Rparen"]),
        (
            "ValueSpec",
            vec!["Doc", "Names", "Type", "Values", "Comment"],
        ),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        ("Field", "Type", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "BlockStmt",
            "List",
            "DeclStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "DeclStmt",
            "Decl",
            "GenDecl",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "GenDecl",
            "Tok",
            "go_token_declaration_tokens.VAR",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "GenDecl",
            "Specs",
            "ValueSpec",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ValueSpec",
            "Values",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "ReturnStmt",
            "Results",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == terminal_kind),
            "DeclStmt synthesis should validate finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic DeclStmt registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic DeclStmt registry path should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("DeclStmt", 1),
        ("GenDecl", 1),
        ("ValueSpec", 1),
        ("BasicLit", 1),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(node_kind),
            count,
            "DeclStmt registry path should contain {count} {node_kind} node(s)"
        );
    }
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic DeclStmt registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);

    let literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject a non-v1 DeclStmt literal kind");
    assert_eq!(
        literal_kind_error.field(),
        "go_ast_inventory_decl_stmt_synthesis.payload_spec.literal_kind"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_defer_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID)
        .expect("DeferStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListReturn
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit DeferStmt raw tokens");
    let shim = synthesize_go_ast_inventory_defer_stmt_raw_tokens(roles)
        .expect("per-construct DeferStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_defer_stmt_raw_tokens(roles)
        .expect("registered DeferStmt witness should emit raw tokens through synthesizer");

    assert_eq!(generic.api_id, GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID
    );
    for (key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_DEFER_STMT_PACKAGE_NAME,
        ),
        ("identifier", GO_AST_INVENTORY_SCRATCH_DEFER_STMT_IDENTIFIER),
        (
            "result_type",
            GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RESULT_TYPE,
        ),
        (
            "call_receiver",
            GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CALL_RECEIVER,
        ),
        (
            "call_selector",
            GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CALL_SELECTOR,
        ),
        (
            "return_literal_kind",
            GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RETURN_LITERAL_KIND,
        ),
        (
            "return_literal_value",
            GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RETURN_LITERAL_VALUE,
        ),
        ("go_version", GO_AST_INVENTORY_SCRATCH_DEFER_STMT_GO_VERSION),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {key}")),
            expected
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct DeferStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered DeferStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    for (rule_name, fields) in [
        ("DeferStmt", vec!["Call"]),
        (
            "CallExpr",
            vec!["Fun", "Args", "Ellipsis", "LineBreakBefore"],
        ),
        ("SelectorExpr", vec!["X", "Sel"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "DeferStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Fun",
            "SelectorExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ReturnStmt",
            "Results",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == terminal_kind),
            "DeferStmt synthesis should validate finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic DeferStmt registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic DeferStmt registry path should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("DeferStmt", 1),
        ("CallExpr", 1),
        ("SelectorExpr", 1),
        ("BasicLit", 1),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(node_kind),
            count,
            "DeferStmt registry path should contain {count} {node_kind} node(s)"
        );
    }
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic DeferStmt registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);

    let receiver_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("call_receiver", ""),
    )
    .expect_err("registry payload rules should reject an empty DeferStmt receiver");
    assert_eq!(
        receiver_error.field(),
        "go_ast_inventory_defer_stmt_synthesis.payload_spec.call_receiver"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_go_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID)
        .expect("GoStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementListNoResult
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit GoStmt raw tokens");
    let shim = synthesize_go_ast_inventory_go_stmt_raw_tokens(roles)
        .expect("per-construct GoStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_go_stmt_raw_tokens(roles)
        .expect("registered GoStmt witness should emit raw tokens through synthesizer");

    assert_eq!(generic.api_id, GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID
    );
    for (key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_GO_STMT_PACKAGE_NAME,
        ),
        ("identifier", GO_AST_INVENTORY_SCRATCH_GO_STMT_IDENTIFIER),
        (
            "call_receiver",
            GO_AST_INVENTORY_SCRATCH_GO_STMT_CALL_RECEIVER,
        ),
        (
            "call_selector",
            GO_AST_INVENTORY_SCRATCH_GO_STMT_CALL_SELECTOR,
        ),
        ("go_version", GO_AST_INVENTORY_SCRATCH_GO_STMT_GO_VERSION),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {key}")),
            expected
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct GoStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered GoStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    for (rule_name, fields) in [
        ("GoStmt", vec!["Call"]),
        (
            "CallExpr",
            vec!["Fun", "Args", "Ellipsis", "LineBreakBefore"],
        ),
        ("SelectorExpr", vec!["X", "Sel"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "GoStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Fun",
            "SelectorExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Ellipsis",
            "go_syntax_presence_call_expr_ellipsis.ABSENT",
            GrammarTerminalKind::BooleanPayload,
        ),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == terminal_kind),
            "GoStmt synthesis should validate finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic GoStmt registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic GoStmt registry path should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("GoStmt", 1),
        ("CallExpr", 1),
        ("SelectorExpr", 1),
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(node_kind),
            count,
            "GoStmt registry path should contain {count} {node_kind} node(s)"
        );
    }
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic GoStmt registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);

    let receiver_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("call_receiver", ""),
    )
    .expect_err("registry payload rules should reject an empty GoStmt receiver");
    assert_eq!(
        receiver_error.field(),
        "go_ast_inventory_go_stmt_synthesis.payload_spec.call_receiver"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_if_stmt() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID)
        .expect("IfStmt registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        entry.shell,
        GoAstInventorySynthesisShell::FunctionStatementList
    );

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with entry defaults should emit IfStmt raw tokens");
    let shim = synthesize_go_ast_inventory_if_stmt_raw_tokens(roles)
        .expect("per-construct IfStmt shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_if_stmt_raw_tokens(roles)
        .expect("registered IfStmt witness should emit raw tokens through synthesizer");

    assert_eq!(generic.api_id, GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID);
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID
    );
    for (key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_PACKAGE_NAME,
        ),
        ("identifier", GO_AST_INVENTORY_SCRATCH_IF_STMT_IDENTIFIER),
        ("result_type", GO_AST_INVENTORY_SCRATCH_IF_STMT_RESULT_TYPE),
        (
            "init_identifier",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_IDENTIFIER,
        ),
        (
            "init_operator",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_OPERATOR,
        ),
        (
            "init_literal_kind",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_LITERAL_KIND,
        ),
        (
            "init_literal_value",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_LITERAL_VALUE,
        ),
        (
            "cond_identifier",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_COND_IDENTIFIER,
        ),
        (
            "body_return_literal_kind",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_BODY_RETURN_LITERAL_KIND,
        ),
        (
            "body_return_literal_value",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_BODY_RETURN_LITERAL_VALUE,
        ),
        (
            "else_return_literal_kind",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_ELSE_RETURN_LITERAL_KIND,
        ),
        (
            "else_return_literal_value",
            GO_AST_INVENTORY_SCRATCH_IF_STMT_ELSE_RETURN_LITERAL_VALUE,
        ),
        ("go_version", GO_AST_INVENTORY_SCRATCH_IF_STMT_GO_VERSION),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {key}")),
            expected
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct IfStmt shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered IfStmt scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );
    for (rule_name, fields) in [
        (
            "IfStmt",
            vec!["Init", "Cond", "Body", "Else", "TrailingComment"],
        ),
        ("AssignStmt", vec!["Lhs", "Tok", "Rhs"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "IfStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "IfStmt",
            "Init",
            "AssignStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        ("IfStmt", "Cond", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "IfStmt",
            "Else",
            "BlockStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "AssignStmt",
            "Tok",
            "go_token_assignment_tokens.DEFINE",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "ReturnStmt",
            "Results",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative
                    && decision.terminal == terminal_kind),
            "IfStmt synthesis should validate finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic IfStmt registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic IfStmt registry path should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 3),
        ("IfStmt", 1),
        ("AssignStmt", 1),
        ("Ident", 5),
        ("BasicLit", 3),
        ("ReturnStmt", 2),
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(node_kind),
            count,
            "IfStmt registry path should contain {count} {node_kind} node(s)"
        );
    }
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic IfStmt registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);

    let init_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("init_identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty IfStmt init identifier");
    assert_eq!(
        init_error.field(),
        "go_ast_inventory_if_stmt_synthesis.payload_spec.init_identifier"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_function_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID)
        .expect("FunctionReturn registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with FunctionReturn defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_function_return_raw_tokens(roles)
        .expect("per-construct FunctionReturn shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_function_return_raw_tokens(roles)
        .expect("registered FunctionReturn witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID
    );
    for (key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_PACKAGE_NAME,
        ),
        (
            "identifier",
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_IDENTIFIER,
        ),
        (
            "result_type",
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RESULT_TYPE,
        ),
        (
            "literal_kind",
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_KIND,
        ),
        (
            "literal_value",
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_VALUE,
        ),
        (
            "go_version",
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_GO_VERSION,
        ),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {key}")),
            expected
        );
    }
    for (rule_name, fields) in [("Ident", vec!["Name"]), ("BasicLit", vec!["Kind", "Value"])] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic FunctionReturn registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("Field", "Type", "Ident"),
        ("ReturnStmt", "Results", "BasicLit"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic FunctionReturn registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct FunctionReturn shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered FunctionReturn scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject unsupported FunctionReturn literal kinds");
    assert_eq!(
        literal_kind_error.field(),
        "go_ast_inventory_function_return_synthesis.payload_spec.literal_kind"
    );

    let literal_value_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("literal_value", ""),
    )
    .expect_err("registry payload rules should reject an empty FunctionReturn literal value");
    assert_eq!(
        literal_value_error.field(),
        "go_ast_inventory_function_return_synthesis.payload_spec.literal_value"
    );

    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("FunctionReturn registry synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("FunctionReturn registry synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("FuncDecl"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("ReturnStmt"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("BasicLit"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("FunctionReturn registry synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_array_type_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID)
        .expect("ArrayType return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with ArrayType defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_array_type_return_raw_tokens(roles)
        .expect("per-construct ArrayType shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_array_type_return_raw_tokens(roles)
        .expect("registered ArrayType witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("element_type")
            .expect("merged registry payload should carry the element type default"),
        "int"
    );
    assert_eq!(
        generic
            .payload
            .str_value("return_identifier")
            .expect("merged registry payload should carry the return identifier default"),
        "nil"
    );
    for (rule_name, fields) in [("ArrayType", vec!["Len", "Elt"]), ("Ident", vec!["Name"])] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic ArrayType registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("Field", "Type", "ArrayType"),
        ("ArrayType", "Elt", "Ident"),
        ("ReturnStmt", "Results", "Ident"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic ArrayType registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct ArrayType shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered ArrayType scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let element_type_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("element_type", ""),
    )
    .expect_err("registry payload rules should reject an empty ArrayType element type");
    assert_eq!(
        element_type_error.field(),
        "go_ast_inventory_array_type_return_synthesis.payload_spec.element_type"
    );

    let return_identifier_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("return_identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty ArrayType return identifier");
    assert_eq!(
        return_identifier_error.field(),
        "go_ast_inventory_array_type_return_synthesis.payload_spec.return_identifier"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_chan_type_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID)
        .expect("ChanType return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with ChanType defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_chan_type_return_raw_tokens(roles)
        .expect("per-construct ChanType shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_chan_type_return_raw_tokens(roles)
        .expect("registered ChanType witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("direction")
            .expect("merged registry payload should carry the ChanType direction default"),
        "SEND_RECV"
    );
    assert_eq!(
        generic
            .payload
            .str_value("element_type")
            .expect("merged registry payload should carry the ChanType element type default"),
        "int"
    );
    assert_eq!(
        generic
            .payload
            .str_value("return_identifier")
            .expect("merged registry payload should carry the ChanType return identifier default"),
        "nil"
    );
    for (rule_name, fields) in [("ChanType", vec!["Dir", "Value"]), ("Ident", vec!["Name"])] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic ChanType registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("Field", "Type", "ChanType"),
        ("ChanType", "Dir", "go_ast_channel_directions.SEND_RECV"),
        ("ChanType", "Value", "Ident"),
        ("ReturnStmt", "Results", "Ident"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic ChanType registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct ChanType shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered ChanType scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let direction_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("direction", "SEND_ONLY"),
    )
    .expect_err("registry payload rules should reject a non-v1 ChanType direction");
    assert_eq!(
        direction_error.field(),
        "go_ast_inventory_chan_type_return_synthesis.payload_spec.direction"
    );

    let element_type_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("element_type", ""),
    )
    .expect_err("registry payload rules should reject an empty ChanType element type");
    assert_eq!(
        element_type_error.field(),
        "go_ast_inventory_chan_type_return_synthesis.payload_spec.element_type"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_binary_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("BinaryExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with BinaryExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_binary_expr_return_raw_tokens(roles)
        .expect("per-construct BinaryExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_binary_expr_raw_tokens(roles)
        .expect("registered BinaryExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("left_literal_kind")
            .expect("merged registry payload should carry the left literal kind default"),
        "INT"
    );
    assert_eq!(
        generic
            .payload
            .str_value("operator")
            .expect("merged registry payload should carry the binary operator default"),
        "+"
    );
    assert_eq!(
        generic
            .payload
            .str_value("right_literal_kind")
            .expect("merged registry payload should carry the right literal kind default"),
        "INT"
    );
    for (rule_name, fields) in [
        ("BinaryExpr", vec!["X", "Op", "Y"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic BinaryExpr registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("BinaryExpr", "X", "BasicLit"),
        ("BinaryExpr", "Op", "go_token_binary_operator_tokens.ADD"),
        ("BinaryExpr", "Y", "BasicLit"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic BinaryExpr registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct BinaryExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered BinaryExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let operator_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("operator", "-"),
    )
    .expect_err("registry payload rules should reject a non-v1 BinaryExpr operator");
    assert_eq!(
        operator_error.field(),
        "go_ast_inventory_binary_expr_return_synthesis.payload_spec.operator"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_unary_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("UnaryExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with UnaryExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_unary_expr_return_raw_tokens(roles)
        .expect("per-construct UnaryExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_unary_expr_raw_tokens(roles)
        .expect("registered UnaryExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("operator")
            .expect("merged registry payload should carry the unary operator default"),
        "-"
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct UnaryExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered UnaryExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let operator_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("operator", "+"),
    )
    .expect_err("registry payload rules should reject a non-v1 unary operator");
    assert_eq!(
        operator_error.field(),
        "go_ast_inventory_unary_expr_return_synthesis.payload_spec.operator"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_star_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("StarExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with StarExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_star_expr_return_raw_tokens(roles)
        .expect("per-construct StarExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_star_expr_raw_tokens(roles)
        .expect("registered StarExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("target_identifier")
            .expect("merged registry payload should carry the StarExpr target default"),
        "x"
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct StarExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered StarExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let target_identifier_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("target_identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty StarExpr target identifier");
    assert_eq!(
        target_identifier_error.field(),
        "go_ast_inventory_star_expr_return_synthesis.payload_spec.target_identifier"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_paren_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("ParenExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with ParenExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_paren_expr_return_raw_tokens(roles)
        .expect("per-construct ParenExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_paren_expr_raw_tokens(roles)
        .expect("registered ParenExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("literal_kind")
            .expect("merged registry payload should carry the ParenExpr literal-kind default"),
        "INT"
    );
    assert!(
        generic
            .validated_field_orders
            .iter()
            .any(|order| order.rule_name == "BasicLit"
                && order.fields == vec!["Kind".to_string(), "Value".to_string()]),
        "generic ParenExpr registry path should derive nested BasicLit field order"
    );
    assert!(
        generic
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| decision.rule_name == "ParenExpr"
                && decision.field_name == "X"
                && decision.selected_alternative == "BasicLit"),
        "generic ParenExpr registry path should derive nested BasicLit finite-domain choice"
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct ParenExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered ParenExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject a non-v1 ParenExpr literal kind");
    assert_eq!(
        literal_kind_error.field(),
        "go_ast_inventory_paren_expr_return_synthesis.payload_spec.literal_kind"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_index_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("IndexExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with IndexExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_index_expr_return_raw_tokens(roles)
        .expect("per-construct IndexExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_index_expr_raw_tokens(roles)
        .expect("registered IndexExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("target_identifier")
            .expect("merged registry payload should carry the IndexExpr target default"),
        "xs"
    );
    assert_eq!(
        generic
            .payload
            .str_value("index_literal_kind")
            .expect("merged registry payload should carry the IndexExpr literal-kind default"),
        "INT"
    );
    assert!(
        generic
            .validated_field_orders
            .iter()
            .any(|order| order.rule_name == "IndexExpr"
                && order.fields == vec!["X".to_string(), "Index".to_string()]),
        "generic IndexExpr registry path should derive IndexExpr field order"
    );
    for (field_name, selected_alternative) in [("X", "Ident"), ("Index", "BasicLit")] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == "IndexExpr"
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic IndexExpr registry path should derive IndexExpr.{field_name}={selected_alternative}"
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct IndexExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered IndexExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("index_literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject a non-v1 IndexExpr literal kind");
    assert_eq!(
        literal_kind_error.field(),
        "go_ast_inventory_index_expr_return_synthesis.payload_spec.index_literal_kind"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_index_list_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("IndexListExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with IndexListExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_index_list_expr_return_raw_tokens(roles)
        .expect("per-construct IndexListExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_index_list_expr_raw_tokens(roles)
        .expect("registered IndexListExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("target_identifier")
            .expect("merged registry payload should carry the IndexListExpr target default"),
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_TARGET_IDENTIFIER
    );
    assert_eq!(
        generic
            .payload
            .str_value("first_index_identifier")
            .expect("merged registry payload should carry the first index default"),
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_FIRST_INDEX_IDENTIFIER
    );
    assert_eq!(
        generic
            .payload
            .str_value("second_index_identifier")
            .expect("merged registry payload should carry the second index default"),
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_SECOND_INDEX_IDENTIFIER
    );
    assert!(
        generic
            .validated_field_orders
            .iter()
            .any(|order| order.rule_name == "IndexListExpr"
                && order.fields == vec!["X".to_string(), "Indices".to_string()]),
        "generic IndexListExpr registry path should derive IndexListExpr field order"
    );
    for (field_name, selected_alternative) in [("X", "Ident"), ("Indices", "Ident")] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == "IndexListExpr"
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic IndexListExpr registry path should derive IndexListExpr.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic IndexListExpr registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic IndexListExpr registry path should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("IndexListExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 6);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic IndexListExpr registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RENDERED_SOURCE
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct IndexListExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered IndexListExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    for (payload_key, expected_field) in [
        (
            "target_identifier",
            "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.target_identifier",
        ),
        (
            "first_index_identifier",
            "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.first_index_identifier",
        ),
        (
            "second_index_identifier",
            "go_ast_inventory_index_list_expr_return_synthesis.payload_spec.second_index_identifier",
        ),
    ] {
        let error = match synthesize_go_ast_inventory_registry_raw_tokens(
            roles,
            GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID,
            GoAstInventoryRegistryPayload::new().with_str(payload_key, ""),
        ) {
            Ok(_) => panic!("registry payload rules should reject empty {payload_key}"),
            Err(error) => error,
        };
        assert_eq!(error.field(), expected_field);
    }
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_interface_type_var() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID)
        .expect("InterfaceType var registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::PackageVarType);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with InterfaceType defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_interface_type_var_raw_tokens(roles)
        .expect("per-construct InterfaceType shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_interface_type_raw_tokens(roles)
        .expect("registered InterfaceType witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("methods_policy")
            .expect("merged registry payload should carry the InterfaceType methods policy"),
        "empty_method_set"
    );
    assert!(
        !generic
            .payload
            .bool_value("incomplete")
            .expect("merged registry payload should carry the complete InterfaceType default")
    );
    for (rule_name, fields) in [
        ("InterfaceType", vec!["Methods", "Incomplete"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic InterfaceType registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("GenDecl", "Tok", "go_token_declaration_tokens.VAR"),
        ("GenDecl", "Specs", "ValueSpec"),
        ("ValueSpec", "Type", "InterfaceType"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic InterfaceType registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic InterfaceType registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic InterfaceType registry path should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("GenDecl"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("ValueSpec"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("InterfaceType"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("FieldList"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 2);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic InterfaceType registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct InterfaceType shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered InterfaceType scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    for (payload_key, expected_field) in [
        (
            "identifier",
            "go_ast_inventory_interface_type_var_synthesis.payload_spec.identifier",
        ),
        (
            "package_name",
            "go_ast_inventory_interface_type_var_synthesis.payload_spec.package_name",
        ),
    ] {
        let error = match synthesize_go_ast_inventory_registry_raw_tokens(
            roles,
            GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
            GoAstInventoryRegistryPayload::new().with_str(payload_key, ""),
        ) {
            Ok(_) => panic!("registry payload rules should reject empty {payload_key}"),
            Err(error) => error,
        };
        assert_eq!(error.field(), expected_field);
    }

    let methods_policy_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("methods_policy", "single_method"),
    )
    .expect_err("registry payload rules should reject unsupported InterfaceType methods policy");
    assert_eq!(
        methods_policy_error.field(),
        "go_ast_inventory_interface_type_var_synthesis.payload_spec.methods_policy"
    );

    let incomplete_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_bool("incomplete", true),
    )
    .expect_err("registry payload rules should reject incomplete v1 InterfaceType synthesis");
    assert_eq!(
        incomplete_error.field(),
        "go_ast_inventory_interface_type_var_synthesis.payload_spec.incomplete"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_slice_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("SliceExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with SliceExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_slice_expr_return_raw_tokens(roles)
        .expect("per-construct SliceExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_slice_expr_raw_tokens(roles)
        .expect("registered SliceExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("target_identifier")
            .expect("merged registry payload should carry the SliceExpr target default"),
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_TARGET_IDENTIFIER
    );
    assert_eq!(
        generic
            .payload
            .str_value("low_literal_value")
            .expect("merged registry payload should carry the low literal default"),
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_VALUE
    );
    assert_eq!(
        generic
            .payload
            .str_value("high_literal_value")
            .expect("merged registry payload should carry the high literal default"),
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_VALUE
    );
    assert_eq!(
        generic
            .payload
            .bool_value("three_index")
            .expect("merged registry payload should carry the Slice3 boolean default"),
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_THREE_INDEX
    );
    for (rule_name, fields) in [
        ("SliceExpr", vec!["X", "Low", "High", "Max", "Slice3"]),
        ("BasicLit", vec!["Kind", "Value"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic SliceExpr registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("ReturnStmt", "Results", "SliceExpr"),
        ("SliceExpr", "X", "Ident"),
        ("SliceExpr", "Low", "BasicLit"),
        ("SliceExpr", "High", "BasicLit"),
        ("SliceExpr", "Slice3", "SliceExpr.Slice3.bool"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic SliceExpr registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic SliceExpr registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic SliceExpr registry path should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("SliceExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("BasicLit"), 2);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 4);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic SliceExpr registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RENDERED_SOURCE
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct SliceExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered SliceExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    for (payload_key, expected_field) in [
        (
            "target_identifier",
            "go_ast_inventory_slice_expr_return_synthesis.payload_spec.target_identifier",
        ),
        (
            "low_literal_value",
            "go_ast_inventory_slice_expr_return_synthesis.payload_spec.low_literal_value",
        ),
        (
            "high_literal_value",
            "go_ast_inventory_slice_expr_return_synthesis.payload_spec.high_literal_value",
        ),
    ] {
        let error = match synthesize_go_ast_inventory_registry_raw_tokens(
            roles,
            GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
            GoAstInventoryRegistryPayload::new().with_str(payload_key, ""),
        ) {
            Ok(_) => panic!("registry payload rules should reject empty {payload_key}"),
            Err(error) => error,
        };
        assert_eq!(error.field(), expected_field);
    }

    let low_literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("low_literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject a non-v1 SliceExpr low literal kind");
    assert_eq!(
        low_literal_kind_error.field(),
        "go_ast_inventory_slice_expr_return_synthesis.payload_spec.low_literal_kind"
    );

    let three_index_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_bool("three_index", true),
    )
    .expect_err("registry payload rules should reject v1 three-index SliceExpr synthesis");
    assert_eq!(
        three_index_error.field(),
        "go_ast_inventory_slice_expr_return_synthesis.payload_spec.three_index"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_selector_call() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID)
        .expect("SelectorCall registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with SelectorCall defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_selector_call_raw_tokens(roles)
        .expect("per-construct SelectorCall shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_selector_call_raw_tokens(roles)
        .expect("registered SelectorCall witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("receiver")
            .expect("merged registry payload should carry the SelectorCall receiver default"),
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RECEIVER
    );
    assert_eq!(
        generic
            .payload
            .str_value("selector")
            .expect("merged registry payload should carry the SelectorCall selector default"),
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_SELECTOR
    );
    for (rule_name, fields) in [
        (
            "CallExpr",
            vec!["Fun", "Args", "Ellipsis", "LineBreakBefore"],
        ),
        ("SelectorExpr", vec!["X", "Sel"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic SelectorCall registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("ReturnStmt", "Results", "CallExpr"),
        ("CallExpr", "Fun", "SelectorExpr"),
        (
            "CallExpr",
            "Ellipsis",
            "go_syntax_presence_call_expr_ellipsis.ABSENT",
        ),
        ("SelectorExpr", "X", "Ident"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic SelectorCall registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic SelectorCall registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic SelectorCall registry path should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("CallExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("SelectorExpr"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic SelectorCall registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RENDERED_SOURCE
    );
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct SelectorCall shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered SelectorCall scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    for (payload_key, expected_field) in [
        (
            "result_type",
            "go_ast_inventory_selector_call_synthesis.payload_spec.result_type",
        ),
        (
            "receiver",
            "go_ast_inventory_selector_call_synthesis.payload_spec.receiver",
        ),
        (
            "selector",
            "go_ast_inventory_selector_call_synthesis.payload_spec.selector",
        ),
    ] {
        let error = match synthesize_go_ast_inventory_registry_raw_tokens(
            roles,
            GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID,
            GoAstInventoryRegistryPayload::new().with_str(payload_key, ""),
        ) {
            Ok(_) => panic!("registry payload rules should reject empty {payload_key}"),
            Err(error) => error,
        };
        assert_eq!(error.field(), expected_field);
    }
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_key_value_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("KeyValueExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with KeyValueExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_key_value_expr_return_raw_tokens(roles)
        .expect("per-construct KeyValueExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_key_value_expr_raw_tokens(roles)
        .expect("registered KeyValueExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    for (payload_key, expected) in [
        ("key_type", GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_TYPE),
        (
            "value_type",
            GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_TYPE,
        ),
        (
            "key_literal_kind",
            GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_KIND,
        ),
        (
            "key_literal_value",
            GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_VALUE,
        ),
        (
            "value_literal_kind",
            GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_KIND,
        ),
        (
            "value_literal_value",
            GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_VALUE,
        ),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(payload_key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {payload_key}")),
            expected
        );
    }
    assert!(
        !generic
            .payload
            .bool_value("incomplete")
            .expect("merged registry payload should carry the complete literal default")
    );
    for (rule_name, fields) in [
        (
            "CompositeLit",
            vec!["Type", "Elts", "Incomplete", "LineBreakBefore"],
        ),
        ("MapType", vec!["Key", "Value"]),
        ("KeyValueExpr", vec!["Key", "Value"]),
        ("BasicLit", vec!["Kind", "Value"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic KeyValueExpr registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("Field", "Type", "MapType"),
        ("ReturnStmt", "Results", "CompositeLit"),
        ("CompositeLit", "Type", "MapType"),
        ("CompositeLit", "Elts", "KeyValueExpr"),
        ("CompositeLit", "Incomplete", "CompositeLit.Incomplete.bool"),
        ("MapType", "Key", "Ident"),
        ("MapType", "Value", "Ident"),
        ("KeyValueExpr", "Key", "BasicLit"),
        ("KeyValueExpr", "Value", "BasicLit"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.STRING"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic KeyValueExpr registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic KeyValueExpr registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic KeyValueExpr registry path should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("MapType"), 2);
    assert_eq!(decoded_node_tree.node_kind_count("CompositeLit"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("KeyValueExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("BasicLit"), 2);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic KeyValueExpr registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct KeyValueExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered KeyValueExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    for (payload_key, expected_field) in [
        (
            "key_type",
            "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.key_type",
        ),
        (
            "value_type",
            "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.value_type",
        ),
        (
            "key_literal_value",
            "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.key_literal_value",
        ),
        (
            "value_literal_value",
            "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.value_literal_value",
        ),
    ] {
        let error = match synthesize_go_ast_inventory_registry_raw_tokens(
            roles,
            GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
            GoAstInventoryRegistryPayload::new().with_str(payload_key, ""),
        ) {
            Ok(_) => panic!("registry payload rules should reject empty {payload_key}"),
            Err(error) => error,
        };
        assert_eq!(error.field(), expected_field);
    }

    let key_literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("key_literal_kind", "INT"),
    )
    .expect_err("registry payload rules should reject a non-v1 key literal kind");
    assert_eq!(
        key_literal_kind_error.field(),
        "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.key_literal_kind"
    );

    let value_literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("value_literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject a non-v1 value literal kind");
    assert_eq!(
        value_literal_kind_error.field(),
        "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.value_literal_kind"
    );

    let incomplete_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_bool("incomplete", true),
    )
    .expect_err("registry payload rules should reject incomplete v1 KeyValueExpr synthesis");
    assert_eq!(
        incomplete_error.field(),
        "go_ast_inventory_key_value_expr_return_synthesis.payload_spec.incomplete"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_struct_type_decl() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID)
        .expect("StructType declaration registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::PackageType);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with StructType defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_struct_type_decl_raw_tokens(roles)
        .expect("per-construct StructType shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_struct_type_raw_tokens(roles)
        .expect("registered StructType witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID
    );
    for (payload_key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_PACKAGE_NAME,
        ),
        ("type_name", GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_TYPE_NAME),
        (
            "field_name",
            GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_NAME,
        ),
        (
            "field_type",
            GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_TYPE,
        ),
        ("fields_policy", "single_named_field"),
        (
            "go_version",
            GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_GO_VERSION,
        ),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(payload_key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {payload_key}")),
            expected
        );
    }
    assert!(
        !generic
            .payload
            .bool_value("incomplete")
            .expect("merged registry payload should carry the complete StructType default")
    );
    for (rule_name, fields) in [
        (
            "TypeSpec",
            vec!["Doc", "Name", "TypeParams", "Assign", "Type", "Comment"],
        ),
        ("StructType", vec!["Fields", "Incomplete"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>(),
            "generic StructType registry path should derive {rule_name} field order"
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("GenDecl", "Tok", "go_token_declaration_tokens.TYPE"),
        ("GenDecl", "Specs", "TypeSpec"),
        ("TypeSpec", "Type", "StructType"),
        ("Field", "Type", "Ident"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == rule_name
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic StructType registry path should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic StructType registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic StructType registry path should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("TypeSpec"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("StructType"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("FieldList"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Field"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 4);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic StructType registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct StructType shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered StructType scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    for (payload_key, expected_field) in [
        (
            "package_name",
            "go_ast_inventory_struct_type_decl_synthesis.payload_spec.package_name",
        ),
        (
            "type_name",
            "go_ast_inventory_struct_type_decl_synthesis.payload_spec.type_name",
        ),
        (
            "field_name",
            "go_ast_inventory_struct_type_decl_synthesis.payload_spec.field_name",
        ),
        (
            "field_type",
            "go_ast_inventory_struct_type_decl_synthesis.payload_spec.field_type",
        ),
    ] {
        let error = match synthesize_go_ast_inventory_registry_raw_tokens(
            roles,
            GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID,
            GoAstInventoryRegistryPayload::new().with_str(payload_key, ""),
        ) {
            Ok(_) => panic!("registry payload rules should reject empty {payload_key}"),
            Err(error) => error,
        };
        assert_eq!(error.field(), expected_field);
    }

    let fields_policy_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("fields_policy", "anonymous_field"),
    )
    .expect_err("registry payload rules should reject unsupported StructType fields policy");
    assert_eq!(
        fields_policy_error.field(),
        "go_ast_inventory_struct_type_decl_synthesis.payload_spec.fields_policy"
    );

    let incomplete_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_bool("incomplete", true),
    )
    .expect_err("registry payload rules should reject incomplete v1 StructType synthesis");
    assert_eq!(
        incomplete_error.field(),
        "go_ast_inventory_struct_type_decl_synthesis.payload_spec.incomplete"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_type_assert_expr_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID)
        .expect("TypeAssertExpr return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with TypeAssertExpr defaults should emit raw tokens");
    let shim = synthesize_go_ast_inventory_type_assert_expr_return_raw_tokens(roles)
        .expect("per-construct TypeAssertExpr shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_type_assert_expr_raw_tokens(roles)
        .expect("registered TypeAssertExpr witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic
            .payload
            .str_value("target_identifier")
            .expect("merged registry payload should carry the TypeAssertExpr target default"),
        "x"
    );
    assert_eq!(
        generic
            .payload
            .str_value("asserted_type")
            .expect("merged registry payload should carry the asserted type default"),
        "int"
    );
    for (field_name, selected_alternative) in [("X", "Ident"), ("Type", "Ident")] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| decision.rule_name == "TypeAssertExpr"
                    && decision.field_name == field_name
                    && decision.selected_alternative == selected_alternative),
            "generic TypeAssertExpr registry path should derive TypeAssertExpr.{field_name}={selected_alternative}"
        );
    }
    assert_eq!(
        generic.raw_token_ids, shim.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct TypeAssertExpr shim output"
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered TypeAssertExpr scratch witness"
    );
    assert_eq!(generic.synthesis_plan, shim.synthesis_plan);
    assert_eq!(generic.validated_field_orders, shim.validated_field_orders);
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        shim.validated_finite_domain_alternatives
    );

    let asserted_type_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("asserted_type", ""),
    )
    .expect_err("registry payload rules should reject an empty TypeAssertExpr asserted type");
    assert_eq!(
        asserted_type_error.field(),
        "go_ast_inventory_type_assert_expr_return_synthesis.payload_spec.asserted_type"
    );
}

#[test]
fn go_ast_inventory_import_spec_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_import_spec_raw_tokens(roles)
        .expect("inventory-driven import-spec synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_import_spec_raw_tokens(roles)
        .expect("registered import-spec witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.alias,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ALIAS
    );
    assert_eq!(synthesized.payload_spec.declaration_token, "import");
    assert_eq!(
        synthesized.payload_spec.path_literal_kind,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PATH_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.path_literal_value,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PATH_LITERAL_VALUE
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "ImportSpec")
            .expect("ImportSpec field order should be checked")
            .fields,
        vec![
            "Doc".to_string(),
            "Name".to_string(),
            "Path".to_string(),
            "Comment".to_string(),
        ]
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| decision.selected_alternative == "go_token_declaration_tokens.IMPORT")
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| decision.selected_alternative == "go_token_basic_literal_kinds.STRING")
    );
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered import-spec witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_function_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_function_return_raw_tokens(roles)
        .expect("inventory-driven function-return synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_function_return_raw_tokens(roles)
        .expect("registered function-return witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.literal_kind,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.literal_value,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_GO_VERSION
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "FuncDecl")
            .expect("FuncDecl field order should be checked")
            .fields,
        vec![
            "Doc".to_string(),
            "Recv".to_string(),
            "Name".to_string(),
            "Type".to_string(),
            "Body".to_string(),
        ]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "FuncType")
            .expect("FuncType field order should be checked")
            .fields,
        vec![
            "Func".to_string(),
            "TypeParams".to_string(),
            "Params".to_string(),
            "Results".to_string(),
        ]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "ReturnStmt")
            .expect("ReturnStmt field order should be checked")
            .fields,
        vec!["Results".to_string()]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "Field")
            .expect("Field field order should be checked")
            .fields,
        vec![
            "Doc".to_string(),
            "Names".to_string(),
            "Type".to_string(),
            "Tag".to_string(),
            "Comment".to_string(),
        ]
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| {
                decision.selected_alternative == "go_syntax_presence_func_type_func.PRESENT"
            })
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| decision.selected_alternative == "go_token_basic_literal_kinds.INT")
    );
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered function-return witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_array_type_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_array_type_return_raw_tokens(roles)
        .expect("inventory-driven array-type return synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_array_type_return_raw_tokens(roles)
        .expect("registered array-type return witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.element_type,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_ELEMENT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.return_identifier,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_RETURN_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_GO_VERSION
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "FuncDecl")
            .expect("FuncDecl field order should be checked")
            .fields,
        vec![
            "Doc".to_string(),
            "Recv".to_string(),
            "Name".to_string(),
            "Type".to_string(),
            "Body".to_string(),
        ]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "ArrayType")
            .expect("ArrayType field order should be checked")
            .fields,
        vec!["Len".to_string(), "Elt".to_string()]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "ReturnStmt")
            .expect("ReturnStmt field order should be checked")
            .fields,
        vec!["Results".to_string()]
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| {
                decision.selected_alternative == "go_syntax_presence_func_type_func.PRESENT"
            })
    );
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered array-type return witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_chan_type_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_chan_type_return_raw_tokens(roles)
        .expect("inventory-driven channel-type return synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_chan_type_return_raw_tokens(roles).expect(
        "registered channel-type return witness should emit raw tokens through synthesizer",
    );

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.direction,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_DIRECTION
    );
    assert_eq!(
        synthesized.payload_spec.element_type,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ELEMENT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.return_identifier,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RETURN_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_GO_VERSION
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "FuncDecl")
            .expect("FuncDecl field order should be checked")
            .fields,
        vec![
            "Doc".to_string(),
            "Recv".to_string(),
            "Name".to_string(),
            "Type".to_string(),
            "Body".to_string(),
        ]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "ChanType")
            .expect("ChanType field order should be checked")
            .fields,
        vec!["Dir".to_string(), "Value".to_string()]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "ReturnStmt")
            .expect("ReturnStmt field order should be checked")
            .fields,
        vec!["Results".to_string()]
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| {
                decision.selected_alternative == "go_syntax_presence_func_type_func.PRESENT"
            })
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| {
                decision.selected_alternative == "go_ast_channel_directions.SEND_RECV"
            })
    );
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered channel-type return witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_selector_call_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_selector_call_raw_tokens(roles)
        .expect("inventory-driven selector-call synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_selector_call_raw_tokens(roles)
        .expect("registered selector-call witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.receiver,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RECEIVER
    );
    assert_eq!(
        synthesized.payload_spec.selector,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_SELECTOR
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_GO_VERSION
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "CallExpr")
            .expect("CallExpr field order should be checked")
            .fields,
        vec![
            "Fun".to_string(),
            "Args".to_string(),
            "Ellipsis".to_string(),
            "LineBreakBefore".to_string()
        ]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "SelectorExpr")
            .expect("SelectorExpr field order should be checked")
            .fields,
        vec!["X".to_string(), "Sel".to_string()]
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| {
                decision.rule_name == "CallExpr"
                    && decision.field_name == "Fun"
                    && decision.selected_alternative == "SelectorExpr"
            })
    );
    assert!(
        synthesized
            .validated_finite_domain_alternatives
            .iter()
            .any(|decision| {
                decision.selected_alternative == "go_syntax_presence_call_expr_ellipsis.ABSENT"
            })
    );
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered selector-call witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_binary_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_binary_expr_return_raw_tokens(roles)
        .expect("inventory-driven binary-expression return synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_binary_expr_raw_tokens(roles)
        .expect("registered binary-expression witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.left_literal_kind,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_LEFT_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.left_literal_value,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_LEFT_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.operator,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_OPERATOR
    );
    assert_eq!(
        synthesized.payload_spec.right_literal_kind,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RIGHT_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.right_literal_value,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RIGHT_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_GO_VERSION
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "BinaryExpr")
            .expect("BinaryExpr field order should be checked")
            .fields,
        vec!["X".to_string(), "Op".to_string(), "Y".to_string()]
    );
    assert_eq!(
        synthesized
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "BasicLit")
            .expect("BasicLit field order should be checked")
            .fields,
        vec!["Kind".to_string(), "Value".to_string()]
    );
    for (rule_name, field_name, selected_alternative) in [
        ("BinaryExpr", "X", "BasicLit"),
        ("BinaryExpr", "Op", "go_token_binary_operator_tokens.ADD"),
        ("BinaryExpr", "Y", "BasicLit"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "binary-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered binary-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_unary_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_unary_expr_return_raw_tokens(roles)
        .expect("inventory-driven unary-expression synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_unary_expr_raw_tokens(roles)
        .expect("registered unary-expression witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.operator,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_OPERATOR
    );
    assert_eq!(
        synthesized.payload_spec.target_identifier,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_TARGET_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
        ("UnaryExpr", vec!["Op", "X"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("ReturnStmt", "Results", "UnaryExpr"),
        ("UnaryExpr", "Op", "go_token_unary_operator_tokens.SUB"),
        ("UnaryExpr", "X", "Ident"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "unary-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("unary-expression synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("unary-expression synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("UnaryExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 4);
    assert_eq!(decoded_node_tree.node_kind_count("ReturnStmt"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("unary-expression synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered unary-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_composite_lit_return() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID)
        .expect("CompositeLit return registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with CompositeLit defaults should emit raw tokens");
    let synthesized = synthesize_go_ast_inventory_composite_lit_return_raw_tokens(roles)
        .expect("per-construct CompositeLit shim should emit raw tokens");
    let registered = go_ast_inventory_scratch_composite_lit_raw_tokens(roles)
        .expect("registered composite-literal witness should emit raw tokens through synthesizer");

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.root_rule,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ROOT
    );
    for (payload_key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_PACKAGE_NAME,
        ),
        (
            "identifier",
            GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_IDENTIFIER,
        ),
        (
            "element_type",
            GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ELEMENT_TYPE,
        ),
        (
            "literal_kind",
            GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_LITERAL_KIND,
        ),
        (
            "literal_value",
            GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_LITERAL_VALUE,
        ),
        (
            "go_version",
            GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_GO_VERSION,
        ),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(payload_key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {payload_key}")),
            expected
        );
    }
    assert!(
        !generic
            .payload
            .bool_value("incomplete")
            .expect("merged registry payload should carry the complete literal default")
    );
    assert_eq!(
        generic
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "CompositeLit")
            .expect("CompositeLit field order should be checked")
            .fields,
        vec![
            "Type".to_string(),
            "Elts".to_string(),
            "Incomplete".to_string(),
            "LineBreakBefore".to_string()
        ]
    );
    assert_eq!(
        generic
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "ArrayType")
            .expect("ArrayType field order should be checked")
            .fields,
        vec!["Len".to_string(), "Elt".to_string()]
    );
    assert_eq!(
        generic
            .validated_field_orders
            .iter()
            .find(|order| order.rule_name == "BasicLit")
            .expect("BasicLit field order should be checked")
            .fields,
        vec!["Kind".to_string(), "Value".to_string()]
    );
    for (rule_name, field_name, selected_alternative) in [
        ("CompositeLit", "Type", "ArrayType"),
        ("CompositeLit", "Elts", "BasicLit"),
        ("CompositeLit", "Incomplete", "CompositeLit.Incomplete.bool"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "composite-literal synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic CompositeLit registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic CompositeLit registry path should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("ArrayType"), 2);
    assert_eq!(decoded_node_tree.node_kind_count("CompositeLit"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("BasicLit"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic CompositeLit registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        generic.raw_token_ids, synthesized.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct CompositeLit shim output"
    );
    assert_eq!(generic.synthesis_plan, synthesized.synthesis_plan);
    assert_eq!(
        generic.validated_field_orders,
        synthesized.validated_field_orders
    );
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        synthesized.validated_finite_domain_alternatives
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered CompositeLit scratch witness"
    );

    for (payload_key, expected_field) in [
        (
            "element_type",
            "go_ast_inventory_composite_lit_return_synthesis.payload_spec.element_type",
        ),
        (
            "literal_value",
            "go_ast_inventory_composite_lit_return_synthesis.payload_spec.literal_value",
        ),
    ] {
        let error = match synthesize_go_ast_inventory_registry_raw_tokens(
            roles,
            GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID,
            GoAstInventoryRegistryPayload::new().with_str(payload_key, ""),
        ) {
            Ok(_) => panic!("registry payload rules should reject empty {payload_key}"),
            Err(error) => error,
        };
        assert_eq!(error.field(), expected_field);
    }

    let literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject a non-v1 CompositeLit literal kind");
    assert_eq!(
        literal_kind_error.field(),
        "go_ast_inventory_composite_lit_return_synthesis.payload_spec.literal_kind"
    );

    let incomplete_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_bool("incomplete", true),
    )
    .expect_err("registry payload rules should reject incomplete v1 CompositeLit synthesis");
    assert_eq!(
        incomplete_error.field(),
        "go_ast_inventory_composite_lit_return_synthesis.payload_spec.incomplete"
    );
}

#[test]
fn go_ast_inventory_synthesis_registry_routes_ellipsis_array_literal() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID)
        .expect("EllipsisArrayLiteral registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::PackageVarValue);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with EllipsisArrayLiteral defaults should emit raw tokens");
    let synthesized = synthesize_go_ast_inventory_ellipsis_array_literal_raw_tokens(roles)
        .expect("inventory-driven ellipsis array-literal synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_ellipsis_array_literal_raw_tokens(roles).expect(
        "registered ellipsis array-literal witness should emit raw tokens through synthesizer",
    );

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.root_rule,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ROOT
    );
    for (payload_key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_PACKAGE_NAME,
        ),
        (
            "identifier",
            GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_IDENTIFIER,
        ),
        (
            "element_type",
            GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ELEMENT_TYPE,
        ),
        (
            "literal_kind",
            GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_KIND,
        ),
        (
            "literal_value",
            GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_VALUE,
        ),
        (
            "go_version",
            GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_GO_VERSION,
        ),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(payload_key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {payload_key}")),
            expected
        );
    }
    assert!(
        !generic
            .payload
            .bool_value("incomplete")
            .expect("merged registry payload should carry the complete literal default")
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.element_type,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ELEMENT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.literal_kind,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.literal_value,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_GO_VERSION
    );
    for (rule_name, fields) in [
        ("GenDecl", vec!["Doc", "Tok", "Lparen", "Specs", "Rparen"]),
        (
            "ValueSpec",
            vec!["Doc", "Names", "Type", "Values", "Comment"],
        ),
        ("ArrayType", vec!["Len", "Elt"]),
        ("Ellipsis", vec!["Elt"]),
        (
            "CompositeLit",
            vec!["Type", "Elts", "Incomplete", "LineBreakBefore"],
        ),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("GenDecl", "Tok", "go_token_declaration_tokens.VAR"),
        ("GenDecl", "Specs", "ValueSpec"),
        ("ValueSpec", "Values", "CompositeLit"),
        ("ArrayType", "Len", "Ellipsis"),
        ("ArrayType", "Elt", "Ident"),
        ("CompositeLit", "Type", "ArrayType"),
        ("CompositeLit", "Elts", "BasicLit"),
        ("CompositeLit", "Incomplete", "CompositeLit.Incomplete.bool"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "ellipsis array-literal synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic EllipsisArrayLiteral registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic EllipsisArrayLiteral registry path should materialize a nested node tree");
    for (node_kind, count) in [
        ("GenDecl", 1),
        ("ValueSpec", 1),
        ("CompositeLit", 1),
        ("ArrayType", 1),
        ("Ellipsis", 1),
        ("BasicLit", 1),
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(node_kind),
            count,
            "ellipsis array-literal registry path should contain {count} {node_kind} node(s)"
        );
    }
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic EllipsisArrayLiteral registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        generic.raw_token_ids, synthesized.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct EllipsisArrayLiteral shim output"
    );
    assert_eq!(generic.synthesis_plan, synthesized.synthesis_plan);
    assert_eq!(
        generic.validated_field_orders,
        synthesized.validated_field_orders
    );
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        synthesized.validated_finite_domain_alternatives
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered EllipsisArrayLiteral scratch witness"
    );

    for (payload_key, expected_field) in [
        (
            "element_type",
            "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.element_type",
        ),
        (
            "literal_value",
            "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.literal_value",
        ),
    ] {
        let error = match synthesize_go_ast_inventory_registry_raw_tokens(
            roles,
            GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID,
            GoAstInventoryRegistryPayload::new().with_str(payload_key, ""),
        ) {
            Ok(_) => panic!("registry payload rules should reject empty {payload_key}"),
            Err(error) => error,
        };
        assert_eq!(error.field(), expected_field);
    }

    let literal_kind_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("literal_kind", "STRING"),
    )
    .expect_err("registry payload rules should reject a non-v1 EllipsisArrayLiteral literal kind");
    assert_eq!(
        literal_kind_error.field(),
        "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.literal_kind"
    );

    let incomplete_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_bool("incomplete", true),
    )
    .expect_err(
        "registry payload rules should reject incomplete v1 EllipsisArrayLiteral synthesis",
    );
    assert_eq!(
        incomplete_error.field(),
        "go_ast_inventory_ellipsis_array_literal_synthesis.payload_spec.incomplete"
    );
}

#[test]
fn go_ast_inventory_func_lit_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID)
        .expect("FuncLit registry entry should be registered");
    assert_eq!(entry.api_id, GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID);
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::PackageVarValue);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with FuncLit defaults should emit raw tokens");
    let synthesized = synthesize_go_ast_inventory_func_lit_raw_tokens(roles)
        .expect("inventory-driven function-literal synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_func_lit_raw_tokens(roles)
        .expect("registered function-literal witness should emit raw tokens through synthesizer");

    assert_eq!(generic.api_id, GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID);
    assert_eq!(
        generic.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID
    );
    assert_eq!(generic.root_rule, GO_AST_INVENTORY_SCRATCH_FUNC_LIT_ROOT);
    for (payload_key, expected) in [
        (
            "package_name",
            GO_AST_INVENTORY_SCRATCH_FUNC_LIT_PACKAGE_NAME,
        ),
        ("identifier", GO_AST_INVENTORY_SCRATCH_FUNC_LIT_IDENTIFIER),
        ("signature_policy", "no_params_no_results"),
        ("body_policy", "empty_block"),
        ("go_version", GO_AST_INVENTORY_SCRATCH_FUNC_LIT_GO_VERSION),
    ] {
        assert_eq!(
            generic
                .payload
                .str_value(payload_key)
                .unwrap_or_else(|_| panic!("merged registry payload should carry {payload_key}")),
            expected
        );
    }
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.signature_policy,
        "no_params_no_results"
    );
    assert_eq!(synthesized.payload_spec.body_policy, "empty_block");
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_GO_VERSION
    );
    for (rule_name, fields) in [
        ("GenDecl", vec!["Doc", "Tok", "Lparen", "Specs", "Rparen"]),
        (
            "ValueSpec",
            vec!["Doc", "Names", "Type", "Values", "Comment"],
        ),
        ("FuncLit", vec!["Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("GenDecl", "Tok", "go_token_declaration_tokens.VAR"),
        ("GenDecl", "Specs", "ValueSpec"),
        ("ValueSpec", "Values", "FuncLit"),
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "function-literal synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("generic FuncLit registry path should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("generic FuncLit registry path should materialize a nested node tree");
    for (node_kind, count) in [
        ("GenDecl", 1),
        ("ValueSpec", 1),
        ("FuncLit", 1),
        ("FuncType", 1),
        ("FieldList", 1),
        ("BlockStmt", 1),
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(node_kind),
            count,
            "FuncLit registry path should contain {count} {node_kind} node(s)"
        );
    }
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("generic FuncLit registry path should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        generic.raw_token_ids, synthesized.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct FuncLit shim output"
    );
    assert_eq!(generic.synthesis_plan, synthesized.synthesis_plan);
    assert_eq!(
        generic.validated_field_orders,
        synthesized.validated_field_orders
    );
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        synthesized.validated_finite_domain_alternatives
    );
    assert_eq!(
        generic.raw_token_ids, registered,
        "generic registry synthesis should be byte-identical to the registered FuncLit scratch witness"
    );

    let signature_policy_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("signature_policy", "single_result"),
    )
    .expect_err("registry payload rules should reject unsupported FuncLit signatures");
    assert_eq!(
        signature_policy_error.field(),
        "go_ast_inventory_func_lit_synthesis.payload_spec.signature_policy"
    );

    let body_policy_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("body_policy", "return_nil"),
    )
    .expect_err("registry payload rules should reject unsupported FuncLit bodies");
    assert_eq!(
        body_policy_error.field(),
        "go_ast_inventory_func_lit_synthesis.payload_spec.body_policy"
    );

    let identifier_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty FuncLit identifier");
    assert_eq!(
        identifier_error.field(),
        "go_ast_inventory_func_lit_synthesis.payload_spec.identifier"
    );
}

#[test]
fn go_ast_inventory_func_type_result_synthesizer_emits_valid_type_context() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let registry =
        go_ast_inventory_synthesis_registry().expect("synthesis registry should construct");
    let entry = registry
        .entry(GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID)
        .expect("FuncTypeResult registry entry should be registered");
    assert_eq!(
        entry.api_id,
        GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID
    );
    assert_eq!(entry.shell, GoAstInventorySynthesisShell::FunctionReturn);

    let generic = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new(),
    )
    .expect("generic registry synthesis with FuncTypeResult defaults should emit raw tokens");
    let synthesized = synthesize_go_ast_inventory_func_type_result_raw_tokens(roles)
        .expect("inventory-driven function-type result synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_func_type_result_raw_tokens(roles).expect(
        "registered function-type result witness should emit raw tokens through synthesizer",
    );

    assert_eq!(
        generic.api_id,
        GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        generic.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID
    );
    assert_eq!(generic.root_rule, "GoSourceFile");
    assert_eq!(
        generic
            .payload
            .str_value("signature_policy")
            .expect("merged registry payload should carry the signature policy default"),
        "no_params_no_results"
    );
    assert_eq!(
        generic
            .payload
            .str_value("return_identifier")
            .expect("merged registry payload should carry the return identifier default"),
        "nil"
    );
    assert_eq!(synthesized.payload_spec.package_name, "p");
    assert_eq!(synthesized.payload_spec.identifier, "f");
    assert_eq!(
        synthesized.payload_spec.signature_policy,
        "no_params_no_results"
    );
    assert_eq!(synthesized.payload_spec.return_identifier, "nil");
    assert_eq!(synthesized.payload_spec.go_version, "go1.24");
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
    ] {
        assert_eq!(
            generic
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("Field", "Type", "FuncType"),
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("ReturnStmt", "Results", "Ident"),
    ] {
        assert!(
            generic
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "function-type result synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    assert_eq!(
        generic.raw_token_ids, synthesized.raw_token_ids,
        "generic registry synthesis should be byte-identical to the per-construct FuncTypeResult shim output"
    );
    assert_eq!(
        registered, synthesized.raw_token_ids,
        "registered function-type result witness should be byte-identical to the per-construct FuncTypeResult shim output"
    );
    assert_eq!(generic.synthesis_plan, synthesized.synthesis_plan);
    assert_eq!(
        generic.validated_field_orders,
        synthesized.validated_field_orders
    );
    assert_eq!(
        generic.validated_finite_domain_alternatives,
        synthesized.validated_finite_domain_alternatives
    );

    let signature_policy_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("signature_policy", "single_result"),
    )
    .expect_err("registry payload rules should reject unsupported FuncTypeResult signatures");
    assert_eq!(
        signature_policy_error.field(),
        "go_ast_inventory_func_type_result_synthesis.payload_spec.signature_policy"
    );

    let return_identifier_error = synthesize_go_ast_inventory_registry_raw_tokens(
        roles,
        GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID,
        GoAstInventoryRegistryPayload::new().with_str("return_identifier", ""),
    )
    .expect_err("registry payload rules should reject an empty FuncTypeResult return identifier");
    assert_eq!(
        return_identifier_error.field(),
        "go_ast_inventory_func_type_result_synthesis.payload_spec.return_identifier"
    );

    let decoded_tree = model
        .decode_complete_tree(&generic.raw_token_ids)
        .expect("function-type result synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("function-type result synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("FuncDecl"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("FuncType"), 2);
    assert_eq!(decoded_node_tree.node_kind_count("ReturnStmt"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("function-type result synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        "package p\n\nfunc f() func() {\n\treturn nil\n}\n"
    );
    assert_go_parser_formatter_accepts(&rendered.source);
}

#[test]
fn go_ast_inventory_index_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_index_expr_return_raw_tokens(roles)
        .expect("inventory-driven index-expression synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_index_expr_raw_tokens(roles)
        .expect("registered index-expression witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.target_identifier,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_TARGET_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.index_literal_kind,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_INDEX_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.index_literal_value,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_INDEX_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        ("BasicLit", vec!["Kind", "Value"]),
        ("IndexExpr", vec!["X", "Index"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
        ("IndexExpr", "X", "Ident"),
        ("IndexExpr", "Index", "BasicLit"),
        ("ReturnStmt", "Results", "IndexExpr"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "index-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("index-expression synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("index-expression synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("IndexExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("BasicLit"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 4);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("index-expression synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered index-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_index_list_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_index_list_expr_return_raw_tokens(roles)
        .expect("inventory-driven index-list-expression synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_index_list_expr_raw_tokens(roles).expect(
        "registered index-list-expression witness should emit raw tokens through synthesizer",
    );

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.target_identifier,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_TARGET_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.first_index_identifier,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_FIRST_INDEX_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.second_index_identifier,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_SECOND_INDEX_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        ("IndexListExpr", vec!["X", "Indices"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("IndexListExpr", "X", "Ident"),
        ("IndexListExpr", "Indices", "Ident"),
        ("ReturnStmt", "Results", "IndexListExpr"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "index-list-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("index-list-expression synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("index-list-expression synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("IndexListExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 6);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("index-list-expression synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered index-list-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_slice_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_slice_expr_return_raw_tokens(roles)
        .expect("inventory-driven slice-expression synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_slice_expr_raw_tokens(roles)
        .expect("registered slice-expression witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.target_identifier,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_TARGET_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.low_literal_kind,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.low_literal_value,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.high_literal_kind,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.high_literal_value,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.three_index,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_THREE_INDEX
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
        ("SliceExpr", vec!["X", "Low", "High", "Max", "Slice3"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("ReturnStmt", "Results", "SliceExpr"),
        ("SliceExpr", "X", "Ident"),
        ("SliceExpr", "Low", "BasicLit"),
        ("SliceExpr", "High", "BasicLit"),
        ("SliceExpr", "Slice3", "SliceExpr.Slice3.bool"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "slice-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("slice-expression synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("slice-expression synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("SliceExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("BasicLit"), 2);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 4);
    assert_eq!(decoded_node_tree.node_kind_count("ReturnStmt"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("slice-expression synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered slice-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_star_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_star_expr_return_raw_tokens(roles)
        .expect("inventory-driven star-expression synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_star_expr_raw_tokens(roles)
        .expect("registered star-expression witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.target_identifier,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_TARGET_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
        ("StarExpr", vec!["X"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("ReturnStmt", "Results", "StarExpr"),
        ("StarExpr", "X", "Ident"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "star-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("star-expression synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("star-expression synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("StarExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 4);
    assert_eq!(decoded_node_tree.node_kind_count("ReturnStmt"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("star-expression synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered star-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_struct_type_decl_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_struct_type_decl_raw_tokens(roles)
        .expect("inventory-driven struct-type declaration synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_struct_type_raw_tokens(roles)
        .expect("registered struct-type witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.type_name,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_TYPE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.field_name,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_NAME
    );
    assert_eq!(
        synthesized.payload_spec.field_type,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_TYPE
    );
    assert_eq!(synthesized.payload_spec.fields_policy, "single_named_field");
    assert!(!synthesized.payload_spec.incomplete);
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_GO_VERSION
    );
    for (rule_name, fields) in [
        (
            "File",
            vec![
                "Doc",
                "Name",
                "Decls",
                "Comments",
                "GoVersion",
                "CommentBefore",
                "NoBlankLineBefore",
            ],
        ),
        ("GenDecl", vec!["Doc", "Tok", "Lparen", "Specs", "Rparen"]),
        (
            "TypeSpec",
            vec!["Doc", "Name", "TypeParams", "Assign", "Type", "Comment"],
        ),
        ("Ident", vec!["Name"]),
        ("StructType", vec!["Fields", "Incomplete"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("GenDecl", "Tok", "go_token_declaration_tokens.TYPE"),
        ("GenDecl", "Specs", "TypeSpec"),
        ("TypeSpec", "Type", "StructType"),
        ("Field", "Type", "Ident"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "struct-type declaration synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("struct-type declaration synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("struct-type declaration synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("TypeSpec"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("StructType"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("FieldList"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Field"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 4);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("struct-type declaration synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered struct-type witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_type_assert_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_type_assert_expr_return_raw_tokens(roles)
        .expect("inventory-driven type-assert-expression synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_type_assert_expr_raw_tokens(roles).expect(
        "registered type-assert-expression witness should emit raw tokens through synthesizer",
    );

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.target_identifier,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_TARGET_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.asserted_type,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ASSERTED_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
        ("TypeAssertExpr", vec!["X", "Type"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("ReturnStmt", "Results", "TypeAssertExpr"),
        ("TypeAssertExpr", "X", "Ident"),
        ("TypeAssertExpr", "Type", "Ident"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "type-assert-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("type-assert-expression synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("type-assert-expression synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("TypeAssertExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 5);
    assert_eq!(decoded_node_tree.node_kind_count("ReturnStmt"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("type-assert-expression synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered type-assert-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_interface_type_var_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_interface_type_var_raw_tokens(roles)
        .expect("inventory-driven interface-type synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_interface_type_raw_tokens(roles)
        .expect("registered interface-type witness should emit raw tokens through synthesizer");

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_IDENTIFIER
    );
    assert_eq!(synthesized.payload_spec.methods_policy, "empty_method_set");
    assert!(!synthesized.payload_spec.incomplete);
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_GO_VERSION
    );
    for (rule_name, fields) in [
        (
            "File",
            vec![
                "Doc",
                "Name",
                "Decls",
                "Comments",
                "GoVersion",
                "CommentBefore",
                "NoBlankLineBefore",
            ],
        ),
        ("GenDecl", vec!["Doc", "Tok", "Lparen", "Specs", "Rparen"]),
        (
            "ValueSpec",
            vec!["Doc", "Names", "Type", "Values", "Comment"],
        ),
        ("Ident", vec!["Name"]),
        ("InterfaceType", vec!["Methods", "Incomplete"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        ("GenDecl", "Tok", "go_token_declaration_tokens.VAR"),
        ("GenDecl", "Specs", "ValueSpec"),
        ("ValueSpec", "Type", "InterfaceType"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "interface-type synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("interface-type synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("interface-type synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("GenDecl"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("ValueSpec"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("InterfaceType"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("FieldList"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("Ident"), 2);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("interface-type synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered interface-type witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_key_value_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_key_value_expr_return_raw_tokens(roles)
        .expect("inventory-driven key-value-expression synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_key_value_expr_raw_tokens(roles).expect(
        "registered key-value-expression witness should emit raw tokens through synthesizer",
    );

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.key_type,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.value_type,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.key_literal_kind,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.key_literal_value,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.value_literal_kind,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.value_literal_value,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_VALUE
    );
    assert!(!synthesized.payload_spec.incomplete);
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_GO_VERSION
    );
    for (rule_name, fields) in [
        (
            "File",
            vec![
                "Doc",
                "Name",
                "Decls",
                "Comments",
                "GoVersion",
                "CommentBefore",
                "NoBlankLineBefore",
            ],
        ),
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
        (
            "CompositeLit",
            vec!["Type", "Elts", "Incomplete", "LineBreakBefore"],
        ),
        ("MapType", vec!["Key", "Value"]),
        ("KeyValueExpr", vec!["Key", "Value"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("ReturnStmt", "Results", "CompositeLit"),
        ("CompositeLit", "Type", "MapType"),
        ("CompositeLit", "Elts", "KeyValueExpr"),
        ("CompositeLit", "Incomplete", "CompositeLit.Incomplete.bool"),
        ("MapType", "Key", "Ident"),
        ("MapType", "Value", "Ident"),
        ("KeyValueExpr", "Key", "BasicLit"),
        ("KeyValueExpr", "Value", "BasicLit"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.STRING"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "key-value-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("key-value-expression synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("key-value-expression synthesis should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("MapType", 2),
        ("CompositeLit", 1),
        ("KeyValueExpr", 1),
        ("BasicLit", 2),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(node_kind),
            count,
            "key-value-expression synthesis should contain {count} {node_kind} node(s)"
        );
    }
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("key-value-expression synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered key-value-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_paren_expr_return_synthesizer_matches_registered_witness() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let synthesized = synthesize_go_ast_inventory_paren_expr_return_raw_tokens(roles)
        .expect("inventory-driven parenthesized-expression synthesis should emit raw tokens");
    let registered = go_ast_inventory_scratch_paren_expr_raw_tokens(roles).expect(
        "registered parenthesized-expression witness should emit raw tokens through synthesizer",
    );

    assert_eq!(
        synthesized.api_id,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.api_id,
        GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID
    );
    assert_eq!(
        synthesized.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        synthesized.root_rule,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_ROOT
    );
    assert_eq!(
        synthesized.payload_spec.package_name,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_PACKAGE_NAME
    );
    assert_eq!(
        synthesized.payload_spec.identifier,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_IDENTIFIER
    );
    assert_eq!(
        synthesized.payload_spec.result_type,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RESULT_TYPE
    );
    assert_eq!(
        synthesized.payload_spec.literal_kind,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_LITERAL_KIND
    );
    assert_eq!(
        synthesized.payload_spec.literal_value,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_LITERAL_VALUE
    );
    assert_eq!(
        synthesized.payload_spec.go_version,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_GO_VERSION
    );
    for (rule_name, fields) in [
        ("FuncDecl", vec!["Doc", "Recv", "Name", "Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
        ("ParenExpr", vec!["X"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            synthesized
                .validated_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be checked"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative) in [
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
        ),
        ("ReturnStmt", "Results", "ParenExpr"),
        ("ParenExpr", "X", "BasicLit"),
        ("BasicLit", "Kind", "go_token_basic_literal_kinds.INT"),
    ] {
        assert!(
            synthesized
                .validated_finite_domain_alternatives
                .iter()
                .any(|decision| {
                    decision.rule_name == rule_name
                        && decision.field_name == field_name
                        && decision.selected_alternative == selected_alternative
                }),
            "parenthesized-expression synthesis should validate {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decoded_tree = model
        .decode_complete_tree(&synthesized.raw_token_ids)
        .expect("parenthesized-expression synthesis should strict-decode");
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("parenthesized-expression synthesis should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind_count("ParenExpr"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("BasicLit"), 1);
    assert_eq!(decoded_node_tree.node_kind_count("ReturnStmt"), 1);
    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("parenthesized-expression synthesis should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&rendered.source);
    assert_eq!(
        synthesized.raw_token_ids, registered,
        "registered parenthesized-expression witness should remain a regression check over inventory synthesis"
    );
}

#[test]
fn go_ast_inventory_declarative_index_list_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_index_list_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "IndexListExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative IndexListExpr spec should validate against checked inventory");
    assert_eq!(
        checked
            .expected_field_orders
            .iter()
            .find(|order| order.rule_name == "IndexListExpr")
            .expect("IndexListExpr field order should be derived")
            .fields,
        vec!["X".to_string(), "Indices".to_string()]
    );
    assert_eq!(
        checked
            .expected_field_orders
            .iter()
            .find(|order| order.rule_name == "Ident")
            .expect("Ident field order should be derived")
            .fields,
        vec!["Name".to_string()]
    );
    assert_eq!(checked.finite_domain_requirements.len(), 3);
    for (rule_name, field_name, selected_alternative) in [
        ("ReturnStmt", "Results", "IndexListExpr"),
        ("IndexListExpr", "X", "Ident"),
        ("IndexListExpr", "Indices", "Ident"),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == GrammarTerminalKind::Nonterminal
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let indices_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Indices")
        .expect("normalized declarative spec should preserve Indices field");
    match &indices_field.value {
        GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes) => {
            assert_eq!(nodes.len(), 2);
            assert!(nodes.iter().all(|node| node.node_kind == "Ident"));
        }
        other => panic!("expected repeated Indices node policy, got {other:?}"),
    }

    let mut invalid = spec.expression.clone();
    let x_value = invalid
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "X")
        .expect("X field should exist")
        .value
        .clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Indices")
        .expect("Indices field should exist")
        .value = x_value;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("single-node Indices policy should violate checked repeated-field metadata");
    assert!(
        format!("{error}").contains("repeated-alternative policy"),
        "unexpected declarative repeated-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_interface_type_spec_derives_checked_plan() {
    let spec = go_ast_inventory_interface_type_var_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ValueSpec");
    assert_eq!(spec.parent_field.field_name, "Type");
    assert_eq!(spec.expression.root_node_kind(), "InterfaceType");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative InterfaceType spec should validate against checked inventory");
    assert_eq!(
        checked
            .expected_field_orders
            .iter()
            .find(|order| order.rule_name == "InterfaceType")
            .expect("InterfaceType field order should be derived")
            .fields,
        vec!["Methods".to_string(), "Incomplete".to_string()]
    );
    assert_eq!(
        checked
            .expected_field_orders
            .iter()
            .find(|order| order.rule_name == "FieldList")
            .expect("FieldList field order should be derived")
            .fields,
        vec!["List".to_string(), "BlankLineBefore".to_string()]
    );
    assert_eq!(checked.finite_domain_requirements.len(), 1);
    assert!(
        checked
            .finite_domain_requirements
            .iter()
            .any(|requirement| {
                requirement.rule_name == "ValueSpec"
                    && requirement.field_name == "Type"
                    && requirement.selected_alternative == "InterfaceType"
                    && requirement.terminal == GrammarTerminalKind::Nonterminal
            }),
        "declarative spec should derive finite-domain requirement ValueSpec.Type=InterfaceType"
    );
    let methods_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Methods")
        .expect("normalized declarative spec should preserve Methods field");
    match &methods_field.value {
        GoAstInventoryDeclarativeFieldValue::AlternativeNode(field_list) => {
            assert_eq!(field_list.node_kind, "FieldList");
            let list_field = field_list
                .fields
                .iter()
                .find(|field| field.field_name == "List")
                .expect("normalized FieldList should preserve List field");
            match &list_field.value {
                GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes) => {
                    assert!(nodes.is_empty());
                }
                other => panic!("expected empty repeated List policy, got {other:?}"),
            }
        }
        other => panic!("expected Methods FieldList node policy, got {other:?}"),
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .retain(|field| field.field_name != "Methods");
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("omitted Methods policy should violate the declarative spec contract");
    assert!(
        format!("{error}").contains("must spell policy"),
        "unexpected declarative missing-field error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_labeled_stmt_spec_derives_checked_optional_policy() {
    let spec = go_ast_inventory_labeled_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "LabeledStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative LabeledStmt spec should validate against checked inventory");
    assert_eq!(
        checked
            .expected_field_orders
            .iter()
            .find(|order| order.rule_name == "LabeledStmt")
            .expect("LabeledStmt field order should be derived")
            .fields,
        vec!["Label".to_string(), "Stmt".to_string()]
    );
    assert!(
        checked
            .finite_domain_requirements
            .iter()
            .any(|requirement| {
                requirement.rule_name == "BlockStmt"
                    && requirement.field_name == "List"
                    && requirement.selected_alternative == "LabeledStmt"
                    && requirement.terminal == GrammarTerminalKind::Nonterminal
            }),
        "declarative spec should derive finite-domain requirement BlockStmt.List=LabeledStmt"
    );
    assert!(
        checked
            .finite_domain_requirements
            .iter()
            .any(|requirement| {
                requirement.rule_name == "LabeledStmt"
                    && requirement.field_name == "Stmt"
                    && requirement.selected_alternative == "ReturnStmt"
                    && requirement.terminal == GrammarTerminalKind::Nonterminal
            }),
        "declarative spec should derive finite-domain requirement LabeledStmt.Stmt=ReturnStmt"
    );
    let label_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Label")
        .expect("normalized declarative spec should preserve Label field");
    match &label_field.value {
        GoAstInventoryDeclarativeFieldValue::OptionalNode(Some(label)) => {
            assert_eq!(label.node_kind, "Ident");
        }
        other => panic!("expected present optional Label node policy, got {other:?}"),
    }
}

#[test]
fn go_ast_inventory_template_synthesizer_reuses_function_return_specs() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let roles = model.token_roles();

    let base_function_return_orders = || {
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
    };
    let function_presence_requirement = || {
        GoAstInventoryFiniteDomainRequirement::new(
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        )
    };

    let mut literal_orders = base_function_return_orders();
    literal_orders.push(GoAstInventoryExpectedFieldOrder::new(
        "BasicLit",
        &["Kind", "Value"][..],
    ));
    let literal_template = synthesize_go_ast_inventory_template_raw_tokens(
        roles,
        GoAstInventoryTemplateSpec::function_return_expression(
            GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID,
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_PACKAGE_NAME,
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_GO_VERSION,
            GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_IDENTIFIER,
            GoAstInventoryExpressionTemplate::Ident {
                payload: GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RESULT_TYPE.to_string(),
            },
            GoAstInventoryExpressionTemplate::BasicLit {
                literal_kind: GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_KIND.to_string(),
                literal_value: GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_VALUE.to_string(),
            },
            literal_orders,
            vec![
                function_presence_requirement(),
                GoAstInventoryFiniteDomainRequirement::new(
                    "BasicLit",
                    "Kind",
                    "go_token_basic_literal_kinds.INT",
                    GrammarTerminalKind::IdentifierPayload,
                ),
            ],
        ),
    )
    .expect("template synthesizer should emit the integer return raw-token stream");
    let literal_category = synthesize_go_ast_inventory_function_return_raw_tokens(roles)
        .expect("typed function-return synthesizer should remain available");
    assert_eq!(
        literal_template.api_id,
        GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_API_ID
    );
    assert_eq!(
        literal_template.template_id,
        GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID
    );
    assert_eq!(
        literal_template.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(
        literal_template.raw_token_ids,
        literal_category.raw_token_ids
    );

    let mut selector_orders = base_function_return_orders();
    selector_orders.push(GoAstInventoryExpectedFieldOrder::new(
        "CallExpr",
        &["Fun", "Args", "Ellipsis", "LineBreakBefore"][..],
    ));
    selector_orders.push(GoAstInventoryExpectedFieldOrder::new(
        "SelectorExpr",
        &["X", "Sel"][..],
    ));
    let selector_template = synthesize_go_ast_inventory_template_raw_tokens(
        roles,
        GoAstInventoryTemplateSpec::function_return_expression(
            GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID,
            GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_PACKAGE_NAME,
            GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_GO_VERSION,
            GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_IDENTIFIER,
            GoAstInventoryExpressionTemplate::Ident {
                payload: GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RESULT_TYPE.to_string(),
            },
            GoAstInventoryExpressionTemplate::SelectorCall {
                receiver: GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RECEIVER.to_string(),
                selector: GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_SELECTOR.to_string(),
            },
            selector_orders,
            vec![
                function_presence_requirement(),
                GoAstInventoryFiniteDomainRequirement::new(
                    "CallExpr",
                    "Fun",
                    "SelectorExpr",
                    GrammarTerminalKind::Nonterminal,
                ),
                GoAstInventoryFiniteDomainRequirement::new(
                    "CallExpr",
                    "Ellipsis",
                    "go_syntax_presence_call_expr_ellipsis.ABSENT",
                    GrammarTerminalKind::BooleanPayload,
                ),
                GoAstInventoryFiniteDomainRequirement::new(
                    "SelectorExpr",
                    "X",
                    "Ident",
                    GrammarTerminalKind::Nonterminal,
                ),
                GoAstInventoryFiniteDomainRequirement::new(
                    "SelectorExpr",
                    "Sel",
                    "Ident",
                    GrammarTerminalKind::Nonterminal,
                ),
            ],
        ),
    )
    .expect("template synthesizer should emit the selector-call raw-token stream");
    let selector_category = synthesize_go_ast_inventory_selector_call_raw_tokens(roles)
        .expect("typed selector-call synthesizer should remain available");
    assert_eq!(
        selector_template.template_id,
        GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID
    );
    assert_eq!(
        selector_template.synthesis_plan.synthesis_id,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID
    );
    assert_eq!(
        selector_template.raw_token_ids,
        selector_category.raw_token_ids
    );
}

#[test]
fn go_ast_inventory_scratch_witness_registry_decodes_renders_and_reparses() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let core: &dyn RawSyntaxStateMachineCore = &model;
    let roles = model.token_roles();

    let specs = go_ast_inventory_scratch_witness_specs();
    let seeds =
        go_ast_inventory_scratch_seeds(core).expect("scratch seed list should build all seeds");
    assert_eq!(seeds.len(), specs.len());

    for (spec, seed) in specs.iter().zip(seeds.iter()) {
        assert_eq!(seed.seed_id, spec.seed_id, "seed id for {}", spec.seed_id);
        assert_eq!(
            seed.grammar_category, spec.grammar_category,
            "grammar category for {}",
            spec.seed_id
        );
        assert_eq!(seed.root, spec.root, "root for {}", spec.seed_id);
        assert_eq!(
            seed.expected_rendered_source, spec.expected_rendered_source,
            "expected rendered source for {}",
            spec.seed_id
        );
        assert_eq!(
            seed.contract, spec.contract,
            "contract for {}",
            spec.seed_id
        );
        assert_eq!(
            seed.raw_token_ids,
            spec.raw_token_ids(roles)
                .unwrap_or_else(|error| panic!("raw tokens for {}: {error}", spec.seed_id)),
            "registry raw-token builder should preserve {}",
            spec.seed_id
        );
        assert_eq!(
            seed.raw_token_ids.first().copied(),
            Some(roles.node_start),
            "{} starts from the generic GoSourceFile node_start",
            spec.seed_id
        );

        let admissibility = &seed.admissibility;
        assert_eq!(
            admissibility.raw_token_count,
            seed.raw_token_ids.len(),
            "{} admissibility raw-token count",
            spec.seed_id
        );
        assert_eq!(
            admissibility.admissible_mask_query_count,
            seed.raw_token_ids.len(),
            "{} admissible-mask query count",
            spec.seed_id
        );
        assert!(
            admissibility.finite_admissible_mask_available,
            "{} should expose finite admissible masks",
            spec.seed_id
        );
        assert!(
            admissibility.every_selected_raw_token_admitted,
            "{} should select only admitted raw tokens",
            spec.seed_id
        );
        assert_eq!(
            admissibility.selected_transition_accepted_count,
            seed.raw_token_ids.len(),
            "{} accepted transition count",
            spec.seed_id
        );
        assert_eq!(
            admissibility.steps.len(),
            seed.raw_token_ids.len(),
            "{} step trace length",
            spec.seed_id
        );
        assert_eq!(
            admissibility.final_state.status,
            RawSyntaxStatus::Complete,
            "{} final raw state",
            spec.seed_id
        );
        assert!(
            admissibility.final_state.eof_valid,
            "{} final state should accept EOF",
            spec.seed_id
        );
        for step in &admissibility.steps {
            assert_eq!(
                step.prefix_raw_token_count, step.step_index,
                "{} step {} prefix count",
                spec.seed_id, step.step_index
            );
            assert!(
                step.finite_admissible_raw_token_count > 0,
                "{} step {} should expose a finite raw-token mask",
                spec.seed_id,
                step.step_index
            );
            assert!(
                step.selected_raw_token_admitted,
                "{} step {} selected raw token should be admitted",
                spec.seed_id, step.step_index
            );
            assert!(
                step.selected_transition_accepted,
                "{} step {} transition should be accepted",
                spec.seed_id, step.step_index
            );
        }

        let complete = core
            .strict_decode_complete(&seed.raw_token_ids)
            .unwrap_or_else(|error| panic!("strict decode for {}: {error:?}", spec.seed_id));
        assert_eq!(
            complete.status,
            RawSyntaxStatus::Complete,
            "{}",
            spec.seed_id
        );
        assert!(complete.eof_valid, "{}", spec.seed_id);
        assert_eq!(
            complete.consumed_raw_tokens,
            seed.raw_token_ids.len(),
            "{} consumed raw tokens",
            spec.seed_id
        );
        assert!(
            complete.admissible.finite_raw_token_ids.is_empty(),
            "{} complete state should have no finite next raw tokens",
            spec.seed_id
        );

        let decoded_tree = core
            .decode_complete_tree(&seed.raw_token_ids)
            .unwrap_or_else(|error| panic!("decoded tree for {}: {error:?}", spec.seed_id));
        assert_eq!(decoded_tree.root_rule(), Some("File"), "{}", spec.seed_id);
        assert_eq!(
            decoded_tree.root_node_kind(),
            Some("File"),
            "{}",
            spec.seed_id
        );
        let decoded_node_tree = decoded_tree
            .decoded_node_tree()
            .unwrap_or_else(|error| panic!("decoded node tree for {}: {error:?}", spec.seed_id));
        assert_eq!(decoded_node_tree.node_kind, "File", "{}", spec.seed_id);

        let rendered = render_go_source_from_raw_tree(&decoded_tree)
            .unwrap_or_else(|error| panic!("render for {}: {error}", spec.seed_id));
        assert_eq!(
            rendered.source, spec.expected_rendered_source,
            "{} rendered source",
            spec.seed_id
        );
        assert_go_parser_formatter_accepts(&rendered.source);
    }
}

#[test]
fn go_ast_inventory_token_only_scratch_stream_renders_and_reparses() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let core: &dyn RawSyntaxStateMachineCore = &model;

    let seed = go_ast_inventory_scratch_var_decl_seed(core)
        .expect("reusable Go AST scratch seed generator should emit an admitted token stream");
    assert_eq!(seed.root, GO_AST_INVENTORY_SCRATCH_VAR_DECL_ROOT);
    assert_eq!(seed.contract, GO_AST_INVENTORY_SCRATCH_VAR_DECL_CONTRACT);
    assert_eq!(
        seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_RENDERED_SOURCE
    );
    let raw_token_ids = seed.raw_token_ids.clone();
    let roles = model.token_roles();
    assert_eq!(
        raw_token_ids.first().copied(),
        Some(roles.node_start),
        "scratch seed starts from generic GoSourceFile node_start"
    );
    for node_kind in ["GenDecl", "ValueSpec", "BasicLit"] {
        let alternative_start = roles
            .node_start_alternative_tokens
            .get(node_kind)
            .copied()
            .unwrap_or_else(|| panic!("missing alternative node-start token for {node_kind}"));
        assert!(
            raw_token_ids.contains(&alternative_start),
            "{node_kind} should be selected by an alternative-specific node_start token"
        );
    }
    assert!(
        raw_token_ids
            .iter()
            .filter(|token| **token == roles.node_start)
            .count()
            >= 3,
        "File and Ident fields should keep generic single-alternative node_start selection explicit"
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_PACKAGE_NAME,
        "var",
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                raw_token_ids.contains(&(roles.utf8_range.start + u32::from(byte))),
                "scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let admissibility = &seed.admissibility;
    assert_eq!(admissibility.raw_token_count, raw_token_ids.len());
    assert_eq!(
        admissibility.admissible_mask_query_count,
        raw_token_ids.len()
    );
    assert!(admissibility.finite_admissible_mask_available);
    assert!(admissibility.every_selected_raw_token_admitted);
    assert_eq!(
        admissibility.selected_transition_accepted_count,
        raw_token_ids.len()
    );
    assert_eq!(admissibility.steps.len(), raw_token_ids.len());
    assert_eq!(admissibility.final_state.status, RawSyntaxStatus::Complete);
    assert!(admissibility.final_state.eof_valid);
    for step in &admissibility.steps {
        assert_eq!(step.prefix_raw_token_count, step.step_index);
        assert!(step.finite_admissible_raw_token_count > 0);
        assert!(step.selected_raw_token_admitted);
        assert!(step.selected_transition_accepted);
    }

    let complete = core
        .strict_decode_complete(&raw_token_ids)
        .expect("token-only Go AST inventory scratch stream should decode completely");
    assert_eq!(complete.status, RawSyntaxStatus::Complete);
    assert!(complete.eof_valid);
    assert_eq!(complete.consumed_raw_tokens, raw_token_ids.len());
    assert!(complete.admissible.finite_raw_token_ids.is_empty());

    let decoded_tree = core
        .decode_complete_tree(&raw_token_ids)
        .expect("token-only scratch stream should materialize a raw decoded tree");
    assert_eq!(decoded_tree.root_rule(), Some("File"));
    assert_eq!(decoded_tree.root_node_kind(), Some("File"));
    assert_eq!(decoded_tree.node_start_count(), 6);
    assert!(decoded_tree.payload_token_byte_count() > 0);
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("decoded scratch stream should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind, "File");
    assert!(
        decoded_node_tree
            .field("Comments")
            .expect("File.Comments should be present")
            .repeated_empty
    );
    let go_version = decoded_node_tree
        .field("GoVersion")
        .expect("File.GoVersion should be present");
    assert_eq!(
        go_version.values.len(),
        1,
        "File.GoVersion is a required terminal payload"
    );
    let RawSyntaxDecodedValue::Terminal(go_version_terminal) = &go_version.values[0] else {
        panic!("File.GoVersion should decode as a terminal payload");
    };
    assert_eq!(
        go_version_terminal.payload,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_GO_VERSION
    );
    assert!(
        decoded_node_tree.node_kind_count("GenDecl") == 1
            && decoded_node_tree.node_kind_count("ValueSpec") == 1
            && decoded_node_tree.node_kind_count("BasicLit") == 1,
        "scratch stream should be the smallest rendered package-level var declaration"
    );

    let rendered = render_go_source_from_raw_tree(&decoded_tree)
        .expect("raw decoded scratch tree should render canonical Go");
    assert_eq!(
        rendered.source,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_RENDERED_SOURCE
    );
    assert_eq!(rendered.root_file_comment_group_count, 0);
    assert!(
        rendered
            .selected_generated_entrypoints
            .iter()
            .any(|entrypoint| entrypoint.generated_entrypoint == "render_generated_file_skeleton"),
        "rendering should enter through the generated File skeleton dispatch"
    );
    assert!(
        rendered
            .selected_generated_entrypoints
            .iter()
            .any(|entrypoint| {
                entrypoint.generated_entrypoint == "render_generated_value_spec_skeleton"
            }),
        "rendering should exercise generated ValueSpec field traversal"
    );

    assert_go_parser_formatter_accepts(&rendered.source);

    let import_spec_seed = go_ast_inventory_scratch_import_spec_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted ImportSpec stream");
    assert_eq!(
        import_spec_seed.root,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ROOT
    );
    assert_eq!(
        import_spec_seed.contract,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_CONTRACT
    );
    assert_eq!(
        import_spec_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PACKAGE_NAME,
        "import",
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ALIAS,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PATH_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PATH_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                import_spec_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "import-spec scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let import_spec_decoded_tree = core
        .decode_complete_tree(&import_spec_seed.raw_token_ids)
        .expect("import-spec scratch stream should materialize a raw decoded tree");
    let import_spec_decoded_node_tree = import_spec_decoded_tree
        .decoded_node_tree()
        .expect("import-spec scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("GenDecl", 1),
        ("ImportSpec", 1),
        ("Ident", 2),
        ("BasicLit", 1),
    ] {
        assert_eq!(
            import_spec_decoded_node_tree.node_kind_count(node_kind),
            count,
            "import-spec scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let import_spec_rendered = render_go_source_from_raw_tree(&import_spec_decoded_tree)
        .expect("raw decoded import-spec scratch tree should render canonical Go");
    assert_eq!(
        import_spec_rendered.source,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&import_spec_rendered.source);

    let function_seed = go_ast_inventory_scratch_function_return_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted function-return stream");
    assert_eq!(
        function_seed.root,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_ROOT
    );
    assert_eq!(
        function_seed.contract,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_CONTRACT
    );
    assert_eq!(
        function_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RENDERED_SOURCE
    );
    let function_raw_token_ids = function_seed.raw_token_ids.clone();
    assert_eq!(
        function_raw_token_ids.first().copied(),
        Some(roles.node_start),
        "function scratch seed starts from generic GoSourceFile node_start"
    );
    for node_kind in ["FuncDecl", "Ident", "ReturnStmt", "BasicLit"] {
        let alternative_start = roles
            .node_start_alternative_tokens
            .get(node_kind)
            .copied()
            .unwrap_or_else(|| panic!("missing alternative node-start token for {node_kind}"));
        assert!(
            function_raw_token_ids.contains(&alternative_start),
            "{node_kind} should be selected by an alternative-specific node_start token"
        );
    }
    assert!(
        function_raw_token_ids.contains(&roles.boolean_true),
        "function scratch seed should select the present FuncType.Func syntax marker"
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                function_raw_token_ids.contains(&(roles.utf8_range.start + u32::from(byte))),
                "function scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let function_admissibility = &function_seed.admissibility;
    assert_eq!(
        function_admissibility.raw_token_count,
        function_raw_token_ids.len()
    );
    assert_eq!(
        function_admissibility.admissible_mask_query_count,
        function_raw_token_ids.len()
    );
    assert!(function_admissibility.finite_admissible_mask_available);
    assert!(function_admissibility.every_selected_raw_token_admitted);
    assert_eq!(
        function_admissibility.selected_transition_accepted_count,
        function_raw_token_ids.len()
    );
    assert_eq!(
        function_admissibility.final_state.status,
        RawSyntaxStatus::Complete
    );
    assert!(function_admissibility.final_state.eof_valid);

    let function_decoded_tree = core
        .decode_complete_tree(&function_raw_token_ids)
        .expect("function-return scratch stream should materialize a raw decoded tree");
    assert_eq!(function_decoded_tree.root_rule(), Some("File"));
    assert_eq!(function_decoded_tree.root_node_kind(), Some("File"));
    assert!(function_decoded_tree.node_start_count() >= 12);
    let function_decoded_node_tree = function_decoded_tree
        .decoded_node_tree()
        .expect("function scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("FuncType", 1),
        ("BlockStmt", 1),
        ("ReturnStmt", 1),
        ("BasicLit", 1),
    ] {
        assert_eq!(
            function_decoded_node_tree.node_kind_count(node_kind),
            count,
            "function scratch stream should contain {count} {node_kind} node(s)"
        );
    }

    let function_rendered = render_go_source_from_raw_tree(&function_decoded_tree)
        .expect("raw decoded function scratch tree should render canonical Go");
    assert_eq!(
        function_rendered.source,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RENDERED_SOURCE
    );
    assert!(
        function_rendered
            .selected_generated_entrypoints
            .iter()
            .any(|entrypoint| {
                entrypoint.generated_entrypoint == "render_generated_return_stmt_skeleton"
            }),
        "rendering should exercise generated ReturnStmt traversal"
    );
    assert_go_parser_formatter_accepts(&function_rendered.source);

    let array_type_seed = go_ast_inventory_scratch_array_type_return_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted ArrayType stream");
    let array_type_decoded_tree = core
        .decode_complete_tree(&array_type_seed.raw_token_ids)
        .expect("array-type scratch stream should materialize a raw decoded tree");
    let array_type_decoded_node_tree = array_type_decoded_tree
        .decoded_node_tree()
        .expect("array-type scratch stream should materialize a nested node tree");
    for node_kind in ["ArrayType", "Ident", "ReturnStmt"] {
        assert!(
            array_type_decoded_node_tree.node_kind_count(node_kind) >= 1,
            "array-type scratch stream should contain {node_kind}"
        );
    }
    let array_type_rendered = render_go_source_from_raw_tree(&array_type_decoded_tree)
        .expect("raw decoded array-type scratch tree should render canonical Go");
    assert_eq!(
        array_type_rendered.source,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&array_type_rendered.source);

    let chan_type_seed = go_ast_inventory_scratch_chan_type_return_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted ChanType stream");
    assert_eq!(
        chan_type_seed.root,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ROOT
    );
    assert_eq!(
        chan_type_seed.contract,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_CONTRACT
    );
    assert_eq!(
        chan_type_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_DIRECTION,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ELEMENT_TYPE,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RETURN_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                chan_type_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "channel-type scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let chan_type_decoded_tree = core
        .decode_complete_tree(&chan_type_seed.raw_token_ids)
        .expect("channel-type scratch stream should materialize a raw decoded tree");
    let chan_type_decoded_node_tree = chan_type_decoded_tree
        .decoded_node_tree()
        .expect("channel-type scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("ChanType", 1),
        ("Ident", 4),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            chan_type_decoded_node_tree.node_kind_count(node_kind),
            count,
            "channel-type scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let chan_type_rendered = render_go_source_from_raw_tree(&chan_type_decoded_tree)
        .expect("raw decoded channel-type scratch tree should render canonical Go");
    assert_eq!(
        chan_type_rendered.source,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&chan_type_rendered.source);

    let selector_seed = go_ast_inventory_scratch_selector_call_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted selector-call stream");
    let selector_decoded_tree = core
        .decode_complete_tree(&selector_seed.raw_token_ids)
        .expect("selector-call scratch stream should materialize a raw decoded tree");
    let selector_decoded_node_tree = selector_decoded_tree
        .decoded_node_tree()
        .expect("selector scratch stream should materialize a nested node tree");
    for node_kind in ["CallExpr", "SelectorExpr", "ReturnStmt"] {
        assert_eq!(
            selector_decoded_node_tree.node_kind_count(node_kind),
            1,
            "selector-call scratch stream should contain one {node_kind} node"
        );
    }
    let selector_rendered = render_go_source_from_raw_tree(&selector_decoded_tree)
        .expect("raw decoded selector-call scratch tree should render canonical Go");
    assert_eq!(
        selector_rendered.source,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&selector_rendered.source);

    let assignment_seed = go_ast_inventory_scratch_assignment_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted assignment stream");
    assert_eq!(
        assignment_seed.root,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_ROOT
    );
    assert_eq!(
        assignment_seed.contract,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_CONTRACT
    );
    assert_eq!(
        assignment_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LHS_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_OPERATOR,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                assignment_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "assignment scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let assignment_decoded_tree = core
        .decode_complete_tree(&assignment_seed.raw_token_ids)
        .expect("assignment scratch stream should materialize a raw decoded tree");
    let assignment_decoded_node_tree = assignment_decoded_tree
        .decoded_node_tree()
        .expect("assignment scratch stream should materialize a nested node tree");
    for (node_kind, count) in [("AssignStmt", 1), ("BasicLit", 1), ("ReturnStmt", 1)] {
        assert_eq!(
            assignment_decoded_node_tree.node_kind_count(node_kind),
            count,
            "assignment scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let assignment_rendered = render_go_source_from_raw_tree(&assignment_decoded_tree)
        .expect("raw decoded assignment scratch tree should render canonical Go");
    assert_eq!(
        assignment_rendered.source,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&assignment_rendered.source);

    let binary_expr_seed = go_ast_inventory_scratch_binary_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted binary-expression stream");
    assert_eq!(
        binary_expr_seed.root,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_ROOT
    );
    assert_eq!(
        binary_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_CONTRACT
    );
    assert_eq!(
        binary_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_LEFT_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_LEFT_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_OPERATOR,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RIGHT_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RIGHT_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                binary_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "binary-expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let binary_expr_decoded_tree = core
        .decode_complete_tree(&binary_expr_seed.raw_token_ids)
        .expect("binary-expression scratch stream should materialize a raw decoded tree");
    let binary_expr_decoded_node_tree = binary_expr_decoded_tree
        .decoded_node_tree()
        .expect("binary-expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BinaryExpr", 1),
        ("BasicLit", 2),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            binary_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "binary-expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let binary_expr_rendered = render_go_source_from_raw_tree(&binary_expr_decoded_tree)
        .expect("raw decoded binary-expression scratch tree should render canonical Go");
    assert_eq!(
        binary_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&binary_expr_rendered.source);

    let unary_expr_seed = go_ast_inventory_scratch_unary_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted unary-expression stream");
    assert_eq!(
        unary_expr_seed.root,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_ROOT
    );
    assert_eq!(
        unary_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_CONTRACT
    );
    assert_eq!(
        unary_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_OPERATOR,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_TARGET_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                unary_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "unary-expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let unary_expr_decoded_tree = core
        .decode_complete_tree(&unary_expr_seed.raw_token_ids)
        .expect("unary-expression scratch stream should materialize a raw decoded tree");
    let unary_expr_decoded_node_tree = unary_expr_decoded_tree
        .decoded_node_tree()
        .expect("unary-expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("UnaryExpr", 1),
        ("Ident", 4),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            unary_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "unary-expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let unary_expr_rendered = render_go_source_from_raw_tree(&unary_expr_decoded_tree)
        .expect("raw decoded unary-expression scratch tree should render canonical Go");
    assert_eq!(
        unary_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&unary_expr_rendered.source);

    let paren_expr_seed = go_ast_inventory_scratch_paren_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted ParenExpr stream");
    assert_eq!(
        paren_expr_seed.root,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_ROOT
    );
    assert_eq!(
        paren_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_CONTRACT
    );
    assert_eq!(
        paren_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                paren_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "parenthesized-expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let paren_expr_decoded_tree = core
        .decode_complete_tree(&paren_expr_seed.raw_token_ids)
        .expect("parenthesized-expression scratch stream should materialize a raw decoded tree");
    let paren_expr_decoded_node_tree = paren_expr_decoded_tree
        .decoded_node_tree()
        .expect("parenthesized-expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("ParenExpr", 1),
        ("BasicLit", 1),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            paren_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "parenthesized-expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let paren_expr_rendered = render_go_source_from_raw_tree(&paren_expr_decoded_tree)
        .expect("raw decoded parenthesized-expression scratch tree should render canonical Go");
    assert_eq!(
        paren_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&paren_expr_rendered.source);

    let composite_lit_seed = go_ast_inventory_scratch_composite_lit_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted composite-literal stream");
    assert_eq!(
        composite_lit_seed.root,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ROOT
    );
    assert_eq!(
        composite_lit_seed.contract,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_CONTRACT
    );
    assert_eq!(
        composite_lit_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ELEMENT_TYPE,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                composite_lit_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "composite-literal scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    assert!(
        composite_lit_seed
            .raw_token_ids
            .contains(&roles.boolean_false),
        "composite-literal scratch seed should select CompositeLit.Incomplete=false"
    );
    let composite_lit_decoded_tree = core
        .decode_complete_tree(&composite_lit_seed.raw_token_ids)
        .expect("composite-literal scratch stream should materialize a raw decoded tree");
    let composite_lit_decoded_node_tree = composite_lit_decoded_tree
        .decoded_node_tree()
        .expect("composite-literal scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("ArrayType", 2),
        ("CompositeLit", 1),
        ("BasicLit", 1),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            composite_lit_decoded_node_tree.node_kind_count(node_kind),
            count,
            "composite-literal scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let composite_lit_rendered = render_go_source_from_raw_tree(&composite_lit_decoded_tree)
        .expect("raw decoded composite-literal scratch tree should render canonical Go");
    assert_eq!(
        composite_lit_rendered.source,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&composite_lit_rendered.source);

    let ellipsis_seed = go_ast_inventory_scratch_ellipsis_array_literal_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted Ellipsis stream");
    assert_eq!(
        ellipsis_seed.root,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ROOT
    );
    assert_eq!(
        ellipsis_seed.contract,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_CONTRACT
    );
    assert_eq!(
        ellipsis_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_PACKAGE_NAME,
        "var",
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ELEMENT_TYPE,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                ellipsis_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "ellipsis array-literal scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    assert!(
        ellipsis_seed.raw_token_ids.contains(&roles.boolean_false),
        "ellipsis array-literal scratch seed should select CompositeLit.Incomplete=false"
    );
    let ellipsis_decoded_tree = core
        .decode_complete_tree(&ellipsis_seed.raw_token_ids)
        .expect("ellipsis scratch stream should materialize a raw decoded tree");
    let ellipsis_decoded_node_tree = ellipsis_decoded_tree
        .decoded_node_tree()
        .expect("ellipsis scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("GenDecl", 1),
        ("ValueSpec", 1),
        ("CompositeLit", 1),
        ("ArrayType", 1),
        ("Ellipsis", 1),
        ("BasicLit", 1),
    ] {
        assert_eq!(
            ellipsis_decoded_node_tree.node_kind_count(node_kind),
            count,
            "ellipsis scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let ellipsis_rendered = render_go_source_from_raw_tree(&ellipsis_decoded_tree)
        .expect("raw decoded Ellipsis scratch tree should render canonical Go");
    assert_eq!(
        ellipsis_rendered.source,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&ellipsis_rendered.source);

    let decl_stmt_seed = go_ast_inventory_scratch_decl_stmt_seed(core).expect(
        "Go AST scratch seed generator should emit an admitted declaration-statement stream",
    );
    assert_eq!(decl_stmt_seed.root, GO_AST_INVENTORY_SCRATCH_DECL_STMT_ROOT);
    assert_eq!(
        decl_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_CONTRACT
    );
    assert_eq!(
        decl_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_DECL_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_RETURN_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                decl_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "declaration-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let decl_stmt_decoded_tree = core
        .decode_complete_tree(&decl_stmt_seed.raw_token_ids)
        .expect("declaration-statement scratch stream should materialize a raw decoded tree");
    let decl_stmt_decoded_node_tree = decl_stmt_decoded_tree
        .decoded_node_tree()
        .expect("declaration-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("DeclStmt", 1),
        ("GenDecl", 1),
        ("ValueSpec", 1),
        ("BasicLit", 1),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            decl_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "declaration-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let decl_stmt_rendered = render_go_source_from_raw_tree(&decl_stmt_decoded_tree)
        .expect("raw decoded declaration-statement scratch tree should render canonical Go");
    assert_eq!(
        decl_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&decl_stmt_rendered.source);

    let defer_stmt_seed = go_ast_inventory_scratch_defer_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted defer-statement stream");
    assert_eq!(
        defer_stmt_seed.root,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_ROOT
    );
    assert_eq!(
        defer_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CONTRACT
    );
    assert_eq!(
        defer_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CALL_RECEIVER,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CALL_SELECTOR,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RETURN_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RETURN_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                defer_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "defer-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let defer_stmt_decoded_tree = core
        .decode_complete_tree(&defer_stmt_seed.raw_token_ids)
        .expect("defer-statement scratch stream should materialize a raw decoded tree");
    let defer_stmt_decoded_node_tree = defer_stmt_decoded_tree
        .decoded_node_tree()
        .expect("defer-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("DeferStmt", 1),
        ("CallExpr", 1),
        ("SelectorExpr", 1),
        ("BasicLit", 1),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            defer_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "defer-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let defer_stmt_rendered = render_go_source_from_raw_tree(&defer_stmt_decoded_tree)
        .expect("raw decoded defer-statement scratch tree should render canonical Go");
    assert_eq!(
        defer_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&defer_stmt_rendered.source);

    let go_stmt_seed = go_ast_inventory_scratch_go_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted go-statement stream");
    assert_eq!(go_stmt_seed.root, GO_AST_INVENTORY_SCRATCH_GO_STMT_ROOT);
    assert_eq!(
        go_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_CONTRACT
    );
    assert_eq!(
        go_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_GO_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_CALL_RECEIVER,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_CALL_SELECTOR,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                go_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "go-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let go_stmt_decoded_tree = core
        .decode_complete_tree(&go_stmt_seed.raw_token_ids)
        .expect("go-statement scratch stream should materialize a raw decoded tree");
    let go_stmt_decoded_node_tree = go_stmt_decoded_tree
        .decoded_node_tree()
        .expect("go-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("GoStmt", 1),
        ("CallExpr", 1),
        ("SelectorExpr", 1),
    ] {
        assert_eq!(
            go_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "go-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let go_stmt_rendered = render_go_source_from_raw_tree(&go_stmt_decoded_tree)
        .expect("raw decoded go-statement scratch tree should render canonical Go");
    assert_eq!(
        go_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&go_stmt_rendered.source);
    assert_eq!(
        go_stmt_seed.inventory_synthesis_api_id.as_deref(),
        Some(GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        go_stmt_seed.inventory_synthesis_core_api_id.as_deref(),
        Some(GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID)
    );

    let if_stmt_seed = go_ast_inventory_scratch_if_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted if-statement stream");
    assert_eq!(if_stmt_seed.root, GO_AST_INVENTORY_SCRATCH_IF_STMT_ROOT);
    assert_eq!(
        if_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_CONTRACT
    );
    assert_eq!(
        if_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_IF_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_OPERATOR,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_COND_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_BODY_RETURN_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_BODY_RETURN_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_ELSE_RETURN_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_ELSE_RETURN_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                if_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "if-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let if_stmt_decoded_tree = core
        .decode_complete_tree(&if_stmt_seed.raw_token_ids)
        .expect("if-statement scratch stream should materialize a raw decoded tree");
    let if_stmt_decoded_node_tree = if_stmt_decoded_tree
        .decoded_node_tree()
        .expect("if-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 3),
        ("IfStmt", 1),
        ("AssignStmt", 1),
        ("Ident", 5),
        ("BasicLit", 3),
        ("ReturnStmt", 2),
    ] {
        assert_eq!(
            if_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "if-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let if_stmt_rendered = render_go_source_from_raw_tree(&if_stmt_decoded_tree)
        .expect("raw decoded if-statement scratch tree should render canonical Go");
    assert_eq!(
        if_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&if_stmt_rendered.source);
    assert_eq!(
        if_stmt_seed.inventory_synthesis_api_id.as_deref(),
        Some(GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        if_stmt_seed.inventory_synthesis_core_api_id.as_deref(),
        Some(GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID)
    );

    let branch_stmt_seed = go_ast_inventory_scratch_branch_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted branch-statement stream");
    assert_eq!(
        branch_stmt_seed.root,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_ROOT
    );
    assert_eq!(
        branch_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_CONTRACT
    );
    assert_eq!(
        branch_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_TOKEN,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RETURN_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RETURN_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                branch_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "branch-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let branch_stmt_decoded_tree = core
        .decode_complete_tree(&branch_stmt_seed.raw_token_ids)
        .expect("branch-statement scratch stream should materialize a raw decoded tree");
    let branch_stmt_decoded_node_tree = branch_stmt_decoded_tree
        .decoded_node_tree()
        .expect("branch-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("ForStmt", 1),
        ("BranchStmt", 1),
        ("BasicLit", 1),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            branch_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "branch-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let branch_stmt_rendered = render_go_source_from_raw_tree(&branch_stmt_decoded_tree)
        .expect("raw decoded branch-statement scratch tree should render canonical Go");
    assert_eq!(
        branch_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&branch_stmt_rendered.source);

    let range_stmt_seed = go_ast_inventory_scratch_range_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted range-statement stream");
    assert_eq!(
        range_stmt_seed.root,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_ROOT
    );
    assert_eq!(
        range_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_CONTRACT
    );
    assert_eq!(
        range_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_RENDERED_SOURCE
    );
    assert_eq!(
        range_stmt_seed.inventory_synthesis_api_id.as_deref(),
        Some(GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        range_stmt_seed.inventory_synthesis_core_api_id.as_deref(),
        Some(GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID)
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_KEY_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_TOKEN,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_SOURCE_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                range_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "range-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let range_stmt_decoded_tree = core
        .decode_complete_tree(&range_stmt_seed.raw_token_ids)
        .expect("range-statement scratch stream should materialize a raw decoded tree");
    let range_stmt_decoded_node_tree = range_stmt_decoded_tree
        .decoded_node_tree()
        .expect("range-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("RangeStmt", 1),
        ("BlockStmt", 2),
        ("Ident", 4),
    ] {
        assert_eq!(
            range_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "range-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let range_stmt_rendered = render_go_source_from_raw_tree(&range_stmt_decoded_tree)
        .expect("raw decoded range-statement scratch tree should render canonical Go");
    assert_eq!(
        range_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&range_stmt_rendered.source);

    let send_stmt_seed = go_ast_inventory_scratch_send_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted send-statement stream");
    assert_eq!(send_stmt_seed.root, GO_AST_INVENTORY_SCRATCH_SEND_STMT_ROOT);
    assert_eq!(
        send_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_CONTRACT
    );
    assert_eq!(
        send_stmt_seed.inventory_synthesis_api_id.as_deref(),
        Some(GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        send_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_CHANNEL_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_VALUE_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                send_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "send-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let send_stmt_decoded_tree = core
        .decode_complete_tree(&send_stmt_seed.raw_token_ids)
        .expect("send-statement scratch stream should materialize a raw decoded tree");
    let send_stmt_decoded_node_tree = send_stmt_decoded_tree
        .decoded_node_tree()
        .expect("send-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("SendStmt", 1),
        ("Ident", 4),
    ] {
        assert_eq!(
            send_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "send-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let send_stmt_rendered = render_go_source_from_raw_tree(&send_stmt_decoded_tree)
        .expect("raw decoded send-statement scratch tree should render canonical Go");
    assert_eq!(
        send_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&send_stmt_rendered.source);

    let case_clause_seed = go_ast_inventory_scratch_case_clause_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted case-clause stream");
    assert_eq!(
        case_clause_seed.root,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_ROOT
    );
    assert_eq!(
        case_clause_seed.contract,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CONTRACT
    );
    assert_eq!(
        case_clause_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SWITCH_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SWITCH_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CASE_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CASE_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RETURN_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RETURN_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                case_clause_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "case-clause scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let case_clause_decoded_tree = core
        .decode_complete_tree(&case_clause_seed.raw_token_ids)
        .expect("case-clause scratch stream should materialize a raw decoded tree");
    let case_clause_decoded_node_tree = case_clause_decoded_tree
        .decoded_node_tree()
        .expect("case-clause scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("SwitchStmt", 1),
        ("CaseClause", 1),
        ("BasicLit", 4),
        ("ReturnStmt", 2),
    ] {
        assert_eq!(
            case_clause_decoded_node_tree.node_kind_count(node_kind),
            count,
            "case-clause scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let case_clause_rendered = render_go_source_from_raw_tree(&case_clause_decoded_tree)
        .expect("raw decoded case-clause scratch tree should render canonical Go");
    assert_eq!(
        case_clause_rendered.source,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&case_clause_rendered.source);

    let comm_clause_seed = go_ast_inventory_scratch_comm_clause_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted comm-clause stream");
    assert_eq!(
        comm_clause_seed.root,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_ROOT
    );
    assert_eq!(
        comm_clause_seed.contract,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_CONTRACT
    );
    assert_eq!(
        comm_clause_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_GO_VERSION,
        "// tct: comm clause",
    ] {
        for byte in payload.bytes() {
            assert!(
                comm_clause_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "comm-clause scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let comm_clause_decoded_tree = core
        .decode_complete_tree(&comm_clause_seed.raw_token_ids)
        .expect("comm-clause scratch stream should materialize a raw decoded tree");
    let comm_clause_decoded_node_tree = comm_clause_decoded_tree
        .decoded_node_tree()
        .expect("comm-clause scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("SelectStmt", 1),
        ("CommClause", 1),
        ("CommentGroup", 1),
        ("Comment", 1),
    ] {
        assert_eq!(
            comm_clause_decoded_node_tree.node_kind_count(node_kind),
            count,
            "comm-clause scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let comm_clause_rendered = render_go_source_from_raw_tree(&comm_clause_decoded_tree)
        .expect("raw decoded comm-clause scratch tree should render canonical Go");
    assert_eq!(
        comm_clause_rendered.source,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&comm_clause_rendered.source);

    let empty_stmt_seed = go_ast_inventory_scratch_empty_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted EmptyStmt stream");
    assert_eq!(
        empty_stmt_seed.root,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_ROOT
    );
    assert_eq!(
        empty_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_CONTRACT
    );
    assert_eq!(
        empty_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                empty_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "empty-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    assert!(
        empty_stmt_seed.raw_token_ids.contains(&roles.boolean_false),
        "explicit empty-statement scratch seed should select EmptyStmt.Implicit=false"
    );
    let empty_stmt_decoded_tree = core
        .decode_complete_tree(&empty_stmt_seed.raw_token_ids)
        .expect("empty-statement scratch stream should materialize a raw decoded tree");
    let empty_stmt_decoded_node_tree = empty_stmt_decoded_tree
        .decoded_node_tree()
        .expect("empty-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [("FuncDecl", 1), ("BlockStmt", 1), ("EmptyStmt", 1)] {
        assert_eq!(
            empty_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "empty-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let empty_stmt_rendered = render_go_source_from_raw_tree(&empty_stmt_decoded_tree)
        .expect("raw decoded empty-statement scratch tree should render canonical Go");
    assert_eq!(
        empty_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&empty_stmt_rendered.source);

    let expr_stmt_seed = go_ast_inventory_scratch_expr_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted ExprStmt stream");
    assert_eq!(expr_stmt_seed.root, GO_AST_INVENTORY_SCRATCH_EXPR_STMT_ROOT);
    assert_eq!(
        expr_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CONTRACT
    );
    assert_eq!(
        expr_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CALL_RECEIVER,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CALL_SELECTOR,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                expr_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "expression-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let expr_stmt_decoded_tree = core
        .decode_complete_tree(&expr_stmt_seed.raw_token_ids)
        .expect("expression-statement scratch stream should materialize a raw decoded tree");
    let expr_stmt_decoded_node_tree = expr_stmt_decoded_tree
        .decoded_node_tree()
        .expect("expression-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("ExprStmt", 1),
        ("CallExpr", 1),
        ("SelectorExpr", 1),
    ] {
        assert_eq!(
            expr_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "expression-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let expr_stmt_rendered = render_go_source_from_raw_tree(&expr_stmt_decoded_tree)
        .expect("raw decoded expression-statement scratch tree should render canonical Go");
    assert_eq!(
        expr_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&expr_stmt_rendered.source);

    let inc_dec_stmt_seed = go_ast_inventory_scratch_inc_dec_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted IncDecStmt stream");
    assert_eq!(
        inc_dec_stmt_seed.root,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_ROOT
    );
    assert_eq!(
        inc_dec_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_CONTRACT
    );
    assert_eq!(
        inc_dec_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_TARGET_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_TOKEN,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                inc_dec_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "increment/decrement statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let inc_dec_stmt_decoded_tree = core
        .decode_complete_tree(&inc_dec_stmt_seed.raw_token_ids)
        .expect(
            "increment/decrement statement scratch stream should materialize a raw decoded tree",
        );
    let inc_dec_stmt_decoded_node_tree = inc_dec_stmt_decoded_tree.decoded_node_tree().expect(
        "increment/decrement statement scratch stream should materialize a nested node tree",
    );
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("IncDecStmt", 1),
        ("Ident", 3),
    ] {
        assert_eq!(
            inc_dec_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "increment/decrement statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let inc_dec_stmt_rendered = render_go_source_from_raw_tree(&inc_dec_stmt_decoded_tree).expect(
        "raw decoded increment/decrement statement scratch tree should render canonical Go",
    );
    assert_eq!(
        inc_dec_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&inc_dec_stmt_rendered.source);

    let index_expr_seed = go_ast_inventory_scratch_index_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted IndexExpr stream");
    assert_eq!(
        index_expr_seed.root,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_ROOT
    );
    assert_eq!(
        index_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_CONTRACT
    );
    assert_eq!(
        index_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_TARGET_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_INDEX_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_INDEX_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                index_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "index-expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let index_expr_decoded_tree = core
        .decode_complete_tree(&index_expr_seed.raw_token_ids)
        .expect("index-expression scratch stream should materialize a raw decoded tree");
    let index_expr_decoded_node_tree = index_expr_decoded_tree
        .decoded_node_tree()
        .expect("index-expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("ReturnStmt", 1),
        ("IndexExpr", 1),
        ("BasicLit", 1),
        ("Ident", 4),
    ] {
        assert_eq!(
            index_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "index-expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let index_expr_rendered = render_go_source_from_raw_tree(&index_expr_decoded_tree)
        .expect("raw decoded index-expression scratch tree should render canonical Go");
    assert_eq!(
        index_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&index_expr_rendered.source);

    let index_list_expr_seed = go_ast_inventory_scratch_index_list_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted IndexListExpr stream");
    assert_eq!(
        index_list_expr_seed.root,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_ROOT
    );
    assert_eq!(
        index_list_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_CONTRACT
    );
    assert_eq!(
        index_list_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_TARGET_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_FIRST_INDEX_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_SECOND_INDEX_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                index_list_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "index-list-expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let index_list_expr_decoded_tree = core
        .decode_complete_tree(&index_list_expr_seed.raw_token_ids)
        .expect("index-list-expression scratch stream should materialize a raw decoded tree");
    let index_list_expr_decoded_node_tree = index_list_expr_decoded_tree
        .decoded_node_tree()
        .expect("index-list-expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("ReturnStmt", 1),
        ("IndexListExpr", 1),
        ("Ident", 6),
    ] {
        assert_eq!(
            index_list_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "index-list-expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let index_list_expr_rendered = render_go_source_from_raw_tree(&index_list_expr_decoded_tree)
        .expect("raw decoded index-list-expression scratch tree should render canonical Go");
    assert_eq!(
        index_list_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&index_list_expr_rendered.source);

    let slice_expr_seed = go_ast_inventory_scratch_slice_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted SliceExpr stream");
    assert_eq!(
        slice_expr_seed.root,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_ROOT
    );
    assert_eq!(
        slice_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_CONTRACT
    );
    assert_eq!(
        slice_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_TARGET_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                slice_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "slice-expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    assert!(
        slice_expr_seed.raw_token_ids.contains(&roles.boolean_false),
        "slice-expression scratch seed should select SliceExpr.Slice3=false"
    );
    let slice_expr_decoded_tree = core
        .decode_complete_tree(&slice_expr_seed.raw_token_ids)
        .expect("slice-expression scratch stream should materialize a raw decoded tree");
    let slice_expr_decoded_node_tree = slice_expr_decoded_tree
        .decoded_node_tree()
        .expect("slice-expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("ReturnStmt", 1),
        ("SliceExpr", 1),
        ("BasicLit", 2),
        ("Ident", 4),
    ] {
        assert_eq!(
            slice_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "slice-expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let slice_expr_rendered = render_go_source_from_raw_tree(&slice_expr_decoded_tree)
        .expect("raw decoded slice-expression scratch tree should render canonical Go");
    assert_eq!(
        slice_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&slice_expr_rendered.source);

    let star_expr_seed = go_ast_inventory_scratch_star_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted StarExpr stream");
    assert_eq!(star_expr_seed.root, GO_AST_INVENTORY_SCRATCH_STAR_EXPR_ROOT);
    assert_eq!(
        star_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_CONTRACT
    );
    assert_eq!(
        star_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_TARGET_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                star_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "star-expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let star_expr_decoded_tree = core
        .decode_complete_tree(&star_expr_seed.raw_token_ids)
        .expect("star-expression scratch stream should materialize a raw decoded tree");
    let star_expr_decoded_node_tree = star_expr_decoded_tree
        .decoded_node_tree()
        .expect("star-expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("ReturnStmt", 1),
        ("StarExpr", 1),
        ("Ident", 4),
    ] {
        assert_eq!(
            star_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "star-expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let star_expr_rendered = render_go_source_from_raw_tree(&star_expr_decoded_tree)
        .expect("raw decoded star-expression scratch tree should render canonical Go");
    assert_eq!(
        star_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&star_expr_rendered.source);

    let type_assert_expr_seed = go_ast_inventory_scratch_type_assert_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted TypeAssertExpr stream");
    assert_eq!(
        type_assert_expr_seed.root,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ROOT
    );
    assert_eq!(
        type_assert_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_CONTRACT
    );
    assert_eq!(
        type_assert_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_TARGET_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ASSERTED_TYPE,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                type_assert_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "type-assertion expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let type_assert_expr_decoded_tree = core
        .decode_complete_tree(&type_assert_expr_seed.raw_token_ids)
        .expect("type-assertion expression scratch stream should materialize a raw decoded tree");
    let type_assert_expr_decoded_node_tree = type_assert_expr_decoded_tree
        .decoded_node_tree()
        .expect("type-assertion expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("ReturnStmt", 1),
        ("TypeAssertExpr", 1),
        ("Ident", 5),
    ] {
        assert_eq!(
            type_assert_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "type-assertion expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let type_assert_expr_rendered = render_go_source_from_raw_tree(&type_assert_expr_decoded_tree)
        .expect("raw decoded type-assertion expression scratch tree should render canonical Go");
    assert_eq!(
        type_assert_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&type_assert_expr_rendered.source);

    let type_switch_stmt_seed = go_ast_inventory_scratch_type_switch_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted TypeSwitchStmt stream");
    assert_eq!(
        type_switch_stmt_seed.root,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_ROOT
    );
    assert_eq!(
        type_switch_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_CONTRACT
    );
    assert_eq!(
        type_switch_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_TARGET_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                type_switch_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "type-switch statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let type_switch_stmt_decoded_tree = core
        .decode_complete_tree(&type_switch_stmt_seed.raw_token_ids)
        .expect("type-switch statement scratch stream should materialize a raw decoded tree");
    let type_switch_stmt_decoded_node_tree = type_switch_stmt_decoded_tree
        .decoded_node_tree()
        .expect("type-switch statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 2),
        ("TypeSwitchStmt", 1),
        ("ExprStmt", 1),
        ("TypeAssertExpr", 1),
        ("Ident", 3),
    ] {
        assert_eq!(
            type_switch_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "type-switch statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let type_switch_stmt_rendered = render_go_source_from_raw_tree(&type_switch_stmt_decoded_tree)
        .expect("raw decoded type-switch statement scratch tree should render canonical Go");
    assert_eq!(
        type_switch_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&type_switch_stmt_rendered.source);

    let struct_type_seed = go_ast_inventory_scratch_struct_type_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted StructType stream");
    assert_eq!(
        struct_type_seed.root,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_ROOT
    );
    assert_eq!(
        struct_type_seed.contract,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_CONTRACT
    );
    assert_eq!(
        struct_type_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_PACKAGE_NAME,
        "type",
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_TYPE_NAME,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_NAME,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_TYPE,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                struct_type_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "struct-type scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    assert!(
        struct_type_seed
            .raw_token_ids
            .contains(&roles.boolean_false),
        "struct-type scratch seed should select StructType.Incomplete=false"
    );
    let struct_type_decoded_tree = core
        .decode_complete_tree(&struct_type_seed.raw_token_ids)
        .expect("struct-type scratch stream should materialize a raw decoded tree");
    let struct_type_decoded_node_tree = struct_type_decoded_tree
        .decoded_node_tree()
        .expect("struct-type scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("GenDecl", 1),
        ("TypeSpec", 1),
        ("StructType", 1),
        ("FieldList", 1),
        ("Field", 1),
        ("Ident", 4),
    ] {
        assert_eq!(
            struct_type_decoded_node_tree.node_kind_count(node_kind),
            count,
            "struct-type scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let struct_type_rendered = render_go_source_from_raw_tree(&struct_type_decoded_tree)
        .expect("raw decoded struct-type scratch tree should render canonical Go");
    assert_eq!(
        struct_type_rendered.source,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&struct_type_rendered.source);

    let interface_type_seed = go_ast_inventory_scratch_interface_type_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted InterfaceType stream");
    assert_eq!(
        interface_type_seed.root,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_ROOT
    );
    assert_eq!(
        interface_type_seed.contract,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_CONTRACT
    );
    assert_eq!(
        interface_type_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                interface_type_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "interface-type scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    assert!(
        interface_type_seed
            .raw_token_ids
            .contains(&roles.boolean_false),
        "interface-type scratch seed should select InterfaceType.Incomplete=false"
    );
    let interface_type_decoded_tree = core
        .decode_complete_tree(&interface_type_seed.raw_token_ids)
        .expect("interface-type scratch stream should materialize a raw decoded tree");
    let interface_type_decoded_node_tree = interface_type_decoded_tree
        .decoded_node_tree()
        .expect("interface-type scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("GenDecl", 1),
        ("ValueSpec", 1),
        ("InterfaceType", 1),
        ("FieldList", 1),
        ("Ident", 2),
    ] {
        assert_eq!(
            interface_type_decoded_node_tree.node_kind_count(node_kind),
            count,
            "interface-type scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let interface_type_rendered = render_go_source_from_raw_tree(&interface_type_decoded_tree)
        .expect("raw decoded interface-type scratch tree should render canonical Go");
    assert_eq!(
        interface_type_rendered.source,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&interface_type_rendered.source);

    let key_value_expr_seed = go_ast_inventory_scratch_key_value_expr_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted KeyValueExpr stream");
    assert_eq!(
        key_value_expr_seed.root,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_ROOT
    );
    assert_eq!(
        key_value_expr_seed.contract,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_CONTRACT
    );
    assert_eq!(
        key_value_expr_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_TYPE,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_TYPE,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                key_value_expr_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "key-value-expression scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    assert!(
        key_value_expr_seed
            .raw_token_ids
            .contains(&roles.boolean_false),
        "key-value-expression scratch seed should select CompositeLit.Incomplete=false"
    );
    let key_value_expr_decoded_tree = core
        .decode_complete_tree(&key_value_expr_seed.raw_token_ids)
        .expect("key-value-expression scratch stream should materialize a raw decoded tree");
    let key_value_expr_decoded_node_tree = key_value_expr_decoded_tree
        .decoded_node_tree()
        .expect("key-value-expression scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("MapType", 2),
        ("CompositeLit", 1),
        ("KeyValueExpr", 1),
        ("BasicLit", 2),
        ("ReturnStmt", 1),
    ] {
        assert_eq!(
            key_value_expr_decoded_node_tree.node_kind_count(node_kind),
            count,
            "key-value-expression scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let key_value_expr_rendered = render_go_source_from_raw_tree(&key_value_expr_decoded_tree)
        .expect("raw decoded key-value-expression scratch tree should render canonical Go");
    assert_eq!(
        key_value_expr_rendered.source,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&key_value_expr_rendered.source);

    let func_lit_seed = go_ast_inventory_scratch_func_lit_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted FuncLit stream");
    assert_eq!(func_lit_seed.root, GO_AST_INVENTORY_SCRATCH_FUNC_LIT_ROOT);
    assert_eq!(
        func_lit_seed.contract,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_CONTRACT
    );
    assert_eq!(
        func_lit_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                func_lit_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "function-literal scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let func_lit_decoded_tree = core
        .decode_complete_tree(&func_lit_seed.raw_token_ids)
        .expect("function-literal scratch stream should materialize a raw decoded tree");
    let func_lit_decoded_node_tree = func_lit_decoded_tree
        .decoded_node_tree()
        .expect("function-literal scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("GenDecl", 1),
        ("ValueSpec", 1),
        ("FuncLit", 1),
        ("FuncType", 1),
        ("BlockStmt", 1),
    ] {
        assert_eq!(
            func_lit_decoded_node_tree.node_kind_count(node_kind),
            count,
            "function-literal scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let func_lit_rendered = render_go_source_from_raw_tree(&func_lit_decoded_tree)
        .expect("raw decoded function-literal scratch tree should render canonical Go");
    assert_eq!(
        func_lit_rendered.source,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&func_lit_rendered.source);

    let labeled_stmt_seed = go_ast_inventory_scratch_labeled_stmt_seed(core)
        .expect("Go AST scratch seed generator should emit an admitted LabeledStmt stream");
    assert_eq!(
        labeled_stmt_seed.root,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_ROOT
    );
    assert_eq!(
        labeled_stmt_seed.contract,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_CONTRACT
    );
    assert_eq!(
        labeled_stmt_seed.expected_rendered_source,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RENDERED_SOURCE
    );
    for payload in [
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RESULT_TYPE,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_LABEL_IDENTIFIER,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RETURN_LITERAL_KIND,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RETURN_LITERAL_VALUE,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_GO_VERSION,
    ] {
        for byte in payload.bytes() {
            assert!(
                labeled_stmt_seed
                    .raw_token_ids
                    .contains(&(roles.utf8_range.start + u32::from(byte))),
                "labeled-statement scratch seed should emit UTF-8 payload byte {byte} for {payload}"
            );
        }
    }
    let labeled_stmt_decoded_tree = core
        .decode_complete_tree(&labeled_stmt_seed.raw_token_ids)
        .expect("labeled-statement scratch stream should materialize a raw decoded tree");
    let labeled_stmt_decoded_node_tree = labeled_stmt_decoded_tree
        .decoded_node_tree()
        .expect("labeled-statement scratch stream should materialize a nested node tree");
    for (node_kind, count) in [
        ("FuncDecl", 1),
        ("BlockStmt", 1),
        ("LabeledStmt", 1),
        ("Ident", 4),
        ("ReturnStmt", 1),
        ("BasicLit", 1),
    ] {
        assert_eq!(
            labeled_stmt_decoded_node_tree.node_kind_count(node_kind),
            count,
            "labeled-statement scratch stream should contain {count} {node_kind} node(s)"
        );
    }
    let labeled_stmt_rendered = render_go_source_from_raw_tree(&labeled_stmt_decoded_tree)
        .expect("raw decoded labeled-statement scratch tree should render canonical Go");
    assert_eq!(
        labeled_stmt_rendered.source,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RENDERED_SOURCE
    );
    assert_go_parser_formatter_accepts(&labeled_stmt_rendered.source);
    assert_eq!(
        labeled_stmt_seed.inventory_synthesis_api_id.as_deref(),
        Some(GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID)
    );
    assert_eq!(
        labeled_stmt_seed.inventory_synthesis_core_api_id.as_deref(),
        Some(GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID)
    );

    let seeds =
        go_ast_inventory_scratch_seeds(core).expect("scratch seed list should build all seeds");
    assert_eq!(seeds.len(), 36);
    assert!(seeds.iter().any(|seed| seed.raw_token_ids == raw_token_ids));
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == import_spec_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == struct_type_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == type_assert_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == type_switch_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == unary_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == function_raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == array_type_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == chan_type_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == selector_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == index_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == index_list_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == slice_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == star_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == interface_type_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == key_value_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == assignment_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == binary_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == paren_expr_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == composite_lit_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == ellipsis_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == decl_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == defer_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == go_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == if_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == branch_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == range_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == send_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == case_clause_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == comm_clause_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == empty_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == expr_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == inc_dec_stmt_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == func_lit_seed.raw_token_ids)
    );
    assert!(
        seeds
            .iter()
            .any(|seed| seed.raw_token_ids == labeled_stmt_seed.raw_token_ids)
    );
}

#[test]
fn checked_go_ast_schema_inventory_classifies_raw_tree_renderer_codegen_coverage() {
    let coverage = GoAstSchemaInventoryAdapter::raw_tree_renderer_codegen_coverage_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should classify renderer codegen coverage");

    coverage
        .validate()
        .expect("renderer codegen coverage should validate");
    assert_eq!(
        coverage.status,
        GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY
    );
    assert_eq!(coverage.tokenizable_node_count, 53);
    assert_eq!(
        coverage.classified_node_count,
        coverage.tokenizable_node_count
    );
    assert_eq!(coverage.missing_policy_count, 0);
    assert!(coverage.first_blocker.is_none());
    assert!(
        coverage.generated_skeleton_ready_count >= 50,
        "every source-file go/ast node should have generated skeleton coverage"
    );
    assert!(
        coverage.manual_policy_hook_node_count > 0,
        "semantic renderer policy should stay explicit instead of being hidden in source batches"
    );
    let package = coverage
        .node_coverages
        .iter()
        .find(|node| node.node_kind == "Package")
        .expect("Package should be classified explicitly");
    assert!(package.source_file_root_excluded);
    assert!(!package.generated_skeleton_ready);
    let field = coverage
        .node_coverages
        .iter()
        .find(|node| node.node_kind == "Field")
        .expect("Field should be classified explicitly");
    assert!(field.contextual_renderer);
    assert!(field.generated_skeleton_ready);
    let empty_stmt = coverage
        .node_coverages
        .iter()
        .find(|node| node.node_kind == "EmptyStmt")
        .expect("EmptyStmt should be classified explicitly");
    assert!(
        empty_stmt
            .manual_policy_hooks
            .iter()
            .any(|hook| hook == "fixed_semicolon_grammar_mode"),
        "EmptyStmt should use the grammar-mode semicolon policy"
    );
    assert_eq!(
        coverage.next_ready_task,
        GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_NEXT_READY_TASK
    );
    assert_ne!(
        coverage.next_ready_task,
        STALE_SOURCE_PROGRAM_SUCCESSOR_TASK
    );

    let dispatch_audit = go_raw_tree_renderer_skeleton_dispatch_audit();
    assert_eq!(
        dispatch_audit.inventory_generated_skeleton_node_count,
        coverage.generated_skeleton_ready_count,
        "renderer dispatch should be driven from the inventory-generated skeleton policy table"
    );
    assert_eq!(
        dispatch_audit.dispatch_node_count, coverage.generated_skeleton_ready_count,
        "every generated-skeleton-ready node should have one raw-tree dispatch entry"
    );
    assert!(
        dispatch_audit
            .missing_generated_skeleton_node_kinds
            .is_empty(),
        "renderer dispatch is missing generated-skeleton node kinds: {:?}",
        dispatch_audit.missing_generated_skeleton_node_kinds
    );
    assert!(
        dispatch_audit.extra_dispatch_node_kinds.is_empty(),
        "renderer dispatch should not contain nodes outside the inventory skeleton policy table: {:?}",
        dispatch_audit.extra_dispatch_node_kinds
    );
    for expected_contextual in ["BlockStmt", "CommentGroup", "Field", "FieldList"] {
        assert!(
            dispatch_audit
                .contextual_dispatch_node_kinds
                .iter()
                .any(|node_kind| node_kind == expected_contextual),
            "{expected_contextual} should remain an explicit contextual renderer hook"
        );
    }

    let contract = go_raw_tree_renderer_skeleton_generation_contract(&coverage)
        .expect("renderer skeleton generation contract should build from checked coverage");
    contract
        .validate()
        .expect("renderer skeleton generation contract should validate");
    assert_eq!(
        contract.artifact_kind,
        GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND
    );
    assert_eq!(
        contract.status,
        GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY
    );
    assert_eq!(
        contract.inventory_artifact_path,
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH
    );
    assert_eq!(
        contract.inventory_generated_skeleton_node_count,
        coverage.generated_skeleton_ready_count
    );
    assert_eq!(
        contract.dispatch_node_count,
        coverage.generated_skeleton_ready_count
    );
    assert_eq!(
        contract.generated_skeletons.len(),
        coverage.generated_skeleton_ready_count
    );
    assert!(
        contract
            .source_file_root_exclusions
            .iter()
            .any(|exclusion| exclusion.node_kind == "Package"
                && exclusion.reason.contains("source root is File")),
        "go/ast.Package should stay outside the generated source-file skeleton root"
    );

    let file = contract
        .generated_skeletons
        .iter()
        .find(|node| node.node_kind == "File")
        .expect("File should be a generated skeleton root");
    assert_eq!(file.dispatch_contexts, vec!["source_file_root".to_string()]);
    assert!(
        file.manual_policy_hooks
            .iter()
            .any(|hook| hook == "source_file_layout")
    );

    let field = contract
        .generated_skeletons
        .iter()
        .find(|node| node.node_kind == "Field")
        .expect("Field should be a contextual generated skeleton");
    assert!(field.contextual_renderer);
    assert_eq!(field.dispatch_contexts, vec!["contextual".to_string()]);
    assert!(
        field
            .manual_policy_hooks
            .iter()
            .any(|hook| hook == "field_comment_layout")
    );

    let empty_stmt = contract
        .generated_skeletons
        .iter()
        .find(|node| node.node_kind == "EmptyStmt")
        .expect("EmptyStmt should retain its manual semicolon policy hook");
    assert!(
        empty_stmt
            .manual_policy_hooks
            .iter()
            .any(|hook| hook == "fixed_semicolon_grammar_mode")
    );
    let comment_group = contract
        .generated_skeletons
        .iter()
        .find(|node| node.node_kind == "CommentGroup")
        .expect("CommentGroup should retain comment-trivia dispatch");
    assert_eq!(
        comment_group.dispatch_contexts,
        vec!["comment_trivia".to_string()]
    );
    assert!(
        contract.claim_boundary.contains("not validity authority"),
        "contract must keep parser/formatter/BPE sidecars out of the validity claim"
    );
}

#[test]
fn go_ast_inventory_declarative_composite_lit_return_spec_derives_checked_plan() {
    let spec = go_ast_inventory_composite_lit_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "CompositeLit");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect(
        "declarative CompositeLit array-literal context should validate against checked inventory",
    );
    for (rule_name, fields) in [
        (
            "CompositeLit",
            vec!["Type", "Elts", "Incomplete", "LineBreakBefore"],
        ),
        ("ArrayType", vec!["Len", "Elt"]),
        ("BasicLit", vec!["Kind", "Value"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 5);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "CompositeLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CompositeLit",
            "Type",
            "ArrayType",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CompositeLit",
            "Elts",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ArrayType",
            "Elt",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let elts_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Elts")
        .expect("normalized CompositeLit should preserve Elts field");
    match &elts_field.value {
        GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes)
        | GoAstInventoryDeclarativeFieldValue::RepeatedGenericNodes(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].node_kind, "BasicLit");
        }
        other => panic!("expected repeated Elts node policy, got {other:?}"),
    }
}

#[test]
fn go_ast_inventory_declarative_key_value_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_key_value_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "CompositeLit");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect(
        "declarative KeyValueExpr map-literal context should validate against checked inventory",
    );
    for (rule_name, fields) in [
        (
            "CompositeLit",
            vec!["Type", "Elts", "Incomplete", "LineBreakBefore"],
        ),
        ("MapType", vec!["Key", "Value"]),
        ("KeyValueExpr", vec!["Key", "Value"]),
        ("BasicLit", vec!["Kind", "Value"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 9);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "CompositeLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CompositeLit",
            "Type",
            "MapType",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CompositeLit",
            "Elts",
            "KeyValueExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        ("MapType", "Key", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "MapType",
            "Value",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "KeyValueExpr",
            "Key",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "KeyValueExpr",
            "Value",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.STRING",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let elts_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Elts")
        .expect("normalized CompositeLit should preserve Elts field");
    match &elts_field.value {
        GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].node_kind, "KeyValueExpr");
        }
        other => panic!("expected repeated Elts node policy, got {other:?}"),
    }

    let mut invalid = spec.expression.clone();
    let type_value = invalid
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Type")
        .expect("Type field should exist")
        .value
        .clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Elts")
        .expect("Elts field should exist")
        .value = type_value;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("single-node Elts policy should violate checked repeated-field metadata");
    assert!(
        format!("{error}").contains("repeated-alternative policy"),
        "unexpected declarative repeated-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_func_lit_spec_derives_checked_plan() {
    let spec = go_ast_inventory_func_lit_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ValueSpec");
    assert_eq!(spec.parent_field.field_name, "Values");
    assert_eq!(spec.expression.root_node_kind(), "FuncLit");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative FuncLit spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("FuncLit", vec!["Type", "Body"]),
        ("FuncType", vec!["Func", "TypeParams", "Params", "Results"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 2);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ValueSpec",
            "Values",
            "FuncLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let body_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Body")
        .expect("normalized FuncLit should preserve Body field");
    match &body_field.value {
        GoAstInventoryDeclarativeFieldValue::GenericNode(body) => {
            assert_eq!(body.node_kind, "BlockStmt");
            let list_field = body
                .fields
                .iter()
                .find(|field| field.field_name == "List")
                .expect("normalized BlockStmt should preserve List field");
            assert!(matches!(
                &list_field.value,
                GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes)
                    if nodes.is_empty()
            ));
        }
        other => panic!("expected generic Body block policy, got {other:?}"),
    }

    let mut invalid = spec.expression.clone();
    let body_value = invalid
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Body")
        .expect("Body field should exist")
        .value
        .clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Type")
        .expect("Type field should exist")
        .value = body_value;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("BlockStmt Type policy should violate checked FuncLit field metadata");
    assert!(
        format!("{error}").contains("field FuncLit.Type missing nonterminal alternative BlockStmt"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_array_type_return_spec_derives_checked_plan() {
    let spec = go_ast_inventory_array_type_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "Field");
    assert_eq!(spec.parent_field.field_name, "Type");
    assert_eq!(spec.expression.root_node_kind(), "ArrayType");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative ArrayType spec should validate against checked inventory");
    for (rule_name, fields) in [("ArrayType", vec!["Len", "Elt"]), ("Ident", vec!["Name"])] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 2);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "Field",
            "Type",
            "ArrayType",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ArrayType",
            "Elt",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let len_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Len")
        .expect("normalized ArrayType should preserve Len field");
    assert!(matches!(
        &len_field.value,
        GoAstInventoryDeclarativeFieldValue::Missing
    ));

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Elt")
        .expect("Elt field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Elt policy should violate checked ArrayType field metadata");
    assert!(
        format!("{error}")
            .contains("ArrayType.Elt cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_chan_type_return_spec_derives_checked_plan() {
    let spec = go_ast_inventory_chan_type_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "Field");
    assert_eq!(spec.parent_field.field_name, "Type");
    assert_eq!(spec.expression.root_node_kind(), "ChanType");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative ChanType spec should validate against checked inventory");
    for (rule_name, fields) in [("ChanType", vec!["Dir", "Value"]), ("Ident", vec!["Name"])] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 3);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "Field",
            "Type",
            "ChanType",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ChanType",
            "Dir",
            "go_ast_channel_directions.SEND_RECV",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "ChanType",
            "Value",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Value")
        .expect("Value field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Value policy should violate checked ChanType field metadata");
    assert!(
        format!("{error}")
            .contains("ChanType.Value cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_paren_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_paren_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "ParenExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative ParenExpr spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("ParenExpr", vec!["X"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 3);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "ParenExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ParenExpr",
            "X",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let mut invalid = spec.expression.clone();
    invalid.root.fields.clear();
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("omitted X policy should violate checked ParenExpr field metadata");
    assert!(
        format!("{error}").contains("must spell policy for checked field X"),
        "unexpected declarative missing-field error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_binary_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_binary_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "BinaryExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative BinaryExpr spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("BinaryExpr", vec!["X", "Op", "Y"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 5);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "BinaryExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BinaryExpr",
            "X",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BinaryExpr",
            "Op",
            "go_token_binary_operator_tokens.ADD",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "BinaryExpr",
            "Y",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Y")
        .expect("Y field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Y policy should violate checked BinaryExpr field metadata");
    assert!(
        format!("{error}")
            .contains("BinaryExpr.Y cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_index_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_index_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "IndexExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative IndexExpr spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("IndexExpr", vec!["X", "Index"]),
        ("Ident", vec!["Name"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 4);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "IndexExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        ("IndexExpr", "X", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "IndexExpr",
            "Index",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Index")
        .expect("Index field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Index policy should violate checked IndexExpr field metadata");
    assert!(
        format!("{error}")
            .contains("IndexExpr.Index cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_selector_call_spec_derives_checked_plan() {
    let spec = go_ast_inventory_selector_call_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "CallExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative SelectorCall spec should validate against checked inventory");
    for (rule_name, fields) in [
        (
            "CallExpr",
            vec!["Fun", "Args", "Ellipsis", "LineBreakBefore"],
        ),
        ("SelectorExpr", vec!["X", "Sel"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 4);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "CallExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Fun",
            "SelectorExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Ellipsis",
            "go_syntax_presence_call_expr_ellipsis.ABSENT",
            GrammarTerminalKind::BooleanPayload,
        ),
        (
            "SelectorExpr",
            "X",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let args_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Args")
        .expect("normalized CallExpr should preserve Args field");
    match &args_field.value {
        GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes) => {
            assert!(nodes.is_empty());
        }
        other => panic!("expected empty repeated Args policy, got {other:?}"),
    }
    let fun_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Fun")
        .expect("normalized CallExpr should preserve Fun field");
    let selector_fields = match &fun_field.value {
        GoAstInventoryDeclarativeFieldValue::AlternativeNode(node) => &node.fields,
        other => panic!("expected selector-call Fun alternative policy, got {other:?}"),
    };
    let sel_field = selector_fields
        .iter()
        .find(|field| field.field_name == "Sel")
        .expect("normalized SelectorExpr should preserve Sel field");
    assert!(
        matches!(
            &sel_field.value,
            GoAstInventoryDeclarativeFieldValue::GenericNode(_)
        ),
        "SelectorExpr.Sel should use the singleton generic-node policy"
    );

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Args")
        .expect("Args field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Args policy should violate checked CallExpr field metadata");
    assert!(
        format!("{error}")
            .contains("CallExpr.Args cannot use missing policy because it is a repeated field"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_decl_stmt_spec_derives_checked_required_decl_plan() {
    let spec = go_ast_inventory_decl_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "DeclStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative DeclStmt spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("DeclStmt", vec!["Decl"]),
        ("GenDecl", vec!["Doc", "Tok", "Lparen", "Specs", "Rparen"]),
        (
            "ValueSpec",
            vec!["Doc", "Names", "Type", "Values", "Comment"],
        ),
        ("BasicLit", vec!["Kind", "Value"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 6);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "DeclStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "DeclStmt",
            "Decl",
            "GenDecl",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "GenDecl",
            "Tok",
            "go_token_declaration_tokens.VAR",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "GenDecl",
            "Specs",
            "ValueSpec",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ValueSpec",
            "Values",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative DeclStmt spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let decl = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Decl")
        .expect("DeclStmt.Decl should exist");
    assert!(
        matches!(
            &decl.value,
            GoAstInventoryDeclarativeFieldValue::AlternativeNode(node) if node.node_kind == "GenDecl"
        ),
        "DeclStmt.Decl should use the checked required alternative-node policy"
    );

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Decl")
        .expect("DeclStmt.Decl should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Decl policy should violate checked DeclStmt field metadata");
    assert!(
        format!("{error}")
            .contains("DeclStmt.Decl cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_empty_stmt_spec_derives_checked_required_implicit_plan() {
    let spec = go_ast_inventory_empty_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "EmptyStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative EmptyStmt spec should validate against checked inventory");
    assert_eq!(
        checked
            .expected_field_orders
            .iter()
            .find(|order| order.rule_name == "EmptyStmt")
            .expect("EmptyStmt field order should be derived")
            .fields,
        vec!["Implicit".to_string()]
    );
    assert!(
        checked
            .finite_domain_requirements
            .iter()
            .any(|requirement| {
                requirement.rule_name == "BlockStmt"
                    && requirement.field_name == "List"
                    && requirement.selected_alternative == "EmptyStmt"
                    && requirement.terminal == GrammarTerminalKind::Nonterminal
            }),
        "declarative EmptyStmt spec should derive BlockStmt.List=EmptyStmt"
    );
    let implicit = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Implicit")
        .expect("EmptyStmt.Implicit should exist");
    assert!(
        matches!(
            &implicit.value,
            GoAstInventoryDeclarativeFieldValue::BooleanPayload {
                selected_alternative: Some(alternative),
                value: false,
            } if alternative == "EmptyStmt.Implicit.bool"
        ),
        "EmptyStmt.Implicit should use checked required boolean-payload coverage"
    );

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Implicit")
        .expect("EmptyStmt.Implicit should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Implicit policy should violate checked EmptyStmt field metadata");
    assert!(
        format!("{error}")
            .contains("EmptyStmt.Implicit cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_inc_dec_stmt_spec_derives_checked_required_target_plan() {
    let spec = go_ast_inventory_inc_dec_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "IncDecStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative IncDecStmt spec should validate against checked inventory");
    assert_eq!(
        checked
            .expected_field_orders
            .iter()
            .find(|order| order.rule_name == "IncDecStmt")
            .expect("IncDecStmt field order should be derived")
            .fields,
        vec!["X".to_string(), "Tok".to_string()]
    );
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "IncDecStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        ("IncDecStmt", "X", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "IncDecStmt",
            "Tok",
            "go_token_inc_dec_tokens.INC",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative IncDecStmt spec should derive {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let target = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "X")
        .expect("IncDecStmt.X should exist");
    assert!(
        matches!(
            &target.value,
            GoAstInventoryDeclarativeFieldValue::AlternativeNode(node) if node.node_kind == "Ident"
        ),
        "IncDecStmt.X should use checked required alternative-node coverage"
    );
    let token = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Tok")
        .expect("IncDecStmt.Tok should exist");
    assert!(
        matches!(
            &token.value,
            GoAstInventoryDeclarativeFieldValue::TerminalPayload {
                selected_alternative: Some(alternative),
                terminal_kind: GrammarTerminalKind::IdentifierPayload,
                payload,
            } if alternative == "go_token_inc_dec_tokens.INC" && payload == "++"
        ),
        "IncDecStmt.Tok should use checked required finite-token coverage"
    );

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "X")
        .expect("IncDecStmt.X should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing X policy should violate checked IncDecStmt field metadata");
    assert!(
        format!("{error}")
            .contains("IncDecStmt.X cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_defer_stmt_spec_derives_checked_optional_call_plan() {
    let spec = go_ast_inventory_defer_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "DeferStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative DeferStmt spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("DeferStmt", vec!["Call"]),
        (
            "CallExpr",
            vec!["Fun", "Args", "Ellipsis", "LineBreakBefore"],
        ),
        ("SelectorExpr", vec!["X", "Sel"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "DeferStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Fun",
            "SelectorExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Ellipsis",
            "go_syntax_presence_call_expr_ellipsis.ABSENT",
            GrammarTerminalKind::BooleanPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative DeferStmt spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let call_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Call")
        .expect("normalized DeferStmt should preserve Call field");
    assert!(
        matches!(
            &call_field.value,
            GoAstInventoryDeclarativeFieldValue::GenericNode(_)
        ),
        "DeferStmt.Call should use the optional singleton generic-node policy"
    );

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Call")
        .expect("Call field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(Vec::new());
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("repeated Call policy should violate checked DeferStmt field metadata");
    assert!(
        format!("{error}").contains("DeferStmt.Call is not repeated"),
        "unexpected declarative repeated-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_go_stmt_spec_derives_checked_optional_call_plan() {
    let spec = go_ast_inventory_go_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "GoStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative GoStmt spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("GoStmt", vec!["Call"]),
        (
            "CallExpr",
            vec!["Fun", "Args", "Ellipsis", "LineBreakBefore"],
        ),
        ("SelectorExpr", vec!["X", "Sel"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "GoStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Fun",
            "SelectorExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CallExpr",
            "Ellipsis",
            "go_syntax_presence_call_expr_ellipsis.ABSENT",
            GrammarTerminalKind::BooleanPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative GoStmt spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let call_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Call")
        .expect("normalized GoStmt should preserve Call field");
    assert!(
        matches!(
            &call_field.value,
            GoAstInventoryDeclarativeFieldValue::GenericNode(_)
        ),
        "GoStmt.Call should use the optional singleton generic-node policy"
    );

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Call")
        .expect("Call field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(Vec::new());
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("repeated Call policy should violate checked GoStmt field metadata");
    assert!(
        format!("{error}").contains("GoStmt.Call is not repeated"),
        "unexpected declarative repeated-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_if_stmt_spec_derives_checked_optional_init_plan() {
    let spec = go_ast_inventory_if_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "IfStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative IfStmt spec should validate against checked inventory");
    for (rule_name, fields) in [
        (
            "IfStmt",
            vec!["Init", "Cond", "Body", "Else", "TrailingComment"],
        ),
        ("AssignStmt", vec!["Lhs", "Tok", "Rhs"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
        ("BasicLit", vec!["Kind", "Value"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "IfStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "IfStmt",
            "Init",
            "AssignStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        ("IfStmt", "Cond", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "IfStmt",
            "Else",
            "BlockStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "AssignStmt",
            "Lhs",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "AssignStmt",
            "Tok",
            "go_token_assignment_tokens.DEFINE",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "AssignStmt",
            "Rhs",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ReturnStmt",
            "Results",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative IfStmt spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let init_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Init")
        .expect("normalized IfStmt should preserve Init field");
    assert!(
        matches!(
            &init_field.value,
            GoAstInventoryDeclarativeFieldValue::AlternativeNode(_)
        ),
        "IfStmt.Init should use the optional singleton alternative-node policy"
    );

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Init")
        .expect("Init field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(Vec::new());
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("repeated Init policy should violate checked IfStmt field metadata");
    assert!(
        format!("{error}").contains("IfStmt.Init is not repeated"),
        "unexpected declarative repeated-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_slice_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_slice_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "SliceExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative SliceExpr spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("SliceExpr", vec!["X", "Low", "High", "Max", "Slice3"]),
        ("Ident", vec!["Name"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 5);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "SliceExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        ("SliceExpr", "X", "Ident", GrammarTerminalKind::Nonterminal),
        (
            "SliceExpr",
            "Low",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "SliceExpr",
            "High",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let max_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Max")
        .expect("normalized SliceExpr should preserve Max field");
    assert!(matches!(
        &max_field.value,
        GoAstInventoryDeclarativeFieldValue::Missing
    ));
    let slice3_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Slice3")
        .expect("normalized SliceExpr should preserve Slice3 field");
    match &slice3_field.value {
        GoAstInventoryDeclarativeFieldValue::BooleanPayload {
            selected_alternative,
            value,
        } => {
            assert_eq!(
                selected_alternative.as_deref(),
                Some("SliceExpr.Slice3.bool")
            );
            assert!(!*value);
        }
        other => panic!("expected Slice3 boolean payload policy, got {other:?}"),
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "X")
        .expect("X field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing X policy should violate checked SliceExpr field metadata");
    assert!(
        format!("{error}").contains("SliceExpr.X cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_star_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_star_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "StarExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative StarExpr spec should validate against checked inventory");
    for (rule_name, fields) in [("StarExpr", vec!["X"]), ("Ident", vec!["Name"])] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 2);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "StarExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        ("StarExpr", "X", "Ident", GrammarTerminalKind::Nonterminal),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "X")
        .expect("X field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing X policy should violate checked StarExpr field metadata");
    assert!(
        format!("{error}").contains("StarExpr.X cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_struct_type_spec_derives_checked_plan() {
    let spec = go_ast_inventory_struct_type_decl_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "TypeSpec");
    assert_eq!(spec.parent_field.field_name, "Type");
    assert_eq!(spec.expression.root_node_kind(), "StructType");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative StructType spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("StructType", vec!["Fields", "Incomplete"]),
        ("FieldList", vec!["List", "BlankLineBefore"]),
        ("Field", vec!["Doc", "Names", "Type", "Tag", "Comment"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 2);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "TypeSpec",
            "Type",
            "StructType",
            GrammarTerminalKind::Nonterminal,
        ),
        ("Field", "Type", "Ident", GrammarTerminalKind::Nonterminal),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let fields_field = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Fields")
        .expect("normalized StructType should preserve Fields field");
    match &fields_field.value {
        GoAstInventoryDeclarativeFieldValue::AlternativeNode(field_list) => {
            assert_eq!(field_list.node_kind, "FieldList");
            let list_field = field_list
                .fields
                .iter()
                .find(|field| field.field_name == "List")
                .expect("normalized FieldList should preserve List field");
            match &list_field.value {
                GoAstInventoryDeclarativeFieldValue::RepeatedGenericNodes(nodes) => {
                    assert_eq!(nodes.len(), 1);
                    assert_eq!(nodes[0].node_kind, "Field");
                }
                other => panic!("expected singleton repeated generic List policy, got {other:?}"),
            }
        }
        other => panic!("expected Fields FieldList node policy, got {other:?}"),
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Incomplete")
        .expect("Incomplete field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Incomplete policy should violate checked StructType field metadata");
    assert!(
        format!("{error}")
            .contains("StructType.Incomplete cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_type_assert_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_type_assert_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "TypeAssertExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative TypeAssertExpr spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("TypeAssertExpr", vec!["X", "Type"]),
        ("Ident", vec!["Name"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 3);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "TypeAssertExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "TypeAssertExpr",
            "X",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "TypeAssertExpr",
            "Type",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "X")
        .expect("X field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing X policy should violate checked TypeAssertExpr field metadata");
    assert!(
        format!("{error}")
            .contains("TypeAssertExpr.X cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_unary_expr_spec_derives_checked_plan() {
    let spec = go_ast_inventory_unary_expr_return_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "ReturnStmt");
    assert_eq!(spec.parent_field.field_name, "Results");
    assert_eq!(spec.expression.root_node_kind(), "UnaryExpr");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative UnaryExpr spec should validate against checked inventory");
    for (rule_name, fields) in [("UnaryExpr", vec!["Op", "X"]), ("Ident", vec!["Name"])] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 3);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "ReturnStmt",
            "Results",
            "UnaryExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "UnaryExpr",
            "Op",
            "go_token_unary_operator_tokens.SUB",
            GrammarTerminalKind::IdentifierPayload,
        ),
        ("UnaryExpr", "X", "Ident", GrammarTerminalKind::Nonterminal),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Op")
        .expect("Op field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Op policy should violate checked UnaryExpr field metadata");
    assert!(
        format!("{error}")
            .contains("UnaryExpr.Op cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_assignment_spec_derives_checked_plan() {
    let spec = go_ast_inventory_assignment_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "AssignStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative AssignStmt spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("AssignStmt", vec!["Lhs", "Tok", "Rhs"]),
        ("Ident", vec!["Name"]),
        ("BasicLit", vec!["Kind", "Value"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 5);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "AssignStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "AssignStmt",
            "Lhs",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "AssignStmt",
            "Tok",
            "go_token_assignment_tokens.ASSIGN",
            GrammarTerminalKind::IdentifierPayload,
        ),
        (
            "AssignStmt",
            "Rhs",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Lhs")
        .expect("Lhs field should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Lhs policy should violate checked AssignStmt field metadata");
    assert!(
        format!("{error}")
            .contains("AssignStmt.Lhs cannot use missing policy because it is a repeated field"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_branch_stmt_spec_derives_checked_optional_policy() {
    let spec = go_ast_inventory_branch_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "ForStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative BranchStmt spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("ForStmt", vec!["Init", "Cond", "Post", "Body"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("BranchStmt", vec!["Tok", "Label"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(checked.finite_domain_requirements.len(), 3);
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "ForStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BlockStmt",
            "List",
            "BranchStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BranchStmt",
            "Tok",
            "go_token_branch_tokens.BREAK",
            GrammarTerminalKind::IdentifierPayload,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let GoAstInventoryDeclarativeFieldValue::GenericNode(body) = &checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Body")
        .expect("ForStmt.Body should exist")
        .value
    else {
        panic!("ForStmt.Body should be a generic BlockStmt node");
    };
    let GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(statements) = &body
        .fields
        .iter()
        .find(|field| field.field_name == "List")
        .expect("BlockStmt.List should exist")
        .value
    else {
        panic!("BlockStmt.List should be a repeated statement field");
    };
    let branch_stmt = statements
        .iter()
        .find(|node| node.node_kind == "BranchStmt")
        .expect("BranchStmt should be nested in the block");
    assert!(
        matches!(
            &branch_stmt
                .fields
                .iter()
                .find(|field| field.field_name == "Label")
                .expect("BranchStmt.Label should be spelled")
                .value,
            GoAstInventoryDeclarativeFieldValue::Missing
        ),
        "BranchStmt.Label should use the checked optional missing policy"
    );

    let mut invalid = spec.expression.clone();
    let GoAstInventoryDeclarativeFieldValue::GenericNode(body) = &mut invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Body")
        .expect("ForStmt.Body should exist")
        .value
    else {
        panic!("ForStmt.Body should be a generic BlockStmt node");
    };
    let GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(statements) = &mut body
        .fields
        .iter_mut()
        .find(|field| field.field_name == "List")
        .expect("BlockStmt.List should exist")
        .value
    else {
        panic!("BlockStmt.List should be a repeated statement field");
    };
    statements
        .iter_mut()
        .find(|node| node.node_kind == "BranchStmt")
        .expect("BranchStmt should be nested in the block")
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Tok")
        .expect("BranchStmt.Tok should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing Tok policy should violate checked BranchStmt field metadata");
    assert!(
        format!("{error}")
            .contains("BranchStmt.Tok cannot use missing policy because it is required"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_range_stmt_spec_derives_checked_optional_alternative_policy() {
    let spec = go_ast_inventory_range_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "RangeStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative RangeStmt spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("RangeStmt", vec!["Key", "Value", "Tok", "X", "Body"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "RangeStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "RangeStmt",
            "Key",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "RangeStmt",
            "Tok",
            "go_token_range_assignment_tokens.DEFINE",
            GrammarTerminalKind::IdentifierPayload,
        ),
        ("RangeStmt", "X", "Ident", GrammarTerminalKind::Nonterminal),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let key = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Key")
        .expect("RangeStmt.Key should exist");
    assert!(
        matches!(
            &key.value,
            GoAstInventoryDeclarativeFieldValue::OptionalAlternativeNode(Some(_))
        ),
        "RangeStmt.Key should use the checked optional alternative-node policy"
    );
    let value = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Value")
        .expect("RangeStmt.Value should exist");
    assert!(
        matches!(&value.value, GoAstInventoryDeclarativeFieldValue::Missing),
        "RangeStmt.Value should remain absent for the checked key-only range statement"
    );

    let mut invalid = spec.expression.clone();
    let GoAstInventoryDeclarativeFieldValue::OptionalAlternativeNode(Some(key_node)) = invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Key")
        .expect("RangeStmt.Key should exist")
        .value
        .clone()
    else {
        panic!("RangeStmt.Key should start as an optional alternative node");
    };
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Key")
        .expect("RangeStmt.Key should exist")
        .value = GoAstInventoryDeclarativeFieldValue::optional_node_present(*key_node);
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("singleton optional-node policy should reject multi-alternative RangeStmt.Key");
    assert!(
        format!("{error}").contains(
            "RangeStmt.Key cannot use optional-node policy because it is not a singleton nonterminal field"
        ),
        "unexpected declarative optional-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_type_switch_stmt_spec_derives_checked_optional_alternative_policy()
{
    let spec = go_ast_inventory_type_switch_stmt_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "TypeSwitchStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative TypeSwitchStmt spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("TypeSwitchStmt", vec!["Init", "Assign", "Body"]),
        ("ExprStmt", vec!["X"]),
        ("TypeAssertExpr", vec!["X", "Type"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "TypeSwitchStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "TypeSwitchStmt",
            "Assign",
            "ExprStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "ExprStmt",
            "X",
            "TypeAssertExpr",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "TypeAssertExpr",
            "X",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative TypeSwitchStmt spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }
    let init = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Init")
        .expect("TypeSwitchStmt.Init should exist");
    assert!(
        matches!(
            &init.value,
            GoAstInventoryDeclarativeFieldValue::OptionalAlternativeNode(None)
        ),
        "TypeSwitchStmt.Init should use the absent optional alternative-node policy"
    );
    let assign = checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Assign")
        .expect("TypeSwitchStmt.Assign should exist");
    let GoAstInventoryDeclarativeFieldValue::AlternativeNode(expr_stmt) = &assign.value else {
        panic!("TypeSwitchStmt.Assign should be an ExprStmt alternative node");
    };
    let GoAstInventoryDeclarativeFieldValue::AlternativeNode(type_assert_expr) = &expr_stmt
        .fields
        .iter()
        .find(|field| field.field_name == "X")
        .expect("ExprStmt.X should exist")
        .value
    else {
        panic!("ExprStmt.X should be a TypeAssertExpr alternative node");
    };
    assert!(
        matches!(
            &type_assert_expr
                .fields
                .iter()
                .find(|field| field.field_name == "Type")
                .expect("TypeAssertExpr.Type should exist")
                .value,
            GoAstInventoryDeclarativeFieldValue::Missing
        ),
        "TypeAssertExpr.Type should remain absent for the type-switch assertion form"
    );

    let mut invalid = spec.expression.clone();
    invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Init")
        .expect("TypeSwitchStmt.Init should exist")
        .value = GoAstInventoryDeclarativeFieldValue::optional_node_absent();
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err(
        "singleton optional-node policy should reject multi-alternative TypeSwitchStmt.Init",
    );
    assert!(
        format!("{error}").contains(
            "TypeSwitchStmt.Init cannot use optional-node policy because it is not a singleton nonterminal field"
        ),
        "unexpected declarative optional-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_case_clause_spec_derives_checked_repeated_policy() {
    let spec = go_ast_inventory_case_clause_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "SwitchStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative CaseClause spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("SwitchStmt", vec!["Init", "Tag", "Body"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        (
            "CaseClause",
            vec![
                "List",
                "Body",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        ("ReturnStmt", vec!["Results"]),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "SwitchStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "SwitchStmt",
            "Tag",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BlockStmt",
            "List",
            "CaseClause",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CaseClause",
            "List",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "CaseClause",
            "Body",
            "ReturnStmt",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let GoAstInventoryDeclarativeFieldValue::GenericNode(body) = &checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Body")
        .expect("SwitchStmt.Body should exist")
        .value
    else {
        panic!("SwitchStmt.Body should be a generic BlockStmt node");
    };
    let GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(statements) = &body
        .fields
        .iter()
        .find(|field| field.field_name == "List")
        .expect("SwitchStmt body BlockStmt.List should exist")
        .value
    else {
        panic!("SwitchStmt body BlockStmt.List should be a repeated statement field");
    };
    let case_clause = statements
        .iter()
        .find(|node| node.node_kind == "CaseClause")
        .expect("CaseClause should be nested in the switch body");
    assert!(
        matches!(
            &case_clause
                .fields
                .iter()
                .find(|field| field.field_name == "List")
                .expect("CaseClause.List should be spelled")
                .value,
            GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(_)
        ),
        "CaseClause.List should use the checked repeated-field policy"
    );

    let mut invalid = spec.expression.clone();
    let GoAstInventoryDeclarativeFieldValue::GenericNode(body) = &mut invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Body")
        .expect("SwitchStmt.Body should exist")
        .value
    else {
        panic!("SwitchStmt.Body should be a generic BlockStmt node");
    };
    let GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(statements) = &mut body
        .fields
        .iter_mut()
        .find(|field| field.field_name == "List")
        .expect("SwitchStmt body BlockStmt.List should exist")
        .value
    else {
        panic!("SwitchStmt body BlockStmt.List should be a repeated statement field");
    };
    statements
        .iter_mut()
        .find(|node| node.node_kind == "CaseClause")
        .expect("CaseClause should be nested in the switch body")
        .fields
        .iter_mut()
        .find(|field| field.field_name == "List")
        .expect("CaseClause.List should exist")
        .value = GoAstInventoryDeclarativeFieldValue::Missing;
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("missing List policy should violate checked CaseClause field metadata");
    assert!(
        format!("{error}")
            .contains("CaseClause.List cannot use missing policy because it is a repeated field"),
        "unexpected declarative missing-policy error: {error}"
    );
}

#[test]
fn go_ast_inventory_declarative_comm_clause_spec_derives_checked_optional_policy() {
    let spec = go_ast_inventory_comm_clause_declarative_coverage_spec();
    assert_eq!(
        spec.synthesis_api_id,
        GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID
    );
    assert_eq!(spec.parent_field.rule_name, "BlockStmt");
    assert_eq!(spec.parent_field.field_name, "List");
    assert_eq!(spec.expression.root_node_kind(), "SelectStmt");

    let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &spec.expression,
    )
    .expect("declarative CommClause spec should validate against checked inventory");
    for (rule_name, fields) in [
        ("SelectStmt", vec!["Body"]),
        (
            "BlockStmt",
            vec![
                "List",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
        (
            "CommClause",
            vec![
                "Comm",
                "Body",
                "CommentBefore",
                "TrailingComment",
                "BlankLineBefore",
            ],
        ),
    ] {
        assert_eq!(
            checked
                .expected_field_orders
                .iter()
                .find(|order| order.rule_name == rule_name)
                .unwrap_or_else(|| panic!("{rule_name} field order should be derived"))
                .fields,
            fields
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
        );
    }
    for (rule_name, field_name, selected_alternative, terminal_kind) in [
        (
            "BlockStmt",
            "List",
            "SelectStmt",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "BlockStmt",
            "List",
            "CommClause",
            GrammarTerminalKind::Nonterminal,
        ),
    ] {
        assert!(
            checked
                .finite_domain_requirements
                .iter()
                .any(|requirement| {
                    requirement.rule_name == rule_name
                        && requirement.field_name == field_name
                        && requirement.selected_alternative == selected_alternative
                        && requirement.terminal == terminal_kind
                }),
            "declarative spec should derive finite-domain requirement {rule_name}.{field_name}={selected_alternative}"
        );
    }

    let GoAstInventoryDeclarativeFieldValue::GenericNode(body) = &checked
        .expression
        .root
        .fields
        .iter()
        .find(|field| field.field_name == "Body")
        .expect("SelectStmt.Body should exist")
        .value
    else {
        panic!("SelectStmt.Body should be a generic BlockStmt node");
    };
    let GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(statements) = &body
        .fields
        .iter()
        .find(|field| field.field_name == "List")
        .expect("SelectStmt body BlockStmt.List should exist")
        .value
    else {
        panic!("SelectStmt body BlockStmt.List should be a repeated statement field");
    };
    let comm_clause = statements
        .iter()
        .find(|node| node.node_kind == "CommClause")
        .expect("CommClause should be nested in the select body");
    assert!(
        matches!(
            &comm_clause
                .fields
                .iter()
                .find(|field| field.field_name == "Comm")
                .expect("CommClause.Comm should be spelled")
                .value,
            GoAstInventoryDeclarativeFieldValue::Missing
        ),
        "CommClause.Comm should use the checked optional missing policy"
    );
    assert!(
        matches!(
            &comm_clause
                .fields
                .iter()
                .find(|field| field.field_name == "Body")
                .expect("CommClause.Body should be spelled")
                .value,
            GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(nodes) if nodes.is_empty()
        ),
        "CommClause.Body should use the checked repeated-field empty policy"
    );

    let mut invalid = spec.expression.clone();
    let GoAstInventoryDeclarativeFieldValue::GenericNode(body) = &mut invalid
        .root
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Body")
        .expect("SelectStmt.Body should exist")
        .value
    else {
        panic!("SelectStmt.Body should be a generic BlockStmt node");
    };
    let GoAstInventoryDeclarativeFieldValue::RepeatedAlternativeNodes(statements) = &mut body
        .fields
        .iter_mut()
        .find(|field| field.field_name == "List")
        .expect("SelectStmt body BlockStmt.List should exist")
        .value
    else {
        panic!("SelectStmt body BlockStmt.List should be a repeated statement field");
    };
    statements
        .iter_mut()
        .find(|node| node.node_kind == "CommClause")
        .expect("CommClause should be nested in the select body")
        .fields
        .iter_mut()
        .find(|field| field.field_name == "Comm")
        .expect("CommClause.Comm should exist")
        .value = GoAstInventoryDeclarativeFieldValue::repeated_alternative_nodes(Vec::new());
    let error = check_go_ast_inventory_declarative_expression_spec_from_inventory(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        Some(&spec.parent_field),
        &invalid,
    )
    .expect_err("repeated Comm policy should violate checked CommClause field metadata");
    assert!(
        format!("{error}")
            .contains("CommClause.Comm is not repeated and cannot use repeated-alternative policy"),
        "unexpected declarative Comm policy error: {error}"
    );
}

#[test]
fn go_raw_tree_renderer_consumes_contract_for_source_file_decl_skeleton_slice() {
    let coverage = GoAstSchemaInventoryAdapter::raw_tree_renderer_codegen_coverage_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should classify renderer codegen coverage");
    let contract = go_raw_tree_renderer_skeleton_generation_contract(&coverage)
        .expect("renderer skeleton generation contract should build from checked coverage");

    let slice = go_raw_tree_renderer_source_file_decl_skeleton_slice(&contract)
        .expect("source-file declaration/specification skeleton slice should build");

    slice
        .validate()
        .expect("generated renderer skeleton slice should validate");
    assert_eq!(
        slice.artifact_kind,
        GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_ARTIFACT_KIND
    );
    assert_eq!(
        slice.status,
        GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_STATUS_READY
    );
    assert_eq!(
        slice.slice_id,
        GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_ID
    );
    assert_eq!(
        slice.selected_node_kinds,
        [
            "File",
            "FuncDecl",
            "GenDecl",
            "ImportSpec",
            "TypeSpec",
            "ValueSpec"
        ]
        .into_iter()
        .map(|node_kind| node_kind.to_string())
        .collect::<Vec<_>>()
    );
    assert_eq!(slice.generated_entrypoint_count, 6);
    assert!(
        slice.manual_policy_callback_count > 0,
        "manual policy hooks should stay explicit as callbacks"
    );
    assert_eq!(
        slice.contract_artifact_kind,
        GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND
    );
    assert_eq!(
        slice.contract_status,
        GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY
    );
    assert_eq!(
        slice.inventory_artifact_path,
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH
    );

    let file = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "File")
        .expect("File should be generated in the source-file slice");
    assert_eq!(file.dispatch_contexts, vec!["source_file_root".to_string()]);
    assert_eq!(file.generated_entrypoint, "render_generated_file_skeleton");
    assert!(
        file.manual_policy_callbacks.iter().any(|callback| {
            callback.hook == "source_file_layout"
                && callback.callback_symbol == "manual_file_source_file_layout"
        }),
        "source-file layout should remain an explicit manual callback"
    );

    let func_decl = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "FuncDecl")
        .expect("FuncDecl should be generated in the declaration slice");
    assert_eq!(func_decl.dispatch_contexts, vec!["declaration".to_string()]);
    assert_eq!(
        func_decl.generated_entrypoint,
        "render_generated_func_decl_skeleton"
    );
    assert!(
        func_decl
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "function_signature_layout"),
        "function signature layout should remain a manual callback"
    );

    let value_spec = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "ValueSpec")
        .expect("ValueSpec should be generated in the specification slice");
    assert_eq!(
        value_spec.dispatch_contexts,
        vec!["specification".to_string()]
    );
    assert_eq!(
        value_spec.generated_entrypoint,
        "render_generated_value_spec_skeleton"
    );
    assert!(
        slice
            .source_file_root_exclusions
            .iter()
            .any(|exclusion| exclusion.node_kind == "Package"),
        "go/ast.Package must stay recorded as a source-file-root exclusion"
    );
    assert!(
        slice.claim_boundary.contains("not validity authority"),
        "generated slice must keep sidecar validators out of the validity claim"
    );

    let dispatch_guard =
        go_raw_tree_renderer_source_file_decl_spec_generated_dispatch_guard(&slice)
            .expect("runtime dispatch guard should derive from the generated skeleton slice");
    assert_eq!(
        dispatch_guard.slice_id,
        GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_ID
    );
    assert_eq!(dispatch_guard.dispatch_node_count, 6);
    dispatch_guard
        .require_node_kind_context("File", "source_file_root", "file")
        .expect("File should dispatch through the generated source-file slice");
    dispatch_guard
        .require_node_kind_context("FuncDecl", "declaration", "decl")
        .expect("FuncDecl should dispatch through the generated declaration slice");
    dispatch_guard
        .require_node_kind_context("ValueSpec", "specification", "spec")
        .expect("ValueSpec should dispatch through the generated specification slice");
    let outside_slice_error = dispatch_guard
        .require_node_kind_context("Ident", "declaration", "decl")
        .expect_err("nodes outside the generated source-file slice should reject");
    assert!(
        outside_slice_error
            .to_string()
            .contains("not selected by generated skeleton slice"),
        "outside-slice rejection should name the generated slice: {outside_slice_error}"
    );
    let outside_contract_error = dispatch_guard
        .require_node_kind_context("ImaginaryNode", "declaration", "decl")
        .expect_err("nodes outside the generated skeleton contract should reject");
    assert!(
        outside_contract_error
            .to_string()
            .contains("outside the generated renderer skeleton contract"),
        "outside-contract rejection should name the generated contract: {outside_contract_error}"
    );
}

#[test]
fn go_raw_tree_renderer_consumes_inventory_backed_runtime_skeleton_dispatch() {
    let coverage = GoAstSchemaInventoryAdapter::raw_tree_renderer_codegen_coverage_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should classify renderer codegen coverage");
    let contract = go_raw_tree_renderer_skeleton_generation_contract(&coverage)
        .expect("renderer skeleton generation contract should build from checked coverage");

    let slice = go_raw_tree_renderer_inventory_skeleton_slice(&contract)
        .expect("inventory-wide generated skeleton slice should build");
    slice
        .validate()
        .expect("inventory-wide generated skeleton slice should validate");
    assert_eq!(
        slice.slice_id,
        GO_RAW_TREE_RENDERER_INVENTORY_SKELETON_SLICE_ID
    );
    assert_eq!(
        slice.selected_node_kinds.len(),
        contract.generated_skeletons.len()
    );
    assert_eq!(
        slice.generated_entrypoint_count,
        contract.generated_skeletons.len()
    );
    assert!(
        slice
            .selected_node_kinds
            .iter()
            .any(|node_kind| node_kind == "Ident"),
        "inventory-backed runtime dispatch should include expression skeletons"
    );
    assert!(
        slice
            .selected_node_kinds
            .iter()
            .any(|node_kind| node_kind == "ReturnStmt"),
        "inventory-backed runtime dispatch should include statement skeletons"
    );
    assert!(
        slice
            .selected_node_kinds
            .iter()
            .any(|node_kind| node_kind == "GenDecl"),
        "inventory-backed runtime dispatch should include declaration skeletons"
    );
    assert!(
        slice
            .selected_node_kinds
            .iter()
            .any(|node_kind| node_kind == "Field"),
        "inventory-backed runtime dispatch should include contextual skeletons"
    );
    assert!(
        slice
            .selected_node_kinds
            .iter()
            .any(|node_kind| node_kind == "CommentGroup"),
        "inventory-backed runtime dispatch should include comment-trivia skeletons"
    );
    let basic_lit = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "BasicLit")
        .expect("inventory-backed runtime slice should include BasicLit");
    assert_eq!(
        basic_lit.generated_field_traversals,
        vec!["Value".to_string()],
        "BasicLit.Value should be the first generated payload field traversal"
    );
    assert!(
        basic_lit
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "literal_payload_rendering"),
        "literal payload normalization should remain an explicit manual callback"
    );
    let return_stmt = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "ReturnStmt")
        .expect("inventory-backed runtime slice should include ReturnStmt");
    assert_eq!(
        return_stmt.generated_field_traversals,
        vec!["Results".to_string()],
        "ReturnStmt.Results should be the first generated repeated child traversal"
    );
    assert!(
        return_stmt
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "return_result_layout"),
        "return keyword, comma, spacing, and empty-return layout should remain an explicit manual callback"
    );
    let branch_stmt = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "BranchStmt")
        .expect("inventory-backed runtime slice should include BranchStmt");
    assert_eq!(
        branch_stmt.generated_field_traversals,
        vec!["Tok".to_string(), "Label".to_string()],
        "BranchStmt.Tok and Label should be generated statement field traversals"
    );
    assert!(
        branch_stmt
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "branch_token_layout"),
        "branch token, optional label, and comment policy should remain an explicit manual callback"
    );
    let field_list = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "FieldList")
        .expect("inventory-backed runtime slice should include FieldList");
    assert_eq!(
        field_list.generated_field_traversals,
        vec!["List".to_string()],
        "FieldList.List should be the generated contextual child traversal"
    );
    assert!(
        field_list
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "comma_list_layout"),
        "field-list comma and empty-list rendering should remain explicit manual callbacks"
    );
    let field = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "Field")
        .expect("inventory-backed runtime slice should include Field");
    assert_eq!(
        field.generated_field_traversals,
        vec!["Names".to_string(), "Type".to_string()],
        "Field.Names and Type should be generated contextual child traversals"
    );
    assert!(
        field
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "field_names_type_tag_layout"),
        "field name/type/tag rendering should remain an explicit manual callback"
    );
    assert!(
        field
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "interface_method_layout"),
        "interface method rendering should remain an explicit manual callback"
    );
    let gen_decl = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "GenDecl")
        .expect("inventory-backed runtime slice should include GenDecl");
    assert_eq!(
        gen_decl.generated_field_traversals,
        vec!["Specs".to_string()],
        "GenDecl.Specs should be the first generated repeated child traversal"
    );
    assert!(
        gen_decl
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "declaration_keyword_layout"),
        "declaration keyword rendering should remain an explicit manual callback"
    );
    assert!(
        gen_decl
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "grouped_declaration_layout"),
        "grouped declaration layout should remain an explicit manual callback"
    );
    let import_spec = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "ImportSpec")
        .expect("inventory-backed runtime slice should include ImportSpec");
    assert_eq!(
        import_spec.generated_field_traversals,
        vec!["Name".to_string(), "Path".to_string()],
        "ImportSpec.Name and Path should be generated specification child traversals"
    );
    let type_spec = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "TypeSpec")
        .expect("inventory-backed runtime slice should include TypeSpec");
    assert_eq!(
        type_spec.generated_field_traversals,
        vec!["Name".to_string(), "Type".to_string()],
        "TypeSpec.Name and Type should be generated specification child traversals"
    );
    let value_spec = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "ValueSpec")
        .expect("inventory-backed runtime slice should include ValueSpec");
    assert_eq!(
        value_spec.generated_field_traversals,
        vec![
            "Names".to_string(),
            "Type".to_string(),
            "Values".to_string()
        ],
        "ValueSpec.Names, Type, and Values should be generated specification child traversals"
    );
    assert!(
        value_spec
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "value_spec_assignment_layout"),
        "comma layout, optional type spacing, assignment rendering, empty values, and trailing comments should remain an explicit manual callback"
    );
    let assign_stmt = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "AssignStmt")
        .expect("inventory-backed runtime slice should include AssignStmt");
    assert_eq!(
        assign_stmt.generated_field_traversals,
        vec!["Lhs".to_string(), "Tok".to_string(), "Rhs".to_string()],
        "AssignStmt.Lhs, Tok, and Rhs should be generated statement/header traversals"
    );
    assert!(
        assign_stmt
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "assignment_operator_layout"),
        "assignment operator spacing and comma layout should remain an explicit manual callback"
    );
    let expr_stmt = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "ExprStmt")
        .expect("inventory-backed runtime slice should include ExprStmt");
    assert_eq!(
        expr_stmt.generated_field_traversals,
        vec!["X".to_string()],
        "ExprStmt.X should be the generated statement/header traversal"
    );
    assert!(
        expr_stmt
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "expression_statement_layout"),
        "expression statement indentation and expression layout should remain an explicit manual callback"
    );
    let inc_dec_stmt = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "IncDecStmt")
        .expect("inventory-backed runtime slice should include IncDecStmt");
    assert_eq!(
        inc_dec_stmt.generated_field_traversals,
        vec!["X".to_string(), "Tok".to_string()],
        "IncDecStmt.X and Tok should be generated statement/header traversals"
    );
    assert!(
        inc_dec_stmt
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "inc_dec_operator_layout"),
        "inc/dec operand and operator layout should remain an explicit manual callback"
    );
    let paren_expr = slice
        .generated_entrypoints
        .iter()
        .find(|entrypoint| entrypoint.node_kind == "ParenExpr")
        .expect("inventory-backed runtime slice should include ParenExpr");
    assert_eq!(
        paren_expr.generated_field_traversals,
        vec!["X".to_string()],
        "ParenExpr.X should be the generated expression child traversal"
    );
    assert!(
        paren_expr
            .manual_policy_callbacks
            .iter()
            .any(|callback| callback.hook == "parenthesized_expression_preservation"),
        "parenthesis preservation should remain an explicit manual callback"
    );

    let guard = go_raw_tree_renderer_generated_skeleton_dispatch_guard(&slice)
        .expect("inventory-wide runtime dispatch guard should derive from the generated slice");
    assert_eq!(
        guard.dispatch_node_count,
        coverage.generated_skeleton_ready_count
    );
    for (node_kind, context) in [
        ("File", "source_file_root"),
        ("FuncDecl", "declaration"),
        ("TypeSpec", "specification"),
        ("Ident", "expression"),
        ("ReturnStmt", "statement"),
        ("AssignStmt", "simple_statement_header"),
        ("ExprStmt", "simple_statement_header"),
        ("IncDecStmt", "simple_statement_header"),
        ("Field", "contextual"),
        ("CommentGroup", "comment_trivia"),
    ] {
        guard
            .require_node_kind_context(node_kind, context, "inventory_dispatch")
            .unwrap_or_else(|error| {
                panic!("{node_kind} should dispatch in {context} context: {error}")
            });
    }

    let wrong_context_error = guard
        .require_node_kind_context("Ident", "declaration", "inventory_dispatch")
        .expect_err("inventory-wide dispatch should reject unsupported contexts");
    assert!(
        wrong_context_error
            .to_string()
            .contains("does not dispatch in declaration context"),
        "wrong-context rejection should name the generated dispatch context: {wrong_context_error}"
    );
    let excluded_error = guard
        .require_node_kind_context("Package", "source_file_root", "inventory_dispatch")
        .expect_err("source-file-root exclusions must still reject at runtime");
    assert!(
        excluded_error.to_string().contains("is excluded"),
        "excluded node rejection should name the source-root exclusion: {excluded_error}"
    );
    let uncontracted_error = guard
        .require_node_kind_context("ImaginaryNode", "expression", "inventory_dispatch")
        .expect_err("uncontracted nodes must still reject at runtime");
    assert!(
        uncontracted_error
            .to_string()
            .contains("outside the generated renderer skeleton contract"),
        "uncontracted node rejection should name the generated contract: {uncontracted_error}"
    );
}

#[test]
fn go_raw_tree_renderer_generated_skeleton_slice_rejects_uncontracted_or_excluded_nodes() {
    let coverage = GoAstSchemaInventoryAdapter::raw_tree_renderer_codegen_coverage_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should classify renderer codegen coverage");
    let contract = go_raw_tree_renderer_skeleton_generation_contract(&coverage)
        .expect("renderer skeleton generation contract should build from checked coverage");

    let excluded_error = go_raw_tree_renderer_generated_skeleton_slice_for_nodes(
        &contract,
        "test_excluded_slice",
        &["Package"],
    )
    .expect_err("source-file-root exclusions must not be selectable");
    let excluded_message = excluded_error.to_string();
    assert!(
        excluded_message.contains("source-file-root exclusion"),
        "excluded node error should name the source-root exclusion: {excluded_message}"
    );

    let uncontracted_error = go_raw_tree_renderer_generated_skeleton_slice_for_nodes(
        &contract,
        "test_uncontracted_slice",
        &["ImaginaryNode"],
    )
    .expect_err("uncontracted nodes must not be selectable");
    let uncontracted_message = uncontracted_error.to_string();
    assert!(
        uncontracted_message.contains("not present in the skeleton generation contract"),
        "uncontracted node error should name the missing contract entry: {uncontracted_message}"
    );
}

#[test]
fn go_ast_schema_inventory_rejects_stale_renderer_position_policy_strings() {
    for stale_policy in [
        "unclassified_token_pos_field",
        "required_by_go_ast_field_type",
    ] {
        let contents = inventory_with_position_policy("ArrayType", "Lbrack", stale_policy);
        let error = GoAstSchemaInventoryAdapter::renderer_position_fields_str(&contents)
            .expect_err("stale renderer-position policy should fail validation");
        let message = error.to_string();
        assert!(
            message.contains("ArrayType.Lbrack"),
            "error should identify the stale field path: {message}"
        );
        assert!(
            message.contains(stale_policy),
            "error should include the stale policy string: {message}"
        );
    }
}

fn renderer_position_policy(
    fields: &[tct::go_ast_schema_inventory::GoAstRendererPositionField],
    node: &str,
    field: &str,
) -> GoAstRendererPositionPolicy {
    fields
        .iter()
        .find(|candidate| candidate.node_name == node && candidate.field_name == field)
        .unwrap_or_else(|| panic!("missing renderer-position field {node}.{field}"))
        .policy
}

fn inventory_with_position_policy(node: &str, field: &str, policy: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json"
    ))
    .expect("checked inventory fixture should parse");
    let nodes = value
        .get_mut("node_structs")
        .and_then(|nodes| nodes.as_array_mut())
        .expect("checked inventory should contain node_structs");
    let target_node = nodes
        .iter_mut()
        .find(|candidate| candidate.get("name").and_then(|name| name.as_str()) == Some(node))
        .unwrap_or_else(|| panic!("missing node {node}"));
    let positions = target_node
        .get_mut("position_fields")
        .and_then(|fields| fields.as_array_mut())
        .unwrap_or_else(|| panic!("missing position_fields for {node}"));
    let target_field = positions
        .iter_mut()
        .find(|candidate| candidate.get("name").and_then(|name| name.as_str()) == Some(field))
        .unwrap_or_else(|| panic!("missing position field {node}.{field}"));
    target_field["policy"] = serde_json::Value::String(policy.to_string());
    serde_json::to_string(&value).expect("mutated inventory should serialize")
}

fn assert_go_parser_formatter_accepts(source: &str) {
    let dir = Path::new("target/tct-go-ast-scratch-token-stream-render-reparse");
    fs::create_dir_all(dir).expect("scratch Go validation dir should be creatable");
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    let source_path = dir.join(format!("scratch-{:016x}.go", hasher.finish()));
    fs::write(&source_path, source).expect("scratch Go source should be writable");

    let gofmt = Command::new("gofmt")
        .arg(&source_path)
        .output()
        .expect("gofmt should be installed for Go AST scratch render validation");
    assert!(
        gofmt.status.success(),
        "gofmt should parse and format rendered scratch source: {}",
        String::from_utf8_lossy(&gofmt.stderr)
    );
    let formatted = String::from_utf8(gofmt.stdout).expect("gofmt output should be UTF-8");
    assert_eq!(
        formatted, source,
        "scratch render should already be gofmt-stable"
    );
}
