use super::constants::*;
use super::{GoScratchSeedStream, go_ast_inventory_scratch_seed_for_id};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::{
    synthesize_go_ast_inventory_assignment_raw_tokens,
    synthesize_go_ast_inventory_branch_stmt_raw_tokens,
    synthesize_go_ast_inventory_case_clause_raw_tokens,
    synthesize_go_ast_inventory_comm_clause_raw_tokens,
    synthesize_go_ast_inventory_decl_stmt_raw_tokens,
    synthesize_go_ast_inventory_defer_stmt_raw_tokens,
    synthesize_go_ast_inventory_go_stmt_raw_tokens, synthesize_go_ast_inventory_if_stmt_raw_tokens,
    synthesize_go_ast_inventory_inc_dec_stmt_raw_tokens,
    synthesize_go_ast_inventory_labeled_stmt_raw_tokens,
    synthesize_go_ast_inventory_range_stmt_raw_tokens,
};
use crate::syntax_go_raw_token_stream::GoRawTokenStreamBuilder;
use crate::syntax_state_machine::{RawSyntaxStateMachineCore, SyntaxTokenRoles};

pub fn go_ast_inventory_scratch_assignment_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_SEED_ID)
}

pub fn go_ast_inventory_scratch_decl_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_DECL_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_defer_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_DEFER_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_go_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_GO_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_if_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_IF_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_expr_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_EXPR_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_inc_dec_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_branch_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_range_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_RANGE_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_labeled_stmt_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_LABELED_STMT_SEED_ID)
}

pub fn go_ast_inventory_scratch_case_clause_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SEED_ID)
}

pub fn go_ast_inventory_scratch_comm_clause_seed(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
) -> Result<GoScratchSeedStream, SyntaxBackendBoundaryError> {
    go_ast_inventory_scratch_seed_for_id(core, GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_SEED_ID)
}

pub fn go_ast_inventory_scratch_assignment_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_assignment_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_decl_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_decl_stmt_raw_tokens(roles).map(|synthesis| synthesis.raw_token_ids)
}

pub fn go_ast_inventory_scratch_defer_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_defer_stmt_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_go_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_go_stmt_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_if_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_if_stmt_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_expr_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    let mut stream = GoRawTokenStreamBuilder::new(roles);
    stream.source_file(
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_PACKAGE_NAME,
        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_GO_VERSION,
        |stream| {
            stream.func_decl_with_body(GO_AST_INVENTORY_SCRATCH_EXPR_STMT_IDENTIFIER, |stream| {
                stream.repeated_node_field(|stream| {
                    stream.expr_stmt_selector_call_alternative_value(
                        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CALL_RECEIVER,
                        GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CALL_SELECTOR,
                    )
                })
            })
        },
    )?;
    Ok(stream.raw_token_ids)
}

pub fn go_ast_inventory_scratch_inc_dec_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_inc_dec_stmt_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_branch_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    Ok(synthesize_go_ast_inventory_branch_stmt_raw_tokens(roles)?.raw_token_ids)
}

pub fn go_ast_inventory_scratch_range_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_range_stmt_raw_tokens(roles)
        .map(|synthesis| synthesis.raw_token_ids)
}

pub fn go_ast_inventory_scratch_labeled_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_labeled_stmt_raw_tokens(roles)
        .map(|synthesis| synthesis.raw_token_ids)
}

pub fn go_ast_inventory_scratch_case_clause_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_case_clause_raw_tokens(roles)
        .map(|synthesis| synthesis.raw_token_ids)
}

pub fn go_ast_inventory_scratch_comm_clause_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_comm_clause_raw_tokens(roles)
        .map(|synthesis| synthesis.raw_token_ids)
}
