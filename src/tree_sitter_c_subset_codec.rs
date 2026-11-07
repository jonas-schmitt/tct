//! Tree-sitter C construct-subset codec (MG-6).
//!
//! Projects a deliberate minimal-function construct subset from the pinned
//! `tree-sitter-c` node-types export into `GrammarIR` → `StructuralModel` →
//! raw syntax state machine encode/decode, then render/reparse.
//!
//! Claim boundary: syntax-level tree-sitter-C subset only. Not full C, not a
//! Tree-sitter runtime dependency, not tree-sitter-go, and not a Go layout pin.
//!
//! Subset surface (v0): one or more free functions of the shape
//! `int <name>(void) { return <u8>; }` with optional ASCII whitespace.

use crate::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarRule, GrammarTerminalKind,
};
use crate::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
    GrammarStructuralTokenRange,
};
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedTerminal,
    RawSyntaxDecodedTree, RawSyntaxDecodedValue,
};
use crate::tree_sitter_metadata::{
    CHILDREN_FIELD_NAME, SUBTYPE_FIELD_NAME, TreeSitterGrammarPin, TreeSitterMetadataAdapter,
};
use crate::tree_sitter_subset_framework::{
    CompiledSubsetCodecCore, TreeSitterSubsetError, TreeSitterSubsetRoundTrip,
    observe_mixed_payload_risks,
};
use std::path::Path;

/// Phase-tagged C subset inventory / codec errors (shared framework type).
pub type TreeSitterCSubsetError = TreeSitterSubsetError;
/// Claim-boundary round-trip report (shared framework type).
pub type TreeSitterCSubsetRoundTrip = TreeSitterSubsetRoundTrip;
/// Shared mixed-payload risk table entry (framework type).
pub use crate::tree_sitter_subset_framework::MixedPayloadRisk;

/// Stable subset identity for the minimal free function construct set.
pub const SUBSET_ID: &str = "tree_sitter_c_minimal_function_subset_v0";
/// Claim boundary string carried in reports and docs.
pub const CLAIM_BOUNDARY: &str = "syntax_level_tree_sitter_c_subset_only";
/// Grammar id used when lowering the subset GrammarIR.
pub const SUBSET_GRAMMAR_ID: &str = "tree_sitter_c_minimal_function_subset_v0";

const TRANSLATION_UNIT_RULE: &str = "translation_unit";
const FUNCTION_DEFINITION_RULE: &str = "function_definition";
const TYPE_SPECIFIER_RULE: &str = "type_specifier";
const PRIMITIVE_TYPE_RULE: &str = "primitive_type";
const DECLARATOR_RULE: &str = "_declarator";
const FUNCTION_DECLARATOR_RULE: &str = "function_declarator";
const IDENTIFIER_RULE: &str = "identifier";
const PARAMETER_LIST_RULE: &str = "parameter_list";
const PARAMETER_DECLARATION_RULE: &str = "parameter_declaration";
const COMPOUND_STATEMENT_RULE: &str = "compound_statement";
const STATEMENT_RULE: &str = "statement";
const RETURN_STATEMENT_RULE: &str = "return_statement";
const EXPRESSION_RULE: &str = "expression";
const NUMBER_LITERAL_RULE: &str = "number_literal";

const TYPE_FIELD: &str = "type";
const DECLARATOR_FIELD: &str = "declarator";
const BODY_FIELD: &str = "body";
const PARAMETERS_FIELD: &str = "parameters";
/// Synthetic payload field under `primitive_type` / `identifier` wrappers.
const NAME_FIELD: &str = "name";
/// Synthetic IntegerPayload field under the `number_literal` wrapper.
const NUMBER_VALUE_FIELD: &str = "value";

/// Pin rule names that the minimal-function subset projects from.
///
/// Closed construct set for v0:
/// `translation_unit` → one `function_definition` → `primitive_type` +
/// `function_declarator`(`identifier`, `parameter_list`/`parameter_declaration`)
/// + `compound_statement` → `return_statement` → `number_literal`.
pub const SUBSET_PIN_RULE_NAMES: &[&str] = &[
    TRANSLATION_UNIT_RULE,
    FUNCTION_DEFINITION_RULE,
    TYPE_SPECIFIER_RULE,
    PRIMITIVE_TYPE_RULE,
    DECLARATOR_RULE,
    FUNCTION_DECLARATOR_RULE,
    IDENTIFIER_RULE,
    PARAMETER_LIST_RULE,
    PARAMETER_DECLARATION_RULE,
    COMPOUND_STATEMENT_RULE,
    STATEMENT_RULE,
    RETURN_STATEMENT_RULE,
    EXPRESSION_RULE,
    NUMBER_LITERAL_RULE,
];

/// Full-pin fields that mix Nonterminal wrappers with bare payload terminals
/// on a single choice field. Subset projection must homogenize these before
/// raw SM work (see `docs/tree-sitter-json-mixed-payload.md`).
pub const MIXED_PAYLOAD_RISKS: &[MixedPayloadRisk] = &[
    MixedPayloadRisk {
        owner_rule: DECLARATOR_RULE,
        owner_field: SUBTYPE_FIELD_NAME,
        payload_kinds: &["identifier:IdentifierPayload"],
        subset_resolution: "wrap identifier as Nonterminal with synthetic name IdentifierPayload field",
    },
    MixedPayloadRisk {
        owner_rule: EXPRESSION_RULE,
        owner_field: SUBTYPE_FIELD_NAME,
        payload_kinds: &[
            "number_literal:IntegerPayload",
            "identifier:IdentifierPayload",
            "true:BooleanPayload",
            "false:BooleanPayload",
        ],
        subset_resolution: "keep only number_literal as Nonterminal wrapper with synthetic value IntegerPayload field (u8 0..=255)",
    },
];

// MixedPayloadRisk is shared: re-exported via the framework type in MIXED_PAYLOAD_RISKS.

/// Pin-side inventory report for MG-6a (not a codec claim by itself).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSitterCSubsetPinInventory {
    pub pin_id: String,
    pub grammar_package: String,
    pub grammar_version: String,
    pub node_types_sha256: String,
    pub full_rule_count: usize,
    pub subset_pin_rules_present: Vec<String>,
    pub mixed_payload_risks_observed: Vec<String>,
    pub claim_boundary: &'static str,
    pub subset_id: &'static str,
}

/// Restricted primitive type names retained in the subset (`int` / `void`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CSubsetPrimitiveType {
    Int,
    Void,
}

impl CSubsetPrimitiveType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Void => "void",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name {
            "int" => Some(Self::Int),
            "void" => Some(Self::Void),
            _ => None,
        }
    }
}

/// One parameter declaration in the subset: type only (no declarator name).
///
/// Covers the `(void)` form used by the v0 fixtures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSubsetParameter {
    pub ty: CSubsetPrimitiveType,
}

/// One `return <u8>;` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSubsetReturn {
    pub value: u8,
}

/// One free function in the minimal subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSubsetFunction {
    pub return_type: CSubsetPrimitiveType,
    pub name: String,
    pub parameters: Vec<CSubsetParameter>,
    pub returns: Vec<CSubsetReturn>,
}

/// Root translation unit: ordered free functions only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSubsetProgram {
    pub functions: Vec<CSubsetFunction>,
}

