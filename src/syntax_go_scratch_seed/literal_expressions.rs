use super::constants::*;
use super::{GoScratchSeedStream, go_ast_inventory_scratch_seed_for_id};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::{
    synthesize_go_ast_inventory_composite_lit_return_raw_tokens,
    synthesize_go_ast_inventory_ellipsis_array_literal_raw_tokens,
    synthesize_go_ast_inventory_func_lit_raw_tokens,
};
use crate::syntax_state_machine::{RawSyntaxStateMachineCore, SyntaxTokenRoles};

pub fn go_ast_inventory_scratch_composite_lit_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_SEED_ID)
}

pub fn go_ast_inventory_scratch_ellipsis_array_literal_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(
        core,
        GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_SEED_ID,
    )
}

pub fn go_ast_inventory_scratch_func_lit_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_FUNC_LIT_SEED_ID)
}

pub fn go_ast_inventory_scratch_composite_lit_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_composite_lit_return_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_ellipsis_array_literal_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_ellipsis_array_literal_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_func_lit_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_func_lit_raw_tokens(roles)?.raw_token_ids)
}
