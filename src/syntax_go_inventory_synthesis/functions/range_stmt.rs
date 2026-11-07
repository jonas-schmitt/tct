use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::GoAstInventoryDeclarativeTemplateCoverageSpec;
use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryFiniteDomainDecision, GoAstInventoryRuleFieldOrder, GoAstInventorySynthesisPlan,
};
use crate::syntax_go_inventory_synthesis::registry::{
    GO_AST_INVENTORY_RANGE_STMT_DEFAULT_GO_VERSION, GO_AST_INVENTORY_RANGE_STMT_DEFAULT_IDENTIFIER,
    GO_AST_INVENTORY_RANGE_STMT_DEFAULT_KEY_IDENTIFIER,
    GO_AST_INVENTORY_RANGE_STMT_DEFAULT_PACKAGE_NAME,
    GO_AST_INVENTORY_RANGE_STMT_DEFAULT_SOURCE_IDENTIFIER,
    GO_AST_INVENTORY_RANGE_STMT_DEFAULT_TOKEN, GoAstInventoryRegistryPayload,
    go_ast_inventory_range_stmt_parent_field, go_ast_inventory_range_stmt_statement_spec,
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory,
};
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

pub const GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-range-stmt-synthesis-v1";
pub const GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + declarative optional-alternative RangeStmt.Key fixture spec -> token-only GoSourceFile range statement raw-token stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryRangeStmtPayloadSpec {
    pub package_name: String,
    pub identifier: String,
    pub key_identifier: String,
    pub token: String,
    pub source_identifier: String,
    pub go_version: String,
}

impl GoAstInventoryRangeStmtPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_RANGE_STMT_DEFAULT_PACKAGE_NAME.to_string(),
            identifier: GO_AST_INVENTORY_RANGE_STMT_DEFAULT_IDENTIFIER.to_string(),
            key_identifier: GO_AST_INVENTORY_RANGE_STMT_DEFAULT_KEY_IDENTIFIER.to_string(),
            token: GO_AST_INVENTORY_RANGE_STMT_DEFAULT_TOKEN.to_string(),
            source_identifier: GO_AST_INVENTORY_RANGE_STMT_DEFAULT_SOURCE_IDENTIFIER.to_string(),
            go_version: GO_AST_INVENTORY_RANGE_STMT_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryRangeStmtSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventoryRangeStmtPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_range_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventoryRangeStmtSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_range_stmt_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventoryRangeStmtPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_range_stmt_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventoryRangeStmtPayloadSpec,
) -> Result<GoAstInventoryRangeStmtSynthesis, SyntaxBackendBoundaryError> {
    let payload = GoAstInventoryRegistryPayload::new()
        .with_str("package_name", &payload_spec.package_name)
        .with_str("identifier", &payload_spec.identifier)
        .with_str("key_identifier", &payload_spec.key_identifier)
        .with_str("token", &payload_spec.token)
        .with_str("source_identifier", &payload_spec.source_identifier)
        .with_str("go_version", &payload_spec.go_version);
    let synthesis = synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        inventory_path,
        GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID,
        payload,
    )?;

    Ok(GoAstInventoryRangeStmtSynthesis {
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

pub fn go_ast_inventory_range_stmt_declarative_coverage_spec()
-> GoAstInventoryDeclarativeTemplateCoverageSpec {
    let payload_spec = GoAstInventoryRangeStmtPayloadSpec::checked_default();
    GoAstInventoryDeclarativeTemplateCoverageSpec {
        synthesis_api_id: GO_AST_INVENTORY_RANGE_STMT_SYNTHESIS_API_ID.to_string(),
        parent_field: go_ast_inventory_range_stmt_parent_field(),
        expression: go_ast_inventory_range_stmt_statement_spec(
            &payload_spec.key_identifier,
            &payload_spec.token,
            &payload_spec.source_identifier,
        ),
    }
}
