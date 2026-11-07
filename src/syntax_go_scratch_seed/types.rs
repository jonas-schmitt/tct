use super::constants::*;
use super::{GoScratchSeedStream, go_ast_inventory_scratch_seed_for_id};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::{
    synthesize_go_ast_inventory_func_type_result_raw_tokens,
    synthesize_go_ast_inventory_struct_type_decl_raw_tokens,
};
use crate::syntax_state_machine::{RawSyntaxStateMachineCore, SyntaxTokenRoles};

pub fn go_ast_inventory_scratch_func_type_result_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_SEED_ID)
}

pub fn go_ast_inventory_scratch_func_type_result_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_func_type_result_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_struct_type_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_SEED_ID)
}

pub fn go_ast_inventory_scratch_struct_type_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_struct_type_decl_raw_tokens(roles)?.raw_token_ids)
}
