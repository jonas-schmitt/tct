use crate::go_ast_schema_inventory::GoAstSchemaInventoryAdapter;
use crate::grammar_ir::{GrammarField, GrammarIr, GrammarTerminalKind};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use std::path::Path;

pub const GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID: &str = "go-ast-inventory-synthesis-core-v1";
pub const GO_AST_INVENTORY_SYNTHESIS_CORE_BOUNDARY: &str = "checked go/ast inventory field order + finite-domain alternatives -> reusable category synthesis plan for raw Go token emission";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventorySynthesisPlan {
    pub api_id: String,
    pub api_boundary: String,
    pub synthesis_id: String,
    pub inventory_artifact_path: String,
    pub grammar_id: String,
    pub root_rule: String,
    pub validated_field_orders: Vec<GoAstInventoryRuleFieldOrder>,
    pub validated_finite_domain_alternatives: Vec<GoAstInventoryFiniteDomainDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryExpectedFieldOrder {
    pub rule_name: String,
    pub fields: Vec<String>,
}

impl GoAstInventoryExpectedFieldOrder {
    pub fn new(rule_name: &str, fields: &[&str]) -> Self {
        Self {
            rule_name: rule_name.to_string(),
            fields: fields.iter().map(|field| (*field).to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryRuleFieldOrder {
    pub rule_name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryFiniteDomainDecision {
    pub rule_name: String,
    pub field_name: String,
    pub selected_alternative: String,
    pub terminal: GrammarTerminalKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstInventoryFiniteDomainRequirement {
    pub rule_name: String,
    pub field_name: String,
    pub selected_alternative: String,
    pub terminal: GrammarTerminalKind,
}

impl GoAstInventoryFiniteDomainRequirement {
    pub fn new(
        rule_name: &str,
        field_name: &str,
        selected_alternative: &str,
        terminal: GrammarTerminalKind,
    ) -> Self {
        Self {
            rule_name: rule_name.to_string(),
            field_name: field_name.to_string(),
            selected_alternative: selected_alternative.to_string(),
            terminal,
        }
    }
}

pub fn validate_go_ast_inventory_synthesis_plan_from_inventory(
    inventory_path: &Path,
    synthesis_id: &str,
    root_rule: &str,
    expected_field_orders: &[GoAstInventoryExpectedFieldOrder],
    finite_domain_requirements: &[GoAstInventoryFiniteDomainRequirement],
) -> Result<GoAstInventorySynthesisPlan, SyntaxBackendBoundaryError> {
    require_non_empty("go_ast_inventory_synthesis_core.synthesis_id", synthesis_id)?;
    require_non_empty("go_ast_inventory_synthesis_core.root_rule", root_rule)?;
    let grammar =
        GoAstSchemaInventoryAdapter::lower_inventory_file(inventory_path).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_ast_inventory_synthesis_core.inventory",
                format!("failed to lower checked Go AST inventory: {error}"),
            )
        })?;
    if grammar.rule(root_rule).is_none() && root_rule != "GoSourceFile" {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_synthesis_core.root_rule",
            format!("missing inventory root rule {root_rule}"),
        ));
    }
    let validated_field_orders = expected_field_orders
        .iter()
        .map(|expected| validate_rule_field_order(&grammar, expected))
        .collect::<Result<Vec<_>, SyntaxBackendBoundaryError>>()?;
    let validated_finite_domain_alternatives = finite_domain_requirements
        .iter()
        .map(|requirement| validate_field_alternative(&grammar, requirement))
        .collect::<Result<Vec<_>, SyntaxBackendBoundaryError>>()?;

    Ok(GoAstInventorySynthesisPlan {
        api_id: GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID.to_string(),
        api_boundary: GO_AST_INVENTORY_SYNTHESIS_CORE_BOUNDARY.to_string(),
        synthesis_id: synthesis_id.to_string(),
        inventory_artifact_path: inventory_path.display().to_string(),
        grammar_id: grammar.grammar_id,
        root_rule: root_rule.to_string(),
        validated_field_orders,
        validated_finite_domain_alternatives,
    })
}

fn validate_rule_field_order(
    grammar: &GrammarIr,
    expected: &GoAstInventoryExpectedFieldOrder,
) -> Result<GoAstInventoryRuleFieldOrder, SyntaxBackendBoundaryError> {
    let rule_name = expected.rule_name.as_str();
    let rule = grammar.rule(rule_name).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "go_ast_inventory_synthesis_core.field_order",
            format!("missing inventory rule {rule_name}"),
        )
    })?;
    let fields = rule
        .fields
        .iter()
        .map(|field| field.name.clone())
        .collect::<Vec<_>>();
    if fields != expected.fields {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_synthesis_core.field_order",
            format!(
                "rule {rule_name} field order changed: expected {:?}, got {fields:?}",
                expected.fields
            ),
        ));
    }
    Ok(GoAstInventoryRuleFieldOrder {
        rule_name: rule_name.to_string(),
        fields,
    })
}

fn validate_field_alternative(
    grammar: &GrammarIr,
    requirement: &GoAstInventoryFiniteDomainRequirement,
) -> Result<GoAstInventoryFiniteDomainDecision, SyntaxBackendBoundaryError> {
    let rule_name = requirement.rule_name.as_str();
    let field_name = requirement.field_name.as_str();
    let field = grammar
        .rule(rule_name)
        .and_then(|rule| rule.fields.iter().find(|field| field.name == field_name))
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_inventory_synthesis_core.finite_domain",
                format!("missing inventory field {rule_name}.{field_name}"),
            )
        })?;
    require_field_alternative(
        field,
        &requirement.selected_alternative,
        requirement.terminal,
    )?;
    Ok(GoAstInventoryFiniteDomainDecision {
        rule_name: requirement.rule_name.clone(),
        field_name: requirement.field_name.clone(),
        selected_alternative: requirement.selected_alternative.clone(),
        terminal: requirement.terminal,
    })
}

fn require_field_alternative(
    field: &GrammarField,
    selected_alternative: &str,
    terminal: GrammarTerminalKind,
) -> Result<(), SyntaxBackendBoundaryError> {
    if field.alternatives.iter().any(|alternative| {
        alternative.node_kind == selected_alternative && alternative.terminal == terminal
    }) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_synthesis_core.finite_domain",
            format!(
                "field {} missing finite-domain alternative {selected_alternative}/{terminal:?}",
                field.name
            ),
        ))
    }
}

pub(super) fn require_non_empty(
    field: &'static str,
    actual: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected non-empty value",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn require_string(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected:?}, got {actual:?}"),
        ))
    }
}
