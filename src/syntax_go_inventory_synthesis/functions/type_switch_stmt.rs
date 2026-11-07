use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::GoAstInventoryDeclarativeTemplateCoverageSpec;
use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryFiniteDomainDecision, GoAstInventoryRuleFieldOrder, GoAstInventorySynthesisPlan,
};
use crate::syntax_go_inventory_synthesis::registry::{
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_GO_VERSION,
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_IDENTIFIER,
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_PACKAGE_NAME,
    GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_TARGET_IDENTIFIER, GoAstInventoryRegistryPayload,
    go_ast_inventory_type_switch_stmt_parent_field,
    go_ast_inventory_type_switch_stmt_statement_spec,
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory,
};
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

pub const GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-type-switch-stmt-synthesis-v1";
pub const GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + declarative optional-alternative TypeSwitchStmt.Init fixture spec -> token-only GoSourceFile type-switch statement raw-token stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryTypeSwitchStmtPayloadSpec {
    pub package_name: String,
    pub identifier: String,
    pub target_identifier: String,
    pub go_version: String,
}

impl GoAstInventoryTypeSwitchStmtPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_PACKAGE_NAME.to_string(),
            identifier: GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_IDENTIFIER.to_string(),
            target_identifier: GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_TARGET_IDENTIFIER
                .to_string(),
            go_version: GO_AST_INVENTORY_TYPE_SWITCH_STMT_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryTypeSwitchStmtSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventoryTypeSwitchStmtPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_type_switch_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventoryTypeSwitchStmtSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_type_switch_stmt_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventoryTypeSwitchStmtPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_type_switch_stmt_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventoryTypeSwitchStmtPayloadSpec,
) -> Result<GoAstInventoryTypeSwitchStmtSynthesis, SyntaxBackendBoundaryError> {
    let payload = GoAstInventoryRegistryPayload::new()
        .with_str("package_name", &payload_spec.package_name)
        .with_str("identifier", &payload_spec.identifier)
        .with_str("target_identifier", &payload_spec.target_identifier)
        .with_str("go_version", &payload_spec.go_version);
    let synthesis = synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        inventory_path,
        GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID,
        payload,
    )?;

    Ok(GoAstInventoryTypeSwitchStmtSynthesis {
        api_id: synthesis.api_id,
        api_boundary: synthesis.api_boundary,
        inventory_artifact_path: synthesis.inventory_artifact_path,
        grammar_id: synthesis.grammar_id,
        root_rule: synthesis.root_rule,
        validated_field_orders: synthesis.validated_field_orders,
        validated_finite_domain_alternatives: synthesis.validated_finite_domain_alternatives,
        payload_spec,
        raw_token_ids: synthesis.raw_token_ids,
        synthesis_plan: synthesis.synthesis_plan,
    })
}

pub fn go_ast_inventory_type_switch_stmt_declarative_coverage_spec()
-> GoAstInventoryDeclarativeTemplateCoverageSpec {
    let payload_spec = GoAstInventoryTypeSwitchStmtPayloadSpec::checked_default();
    GoAstInventoryDeclarativeTemplateCoverageSpec {
        synthesis_api_id: GO_AST_INVENTORY_TYPE_SWITCH_STMT_SYNTHESIS_API_ID.to_string(),
        parent_field: go_ast_inventory_type_switch_stmt_parent_field(),
        expression: go_ast_inventory_type_switch_stmt_statement_spec(
            &payload_spec.target_identifier,
        ),
    }
}
