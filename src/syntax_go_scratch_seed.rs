//! Go scratch-generation seed streams.
//!
//! This module is intentionally small: it owns token-only seed construction
//! from the precompiled syntax model boundary and proves that each selected raw
//! token is admitted by the raw syntax state machine before strict decode.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::synthesize_go_ast_inventory_var_decl_raw_tokens;
use crate::syntax_state_machine::{RawSyntaxStateMachineCore, SyntaxTokenRoles};

mod admissibility;
pub(crate) mod constants;
mod declarations;
mod empty_statements;
mod expressions;
mod literal_expressions;
mod paren_expression;
mod send_statement;
mod slice_expression;
mod star_expression;
mod statements;
mod type_assert_expression;
mod type_switch_statement;
mod types;

use crate::syntax_go_inventory_synthesis::{
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
    GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID,
    GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID,
};
pub use admissibility::{
    GoScratchSeedAdmissibilityProof, GoScratchSeedAdmissibilityStep,
    verify_go_scratch_seed_admissibility,
};
pub use constants::*;
pub use declarations::{
    go_ast_inventory_scratch_import_spec_raw_tokens, go_ast_inventory_scratch_import_spec_seed,
};
pub use empty_statements::{
    go_ast_inventory_scratch_empty_stmt_raw_tokens, go_ast_inventory_scratch_empty_stmt_seed,
};
pub use expressions::{
    go_ast_inventory_scratch_array_type_return_raw_tokens,
    go_ast_inventory_scratch_array_type_return_seed,
    go_ast_inventory_scratch_binary_expr_raw_tokens, go_ast_inventory_scratch_binary_expr_seed,
    go_ast_inventory_scratch_chan_type_return_raw_tokens,
    go_ast_inventory_scratch_chan_type_return_seed,
    go_ast_inventory_scratch_function_return_raw_tokens,
    go_ast_inventory_scratch_function_return_seed, go_ast_inventory_scratch_index_expr_raw_tokens,
    go_ast_inventory_scratch_index_expr_seed, go_ast_inventory_scratch_index_list_expr_raw_tokens,
    go_ast_inventory_scratch_index_list_expr_seed,
    go_ast_inventory_scratch_interface_type_raw_tokens,
    go_ast_inventory_scratch_interface_type_seed,
    go_ast_inventory_scratch_key_value_expr_raw_tokens,
    go_ast_inventory_scratch_key_value_expr_seed,
    go_ast_inventory_scratch_selector_call_raw_tokens, go_ast_inventory_scratch_selector_call_seed,
    go_ast_inventory_scratch_unary_expr_raw_tokens, go_ast_inventory_scratch_unary_expr_seed,
};
pub use literal_expressions::{
    go_ast_inventory_scratch_composite_lit_raw_tokens, go_ast_inventory_scratch_composite_lit_seed,
    go_ast_inventory_scratch_ellipsis_array_literal_raw_tokens,
    go_ast_inventory_scratch_ellipsis_array_literal_seed,
    go_ast_inventory_scratch_func_lit_raw_tokens, go_ast_inventory_scratch_func_lit_seed,
};
pub use paren_expression::{
    go_ast_inventory_scratch_paren_expr_raw_tokens, go_ast_inventory_scratch_paren_expr_seed,
};
pub use send_statement::{
    go_ast_inventory_scratch_send_stmt_raw_tokens, go_ast_inventory_scratch_send_stmt_seed,
};
pub use slice_expression::{
    go_ast_inventory_scratch_slice_expr_raw_tokens, go_ast_inventory_scratch_slice_expr_seed,
};
pub use star_expression::{
    go_ast_inventory_scratch_star_expr_raw_tokens, go_ast_inventory_scratch_star_expr_seed,
};
pub use statements::{
    go_ast_inventory_scratch_assignment_raw_tokens, go_ast_inventory_scratch_assignment_seed,
    go_ast_inventory_scratch_branch_stmt_raw_tokens, go_ast_inventory_scratch_branch_stmt_seed,
    go_ast_inventory_scratch_case_clause_raw_tokens, go_ast_inventory_scratch_case_clause_seed,
    go_ast_inventory_scratch_comm_clause_raw_tokens, go_ast_inventory_scratch_comm_clause_seed,
    go_ast_inventory_scratch_decl_stmt_raw_tokens, go_ast_inventory_scratch_decl_stmt_seed,
    go_ast_inventory_scratch_defer_stmt_raw_tokens, go_ast_inventory_scratch_defer_stmt_seed,
    go_ast_inventory_scratch_expr_stmt_raw_tokens, go_ast_inventory_scratch_expr_stmt_seed,
    go_ast_inventory_scratch_go_stmt_raw_tokens, go_ast_inventory_scratch_go_stmt_seed,
    go_ast_inventory_scratch_if_stmt_raw_tokens, go_ast_inventory_scratch_if_stmt_seed,
    go_ast_inventory_scratch_inc_dec_stmt_raw_tokens, go_ast_inventory_scratch_inc_dec_stmt_seed,
    go_ast_inventory_scratch_labeled_stmt_raw_tokens, go_ast_inventory_scratch_labeled_stmt_seed,
    go_ast_inventory_scratch_range_stmt_raw_tokens, go_ast_inventory_scratch_range_stmt_seed,
};
pub use type_assert_expression::{
    go_ast_inventory_scratch_type_assert_expr_raw_tokens,
    go_ast_inventory_scratch_type_assert_expr_seed,
};
pub use type_switch_statement::{
    go_ast_inventory_scratch_type_switch_stmt_raw_tokens,
    go_ast_inventory_scratch_type_switch_stmt_seed,
};
pub use types::{
    go_ast_inventory_scratch_func_type_result_raw_tokens,
    go_ast_inventory_scratch_func_type_result_seed,
    go_ast_inventory_scratch_struct_type_raw_tokens, go_ast_inventory_scratch_struct_type_seed,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoScratchSeedStream {
    pub seed_id: String,
    pub grammar_category: String,
    pub root: String,
    pub expected_rendered_source: String,
    pub contract: String,
    pub inventory_synthesis_api_id: Option<String>,
    pub inventory_synthesis_core_api_id: Option<String>,
    pub raw_token_ids: Vec<u32>,
    pub admissibility: GoScratchSeedAdmissibilityProof,
}

type GoScratchRawTokenBuilder =
    fn(&SyntaxTokenRoles) -> Result<Vec<u32>, SyntaxBackendBoundaryError>;

#[derive(Clone, Copy)]
pub struct GoScratchWitnessSpec {
    pub seed_id: &'static str,
    pub grammar_category: &'static str,
    pub root: &'static str,
    pub expected_rendered_source: &'static str,
    pub contract: &'static str,
    pub inventory_synthesis_api_id: Option<&'static str>,
    pub inventory_synthesis_core_api_id: Option<&'static str>,
    raw_token_builder: GoScratchRawTokenBuilder,
}

impl GoScratchWitnessSpec {
    pub fn raw_token_ids(
        &self,
        roles: &SyntaxTokenRoles,
    ) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
        (self.raw_token_builder)(roles)
    }
}

