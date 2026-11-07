use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::GoAstInventoryDeclarativeTemplateCoverageSpec;
use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryFiniteDomainDecision, GoAstInventoryRuleFieldOrder, GoAstInventorySynthesisPlan,
};
use crate::syntax_go_inventory_synthesis::registry::{
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELD_NAME,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELD_TYPE,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELDS_POLICY,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_GO_VERSION,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_INCOMPLETE,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_PACKAGE_NAME,
    GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_TYPE_NAME, GoAstInventoryRegistryPayload,
    go_ast_inventory_struct_type_decl_expression_spec,
    go_ast_inventory_struct_type_decl_parent_field,
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory,
};
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

pub const GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-struct-type-decl-synthesis-v1";
pub const GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + declarative struct-type payload fixture spec -> token-only GoSourceFile package type-declaration StructType raw-token stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryStructTypeDeclPayloadSpec {
    pub package_name: String,
    pub type_name: String,
    pub field_name: String,
    pub field_type: String,
    pub fields_policy: String,
    pub incomplete: bool,
    pub go_version: String,
}

impl GoAstInventoryStructTypeDeclPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_PACKAGE_NAME.to_string(),
            type_name: GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_TYPE_NAME.to_string(),
            field_name: GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELD_NAME.to_string(),
            field_type: GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELD_TYPE.to_string(),
            fields_policy: GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_FIELDS_POLICY.to_string(),
            incomplete: GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_INCOMPLETE,
            go_version: GO_AST_INVENTORY_STRUCT_TYPE_DECL_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryStructTypeDeclSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventoryStructTypeDeclPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_struct_type_decl_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventoryStructTypeDeclSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_struct_type_decl_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventoryStructTypeDeclPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_struct_type_decl_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventoryStructTypeDeclPayloadSpec,
) -> Result<GoAstInventoryStructTypeDeclSynthesis, SyntaxBackendBoundaryError> {
    let payload = GoAstInventoryRegistryPayload::new()
        .with_str("package_name", &payload_spec.package_name)
        .with_str("type_name", &payload_spec.type_name)
        .with_str("field_name", &payload_spec.field_name)
        .with_str("field_type", &payload_spec.field_type)
        .with_str("fields_policy", &payload_spec.fields_policy)
        .with_bool("incomplete", payload_spec.incomplete)
        .with_str("go_version", &payload_spec.go_version);
    let synthesis = synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        inventory_path,
        GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID,
        payload,
    )?;

    Ok(GoAstInventoryStructTypeDeclSynthesis {
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

pub fn go_ast_inventory_struct_type_decl_declarative_coverage_spec()
-> GoAstInventoryDeclarativeTemplateCoverageSpec {
    go_ast_inventory_struct_type_decl_declarative_coverage_spec_for_payload(
        &GoAstInventoryStructTypeDeclPayloadSpec::checked_default(),
    )
}

fn go_ast_inventory_struct_type_decl_declarative_coverage_spec_for_payload(
    payload_spec: &GoAstInventoryStructTypeDeclPayloadSpec,
) -> GoAstInventoryDeclarativeTemplateCoverageSpec {
    GoAstInventoryDeclarativeTemplateCoverageSpec {
        synthesis_api_id: GO_AST_INVENTORY_STRUCT_TYPE_DECL_SYNTHESIS_API_ID.to_string(),
        parent_field: go_ast_inventory_struct_type_decl_parent_field(),
        expression: go_ast_inventory_struct_type_decl_expression_spec(
            &payload_spec.field_name,
            &payload_spec.field_type,
            payload_spec.incomplete,
        ),
    }
}