impl CSubsetProgram {
    /// Parse a fixed subset C program (ASCII whitespace allowed between tokens).
    pub fn parse(source: &str) -> Result<Self, TreeSitterCSubsetError> {
        let mut cursor = source;
        let mut functions = Vec::new();
        loop {
            cursor = skip_ws(cursor);
            if cursor.is_empty() {
                break;
            }
            let (function, rest) = parse_function(cursor)?;
            functions.push(function);
            cursor = rest;
        }
        if functions.is_empty() {
            return Err(TreeSitterCSubsetError::new(
                "parse",
                "subset program must contain at least one function",
            ));
        }
        Ok(Self { functions })
    }

    /// Render a canonical pretty form of the subset program.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (index, function) in self.functions.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push_str(function.return_type.as_str());
            out.push(' ');
            out.push_str(&function.name);
            out.push('(');
            for (param_index, param) in function.parameters.iter().enumerate() {
                if param_index > 0 {
                    out.push(',');
                    out.push(' ');
                }
                out.push_str(param.ty.as_str());
            }
            out.push_str(") {\n");
            for ret in &function.returns {
                out.push_str("  return ");
                out.push_str(&ret.value.to_string());
                out.push_str(";\n");
            }
            out.push('}');
            if index + 1 < self.functions.len() {
                out.push('\n');
            }
        }
        out
    }
}

fn skip_ws(source: &str) -> &str {
    source.trim_start_matches(|ch: char| ch.is_ascii_whitespace())
}

fn parse_function(source: &str) -> Result<(CSubsetFunction, &str), TreeSitterCSubsetError> {
    let source = skip_ws(source);
    let (return_type, rest) = parse_primitive_type(source)?;
    if return_type != CSubsetPrimitiveType::Int {
        return Err(TreeSitterCSubsetError::new(
            "parse",
            "subset function return type must be int",
        ));
    }
    let rest = skip_ws(rest);
    let (name, rest) = parse_identifier(rest)?;
    let rest = skip_ws(rest);
    let rest = rest.strip_prefix('(').ok_or_else(|| {
        TreeSitterCSubsetError::new(
            "parse",
            format!("expected '(' after function name near {rest:?}"),
        )
    })?;
    let rest = skip_ws(rest);
    let (parameters, rest) = parse_parameter_list_body(rest)?;
    let rest = skip_ws(rest);
    let rest = rest.strip_prefix('{').ok_or_else(|| {
        TreeSitterCSubsetError::new(
            "parse",
            format!("expected '{{' to open function body near {rest:?}"),
        )
    })?;
    let (returns, rest) = parse_compound_body(rest)?;
    Ok((
        CSubsetFunction {
            return_type,
            name,
            parameters,
            returns,
        },
        rest,
    ))
}

fn parse_parameter_list_body(
    source: &str,
) -> Result<(Vec<CSubsetParameter>, &str), TreeSitterCSubsetError> {
    let source = skip_ws(source);
    if let Some(rest) = source.strip_prefix(')') {
        return Ok((Vec::new(), rest));
    }
    // v0: only a single type-only parameter (typically `void`).
    let (ty, rest) = parse_primitive_type(source)?;
    let rest = skip_ws(rest);
    let rest = rest.strip_prefix(')').ok_or_else(|| {
        TreeSitterCSubsetError::new(
            "parse",
            format!("subset parameter list only allows one type-only parameter near {rest:?}"),
        )
    })?;
    Ok((vec![CSubsetParameter { ty }], rest))
}

fn parse_compound_body(source: &str) -> Result<(Vec<CSubsetReturn>, &str), TreeSitterCSubsetError> {
    let mut rest = skip_ws(source);
    let mut returns = Vec::new();
    loop {
        rest = skip_ws(rest);
        if let Some(after) = rest.strip_prefix('}') {
            return Ok((returns, after));
        }
        let after_return = rest.strip_prefix("return").ok_or_else(|| {
            TreeSitterCSubsetError::new(
                "parse",
                format!("subset compound body only allows return statements near {rest:?}"),
            )
        })?;
        // Require a non-identifier boundary after `return`.
        if after_return
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(TreeSitterCSubsetError::new(
                "parse",
                "return keyword boundary is malformed",
            ));
        }
        rest = skip_ws(after_return);
        let (value, after_number) = parse_u8_number(rest)?;
        rest = skip_ws(after_number);
        rest = rest.strip_prefix(';').ok_or_else(|| {
            TreeSitterCSubsetError::new(
                "parse",
                format!("return statement missing ';' near {rest:?}"),
            )
        })?;
        returns.push(CSubsetReturn { value });
    }
}

fn parse_primitive_type(
    source: &str,
) -> Result<(CSubsetPrimitiveType, &str), TreeSitterCSubsetError> {
    let source = skip_ws(source);
    // Pair each static lexeme with its enum variant so parse cannot fail on
    // the closed candidate set (no unwrap/expect on infallible static data).
    for (candidate, ty) in [
        ("void", CSubsetPrimitiveType::Void),
        ("int", CSubsetPrimitiveType::Int),
    ] {
        if let Some(rest) = source.strip_prefix(candidate) {
            if rest
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                continue;
            }
            return Ok((ty, rest));
        }
    }
    Err(TreeSitterCSubsetError::new(
        "parse",
        format!("expected primitive type int|void near {source:?}"),
    ))
}

fn parse_identifier(source: &str) -> Result<(String, &str), TreeSitterCSubsetError> {
    let source = skip_ws(source);
    let mut chars = source.char_indices();
    let Some((_, first)) = chars.next() else {
        return Err(TreeSitterCSubsetError::new("parse", "expected identifier"));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(TreeSitterCSubsetError::new(
            "parse",
            format!("identifier must start with ASCII letter or '_', near {source:?}"),
        ));
    }
    let mut end = first.len_utf8();
    for (index, ch) in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end = index + ch.len_utf8();
            continue;
        }
        break;
    }
    let name = &source[..end];
    if name == "int" || name == "void" || name == "return" {
        return Err(TreeSitterCSubsetError::new(
            "parse",
            format!("reserved word {name:?} is not a subset identifier"),
        ));
    }
    Ok((name.to_string(), &source[end..]))
}

fn parse_u8_number(source: &str) -> Result<(u8, &str), TreeSitterCSubsetError> {
    let source = skip_ws(source);
    let digit_len = source
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_len == 0 {
        return Err(TreeSitterCSubsetError::new(
            "parse",
            format!("expected unsigned decimal number near {source:?}"),
        ));
    }
    let number = &source[..digit_len];
    if number.len() > 1 && number.starts_with('0') {
        return Err(TreeSitterCSubsetError::new(
            "parse",
            format!("number {number:?} has a leading zero outside the subset"),
        ));
    }
    let value = number.parse::<u8>().map_err(|_| {
        TreeSitterCSubsetError::new(
            "parse",
            format!("number {number:?} is outside the fixed u8 subset 0..=255"),
        )
    })?;
    Ok((value, &source[digit_len..]))
}

/// Compiled subset codec bound to the pinned grammar provenance.
#[derive(Debug)]
pub struct TreeSitterCSubsetCodec {
    /// Shared pin → SM → multi-alt freeze core (CPU-2 framework).
    core: CompiledSubsetCodecCore,
}

