use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::GoAstInventoryDeclarativeTemplateCoverageSpec;
use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryFiniteDomainDecision, GoAstInventoryRuleFieldOrder, GoAstInventorySynthesisPlan,
};
use crate::syntax_go_inventory_synthesis::registry::{
    GO_AST_INVENTORY_IF_STMT_DEFAULT_BODY_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_BODY_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_COND_IDENTIFIER,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_ELSE_RETURN_LITERAL_KIND,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_ELSE_RETURN_LITERAL_VALUE,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_GO_VERSION, GO_AST_INVENTORY_IF_STMT_DEFAULT_IDENTIFIER,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_IDENTIFIER,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_LITERAL_KIND,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_LITERAL_VALUE,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_OPERATOR, GO_AST_INVENTORY_IF_STMT_DEFAULT_PACKAGE_NAME,
    GO_AST_INVENTORY_IF_STMT_DEFAULT_RESULT_TYPE, GoAstInventoryRegistryPayload,
    go_ast_inventory_if_stmt_parent_field, go_ast_inventory_if_stmt_statement_spec,
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory,
};
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

pub const GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID: &str = "go-ast-inventory-if-stmt-synthesis-v1";
pub const GO_AST_INVENTORY_IF_STMT_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + declarative optional-field if statement fixture spec -> token-only GoSourceFile if statement raw-token stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryIfStmtPayloadSpec {
    pub package_name: String,
    pub identifier: String,
    pub result_type: String,
    pub init_identifier: String,
    pub init_operator: String,
    pub init_literal_kind: String,
    pub init_literal_value: String,
    pub cond_identifier: String,
    pub body_return_literal_kind: String,
    pub body_return_literal_value: String,
    pub else_return_literal_kind: String,
    pub else_return_literal_value: String,
    pub go_version: String,
}

impl GoAstInventoryIfStmtPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_IF_STMT_DEFAULT_PACKAGE_NAME.to_string(),
            identifier: GO_AST_INVENTORY_IF_STMT_DEFAULT_IDENTIFIER.to_string(),
            result_type: GO_AST_INVENTORY_IF_STMT_DEFAULT_RESULT_TYPE.to_string(),
            init_identifier: GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_IDENTIFIER.to_string(),
            init_operator: GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_OPERATOR.to_string(),
            init_literal_kind: GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_LITERAL_KIND.to_string(),
            init_literal_value: GO_AST_INVENTORY_IF_STMT_DEFAULT_INIT_LITERAL_VALUE.to_string(),
            cond_identifier: GO_AST_INVENTORY_IF_STMT_DEFAULT_COND_IDENTIFIER.to_string(),
            body_return_literal_kind: GO_AST_INVENTORY_IF_STMT_DEFAULT_BODY_RETURN_LITERAL_KIND
                .to_string(),
            body_return_literal_value: GO_AST_INVENTORY_IF_STMT_DEFAULT_BODY_RETURN_LITERAL_VALUE
                .to_string(),
            else_return_literal_kind: GO_AST_INVENTORY_IF_STMT_DEFAULT_ELSE_RETURN_LITERAL_KIND
                .to_string(),
            else_return_literal_value: GO_AST_INVENTORY_IF_STMT_DEFAULT_ELSE_RETURN_LITERAL_VALUE
                .to_string(),
            go_version: GO_AST_INVENTORY_IF_STMT_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryIfStmtSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventoryIfStmtPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_if_stmt_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventoryIfStmtSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_if_stmt_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventoryIfStmtPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_if_stmt_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventoryIfStmtPayloadSpec,
) -> Result<GoAstInventoryIfStmtSynthesis, SyntaxBackendBoundaryError> {
    let payload = GoAstInventoryRegistryPayload::new()
        .with_str("package_name", &payload_spec.package_name)
        .with_str("identifier", &payload_spec.identifier)
        .with_str("result_type", &payload_spec.result_type)
        .with_str("init_identifier", &payload_spec.init_identifier)
        .with_str("init_operator", &payload_spec.init_operator)
        .with_str("init_literal_kind", &payload_spec.init_literal_kind)
        .with_str("init_literal_value", &payload_spec.init_literal_value)
        .with_str("cond_identifier", &payload_spec.cond_identifier)
        .with_str(
            "body_return_literal_kind",
            &payload_spec.body_return_literal_kind,
        )
        .with_str(
            "body_return_literal_value",
            &payload_spec.body_return_literal_value,
        )
        .with_str(
            "else_return_literal_kind",
            &payload_spec.else_return_literal_kind,
        )
        .with_str(
            "else_return_literal_value",
            &payload_spec.else_return_literal_value,
        )
        .with_str("go_version", &payload_spec.go_version);
    let synthesis = synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        inventory_path,
        GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID,
        payload,
    )?;

    Ok(GoAstInventoryIfStmtSynthesis {
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

pub fn go_ast_inventory_if_stmt_declarative_coverage_spec()
-> GoAstInventoryDeclarativeTemplateCoverageSpec {
    let payload_spec = GoAstInventoryIfStmtPayloadSpec::checked_default();
    GoAstInventoryDeclarativeTemplateCoverageSpec {
        synthesis_api_id: GO_AST_INVENTORY_IF_STMT_SYNTHESIS_API_ID.to_string(),
        parent_field: go_ast_inventory_if_stmt_parent_field(),
        expression: go_ast_inventory_if_stmt_statement_spec(
            &payload_spec.init_identifier,
            &payload_spec.init_operator,
            &payload_spec.init_literal_kind,
            &payload_spec.init_literal_value,
            &payload_spec.cond_identifier,
            &payload_spec.body_return_literal_kind,
            &payload_spec.body_return_literal_value,
            &payload_spec.else_return_literal_kind,
            &payload_spec.else_return_literal_value,
        ),
    }
}
