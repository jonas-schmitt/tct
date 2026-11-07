use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_inventory_synthesis::GoAstInventoryDeclarativeTemplateCoverageSpec;
use crate::syntax_go_inventory_synthesis::core::{
    GoAstInventoryFiniteDomainDecision, GoAstInventoryRuleFieldOrder, GoAstInventorySynthesisPlan,
};
use crate::syntax_go_inventory_synthesis::registry::{
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_GO_VERSION,
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_IDENTIFIER,
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_INCOMPLETE,
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_METHODS_POLICY,
    GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_PACKAGE_NAME, GoAstInventoryRegistryPayload,
    go_ast_inventory_interface_type_var_expression_spec,
    go_ast_inventory_interface_type_var_parent_field,
    synthesize_go_ast_inventory_registry_raw_tokens_from_inventory,
};
use crate::syntax_state_machine::SyntaxTokenRoles;
use std::path::Path;

pub const GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID: &str =
    "go-ast-inventory-interface-type-var-synthesis-v1";
pub const GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives + minimal empty-method-set payload fixture spec -> token-only GoSourceFile package var InterfaceType raw-token stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryInterfaceTypeVarPayloadSpec {
    pub package_name: String,
    pub identifier: String,
    pub methods_policy: String,
    pub incomplete: bool,
    pub go_version: String,
}

impl GoAstInventoryInterfaceTypeVarPayloadSpec {
    pub fn checked_default() -> Self {
        Self {
            package_name: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_PACKAGE_NAME.to_string(),
            identifier: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_IDENTIFIER.to_string(),
            methods_policy: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_METHODS_POLICY.to_string(),
            incomplete: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_INCOMPLETE,
            go_version: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_DEFAULT_GO_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryInterfaceTypeVarSynthesis {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_plan: GoAstInventorySynthesisPlan,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
    pub payload_spec: GoAstInventoryInterfaceTypeVarPayloadSpec,
    pub raw_token_ids: Vec<u32>,
}

pub fn synthesize_go_ast_inventory_interface_type_var_raw_tokens(
    roles: &SyntaxTokenRoles,
) -> Result<GoAstInventoryInterfaceTypeVarSynthesis, SyntaxBackendBoundaryError> {
    synthesize_go_ast_inventory_interface_type_var_raw_tokens_from_inventory(
        roles,
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        GoAstInventoryInterfaceTypeVarPayloadSpec::checked_default(),
    )
}

pub fn synthesize_go_ast_inventory_interface_type_var_raw_tokens_from_inventory(
    roles: &SyntaxTokenRoles,
    inventory_path: &Path,
    payload_spec: GoAstInventoryInterfaceTypeVarPayloadSpec,
) -> Result<GoAstInventoryInterfaceTypeVarSynthesis, SyntaxBackendBoundaryError> {
    let payload = GoAstInventoryRegistryPayload::new()
        .with_str("package_name", &payload_spec.package_name)
        .with_str("identifier", &payload_spec.identifier)
        .with_str("methods_policy", &payload_spec.methods_policy)
        .with_bool("incomplete", payload_spec.incomplete)
        .with_str("go_version", &payload_spec.go_version);
    let synthesis = synthesize_go_ast_inventory_registry_raw_tokens_from_inventory(
        roles,
        inventory_path,
        GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID,
        payload,
    )?;

    Ok(GoAstInventoryInterfaceTypeVarSynthesis {
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

pub fn go_ast_inventory_interface_type_var_declarative_coverage_spec()
-> GoAstInventoryDeclarativeTemplateCoverageSpec {
    let payload_spec = GoAstInventoryInterfaceTypeVarPayloadSpec::checked_default();
    GoAstInventoryDeclarativeTemplateCoverageSpec {
        synthesis_api_id: GO_AST_INVENTORY_INTERFACE_TYPE_VAR_SYNTHESIS_API_ID.to_string(),
        parent_field: go_ast_inventory_interface_type_var_parent_field(),
        expression: go_ast_inventory_interface_type_var_expression_spec(payload_spec.incomplete),
    }
}