impl TreeSitterCSubsetCodec {
    /// Load the C pin, project the fixed subset, and compile the raw SM model.
    pub fn from_pinned_fixture() -> Result<Self, TreeSitterCSubsetError> {
        Self::from_pinned_dir(&TreeSitterGrammarPin::c_fixture_dir())
    }

    /// Load a pinned grammar directory and project the fixed subset.
    pub fn from_pinned_dir(dir: &Path) -> Result<Self, TreeSitterCSubsetError> {
        // Keep inventory residuals honest even when constructing the codec.
        let _inventory = inventory_pinned_c_subset(dir)?;
        let core = CompiledSubsetCodecCore::from_pinned_dir(
            dir,
            SUBSET_GRAMMAR_ID,
            project_minimal_function_subset,
            subset_token_layout(),
        )?;
        Ok(Self { core })
    }

    pub fn pin(&self) -> &TreeSitterGrammarPin {
        &self.core.pin
    }

    pub fn grammar(&self) -> &GrammarIr {
        &self.core.grammar
    }

    pub fn sidecar(&self) -> &GrammarStructuralModelSidecar {
        &self.core.sidecar
    }

    pub fn model(&self) -> &CompiledSyntaxModel {
        &self.core.model
    }

    pub fn claim_boundary(&self) -> &'static str {
        CLAIM_BOUNDARY
    }

    /// Encode a subset program into an expanded raw syntax token stream.
    ///
    /// Language-specific half: lower `CSubsetProgram` to a model-conforming
    /// `RawSyntaxDecodedNode`. Structure walk and payload emission are generic
    /// via `CompiledSubsetCodecCore::encode_decoded_tree`.
    pub fn encode(&self, program: &CSubsetProgram) -> Result<Vec<u32>, TreeSitterCSubsetError> {
        let root = program_to_decoded_node(program)?;
        self.core.encode_decoded_tree(&root)
    }

    /// Strict raw SM decode of a complete expanded token stream.
    pub fn decode_raw(
        &self,
        tokens: &[u32],
    ) -> Result<RawSyntaxDecodedTree, TreeSitterCSubsetError> {
        self.core
            .model
            .decode_complete_tree(tokens)
            .map_err(|error| TreeSitterCSubsetError::new("decode", format!("{error:?}")))
    }

    /// Decode tokens into the subset program AST.
    pub fn decode(&self, tokens: &[u32]) -> Result<CSubsetProgram, TreeSitterCSubsetError> {
        let tree = self.decode_raw(tokens)?;
        let root = tree
            .decoded_node_tree()
            .map_err(|error| TreeSitterCSubsetError::new("decode_tree", error.to_string()))?;
        program_from_decoded_root(&root)
    }

    /// Parse → encode → raw SM decode → render → reparse round-trip.
    pub fn round_trip_source(
        &self,
        source: &str,
    ) -> Result<TreeSitterCSubsetRoundTrip, TreeSitterCSubsetError> {
        self.core.run_round_trip_source(
            source,
            CLAIM_BOUNDARY,
            SUBSET_ID,
            CSubsetProgram::parse,
            |program| self.encode(program),
            |tokens| self.decode(tokens),
            CSubsetProgram::render,
            "program",
        )
    }
}

/// Sidecar-only layout for the C minimal-function subset (not a Go layout pin).
pub fn subset_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 577,
        pad_token: 576,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "tree_sitter_c_subset.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 64,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_c_subset.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 64,
                end_exclusive: 320,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_c_subset.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 320,
                end_exclusive: 576,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "tree_sitter_c_subset.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

/// Inventory the pinned tree-sitter-c export for the minimal-function subset.
pub fn inventory_pinned_c_subset(
    dir: &Path,
) -> Result<TreeSitterCSubsetPinInventory, TreeSitterCSubsetError> {
    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_c_inventory_v0");
    let (full_grammar, pin) = adapter
        .lower_pinned_grammar_dir(dir)
        .map_err(|error| TreeSitterCSubsetError::new("lower", error.to_string()))?;
    inventory_from_full_grammar(&full_grammar, &pin)
}

/// Inventory using the checked-in C metadata fixture.
pub fn inventory_pinned_c_subset_fixture()
-> Result<TreeSitterCSubsetPinInventory, TreeSitterCSubsetError> {
    inventory_pinned_c_subset(&TreeSitterGrammarPin::c_fixture_dir())
}

fn inventory_from_full_grammar(
    full: &GrammarIr,
    pin: &TreeSitterGrammarPin,
) -> Result<TreeSitterCSubsetPinInventory, TreeSitterCSubsetError> {
    let mut subset_pin_rules_present = Vec::with_capacity(SUBSET_PIN_RULE_NAMES.len());
    for name in SUBSET_PIN_RULE_NAMES {
        let rule = full.rule(name).ok_or_else(|| {
            TreeSitterCSubsetError::new(
                "inventory",
                format!("pinned grammar is missing required subset rule {name}"),
            )
        })?;
        subset_pin_rules_present.push(rule.name.clone());
    }

    require_children_includes(full, TRANSLATION_UNIT_RULE, FUNCTION_DEFINITION_RULE)?;
    require_named_field(full, FUNCTION_DEFINITION_RULE, TYPE_FIELD)?;
    require_named_field(full, FUNCTION_DEFINITION_RULE, DECLARATOR_FIELD)?;
    require_named_field(full, FUNCTION_DEFINITION_RULE, BODY_FIELD)?;
    require_subtype_includes(full, TYPE_SPECIFIER_RULE, PRIMITIVE_TYPE_RULE)?;
    require_subtype_includes(full, DECLARATOR_RULE, FUNCTION_DECLARATOR_RULE)?;
    require_subtype_includes(full, DECLARATOR_RULE, IDENTIFIER_RULE)?;
    require_named_field(full, FUNCTION_DECLARATOR_RULE, DECLARATOR_FIELD)?;
    require_named_field(full, FUNCTION_DECLARATOR_RULE, PARAMETERS_FIELD)?;
    require_children_includes(full, PARAMETER_LIST_RULE, PARAMETER_DECLARATION_RULE)?;
    require_named_field(full, PARAMETER_DECLARATION_RULE, TYPE_FIELD)?;
    require_children_includes(full, COMPOUND_STATEMENT_RULE, STATEMENT_RULE)?;
    require_subtype_includes(full, STATEMENT_RULE, RETURN_STATEMENT_RULE)?;
    require_children_includes(full, RETURN_STATEMENT_RULE, EXPRESSION_RULE)?;
    require_subtype_includes(full, EXPRESSION_RULE, NUMBER_LITERAL_RULE)?;

    let mixed_payload_risks_observed = observe_mixed_payload_risks(full, MIXED_PAYLOAD_RISKS)?;

    require_empty_fields(full, IDENTIFIER_RULE)?;
    require_empty_fields(full, NUMBER_LITERAL_RULE)?;
    require_empty_fields(full, PRIMITIVE_TYPE_RULE)?;
    let identifier_on_declarator = subtype_terminal(full, DECLARATOR_RULE, IDENTIFIER_RULE)?;
    if identifier_on_declarator != GrammarTerminalKind::IdentifierPayload {
        return Err(TreeSitterCSubsetError::new(
            "inventory",
            format!(
                "pin _declarator.subtype/identifier must be IdentifierPayload, got {identifier_on_declarator:?}"
            ),
        ));
    }
    let number_on_expression = subtype_terminal(full, EXPRESSION_RULE, NUMBER_LITERAL_RULE)?;
    if number_on_expression != GrammarTerminalKind::IntegerPayload {
        return Err(TreeSitterCSubsetError::new(
            "inventory",
            format!(
                "pin expression.subtype/number_literal must be IntegerPayload, got {number_on_expression:?}"
            ),
        ));
    }

    Ok(TreeSitterCSubsetPinInventory {
        pin_id: pin.pin_id.clone(),
        grammar_package: pin.grammar_package.clone(),
        grammar_version: pin.grammar_version.clone(),
        node_types_sha256: pin.node_types_sha256.clone(),
        full_rule_count: full.rules.len(),
        subset_pin_rules_present,
        mixed_payload_risks_observed,
        claim_boundary: CLAIM_BOUNDARY,
        subset_id: SUBSET_ID,
    })
}

