use super::constants::*;
use super::{GoScratchSeedStream, go_ast_inventory_scratch_seed_for_id};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::{
    synthesize_go_ast_inventory_array_type_return_raw_tokens,
    synthesize_go_ast_inventory_binary_expr_return_raw_tokens,
    synthesize_go_ast_inventory_chan_type_return_raw_tokens,
    synthesize_go_ast_inventory_function_return_raw_tokens,
    synthesize_go_ast_inventory_index_expr_return_raw_tokens,
    synthesize_go_ast_inventory_index_list_expr_return_raw_tokens,
    synthesize_go_ast_inventory_interface_type_var_raw_tokens,
    synthesize_go_ast_inventory_key_value_expr_return_raw_tokens,
    synthesize_go_ast_inventory_selector_call_raw_tokens,
    synthesize_go_ast_inventory_unary_expr_return_raw_tokens,
};
use crate::syntax_state_machine::{RawSyntaxStateMachineCore, SyntaxTokenRoles};

pub fn go_ast_inventory_scratch_function_return_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_SEED_ID)
}

pub fn go_ast_inventory_scratch_selector_call_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_SEED_ID)
}

pub fn go_ast_inventory_scratch_index_expr_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_SEED_ID)
}

pub fn go_ast_inventory_scratch_index_list_expr_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_SEED_ID)
}

pub fn go_ast_inventory_scratch_interface_type_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_SEED_ID)
}

pub fn go_ast_inventory_scratch_key_value_expr_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_SEED_ID)
}

pub fn go_ast_inventory_scratch_array_type_return_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_SEED_ID)
}

pub fn go_ast_inventory_scratch_chan_type_return_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_SEED_ID)
}

pub fn go_ast_inventory_scratch_binary_expr_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_SEED_ID)
}

pub fn go_ast_inventory_scratch_unary_expr_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_SEED_ID)
}

pub fn go_ast_inventory_scratch_function_return_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_function_return_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_selector_call_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_selector_call_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_index_expr_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_index_expr_return_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_index_list_expr_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_index_list_expr_return_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_interface_type_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_interface_type_var_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_key_value_expr_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_key_value_expr_return_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_array_type_return_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_array_type_return_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_chan_type_return_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_chan_type_return_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_binary_expr_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_binary_expr_return_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_unary_expr_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_unary_expr_return_raw_tokens(roles)?.raw_token_ids)
}
