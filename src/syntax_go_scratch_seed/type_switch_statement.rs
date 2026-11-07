use super::constants::*;
use super::{GoScratchSeedStream, go_ast_inventory_scratch_seed_for_id};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::synthesize_go_ast_inventory_type_switch_stmt_raw_tokens;
use crate::syntax_state_machine::{RawSyntaxStateMachineCore, SyntaxTokenRoles};

pub fn go_ast_inventory_scratch_type_switch_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_type_switch_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_type_switch_stmt_raw_tokens(roles)?.raw_token_ids)
}