macro_rules! go_scratch_witness_spec {
    ($seed_id:ident, $grammar_category:ident, $root:ident, $rendered:ident, $contract:ident, $raw_tokens:ident) => {
        GoScratchWitnessSpec {
            seed_id: $seed_id,
            grammar_category: $grammar_category,
            root: $root,
            expected_rendered_source: $rendered,
            contract: $contract,
            inventory_synthesis_api_id: None,
            inventory_synthesis_core_api_id: None,
            raw_token_builder: $raw_tokens,
        }
    };
}

macro_rules! go_scratch_inventory_synthesis_witness_spec {
    ($seed_id:ident, $grammar_category:ident, $root:ident, $rendered:ident, $contract:ident, $raw_tokens:ident, $synthesis_api_id:ident) => {
        GoScratchWitnessSpec {
            seed_id: $seed_id,
            grammar_category: $grammar_category,
            root: $root,
            expected_rendered_source: $rendered,
            contract: $contract,
            inventory_synthesis_api_id: Some($synthesis_api_id),
            inventory_synthesis_core_api_id: Some(GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID),
            raw_token_builder: $raw_tokens,
        }
    };
}

pub const GO_AST_INVENTORY_SCRATCH_WITNESS_SPECS: &[GoScratchWitnessSpec] = &[
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_ROOT,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_VAR_DECL_CONTRACT,
        go_ast_inventory_scratch_var_decl_raw_tokens,
        GO_AST_INVENTORY_VAR_DECL_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ROOT,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_CONTRACT,
        go_ast_inventory_scratch_import_spec_raw_tokens,
        GO_AST_INVENTORY_IMPORT_SPEC_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_ROOT,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_CONTRACT,
        go_ast_inventory_scratch_function_return_raw_tokens,
        GO_AST_INVENTORY_FUNCTION_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_ROOT,
        GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_CONTRACT,
        go_ast_inventory_scratch_func_type_result_raw_tokens,
        GO_AST_INVENTORY_FUNC_TYPE_RESULT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_ROOT,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_CONTRACT,
        go_ast_inventory_scratch_array_type_return_raw_tokens,
        GO_AST_INVENTORY_ARRAY_TYPE_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ROOT,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_CONTRACT,
        go_ast_inventory_scratch_chan_type_return_raw_tokens,
        GO_AST_INVENTORY_CHAN_TYPE_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_ROOT,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_CONTRACT,
        go_ast_inventory_scratch_selector_call_raw_tokens,
        GO_AST_INVENTORY_SELECTOR_CALL_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_CONTRACT,
        go_ast_inventory_scratch_index_expr_raw_tokens,
        GO_AST_INVENTORY_INDEX_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_CONTRACT,
        go_ast_inventory_scratch_index_list_expr_raw_tokens,
        GO_AST_INVENTORY_INDEX_LIST_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_CONTRACT,
        go_ast_inventory_scratch_slice_expr_raw_tokens,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_STAR_EXPR_CONTRACT,
        go_ast_inventory_scratch_star_expr_raw_tokens,
        GO_AST_INVENTORY_STAR_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_ROOT,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_CONTRACT,
        go_ast_inventory_scratch_struct_type_raw_tokens,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_CONTRACT,
        go_ast_inventory_scratch_type_assert_expr_raw_tokens,
        GO_AST_INVENTORY_TYPE_ASSERT_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_ROOT,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_CONTRACT,
        go_ast_inventory_scratch_interface_type_raw_tokens,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_CONTRACT,
        go_ast_inventory_scratch_key_value_expr_raw_tokens,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_ROOT,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_CONTRACT,
        go_ast_inventory_scratch_assignment_raw_tokens,
        GO_AST_INVENTORY_ASSIGNMENT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_CONTRACT,
        go_ast_inventory_scratch_binary_expr_raw_tokens,
        GO_AST_INVENTORY_BINARY_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_CONTRACT,
        go_ast_inventory_scratch_unary_expr_raw_tokens,
        GO_AST_INVENTORY_UNARY_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_ROOT,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_CONTRACT,
        go_ast_inventory_scratch_paren_expr_raw_tokens,
        GO_AST_INVENTORY_PAREN_EXPR_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ROOT,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_CONTRACT,
        go_ast_inventory_scratch_composite_lit_raw_tokens,
        GO_AST_INVENTORY_COMPOSITE_LIT_RETURN_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ROOT,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_CONTRACT,
        go_ast_inventory_scratch_ellipsis_array_literal_raw_tokens,
        GO_AST_INVENTORY_ELLIPSIS_ARRAY_LITERAL_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_ROOT,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_FUNC_LIT_CONTRACT,
        go_ast_inventory_scratch_func_lit_raw_tokens,
        GO_AST_INVENTORY_FUNC_LIT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_DECL_STMT_CONTRACT,
        go_ast_inventory_scratch_decl_stmt_raw_tokens,
        GO_AST_INVENTORY_DECL_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CONTRACT,
        go_ast_inventory_scratch_defer_stmt_raw_tokens,
        GO_AST_INVENTORY_DEFER_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_GO_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_GO_STMT_CONTRACT,
        go_ast_inventory_scratch_go_stmt_raw_tokens,
        GO_AST_INVENTORY_GO_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_IF_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_IF_STMT_CONTRACT,
        go_ast_inventory_scratch_if_stmt_raw_tokens,
        GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_CONTRACT,
        go_ast_inventory_scratch_empty_stmt_raw_tokens,
        GO_AST_INVENTORY_EMPTY_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CONTRACT,
        go_ast_inventory_scratch_expr_stmt_raw_tokens
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_CONTRACT,
        go_ast_inventory_scratch_inc_dec_stmt_raw_tokens,
        GO_AST_INVENTORY_INC_DEC_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_CONTRACT,
        go_ast_inventory_scratch_branch_stmt_raw_tokens,
        GO_AST_INVENTORY_BRANCH_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_RANGE_STMT_CONTRACT,
        go_ast_inventory_scratch_range_stmt_raw_tokens,
        GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_SEND_STMT_CONTRACT,
        go_ast_inventory_scratch_send_stmt_raw_tokens,
        GO_AST_INVENTORY_SEND_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_LABELED_STMT_CONTRACT,
        go_ast_inventory_scratch_labeled_stmt_raw_tokens,
        GO_AST_INVENTORY_LABELED_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_ROOT,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CONTRACT,
        go_ast_inventory_scratch_case_clause_raw_tokens,
        GO_AST_INVENTORY_CASE_CLAUSE_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_ROOT,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_CONTRACT,
        go_ast_inventory_scratch_type_switch_stmt_raw_tokens,
        GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID
    ),
    go_scratch_inventory_synthesis_witness_spec!(
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_SEED_ID,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_GRAMMAR_CATEGORY,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_ROOT,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RENDERED_SOURCE,
        GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_CONTRACT,
        go_ast_inventory_scratch_comm_clause_raw_tokens,
        GO_AST_INVENTORY_COMM_CLAUSE_SYNTHESIS_API_ID
    ),
];

