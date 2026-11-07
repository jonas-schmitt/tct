use super::constants::*;
use super::{GoScratchSeedStream, go_ast_inventory_scratch_seed_for_id};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_raw_token_stream::GoRawTokenStreamBuilder;
use crate::syntax_state_machine::{RawSyntaxStateMachineCore, SyntaxTokenRoles};

pub fn go_ast_inventory_scratch_empty_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_empty_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    let mut stream = GoRawTokenStreamBuilder::new(roles);
    stream.source_file(
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_GO_VERSION,
        |stream| {
            stream.func_decl_with_body(GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IDENTIFIER, |stream| {
                stream.repeated_node_field(|stream| {
                    stream
                        .empty_stmt_alternative_value(GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IMPLICIT)
                })
            })
        },
    )?;
    Ok(stream.raw_token_ids)
}