fn project_minimal_function_subset(full: &GrammarIr) -> Result<GrammarIr, TreeSitterCSubsetError> {
    for required in SUBSET_PIN_RULE_NAMES {
        if full.rule(required).is_none() {
            return Err(TreeSitterCSubsetError::new(
                "subset",
                format!("pinned grammar is missing required rule {required}"),
            ));
        }
    }

    // Confirm the full-pin shapes we project from.
    require_children_includes(full, TRANSLATION_UNIT_RULE, FUNCTION_DEFINITION_RULE)?;
    require_named_field(full, FUNCTION_DEFINITION_RULE, BODY_FIELD)?;
    require_named_field(full, FUNCTION_DEFINITION_RULE, DECLARATOR_FIELD)?;
    require_named_field(full, FUNCTION_DEFINITION_RULE, TYPE_FIELD)?;
    require_subtype_includes(full, TYPE_SPECIFIER_RULE, PRIMITIVE_TYPE_RULE)?;
    require_subtype_includes(full, DECLARATOR_RULE, FUNCTION_DECLARATOR_RULE)?;
    require_subtype_includes(full, DECLARATOR_RULE, IDENTIFIER_RULE)?;
    require_named_field(full, FUNCTION_DECLARATOR_RULE, DECLARATOR_FIELD)?;
    require_named_field(full, FUNCTION_DECLARATOR_RULE, PARAMETERS_FIELD)?;
    require_children_includes(full, PARAMETER_LIST_RULE, PARAMETER_DECLARATION_RULE)?;
    require_named_field(full, PARAMETER_DECLARATION_RULE, TYPE_FIELD)?;
    require_children_includes(full, COMPOUND_STATEMENT_RULE, STATEMENT_RULE)?;
    require_subtype_includes(full, STATEMENT_RULE, RETURN_STATEMENT_RULE)?;
    require_children_includes(full, RETURN_STATEMENT_RULE, EXPRESSION_RULE)?;
    require_subtype_includes(full, EXPRESSION_RULE, NUMBER_LITERAL_RULE)?;

    let pin_identifier_terminal = subtype_terminal(full, DECLARATOR_RULE, IDENTIFIER_RULE)?;
    if pin_identifier_terminal != GrammarTerminalKind::IdentifierPayload {
        return Err(TreeSitterCSubsetError::new(
            "subset",
            format!("unexpected identifier terminal on pin: {pin_identifier_terminal:?}"),
        ));
    }
    let pin_number_terminal = subtype_terminal(full, EXPRESSION_RULE, NUMBER_LITERAL_RULE)?;
    if pin_number_terminal != GrammarTerminalKind::IntegerPayload {
        return Err(TreeSitterCSubsetError::new(
            "subset",
            format!("unexpected number_literal terminal on pin: {pin_number_terminal:?}"),
        ));
    }

    let mut provenance = full.provenance.clone();
    provenance.source_kind = "tree_sitter_node_types_subset".to_string();
    // Root record is records.first() → translation_unit must stay first.
    let mut grammar = GrammarIr::new(SUBSET_GRAMMAR_ID, provenance);

    grammar.rules.push(GrammarRule {
        name: TRANSLATION_UNIT_RULE.to_string(),
        node_kind: TRANSLATION_UNIT_RULE.to_string(),
        fields: vec![GrammarField {
            name: CHILDREN_FIELD_NAME.to_string(),
            required: false,
            multiple: true,
            alternatives: vec![GrammarFieldAlternative {
                node_kind: FUNCTION_DEFINITION_RULE.to_string(),
                terminal: GrammarTerminalKind::Nonterminal,
            }],
        }],
    });

    // Remaining rules sorted alphabetically for stable review (root already first).
    let mut rest = vec![
        // _declarator: function_declarator | identifier (both Nonterminal wrappers)
        GrammarRule {
            name: DECLARATOR_RULE.to_string(),
            node_kind: DECLARATOR_RULE.to_string(),
            fields: vec![GrammarField {
                name: SUBTYPE_FIELD_NAME.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![
                    GrammarFieldAlternative {
                        node_kind: FUNCTION_DECLARATOR_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: IDENTIFIER_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                ],
            }],
        },
        GrammarRule {
            name: COMPOUND_STATEMENT_RULE.to_string(),
            node_kind: COMPOUND_STATEMENT_RULE.to_string(),
            fields: vec![GrammarField {
                name: CHILDREN_FIELD_NAME.to_string(),
                required: false,
                multiple: true,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: STATEMENT_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        },
        GrammarRule {
            name: EXPRESSION_RULE.to_string(),
            node_kind: EXPRESSION_RULE.to_string(),
            fields: vec![GrammarField {
                name: SUBTYPE_FIELD_NAME.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: NUMBER_LITERAL_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        },
        // Pin field order body, declarator, type (BTreeMap alphabetical).
        GrammarRule {
            name: FUNCTION_DEFINITION_RULE.to_string(),
            node_kind: FUNCTION_DEFINITION_RULE.to_string(),
            fields: vec![
                GrammarField {
                    name: BODY_FIELD.to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: COMPOUND_STATEMENT_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
                GrammarField {
                    name: DECLARATOR_FIELD.to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: DECLARATOR_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
                GrammarField {
                    name: TYPE_FIELD.to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: TYPE_SPECIFIER_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
            ],
        },
        GrammarRule {
            name: FUNCTION_DECLARATOR_RULE.to_string(),
            node_kind: FUNCTION_DECLARATOR_RULE.to_string(),
            fields: vec![
                GrammarField {
                    name: DECLARATOR_FIELD.to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: DECLARATOR_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
                GrammarField {
                    name: PARAMETERS_FIELD.to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: PARAMETER_LIST_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
            ],
        },
        // Identifier wrapper: pin leaf becomes Nonterminal with name payload.
        GrammarRule {
            name: IDENTIFIER_RULE.to_string(),
            node_kind: IDENTIFIER_RULE.to_string(),
            fields: vec![GrammarField {
                name: NAME_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: IDENTIFIER_RULE.to_string(),
                    terminal: GrammarTerminalKind::IdentifierPayload,
                }],
            }],
        },
        // number_literal wrapper: IntegerPayload under synthetic value field.
        GrammarRule {
            name: NUMBER_LITERAL_RULE.to_string(),
            node_kind: NUMBER_LITERAL_RULE.to_string(),
            fields: vec![GrammarField {
                name: NUMBER_VALUE_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: NUMBER_LITERAL_RULE.to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                }],
            }],
        },
        GrammarRule {
            name: PARAMETER_DECLARATION_RULE.to_string(),
            node_kind: PARAMETER_DECLARATION_RULE.to_string(),
            fields: vec![GrammarField {
                name: TYPE_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: TYPE_SPECIFIER_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        },
        GrammarRule {
            name: PARAMETER_LIST_RULE.to_string(),
            node_kind: PARAMETER_LIST_RULE.to_string(),
            fields: vec![GrammarField {
                name: CHILDREN_FIELD_NAME.to_string(),
                required: false,
                multiple: true,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: PARAMETER_DECLARATION_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        },
        // primitive_type wrapper: text payload int|void.
        GrammarRule {
            name: PRIMITIVE_TYPE_RULE.to_string(),
            node_kind: PRIMITIVE_TYPE_RULE.to_string(),
            fields: vec![GrammarField {
                name: NAME_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: PRIMITIVE_TYPE_RULE.to_string(),
                    terminal: GrammarTerminalKind::IdentifierPayload,
                }],
            }],
        },
        GrammarRule {
            name: RETURN_STATEMENT_RULE.to_string(),
            node_kind: RETURN_STATEMENT_RULE.to_string(),
            fields: vec![GrammarField {
                name: CHILDREN_FIELD_NAME.to_string(),
                required: false,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: EXPRESSION_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        },
        GrammarRule {
            name: STATEMENT_RULE.to_string(),
            node_kind: STATEMENT_RULE.to_string(),
            fields: vec![GrammarField {
                name: SUBTYPE_FIELD_NAME.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: RETURN_STATEMENT_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        },
        GrammarRule {
            name: TYPE_SPECIFIER_RULE.to_string(),
            node_kind: TYPE_SPECIFIER_RULE.to_string(),
            fields: vec![GrammarField {
                name: SUBTYPE_FIELD_NAME.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: PRIMITIVE_TYPE_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        },
    ];
    rest.sort_by(|left, right| left.name.cmp(&right.name));
    grammar.rules.extend(rest);
    Ok(grammar)
}

/// Lower a subset program AST into a model-conforming decoded tree.
///
/// Mirror of `program_from_decoded_root`: encode builds this tree then hands
/// it to the generic raw-SM encoder. Field order here follows the pin/model
/// order (body, declarator, type on `function_definition`); the generic
/// encoder walks model field order regardless of tree order.
///
/// Public so opt-in layout modes (I5 `SharedAlphabet`) can compile the C
/// subset sidecar under shared roles and run program → decoded-node →
/// encode/decode inverses without using the default PerGrammar codec path or
/// editing absolute golden pins.
pub fn program_to_decoded_node(
    program: &CSubsetProgram,
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    let children_field = if program.functions.is_empty() {
        repeated_empty_field(CHILDREN_FIELD_NAME)
    } else {
        field_with_values(
            CHILDREN_FIELD_NAME,
            program
                .functions
                .iter()
                .map(|function| {
                    Ok(RawSyntaxDecodedValue::Node(Box::new(
                        function_to_decoded_node(function)?,
                    )))
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    };
    Ok(decoded_node(TRANSLATION_UNIT_RULE, vec![children_field]))
}

fn function_to_decoded_node(
    function: &CSubsetFunction,
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    // Pin BTreeMap field order: body, declarator, type.
    Ok(decoded_node(
        FUNCTION_DEFINITION_RULE,
        vec![
            single_child_field(
                BODY_FIELD,
                compound_statement_to_decoded_node(&function.returns)?,
            ),
            single_child_field(
                DECLARATOR_FIELD,
                function_declarator_root_to_decoded_node(&function.name, &function.parameters)?,
            ),
            single_child_field(
                TYPE_FIELD,
                type_specifier_to_decoded_node(function.return_type)?,
            ),
        ],
    ))
}

fn compound_statement_to_decoded_node(
    returns: &[CSubsetReturn],
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    let children_field = if returns.is_empty() {
        repeated_empty_field(CHILDREN_FIELD_NAME)
    } else {
        field_with_values(
            CHILDREN_FIELD_NAME,
            returns
                .iter()
                .map(|ret| {
                    Ok(RawSyntaxDecodedValue::Node(Box::new(
                        statement_return_to_decoded_node(ret.value)?,
                    )))
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    };
    Ok(decoded_node(COMPOUND_STATEMENT_RULE, vec![children_field]))
}

fn statement_return_to_decoded_node(
    value: u8,
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    let expression = decoded_node(
        EXPRESSION_RULE,
        vec![single_child_field(
            SUBTYPE_FIELD_NAME,
            number_literal_to_decoded_node(value)?,
        )],
    );
    let return_statement = decoded_node(
        RETURN_STATEMENT_RULE,
        vec![single_child_field(CHILDREN_FIELD_NAME, expression)],
    );
    Ok(decoded_node(
        STATEMENT_RULE,
        vec![single_child_field(SUBTYPE_FIELD_NAME, return_statement)],
    ))
}

fn number_literal_to_decoded_node(
    value: u8,
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    Ok(decoded_node(
        NUMBER_LITERAL_RULE,
        vec![field_with_values(
            NUMBER_VALUE_FIELD,
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: NUMBER_LITERAL_RULE.to_string(),
                terminal: "integer".to_string(),
                payload: value.to_string(),
            })],
        )],
    ))
}

fn function_declarator_root_to_decoded_node(
    name: &str,
    parameters: &[CSubsetParameter],
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    // function_definition.declarator → _declarator → function_declarator
    let function_declarator = decoded_node(
        FUNCTION_DECLARATOR_RULE,
        vec![
            single_child_field(
                DECLARATOR_FIELD,
                identifier_declarator_to_decoded_node(name)?,
            ),
            single_child_field(
                PARAMETERS_FIELD,
                parameter_list_to_decoded_node(parameters)?,
            ),
        ],
    );
    Ok(decoded_node(
        DECLARATOR_RULE,
        vec![single_child_field(SUBTYPE_FIELD_NAME, function_declarator)],
    ))
}

fn identifier_declarator_to_decoded_node(
    name: &str,
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    // function_declarator.declarator → _declarator → identifier wrapper
    let identifier = identifier_wrapper_to_decoded_node(name)?;
    Ok(decoded_node(
        DECLARATOR_RULE,
        vec![single_child_field(SUBTYPE_FIELD_NAME, identifier)],
    ))
}

fn identifier_wrapper_to_decoded_node(
    name: &str,
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    if name.is_empty() {
        return Err(TreeSitterCSubsetError::new(
            "encode",
            "identifier payload must be non-empty",
        ));
    }
    if !name.is_ascii() {
        return Err(TreeSitterCSubsetError::new(
            "encode",
            "identifier payload must be ASCII",
        ));
    }
    Ok(decoded_node(
        IDENTIFIER_RULE,
        vec![field_with_values(
            NAME_FIELD,
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: IDENTIFIER_RULE.to_string(),
                terminal: "identifier".to_string(),
                payload: name.to_string(),
            })],
        )],
    ))
}

fn parameter_list_to_decoded_node(
    parameters: &[CSubsetParameter],
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    let children_field = if parameters.is_empty() {
        repeated_empty_field(CHILDREN_FIELD_NAME)
    } else {
        field_with_values(
            CHILDREN_FIELD_NAME,
            parameters
                .iter()
                .map(|parameter| {
                    Ok(RawSyntaxDecodedValue::Node(Box::new(
                        parameter_declaration_to_decoded_node(parameter)?,
                    )))
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    };
    Ok(decoded_node(PARAMETER_LIST_RULE, vec![children_field]))
}

fn parameter_declaration_to_decoded_node(
    parameter: &CSubsetParameter,
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    Ok(decoded_node(
        PARAMETER_DECLARATION_RULE,
        vec![single_child_field(
            TYPE_FIELD,
            type_specifier_to_decoded_node(parameter.ty)?,
        )],
    ))
}

fn type_specifier_to_decoded_node(
    ty: CSubsetPrimitiveType,
) -> Result<RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    let text = ty.as_str();
    // primitive_type is a Nonterminal wrapper with an IdentifierPayload name.
    let primitive = decoded_node(
        PRIMITIVE_TYPE_RULE,
        vec![field_with_values(
            NAME_FIELD,
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: PRIMITIVE_TYPE_RULE.to_string(),
                terminal: "identifier".to_string(),
                payload: text.to_string(),
            })],
        )],
    );
    Ok(decoded_node(
        TYPE_SPECIFIER_RULE,
        vec![single_child_field(SUBTYPE_FIELD_NAME, primitive)],
    ))
}

fn decoded_node(rule: &str, fields: Vec<RawSyntaxDecodedField>) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: rule.to_string(),
        node_kind: rule.to_string(),
        fields,
    }
}

fn field_with_values(name: &str, values: Vec<RawSyntaxDecodedValue>) -> RawSyntaxDecodedField {
    RawSyntaxDecodedField {
        name: name.to_string(),
        values,
        missing: false,
        repeated_empty: false,
    }
}

fn repeated_empty_field(name: &str) -> RawSyntaxDecodedField {
    RawSyntaxDecodedField {
        name: name.to_string(),
        values: Vec::new(),
        missing: false,
        repeated_empty: true,
    }
}

fn single_child_field(name: &str, child: RawSyntaxDecodedNode) -> RawSyntaxDecodedField {
    field_with_values(name, vec![RawSyntaxDecodedValue::Node(Box::new(child))])
}

fn program_from_decoded_root(
    node: &RawSyntaxDecodedNode,
) -> Result<CSubsetProgram, TreeSitterCSubsetError> {
    if node.rule_name != TRANSLATION_UNIT_RULE && node.node_kind != TRANSLATION_UNIT_RULE {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!(
                "expected rule {TRANSLATION_UNIT_RULE}, got {} / {}",
                node.rule_name, node.node_kind
            ),
        ));
    }
    let children = node.field(CHILDREN_FIELD_NAME).ok_or_else(|| {
        TreeSitterCSubsetError::new("decode_tree", "translation_unit missing children field")
    })?;
    if children.missing || children.repeated_empty || children.values.is_empty() {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            "translation_unit must contain at least one function_definition",
        ));
    }
    let mut functions = Vec::with_capacity(children.values.len());
    for value in &children.values {
        match value {
            RawSyntaxDecodedValue::Node(child) => {
                functions.push(function_from_decoded_node(child)?);
            }
            other => {
                return Err(TreeSitterCSubsetError::new(
                    "decode_tree",
                    format!("unexpected translation_unit child: {other:?}"),
                ));
            }
        }
    }
    Ok(CSubsetProgram { functions })
}

fn function_from_decoded_node(
    node: &RawSyntaxDecodedNode,
) -> Result<CSubsetFunction, TreeSitterCSubsetError> {
    if node.rule_name != FUNCTION_DEFINITION_RULE && node.node_kind != FUNCTION_DEFINITION_RULE {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!(
                "expected {FUNCTION_DEFINITION_RULE}, got {} / {}",
                node.rule_name, node.node_kind
            ),
        ));
    }
    let body = single_child_node(node, BODY_FIELD, COMPOUND_STATEMENT_RULE)?;
    let returns = returns_from_compound(body)?;
    let declarator_root = single_child_node(node, DECLARATOR_FIELD, DECLARATOR_RULE)?;
    let (name, parameters) = function_declarator_from_declarator_root(declarator_root)?;
    let type_node = single_child_node(node, TYPE_FIELD, TYPE_SPECIFIER_RULE)?;
    let return_type = primitive_from_type_specifier(type_node)?;
    Ok(CSubsetFunction {
        return_type,
        name,
        parameters,
        returns,
    })
}

fn returns_from_compound(
    node: &RawSyntaxDecodedNode,
) -> Result<Vec<CSubsetReturn>, TreeSitterCSubsetError> {
    let children = node.field(CHILDREN_FIELD_NAME).ok_or_else(|| {
        TreeSitterCSubsetError::new("decode_tree", "compound_statement missing children")
    })?;
    if children.missing || children.repeated_empty || children.values.is_empty() {
        return Ok(Vec::new());
    }
    let mut returns = Vec::with_capacity(children.values.len());
    for value in &children.values {
        match value {
            RawSyntaxDecodedValue::Node(statement) => {
                returns.push(return_from_statement(statement)?);
            }
            other => {
                return Err(TreeSitterCSubsetError::new(
                    "decode_tree",
                    format!("unexpected compound_statement child: {other:?}"),
                ));
            }
        }
    }
    Ok(returns)
}

fn return_from_statement(
    node: &RawSyntaxDecodedNode,
) -> Result<CSubsetReturn, TreeSitterCSubsetError> {
    if node.rule_name != STATEMENT_RULE && node.node_kind != STATEMENT_RULE {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!(
                "expected {STATEMENT_RULE}, got {} / {}",
                node.rule_name, node.node_kind
            ),
        ));
    }
    let ret = single_subtype_node(node, RETURN_STATEMENT_RULE)?;
    let children = ret.field(CHILDREN_FIELD_NAME).ok_or_else(|| {
        TreeSitterCSubsetError::new("decode_tree", "return_statement missing children")
    })?;
    if children.missing || children.values.is_empty() {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            "subset return_statement requires an expression",
        ));
    }
    if children.values.len() != 1 {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!(
                "return_statement must carry exactly one expression, got {}",
                children.values.len()
            ),
        ));
    }
    let expression = match &children.values[0] {
        RawSyntaxDecodedValue::Node(expression) => expression,
        other => {
            return Err(TreeSitterCSubsetError::new(
                "decode_tree",
                format!("unexpected return child: {other:?}"),
            ));
        }
    };
    let number = single_subtype_node(expression, NUMBER_LITERAL_RULE)?;
    let value = u8_from_number_literal(number)?;
    Ok(CSubsetReturn { value })
}