pub fn go_ast_inventory_scratch_witness_specs() -> &'static [GoScratchWitnessSpec] {
    GO_AST_INVENTORY_SCRATCH_WITNESS_SPECS
}

fn go_ast_inventory_scratch_witness_spec(seed_id: &str) -> Option<&'static GoScratchWitnessSpec> {
    GO_AST_INVENTORY_SCRATCH_WITNESS_SPECS
        .iter()
        .find(|spec| spec.seed_id == seed_id)
}

fn go_scratch_seed_from_spec(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
    spec: &GoScratchWitnessSpec,
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    let raw_token_ids = spec.raw_token_ids(core.token_roles())?;
    let admissibility = verify_go_scratch_seed_admissibility(core, &raw_token_ids)?;
    Ok(GoScratchSeedStream {
        seed_id: spec.seed_id.to_string(),
        grammar_category: spec.grammar_category.to_string(),
        root: spec.root.to_string(),
        expected_rendered_source: spec.expected_rendered_source.to_string(),
        contract: spec.contract.to_string(),
        inventory_synthesis_api_id: spec.inventory_synthesis_api_id.map(str::to_string),
        inventory_synthesis_core_api_id: spec.inventory_synthesis_core_api_id.map(str::to_string),
        raw_token_ids,
        admissibility,
    })
}

pub(super) fn go_ast_inventory_scratch_seed_for_id(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
    seed_id: &str,
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    let spec = go_ast_inventory_scratch_witness_spec(seed_id)
        .unwrap_or_else(|| unreachable!("Go scratch witness spec is registered: {seed_id}"));
    go_scratch_seed_from_spec(core, spec)
}

pub fn go_ast_inventory_scratch_var_decl_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_VAR_DECL_SEED_ID)
}

pub fn go_ast_inventory_scratch_seeds(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<Vec<GoScratchSeedStream>, SyntaxBackendBoundaryError> {
    GO_AST_INVENTORY_SCRATCH_WITNESS_SPECS
        .iter()
        .map(|spec| go_scratch_seed_from_spec(core, spec))
        .collect()
}

pub fn go_ast_inventory_scratch_var_decl_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_var_decl_raw_tokens(roles)?.raw_token_ids)
}
