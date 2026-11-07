use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::GoAstInventoryDeclarativeTemplateCoverageSpec;
use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryFiniteDomainDecision, GoAstInventoryRuleFieldOrder, GoAstInventorySynthesisPlan,
};
use crate::syntax_go_inventory_synthesis::registry::{
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_GO_VERSION,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_IDENTIFIER,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_INCOMPLETE,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_KIND,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_VALUE,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_TYPE,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_KIND,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_VALUE,
    GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_TYPE, GoAstInventoryRegistryPayload,
    go_ast_inventory_key_value_expr_return_expression_spec,
    go_ast_inventory_key_value_expr_return_parent_field,
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory,
};
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

pub const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-key-value-expr-return-synthesis-v1";
pub const GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + declarative map composite-literal element context + minimal payload fixture spec -> token-only GoSourceFile key-value-expression return raw-token stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryKeyValueExprReturnPayloadSpec {
    pub package_name: String,
    pub identifier: String,
    pub key_type: String,
    pub value_type: String,
    pub key_literal_kind: String,
    pub key_literal_value: String,
    pub value_literal_kind: String,
    pub value_literal_value: String,
    pub incomplete: bool,
    pub go_version: String,
}

impl GoAstInventoryKeyValueExprReturnPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_PACKAGE_NAME.to_string(),
            identifier: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_IDENTIFIER.to_string(),
            key_type: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_TYPE.to_string(),
            value_type: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_TYPE.to_string(),
            key_literal_kind: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_KIND
                .to_string(),
            key_literal_value: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_KEY_LITERAL_VALUE
                .to_string(),
            value_literal_kind: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_KIND
                .to_string(),
            value_literal_value: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_VALUE_LITERAL_VALUE
                .to_string(),
            incomplete: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_INCOMPLETE,
            go_version: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryKeyValueExprReturnSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventoryKeyValueExprReturnPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_key_value_expr_return_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventoryKeyValueExprReturnSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_key_value_expr_return_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventoryKeyValueExprReturnPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_key_value_expr_return_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventoryKeyValueExprReturnPayloadSpec,
) -> Result<GoAstInventoryKeyValueExprReturnSynthesis, SyntaxBackendBoundaryError> {
    let payload = GoAstInventoryRegistryPayload::new()
        .with_str("package_name", &payload_spec.package_name)
        .with_str("identifier", &payload_spec.identifier)
        .with_str("key_type", &payload_spec.key_type)
        .with_str("value_type", &payload_spec.value_type)
        .with_str("key_literal_kind", &payload_spec.key_literal_kind)
        .with_str("key_literal_value", &payload_spec.key_literal_value)
        .with_str("value_literal_kind", &payload_spec.value_literal_kind)
        .with_str("value_literal_value", &payload_spec.value_literal_value)
        .with_bool("incomplete", payload_spec.incomplete)
        .with_str("go_version", &payload_spec.go_version);
    let synthesis = synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        inventory_path,
        GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID,
        payload,
    )?;

    Ok(GoAstInventoryKeyValueExprReturnSynthesis {
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

pub fn go_ast_inventory_key_value_expr_return_declarative_coverage_spec()
-> GoAstInventoryDeclarativeTemplateCoverageSpec {
    let payload_spec = GoAstInventoryKeyValueExprReturnPayloadSpec::checked_default();
    GoAstInventoryDeclarativeTemplateCoverageSpec {
        synthesis_api_id: GO_AST_INVENTORY_KEY_VALUE_EXPR_RETURN_SYNTHESIS_API_ID.to_string(),
        parent_field: go_ast_inventory_key_value_expr_return_parent_field(),
        expression: go_ast_inventory_key_value_expr_return_expression_spec(
            &payload_spec.key_type,
            &payload_spec.value_type,
            &payload_spec.key_literal_kind,
            &payload_spec.key_literal_value,
            &payload_spec.value_literal_kind,
            &payload_spec.value_literal_value,
            payload_spec.incomplete,
        ),
    }
}