fn function_declarator_from_declarator_root(
    node: &RawSyntaxDecodedNode,
) -> Result<(String, Vec<CSubsetParameter>), TreeSitterCSubsetError> {
    let function_declarator = single_subtype_node(node, FUNCTION_DECLARATOR_RULE)?;
    let name_declarator =
        single_child_node(function_declarator, DECLARATOR_FIELD, DECLARATOR_RULE)?;
    let identifier = single_subtype_node(name_declarator, IDENTIFIER_RULE)?;
    let name = text_payload_from_name_field(identifier, IDENTIFIER_RULE)?;
    let parameter_list =
        single_child_node(function_declarator, PARAMETERS_FIELD, PARAMETER_LIST_RULE)?;
    let parameters = parameters_from_list(parameter_list)?;
    Ok((name, parameters))
}

fn parameters_from_list(
    node: &RawSyntaxDecodedNode,
) -> Result<Vec<CSubsetParameter>, TreeSitterCSubsetError> {
    let children = node.field(CHILDREN_FIELD_NAME).ok_or_else(|| {
        TreeSitterCSubsetError::new("decode_tree", "parameter_list missing children")
    })?;
    if children.missing || children.repeated_empty || children.values.is_empty() {
        return Ok(Vec::new());
    }
    let mut parameters = Vec::with_capacity(children.values.len());
    for value in &children.values {
        match value {
            RawSyntaxDecodedValue::Node(param) => {
                if param.rule_name != PARAMETER_DECLARATION_RULE
                    && param.node_kind != PARAMETER_DECLARATION_RULE
                {
                    return Err(TreeSitterCSubsetError::new(
                        "decode_tree",
                        format!(
                            "expected {PARAMETER_DECLARATION_RULE}, got {} / {}",
                            param.rule_name, param.node_kind
                        ),
                    ));
                }
                let type_node = single_child_node(param, TYPE_FIELD, TYPE_SPECIFIER_RULE)?;
                let ty = primitive_from_type_specifier(type_node)?;
                parameters.push(CSubsetParameter { ty });
            }
            other => {
                return Err(TreeSitterCSubsetError::new(
                    "decode_tree",
                    format!("unexpected parameter_list child: {other:?}"),
                ));
            }
        }
    }
    Ok(parameters)
}

