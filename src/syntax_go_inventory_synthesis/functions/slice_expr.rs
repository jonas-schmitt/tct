use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::GoAstInventoryDeclarativeTemplateCoverageSpec;
use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryFiniteDomainDecision, GoAstInventoryRuleFieldOrder, GoAstInventorySynthesisPlan,
};
use crate::syntax_go_inventory_synthesis::registry::{
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_GO_VERSION,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_KIND,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_VALUE,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_IDENTIFIER,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_KIND,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_VALUE,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_PACKAGE_NAME,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_RESULT_TYPE,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER,
    GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_THREE_INDEX, GoAstInventoryRegistryPayload,
    go_ast_inventory_slice_expr_return_expression_spec,
    go_ast_inventory_slice_expr_return_parent_field,
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory,
};
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

pub const GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-slice-expr-return-synthesis-v1";
pub const GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + declarative optional policy and boolean payload fixture spec -> token-only GoSourceFile slice-expression return raw-token stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventorySliceExprReturnPayloadSpec {
    pub package_name: String,
    pub identifier: String,
    pub result_type: String,
    pub target_identifier: String,
    pub low_literal_kind: String,
    pub low_literal_value: String,
    pub high_literal_kind: String,
    pub high_literal_value: String,
    pub three_index: bool,
    pub go_version: String,
}

impl GoAstInventorySliceExprReturnPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_PACKAGE_NAME.to_string(),
            identifier: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_IDENTIFIER.to_string(),
            result_type: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_RESULT_TYPE.to_string(),
            target_identifier: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_TARGET_IDENTIFIER
                .to_string(),
            low_literal_kind: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_KIND
                .to_string(),
            low_literal_value: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_LOW_LITERAL_VALUE
                .to_string(),
            high_literal_kind: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_KIND
                .to_string(),
            high_literal_value: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_HIGH_LITERAL_VALUE
                .to_string(),
            three_index: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_THREE_INDEX,
            go_version: GO_AST_INVENTORY_SLICE_EXPR_RETURN_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventorySliceExprReturnSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventorySliceExprReturnPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_slice_expr_return_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventorySliceExprReturnSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_slice_expr_return_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventorySliceExprReturnPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_slice_expr_return_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventorySliceExprReturnPayloadSpec,
) -> Result<GoAstInventorySliceExprReturnSynthesis, SyntaxBackendBoundaryError> {
    let payload = GoAstInventoryRegistryPayload::new()
        .with_str("package_name", &payload_spec.package_name)
        .with_str("identifier", &payload_spec.identifier)
        .with_str("result_type", &payload_spec.result_type)
        .with_str("target_identifier", &payload_spec.target_identifier)
        .with_str("low_literal_kind", &payload_spec.low_literal_kind)
        .with_str("low_literal_value", &payload_spec.low_literal_value)
        .with_str("high_literal_kind", &payload_spec.high_literal_kind)
        .with_str("high_literal_value", &payload_spec.high_literal_value)
        .with_bool("three_index", payload_spec.three_index)
        .with_str("go_version", &payload_spec.go_version);
    let synthesis = synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        inventory_path,
        GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID,
        payload,
    )?;

    Ok(GoAstInventorySliceExprReturnSynthesis {
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

pub fn go_ast_inventory_slice_expr_return_declarative_coverage_spec()
-> GoAstInventoryDeclarativeTemplateCoverageSpec {
    go_ast_inventory_slice_expr_return_declarative_coverage_spec_for_payload(
        &GoAstInventorySliceExprReturnPayloadSpec::checked_default(),
    )
}

fn go_ast_inventory_slice_expr_return_declarative_coverage_spec_for_payload(
    payload_spec: &GoAstInventorySliceExprReturnPayloadSpec,
) -> GoAstInventoryDeclarativeTemplateCoverageSpec {
    GoAstInventoryDeclarativeTemplateCoverageSpec {
        synthesis_api_id: GO_AST_INVENTORY_SLICE_EXPR_RETURN_SYNTHESIS_API_ID.to_string(),
        parent_field: go_ast_inventory_slice_expr_return_parent_field(),
        expression: go_ast_inventory_slice_expr_return_expression_spec(
            &payload_spec.target_identifier,
            &payload_spec.low_literal_kind,
            &payload_spec.low_literal_value,
            &payload_spec.high_literal_kind,
            &payload_spec.high_literal_value,
            payload_spec.three_index,
        ),
    }
}