fn primitive_from_type_specifier(
    node: &RawSyntaxDecodedNode,
) -> Result<CSubsetPrimitiveType, TreeSitterCSubsetError> {
    let primitive = single_subtype_node(node, PRIMITIVE_TYPE_RULE)?;
    let text = text_payload_from_name_field(primitive, PRIMITIVE_TYPE_RULE)?;
    CSubsetPrimitiveType::parse(&text).ok_or_else(|| {
        TreeSitterCSubsetError::new(
            "decode_tree",
            format!("primitive_type text {text:?} is outside the subset"),
        )
    })
}

fn u8_from_number_literal(node: &RawSyntaxDecodedNode) -> Result<u8, TreeSitterCSubsetError> {
    let value_field = node.field(NUMBER_VALUE_FIELD).ok_or_else(|| {
        TreeSitterCSubsetError::new("decode_tree", "number_literal missing value field")
    })?;
    if value_field.values.len() != 1 {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!(
                "number_literal value must carry exactly one payload, got {}",
                value_field.values.len()
            ),
        ));
    }
    // Strict: accepted number_literal terminals from the subset SM are exactly
    // (node_kind="number_literal", terminal="integer") — verified on encode→decode
    // trees for `return <u8>`. Do not use substring payload-kind matching.
    match &value_field.values[0] {
        RawSyntaxDecodedValue::Terminal(terminal)
            if terminal.node_kind == NUMBER_LITERAL_RULE && terminal.terminal == "integer" =>
        {
            terminal.payload.parse::<u8>().map_err(|error| {
                TreeSitterCSubsetError::new(
                    "decode_tree",
                    format!(
                        "number payload {:?} is not a u8 integer: {error}",
                        terminal.payload
                    ),
                )
            })
        }
        other => Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!("unexpected number_literal value: {other:?}"),
        )),
    }
}

fn text_payload_from_name_field(
    node: &RawSyntaxDecodedNode,
    expected_kind: &str,
) -> Result<String, TreeSitterCSubsetError> {
    let name_field = node.field(NAME_FIELD).ok_or_else(|| {
        TreeSitterCSubsetError::new("decode_tree", format!("{expected_kind} missing name field"))
    })?;
    if name_field.values.len() != 1 {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!(
                "{expected_kind}.name must carry exactly one payload, got {}",
                name_field.values.len()
            ),
        ));
    }
    // Call sites: identifier.name and primitive_type.name. Both emit Terminal
    // payloads whose node_kind equals the owner rule (`identifier` /
    // `primitive_type`); only node_kind is checked (no rule_name on terminals).
    match &name_field.values[0] {
        RawSyntaxDecodedValue::Terminal(terminal) if terminal.node_kind == expected_kind => {
            Ok(terminal.payload.clone())
        }
        other => Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!("unexpected {expected_kind} name payload: {other:?}"),
        )),
    }
}

fn single_child_node<'a>(
    node: &'a RawSyntaxDecodedNode,
    field: &str,
    expected_kind: &str,
) -> Result<&'a RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    let field_value = node.field(field).ok_or_else(|| {
        TreeSitterCSubsetError::new(
            "decode_tree",
            format!("{} missing field {field}", node.rule_name),
        )
    })?;
    if field_value.values.len() != 1 {
        return Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!(
                "{}.{} must carry exactly one value, got {}",
                node.rule_name,
                field,
                field_value.values.len()
            ),
        ));
    }
    // Subset GrammarIR sets `rule.name == rule.node_kind` for every retained
    // rule (structural model record_type/grammar_rule stay equal). Round-trip
    // trees for the v0 fixtures never diverge the two fields, so matching
    // node_kind alone is sufficient for every current call site (body,
    // declarator, type, subtype, parameters, children). Prefer node_kind to
    // stay aligned with terminal.node_kind checks.
    match &field_value.values[0] {
        RawSyntaxDecodedValue::Node(child) if child.node_kind == expected_kind => Ok(child),
        other => Err(TreeSitterCSubsetError::new(
            "decode_tree",
            format!(
                "{}.{} expected {expected_kind} node, got {other:?}",
                node.rule_name, field
            ),
        )),
    }
}

fn single_subtype_node<'a>(
    node: &'a RawSyntaxDecodedNode,
    expected_kind: &str,
) -> Result<&'a RawSyntaxDecodedNode, TreeSitterCSubsetError> {
    single_child_node(node, SUBTYPE_FIELD_NAME, expected_kind)
}

fn require_empty_fields(full: &GrammarIr, rule: &str) -> Result<(), TreeSitterCSubsetError> {
    let rule_ir = full
        .rule(rule)
        .ok_or_else(|| TreeSitterCSubsetError::new("inventory", format!("missing rule {rule}")))?;
    if !rule_ir.fields.is_empty() {
        return Err(TreeSitterCSubsetError::new(
            "inventory",
            format!("expected pin leaf {rule} to have no fields"),
        ));
    }
    Ok(())
}

fn require_named_field(
    full: &GrammarIr,
    rule: &str,
    field: &str,
) -> Result<(), TreeSitterCSubsetError> {
    let rule_ir = full
        .rule(rule)
        .ok_or_else(|| TreeSitterCSubsetError::new("inventory", format!("missing rule {rule}")))?;
    if !rule_ir
        .fields
        .iter()
        .any(|candidate| candidate.name == field)
    {
        return Err(TreeSitterCSubsetError::new(
            "inventory",
            format!("rule {rule} is missing field {field}"),
        ));
    }
    Ok(())
}

fn require_children_includes(
    full: &GrammarIr,
    rule: &str,
    child_kind: &str,
) -> Result<(), TreeSitterCSubsetError> {
    let rule_ir = full
        .rule(rule)
        .ok_or_else(|| TreeSitterCSubsetError::new("inventory", format!("missing rule {rule}")))?;
    let children = rule_ir
        .fields
        .iter()
        .find(|field| field.name == CHILDREN_FIELD_NAME)
        .ok_or_else(|| {
            TreeSitterCSubsetError::new(
                "inventory",
                format!("rule {rule} is missing synthetic children field"),
            )
        })?;
    if !children
        .alternatives
        .iter()
        .any(|alternative| alternative.node_kind == child_kind)
    {
        return Err(TreeSitterCSubsetError::new(
            "inventory",
            format!("rule {rule}.children does not include {child_kind}"),
        ));
    }
    Ok(())
}

fn require_subtype_includes(
    full: &GrammarIr,
    rule: &str,
    subtype_kind: &str,
) -> Result<(), TreeSitterCSubsetError> {
    let rule_ir = full
        .rule(rule)
        .ok_or_else(|| TreeSitterCSubsetError::new("inventory", format!("missing rule {rule}")))?;
    let subtype = rule_ir
        .fields
        .iter()
        .find(|field| field.name == SUBTYPE_FIELD_NAME)
        .ok_or_else(|| {
            TreeSitterCSubsetError::new(
                "inventory",
                format!("rule {rule} is missing synthetic subtype field"),
            )
        })?;
    if !subtype
        .alternatives
        .iter()
        .any(|alternative| alternative.node_kind == subtype_kind)
    {
        return Err(TreeSitterCSubsetError::new(
            "inventory",
            format!("rule {rule}.subtype does not include {subtype_kind}"),
        ));
    }
    Ok(())
}

fn subtype_terminal(
    full: &GrammarIr,
    rule: &str,
    subtype_kind: &str,
) -> Result<GrammarTerminalKind, TreeSitterCSubsetError> {
    let rule_ir = full
        .rule(rule)
        .ok_or_else(|| TreeSitterCSubsetError::new("inventory", format!("missing rule {rule}")))?;
    let subtype = rule_ir
        .fields
        .iter()
        .find(|field| field.name == SUBTYPE_FIELD_NAME)
        .ok_or_else(|| {
            TreeSitterCSubsetError::new(
                "inventory",
                format!("rule {rule} is missing synthetic subtype field"),
            )
        })?;
    subtype
        .alternatives
        .iter()
        .find(|alternative| alternative.node_kind == subtype_kind)
        .map(|alternative| alternative.terminal)
        .ok_or_else(|| {
            TreeSitterCSubsetError::new(
                "inventory",
                format!("rule {rule}.subtype missing {subtype_kind}"),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subset_constants_match_claim_boundary() {
        assert_eq!(SUBSET_ID, SUBSET_GRAMMAR_ID);
        assert_eq!(CLAIM_BOUNDARY, "syntax_level_tree_sitter_c_subset_only");
        assert_eq!(SUBSET_ID, "tree_sitter_c_minimal_function_subset_v0");
        assert!(SUBSET_PIN_RULE_NAMES.contains(&"function_definition"));
        assert!(SUBSET_PIN_RULE_NAMES.contains(&"number_literal"));
        assert_eq!(MIXED_PAYLOAD_RISKS.len(), 2);
    }

    #[test]
    fn inventory_accepts_checked_in_c_pin() {
        let inventory = inventory_pinned_c_subset_fixture().expect("C pin inventory");
        assert_eq!(inventory.pin_id, "tree_sitter_c_node_types_v0.24.2");
        assert_eq!(inventory.grammar_package, "tree-sitter-c");
        assert_eq!(inventory.grammar_version, "0.24.2");
        assert_eq!(
            inventory.node_types_sha256,
            "23e819ef1eefd357bb6eba844f47f8492f6a88da3e5046b06ec4acd4da8a9fd6"
        );
        assert!(inventory.full_rule_count > 100);
        assert_eq!(
            inventory.subset_pin_rules_present.len(),
            SUBSET_PIN_RULE_NAMES.len()
        );
        assert_eq!(inventory.mixed_payload_risks_observed.len(), 2);
        assert_eq!(inventory.claim_boundary, CLAIM_BOUNDARY);
        assert_eq!(inventory.subset_id, SUBSET_ID);
    }

    #[test]
    fn subset_parse_render_main_return_zero() {
        let source = "int main(void) {\n  return 0;\n}";
        let program = CSubsetProgram::parse(source).expect("parse");
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
        assert_eq!(program.functions[0].return_type, CSubsetPrimitiveType::Int);
        assert_eq!(
            program.functions[0].parameters,
            vec![CSubsetParameter {
                ty: CSubsetPrimitiveType::Void
            }]
        );
        assert_eq!(
            program.functions[0].returns,
            vec![CSubsetReturn { value: 0 }]
        );
        assert_eq!(program.render(), source);
    }

    #[test]
    fn number_literal_decode_rejects_wrong_payload_kind() {
        // Strict decode_tree path: payload kind must be exactly "integer".
        // Wrong kinds (including former substring matches like "not_integer")
        // must not be accepted. RawSyntaxDecoded* are public but there is no
        // public entry that takes a hand-built tree, so this targets the
        // private tree helper directly.
        let wrong_kind = RawSyntaxDecodedNode {
            rule_name: NUMBER_LITERAL_RULE.to_string(),
            node_kind: NUMBER_LITERAL_RULE.to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: NUMBER_VALUE_FIELD.to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: NUMBER_LITERAL_RULE.to_string(),
                    terminal: "not_integer".to_string(),
                    payload: "42".to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        let err = u8_from_number_literal(&wrong_kind).expect_err("wrong payload kind");
        assert_eq!(err.phase, "decode_tree");
        assert!(
            err.message.contains("unexpected number_literal value"),
            "unexpected error message: {}",
            err.message
        );

        // node_kind mismatch with correct payload kind must also fail.
        let wrong_node_kind = RawSyntaxDecodedNode {
            rule_name: NUMBER_LITERAL_RULE.to_string(),
            node_kind: NUMBER_LITERAL_RULE.to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: NUMBER_VALUE_FIELD.to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "expression".to_string(),
                    terminal: "integer".to_string(),
                    payload: "7".to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        let err = u8_from_number_literal(&wrong_node_kind).expect_err("wrong node_kind");
        assert_eq!(err.phase, "decode_tree");

        // Exact accepted pair still parses.
        let ok = RawSyntaxDecodedNode {
            rule_name: NUMBER_LITERAL_RULE.to_string(),
            node_kind: NUMBER_LITERAL_RULE.to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: NUMBER_VALUE_FIELD.to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: NUMBER_LITERAL_RULE.to_string(),
                    terminal: "integer".to_string(),
                    payload: "42".to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        assert_eq!(u8_from_number_literal(&ok).expect("exact pair"), 42);
    }
}
