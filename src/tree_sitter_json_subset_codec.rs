//! Tree-sitter JSON text fixed-subset codec smoke (Workstream B1+).
//!
//! Builds on the B0 pinned `tree-sitter-json` node-types export. This module
//! projects a deliberate construct subset into `GrammarIR` → `StructuralModel`
//! → raw syntax state machine encode/decode, then render/reparse.
//!
//! Claim boundary: syntax-level round-trip evidence only. Not JSON Schema TCT,
//! not a Tree-sitter runtime dependency, and not a Go layout pin.

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
use crate::tree_sitter_metadata::{CHILDREN_FIELD_NAME, SUBTYPE_FIELD_NAME, TreeSitterGrammarPin};
use crate::tree_sitter_subset_framework::{
    CompiledSubsetCodecCore, TreeSitterSubsetError, TreeSitterSubsetRoundTrip, require_rule,
};
use std::path::Path;

/// Phase-tagged JSON subset codec errors (shared framework type).
pub type TreeSitterJsonSubsetError = TreeSitterSubsetError;
/// Claim-boundary round-trip report (shared framework type).
pub type TreeSitterJsonSubsetRoundTrip = TreeSitterSubsetRoundTrip;

/// Stable subset identity for the null/bool/string/number/array/object smoke.
pub const SUBSET_ID: &str = "tree_sitter_json_null_bool_string_number_array_object_subset_v0";
/// Claim boundary string carried in reports and docs.
pub const CLAIM_BOUNDARY: &str = "syntax_level_tree_sitter_json_subset_only";
/// Grammar id used when lowering the subset GrammarIR.
pub const SUBSET_GRAMMAR_ID: &str =
    "tree_sitter_json_null_bool_string_number_array_object_subset_v0";

const VALUE_RULE: &str = "_value";
const NULL_RULE: &str = "null";
const TRUE_RULE: &str = "true";
const FALSE_RULE: &str = "false";
const STRING_RULE: &str = "string";
const STRING_CONTENT_KIND: &str = "string_content";
const NUMBER_RULE: &str = "number";
const ARRAY_RULE: &str = "array";
const OBJECT_RULE: &str = "object";
const PAIR_RULE: &str = "pair";
/// Synthetic field carrying the u8 integer payload under the `number` wrapper.
/// Keeps `_value.subtype` all-Nonterminal (no mixed payload kinds on one field).
const NUMBER_VALUE_FIELD: &str = "value";
/// Pin field name for `pair` key (required `string` Nonterminal).
const PAIR_KEY_FIELD: &str = "key";
/// Pin field name for `pair` value (required `_value` Nonterminal).
const PAIR_VALUE_FIELD: &str = "value";

/// One object member: string key + recursive subset value (`pair` in the pin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonSubsetPair {
    pub key: String,
    pub value: JsonSubsetValue,
}

/// Fixed construct subset retained from the pinned tree-sitter-json export.
///
/// `_value` → `null` | `true` | `false` | `string` | `number` | `array` |
/// `object` (all Nonterminal wrappers). Leaves `true`/`false`/`null` are
/// field-less. `string` → optional `string_content` (StringPayload). `number`
/// → required synthetic `value` field (u8 IntegerPayload, 0..=255). `array`
/// → optional repeated `children` of recursive `_value` (empty uses
/// `repeated_empty`). `object` → optional repeated `children` of `pair`;
/// each `pair` has required `key` (`string`) and `value` (`_value`).
/// Floats, negatives, integers outside 0..=255, string escapes, and
/// whitespace remain outside the subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonSubsetValue {
    Null,
    /// JSON `true` leaf (tree-sitter-json named kind `true`).
    True,
    /// JSON `false` leaf (tree-sitter-json named kind `false`).
    False,
    String(String),
    /// Unsigned integer in `0..=255` (u8 layout range).
    Number(u8),
    /// JSON array of subset values (recursive `_value` children).
    Array(Vec<JsonSubsetValue>),
    /// JSON object of ordered pairs (recursive via pair values).
    Object(Vec<JsonSubsetPair>),
}

impl JsonSubsetValue {
    /// Parse a fixed subset JSON text instance (no interior whitespace).
    pub fn parse(source: &str) -> Result<Self, TreeSitterJsonSubsetError> {
        let trimmed = source.trim();
        let (value, rest) = parse_value(trimmed)?;
        if !rest.is_empty() {
            return Err(TreeSitterJsonSubsetError::new(
                "parse",
                format!("trailing source after subset value: {rest:?}"),
            ));
        }
        Ok(value)
    }

    /// Render the subset value as compact JSON text (no interior whitespace).
    pub fn render(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::True => "true".to_string(),
            Self::False => "false".to_string(),
            Self::String(content) => format!("\"{content}\""),
            Self::Number(value) => value.to_string(),
            Self::Array(elements) => {
                let mut out = String::from("[");
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&element.render());
                }
                out.push(']');
                out
            }
            Self::Object(pairs) => {
                let mut out = String::from("{");
                for (index, pair) in pairs.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push('"');
                    out.push_str(&pair.key);
                    out.push('"');
                    out.push(':');
                    out.push_str(&pair.value.render());
                }
                out.push('}');
                out
            }
        }
    }
}

/// Parse one subset value and return the remainder of the source.
fn parse_value(source: &str) -> Result<(JsonSubsetValue, &str), TreeSitterJsonSubsetError> {
    if let Some(rest) = source.strip_prefix("null") {
        return Ok((JsonSubsetValue::Null, rest));
    }
    if let Some(rest) = source.strip_prefix("true") {
        return Ok((JsonSubsetValue::True, rest));
    }
    if let Some(rest) = source.strip_prefix("false") {
        return Ok((JsonSubsetValue::False, rest));
    }
    if let Some(rest) = source.strip_prefix('[') {
        return parse_array_body(rest);
    }
    if let Some(rest) = source.strip_prefix('{') {
        return parse_object_body(rest);
    }
    if let Some(rest) = source.strip_prefix('"') {
        return parse_string_body(rest);
    }
    parse_number_prefix(source)
}

fn parse_array_body(source: &str) -> Result<(JsonSubsetValue, &str), TreeSitterJsonSubsetError> {
    if let Some(rest) = source.strip_prefix(']') {
        return Ok((JsonSubsetValue::Array(Vec::new()), rest));
    }
    let mut elements = Vec::new();
    let mut rest = source;
    loop {
        let (element, after) = parse_value(rest)?;
        elements.push(element);
        if let Some(after_comma) = after.strip_prefix(',') {
            rest = after_comma;
            continue;
        }
        if let Some(after_close) = after.strip_prefix(']') {
            return Ok((JsonSubsetValue::Array(elements), after_close));
        }
        return Err(TreeSitterJsonSubsetError::new(
            "parse",
            format!("array element list is malformed near {after:?}"),
        ));
    }
}

fn parse_object_body(source: &str) -> Result<(JsonSubsetValue, &str), TreeSitterJsonSubsetError> {
    if let Some(rest) = source.strip_prefix('}') {
        return Ok((JsonSubsetValue::Object(Vec::new()), rest));
    }
    let mut pairs = Vec::new();
    let mut rest = source;
    loop {
        let (pair, after) = parse_pair(rest)?;
        pairs.push(pair);
        if let Some(after_comma) = after.strip_prefix(',') {
            rest = after_comma;
            continue;
        }
        if let Some(after_close) = after.strip_prefix('}') {
            return Ok((JsonSubsetValue::Object(pairs), after_close));
        }
        return Err(TreeSitterJsonSubsetError::new(
            "parse",
            format!("object pair list is malformed near {after:?}"),
        ));
    }
}

fn parse_pair(source: &str) -> Result<(JsonSubsetPair, &str), TreeSitterJsonSubsetError> {
    let rest = source.strip_prefix('"').ok_or_else(|| {
        TreeSitterJsonSubsetError::new(
            "parse",
            format!("object pair key must be a subset string near {source:?}"),
        )
    })?;
    let (key_value, after_key) = parse_string_body(rest)?;
    let JsonSubsetValue::String(key) = key_value else {
        return Err(TreeSitterJsonSubsetError::new(
            "parse",
            "object pair key parse did not yield a string",
        ));
    };
    let after_colon = after_key.strip_prefix(':').ok_or_else(|| {
        TreeSitterJsonSubsetError::new(
            "parse",
            format!("object pair is missing ':' after key near {after_key:?}"),
        )
    })?;
    let (value, after_value) = parse_value(after_colon)?;
    Ok((JsonSubsetPair { key, value }, after_value))
}

fn parse_string_body(source: &str) -> Result<(JsonSubsetValue, &str), TreeSitterJsonSubsetError> {
    let end = source.find('"').ok_or_else(|| {
        TreeSitterJsonSubsetError::new("parse", "string is missing closing quote")
    })?;
    let inner = &source[..end];
    if inner.contains('\\') {
        return Err(TreeSitterJsonSubsetError::new(
            "parse",
            "string escapes are outside the fixed subset",
        ));
    }
    if !inner.is_ascii() {
        return Err(TreeSitterJsonSubsetError::new(
            "parse",
            "non-ASCII string content is outside the fixed subset",
        ));
    }
    Ok((
        JsonSubsetValue::String(inner.to_string()),
        &source[end + 1..],
    ))
}

fn parse_number_prefix(source: &str) -> Result<(JsonSubsetValue, &str), TreeSitterJsonSubsetError> {
    let digit_len = source
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_len == 0 {
        return Err(TreeSitterJsonSubsetError::new(
            "parse",
            format!("source is outside the fixed subset: {source:?}"),
        ));
    }
    let number = &source[..digit_len];
    let value = parse_subset_u8_number(number).ok_or_else(|| {
        TreeSitterJsonSubsetError::new(
            "parse",
            format!("number {number:?} is outside the fixed u8 subset"),
        )
    })?;
    Ok((JsonSubsetValue::Number(value), &source[digit_len..]))
}

/// Parse an unsigned decimal integer in `0..=255` with no leading zeros.
fn parse_subset_u8_number(source: &str) -> Option<u8> {
    if source.is_empty() || !source.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if source.len() > 1 && source.starts_with('0') {
        return None;
    }
    source.parse::<u8>().ok()
}

/// Compiled subset codec bound to the pinned grammar provenance.
#[derive(Debug)]
pub struct TreeSitterJsonSubsetCodec {
    /// Shared pin → SM → multi-alt freeze core (CPU-2 framework).
    core: CompiledSubsetCodecCore,
}

impl TreeSitterJsonSubsetCodec {
    /// Load the B0 pin, project the fixed subset, and compile the raw SM model.
    pub fn from_pinned_fixture() -> Result<Self, TreeSitterJsonSubsetError> {
        Self::from_pinned_dir(&TreeSitterGrammarPin::fixture_dir())
    }

    /// Load a pinned grammar directory and project the fixed subset.
    pub fn from_pinned_dir(dir: &Path) -> Result<Self, TreeSitterJsonSubsetError> {
        let core = CompiledSubsetCodecCore::from_pinned_dir(
            dir,
            SUBSET_GRAMMAR_ID,
            project_null_bool_string_number_array_object_subset,
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

    /// Encode a subset value into an expanded raw syntax token stream.
    ///
    /// Lowers the language value to a model-conforming [`RawSyntaxDecodedNode`],
    /// then uses the generic encoder (`CompiledSubsetCodecCore::encode_decoded_tree`).
    pub fn encode(&self, value: &JsonSubsetValue) -> Result<Vec<u32>, TreeSitterJsonSubsetError> {
        let root = value_to_decoded_node(value);
        self.core.encode_decoded_tree(&root)
    }

    /// Strict raw SM decode of a complete expanded token stream.
    pub fn decode_raw(
        &self,
        tokens: &[u32],
    ) -> Result<RawSyntaxDecodedTree, TreeSitterJsonSubsetError> {
        self.core
            .model
            .decode_complete_tree(tokens)
            .map_err(|error| TreeSitterJsonSubsetError::new("decode", format!("{error:?}")))
    }

    /// Decode tokens into the subset value AST.
    pub fn decode(&self, tokens: &[u32]) -> Result<JsonSubsetValue, TreeSitterJsonSubsetError> {
        let tree = self.decode_raw(tokens)?;
        let root = tree
            .decoded_node_tree()
            .map_err(|error| TreeSitterJsonSubsetError::new("decode_tree", error.to_string()))?;
        value_from_decoded_value_node(&root)
    }

    /// Parse → encode → raw SM decode → render → reparse round-trip.
    pub fn round_trip_source(
        &self,
        source: &str,
    ) -> Result<TreeSitterJsonSubsetRoundTrip, TreeSitterJsonSubsetError> {
        self.core.run_round_trip_source(
            source,
            CLAIM_BOUNDARY,
            SUBSET_ID,
            JsonSubsetValue::parse,
            |value| self.encode(value),
            |tokens| self.decode(tokens),
            JsonSubsetValue::render,
            "value",
        )
    }
}

/// Lower a subset value AST to a model-conforming decoded-node tree.
///
/// This is the language-specific half of encode; the structure walk is generic
/// via [`CompiledSubsetCodecCore::encode_decoded_tree`] /
/// [`CompiledSyntaxModel::encode_complete_tree`]. Mirrors the raise path
/// (`value_from_decoded_value_node`) and the shapes decode materializes.
///
/// Public so opt-in layout modes (I5 `SharedAlphabet`) can compile the JSON
/// subset sidecar under shared roles and run language-value → decoded-node →
/// encode/decode inverses without using the default PerGrammar codec path or
/// editing absolute golden pins.
pub fn value_to_decoded_node(value: &JsonSubsetValue) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: VALUE_RULE.to_string(),
        node_kind: VALUE_RULE.to_string(),
        fields: vec![present_field(
            SUBTYPE_FIELD_NAME,
            vec![RawSyntaxDecodedValue::Node(Box::new(
                subtype_to_decoded_node(value),
            ))],
        )],
    }
}

fn subtype_to_decoded_node(value: &JsonSubsetValue) -> RawSyntaxDecodedNode {
    match value {
        JsonSubsetValue::Null => field_less_leaf(NULL_RULE),
        JsonSubsetValue::True => field_less_leaf(TRUE_RULE),
        JsonSubsetValue::False => field_less_leaf(FALSE_RULE),
        JsonSubsetValue::String(content) => string_to_decoded_node(content),
        JsonSubsetValue::Number(n) => number_to_decoded_node(*n),
        JsonSubsetValue::Array(elements) => array_to_decoded_node(elements),
        JsonSubsetValue::Object(pairs) => object_to_decoded_node(pairs),
    }
}

fn field_less_leaf(rule: &str) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: rule.to_string(),
        node_kind: rule.to_string(),
        fields: vec![],
    }
}

fn string_to_decoded_node(content: &str) -> RawSyntaxDecodedNode {
    // Empty string materializes as repeated_empty on optional repeated children
    // (same witness decode produces for the JSON subset).
    let children = if content.is_empty() {
        repeated_empty_field(CHILDREN_FIELD_NAME)
    } else {
        present_field(
            CHILDREN_FIELD_NAME,
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: STRING_CONTENT_KIND.to_string(),
                terminal: "string".to_string(),
                payload: content.to_string(),
            })],
        )
    };
    RawSyntaxDecodedNode {
        rule_name: STRING_RULE.to_string(),
        node_kind: STRING_RULE.to_string(),
        fields: vec![children],
    }
}

fn number_to_decoded_node(value: u8) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: NUMBER_RULE.to_string(),
        node_kind: NUMBER_RULE.to_string(),
        fields: vec![present_field(
            NUMBER_VALUE_FIELD,
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: NUMBER_RULE.to_string(),
                terminal: "integer".to_string(),
                payload: value.to_string(),
            })],
        )],
    }
}

fn array_to_decoded_node(elements: &[JsonSubsetValue]) -> RawSyntaxDecodedNode {
    let children = if elements.is_empty() {
        repeated_empty_field(CHILDREN_FIELD_NAME)
    } else {
        present_field(
            CHILDREN_FIELD_NAME,
            elements
                .iter()
                .map(|element| {
                    RawSyntaxDecodedValue::Node(Box::new(value_to_decoded_node(element)))
                })
                .collect(),
        )
    };
    RawSyntaxDecodedNode {
        rule_name: ARRAY_RULE.to_string(),
        node_kind: ARRAY_RULE.to_string(),
        fields: vec![children],
    }
}

fn object_to_decoded_node(pairs: &[JsonSubsetPair]) -> RawSyntaxDecodedNode {
    let children = if pairs.is_empty() {
        repeated_empty_field(CHILDREN_FIELD_NAME)
    } else {
        present_field(
            CHILDREN_FIELD_NAME,
            pairs
                .iter()
                .map(|pair| RawSyntaxDecodedValue::Node(Box::new(pair_to_decoded_node(pair))))
                .collect(),
        )
    };
    RawSyntaxDecodedNode {
        rule_name: OBJECT_RULE.to_string(),
        node_kind: OBJECT_RULE.to_string(),
        fields: vec![children],
    }
}

fn pair_to_decoded_node(pair: &JsonSubsetPair) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: PAIR_RULE.to_string(),
        node_kind: PAIR_RULE.to_string(),
        fields: vec![
            present_field(
                PAIR_KEY_FIELD,
                vec![RawSyntaxDecodedValue::Node(Box::new(
                    string_to_decoded_node(&pair.key),
                ))],
            ),
            present_field(
                PAIR_VALUE_FIELD,
                vec![RawSyntaxDecodedValue::Node(Box::new(
                    value_to_decoded_node(&pair.value),
                ))],
            ),
        ],
    }
}

fn present_field(name: &str, values: Vec<RawSyntaxDecodedValue>) -> RawSyntaxDecodedField {
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
        values: vec![],
        missing: false,
        repeated_empty: true,
    }
}

/// Sidecar-only layout for the JSON text subset (not a Go/layout pin).
pub fn subset_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 577,
        pad_token: 576,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "tree_sitter_json_subset.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 64,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_json_subset.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 64,
                end_exclusive: 320,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_json_subset.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 320,
                end_exclusive: 576,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "tree_sitter_json_subset.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

fn project_null_bool_string_number_array_object_subset(
    full: &GrammarIr,
) -> Result<GrammarIr, TreeSitterJsonSubsetError> {
    // Require the pin-derived rules we project from so a grammar shape drift
    // fails here instead of silently inventing structure.
    for required in [
        VALUE_RULE,
        NULL_RULE,
        TRUE_RULE,
        FALSE_RULE,
        STRING_RULE,
        STRING_CONTENT_KIND,
        NUMBER_RULE,
        ARRAY_RULE,
        OBJECT_RULE,
        PAIR_RULE,
    ] {
        require_rule(full, required, "subset")?;
    }
    let value = require_rule(full, VALUE_RULE, "subset")?;
    let subtype = value
        .fields
        .iter()
        .find(|field| field.name == SUBTYPE_FIELD_NAME)
        .ok_or_else(|| {
            TreeSitterJsonSubsetError::new("subset", "_value is missing synthetic subtype field")
        })?;
    let keep = [
        NULL_RULE,
        TRUE_RULE,
        FALSE_RULE,
        STRING_RULE,
        NUMBER_RULE,
        ARRAY_RULE,
        OBJECT_RULE,
    ];
    for kind in keep {
        if !subtype
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == kind)
        {
            return Err(TreeSitterJsonSubsetError::new(
                "subset",
                format!("pinned _value subtypes do not include {kind}"),
            ));
        }
    }
    // Pin classifies bare `number` as IntegerPayload on subtype; we wrap it as
    // a Nonterminal so subtype stays uniform Nonterminal alternatives.
    let pin_number_terminal = subtype
        .alternatives
        .iter()
        .find(|alternative| alternative.node_kind == NUMBER_RULE)
        .map(|alternative| alternative.terminal);
    if pin_number_terminal != Some(GrammarTerminalKind::IntegerPayload)
        && pin_number_terminal != Some(GrammarTerminalKind::Nonterminal)
    {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            format!("pinned number subtype terminal is unexpected: {pin_number_terminal:?}"),
        ));
    }
    // true/false/null are leaf named kinds in the pin (empty rules).
    for leaf in [TRUE_RULE, FALSE_RULE, NULL_RULE] {
        let rule = require_rule(full, leaf, "subset")?;
        if !rule.fields.is_empty() {
            return Err(TreeSitterJsonSubsetError::new(
                "subset",
                format!("pinned {leaf} rule unexpectedly has fields"),
            ));
        }
    }
    let string = require_rule(full, STRING_RULE, "subset")?;
    let children = string
        .fields
        .iter()
        .find(|field| field.name == CHILDREN_FIELD_NAME)
        .ok_or_else(|| {
            TreeSitterJsonSubsetError::new("subset", "string is missing synthetic children field")
        })?;
    if !children
        .alternatives
        .iter()
        .any(|alternative| alternative.node_kind == STRING_CONTENT_KIND)
    {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "string children do not include string_content",
        ));
    }
    let array = require_rule(full, ARRAY_RULE, "subset")?;
    let array_children = array
        .fields
        .iter()
        .find(|field| field.name == CHILDREN_FIELD_NAME)
        .ok_or_else(|| {
            TreeSitterJsonSubsetError::new("subset", "array is missing synthetic children field")
        })?;
    if !array_children.multiple || array_children.required {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "array.children must be optional multiple in the pin projection",
        ));
    }
    if !array_children.alternatives.iter().any(|alternative| {
        alternative.node_kind == VALUE_RULE
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }) {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "array children must include recursive Nonterminal _value",
        ));
    }
    let object = require_rule(full, OBJECT_RULE, "subset")?;
    let object_children = object
        .fields
        .iter()
        .find(|field| field.name == CHILDREN_FIELD_NAME)
        .ok_or_else(|| {
            TreeSitterJsonSubsetError::new("subset", "object is missing synthetic children field")
        })?;
    if !object_children.multiple || object_children.required {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "object.children must be optional multiple in the pin projection",
        ));
    }
    if !object_children.alternatives.iter().any(|alternative| {
        alternative.node_kind == PAIR_RULE
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }) {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "object children must include Nonterminal pair",
        ));
    }
    let pair = require_rule(full, PAIR_RULE, "subset")?;
    let pair_key = pair
        .fields
        .iter()
        .find(|field| field.name == PAIR_KEY_FIELD)
        .ok_or_else(|| {
            TreeSitterJsonSubsetError::new("subset", "pair is missing required key field")
        })?;
    if !pair_key.required || pair_key.multiple {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "pair.key must be required single in the pin projection",
        ));
    }
    if !pair_key.alternatives.iter().any(|alternative| {
        alternative.node_kind == STRING_RULE
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }) {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "pair.key must include Nonterminal string",
        ));
    }
    let pair_value = pair
        .fields
        .iter()
        .find(|field| field.name == PAIR_VALUE_FIELD)
        .ok_or_else(|| {
            TreeSitterJsonSubsetError::new("subset", "pair is missing required value field")
        })?;
    if !pair_value.required || pair_value.multiple {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "pair.value must be required single in the pin projection",
        ));
    }
    if !pair_value.alternatives.iter().any(|alternative| {
        alternative.node_kind == VALUE_RULE
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }) {
        return Err(TreeSitterJsonSubsetError::new(
            "subset",
            "pair.value must include recursive Nonterminal _value",
        ));
    }

    let mut provenance = full.provenance.clone();
    provenance.source_kind = "tree_sitter_node_types_subset".to_string();
    // Keep pin sha / package provenance; note the subset projection.
    let mut grammar = GrammarIr::new(SUBSET_GRAMMAR_ID, provenance);
    grammar.rules.push(GrammarRule {
        name: VALUE_RULE.to_string(),
        node_kind: VALUE_RULE.to_string(),
        fields: vec![GrammarField {
            name: SUBTYPE_FIELD_NAME.to_string(),
            required: true,
            multiple: false,
            alternatives: vec![
                GrammarFieldAlternative {
                    node_kind: ARRAY_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
                GrammarFieldAlternative {
                    node_kind: FALSE_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
                GrammarFieldAlternative {
                    node_kind: NULL_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
                GrammarFieldAlternative {
                    node_kind: NUMBER_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
                GrammarFieldAlternative {
                    node_kind: OBJECT_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
                GrammarFieldAlternative {
                    node_kind: STRING_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
                GrammarFieldAlternative {
                    node_kind: TRUE_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
            ],
        }],
    });
    grammar.rules.push(GrammarRule {
        name: ARRAY_RULE.to_string(),
        node_kind: ARRAY_RULE.to_string(),
        fields: vec![GrammarField {
            name: CHILDREN_FIELD_NAME.to_string(),
            required: false,
            multiple: true,
            alternatives: vec![GrammarFieldAlternative {
                node_kind: VALUE_RULE.to_string(),
                terminal: GrammarTerminalKind::Nonterminal,
            }],
        }],
    });
    grammar.rules.push(GrammarRule {
        name: FALSE_RULE.to_string(),
        node_kind: FALSE_RULE.to_string(),
        fields: vec![],
    });
    grammar.rules.push(GrammarRule {
        name: NULL_RULE.to_string(),
        node_kind: NULL_RULE.to_string(),
        fields: vec![],
    });
    // Wrapper Nonterminal with a single IntegerPayload field so `_value.subtype`
    // never mixes Nonterminal and IntegerPayload alternatives.
    grammar.rules.push(GrammarRule {
        name: NUMBER_RULE.to_string(),
        node_kind: NUMBER_RULE.to_string(),
        fields: vec![GrammarField {
            name: NUMBER_VALUE_FIELD.to_string(),
            required: true,
            multiple: false,
            alternatives: vec![GrammarFieldAlternative {
                node_kind: NUMBER_RULE.to_string(),
                terminal: GrammarTerminalKind::IntegerPayload,
            }],
        }],
    });
    grammar.rules.push(GrammarRule {
        name: OBJECT_RULE.to_string(),
        node_kind: OBJECT_RULE.to_string(),
        fields: vec![GrammarField {
            name: CHILDREN_FIELD_NAME.to_string(),
            required: false,
            multiple: true,
            alternatives: vec![GrammarFieldAlternative {
                node_kind: PAIR_RULE.to_string(),
                terminal: GrammarTerminalKind::Nonterminal,
            }],
        }],
    });
    // Pin order is key then value; field order is significant for the raw SM.
    grammar.rules.push(GrammarRule {
        name: PAIR_RULE.to_string(),
        node_kind: PAIR_RULE.to_string(),
        fields: vec![
            GrammarField {
                name: PAIR_KEY_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: STRING_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            },
            GrammarField {
                name: PAIR_VALUE_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: VALUE_RULE.to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            },
        ],
    });
    grammar.rules.push(GrammarRule {
        name: STRING_RULE.to_string(),
        node_kind: STRING_RULE.to_string(),
        fields: vec![GrammarField {
            name: CHILDREN_FIELD_NAME.to_string(),
            required: false,
            multiple: true,
            alternatives: vec![GrammarFieldAlternative {
                node_kind: STRING_CONTENT_KIND.to_string(),
                terminal: GrammarTerminalKind::StringPayload,
            }],
        }],
    });
    grammar.rules.push(GrammarRule {
        name: STRING_CONTENT_KIND.to_string(),
        node_kind: STRING_CONTENT_KIND.to_string(),
        fields: vec![],
    });
    grammar.rules.push(GrammarRule {
        name: TRUE_RULE.to_string(),
        node_kind: TRUE_RULE.to_string(),
        fields: vec![],
    });
    grammar
        .rules
        .sort_by(|left, right| left.name.cmp(&right.name));
    Ok(grammar)
}

fn value_from_decoded_value_node(
    node: &RawSyntaxDecodedNode,
) -> Result<JsonSubsetValue, TreeSitterJsonSubsetError> {
    if node.rule_name != VALUE_RULE && node.node_kind != VALUE_RULE {
        return Err(TreeSitterJsonSubsetError::new(
            "decode_tree",
            format!(
                "expected rule {VALUE_RULE}, got {} / {}",
                node.rule_name, node.node_kind
            ),
        ));
    }
    let subtype = node.field(SUBTYPE_FIELD_NAME).ok_or_else(|| {
        TreeSitterJsonSubsetError::new("decode_tree", "_value missing subtype field")
    })?;
    if subtype.values.len() != 1 {
        return Err(TreeSitterJsonSubsetError::new(
            "decode_tree",
            format!(
                "subtype must carry exactly one value, got {}",
                subtype.values.len()
            ),
        ));
    }
    match &subtype.values[0] {
        RawSyntaxDecodedValue::Node(child) if child.node_kind == NULL_RULE => {
            Ok(JsonSubsetValue::Null)
        }
        RawSyntaxDecodedValue::Node(child) if child.node_kind == TRUE_RULE => {
            Ok(JsonSubsetValue::True)
        }
        RawSyntaxDecodedValue::Node(child) if child.node_kind == FALSE_RULE => {
            Ok(JsonSubsetValue::False)
        }
        RawSyntaxDecodedValue::Node(child) if child.node_kind == STRING_RULE => Ok(
            JsonSubsetValue::String(string_content_from_decoded_node(child)?),
        ),
        RawSyntaxDecodedValue::Node(child) if child.node_kind == NUMBER_RULE => {
            let value_field = child.field(NUMBER_VALUE_FIELD).ok_or_else(|| {
                TreeSitterJsonSubsetError::new("decode_tree", "number missing value field")
            })?;
            if value_field.values.len() != 1 {
                return Err(TreeSitterJsonSubsetError::new(
                    "decode_tree",
                    format!(
                        "number value must carry exactly one payload, got {}",
                        value_field.values.len()
                    ),
                ));
            }
            match &value_field.values[0] {
                RawSyntaxDecodedValue::Terminal(terminal)
                    if terminal.node_kind == NUMBER_RULE
                        || terminal.terminal == "integer"
                        || terminal.terminal.contains("integer") =>
                {
                    let parsed = terminal.payload.parse::<u8>().map_err(|error| {
                        TreeSitterJsonSubsetError::new(
                            "decode_tree",
                            format!(
                                "number payload {:?} is not a u8 integer: {error}",
                                terminal.payload
                            ),
                        )
                    })?;
                    Ok(JsonSubsetValue::Number(parsed))
                }
                other => Err(TreeSitterJsonSubsetError::new(
                    "decode_tree",
                    format!("unexpected number value: {other:?}"),
                )),
            }
        }
        RawSyntaxDecodedValue::Node(child) if child.node_kind == ARRAY_RULE => {
            let children = child.field(CHILDREN_FIELD_NAME).ok_or_else(|| {
                TreeSitterJsonSubsetError::new("decode_tree", "array missing children field")
            })?;
            if children.missing || children.repeated_empty || children.values.is_empty() {
                return Ok(JsonSubsetValue::Array(Vec::new()));
            }
            let mut elements = Vec::with_capacity(children.values.len());
            for value in &children.values {
                match value {
                    RawSyntaxDecodedValue::Node(element) => {
                        elements.push(value_from_decoded_value_node(element)?);
                    }
                    other => {
                        return Err(TreeSitterJsonSubsetError::new(
                            "decode_tree",
                            format!("unexpected array child value: {other:?}"),
                        ));
                    }
                }
            }
            Ok(JsonSubsetValue::Array(elements))
        }
        RawSyntaxDecodedValue::Node(child) if child.node_kind == OBJECT_RULE => {
            let children = child.field(CHILDREN_FIELD_NAME).ok_or_else(|| {
                TreeSitterJsonSubsetError::new("decode_tree", "object missing children field")
            })?;
            if children.missing || children.repeated_empty || children.values.is_empty() {
                return Ok(JsonSubsetValue::Object(Vec::new()));
            }
            let mut pairs = Vec::with_capacity(children.values.len());
            for value in &children.values {
                match value {
                    RawSyntaxDecodedValue::Node(pair_node) => {
                        pairs.push(pair_from_decoded_node(pair_node)?);
                    }
                    other => {
                        return Err(TreeSitterJsonSubsetError::new(
                            "decode_tree",
                            format!("unexpected object child value: {other:?}"),
                        ));
                    }
                }
            }
            Ok(JsonSubsetValue::Object(pairs))
        }
        other => Err(TreeSitterJsonSubsetError::new(
            "decode_tree",
            format!("unsupported subtype value: {other:?}"),
        )),
    }
}

fn pair_from_decoded_node(
    node: &RawSyntaxDecodedNode,
) -> Result<JsonSubsetPair, TreeSitterJsonSubsetError> {
    if node.rule_name != PAIR_RULE && node.node_kind != PAIR_RULE {
        return Err(TreeSitterJsonSubsetError::new(
            "decode_tree",
            format!(
                "expected rule {PAIR_RULE}, got {} / {}",
                node.rule_name, node.node_kind
            ),
        ));
    }
    let key_field = node
        .field(PAIR_KEY_FIELD)
        .ok_or_else(|| TreeSitterJsonSubsetError::new("decode_tree", "pair missing key field"))?;
    if key_field.values.len() != 1 {
        return Err(TreeSitterJsonSubsetError::new(
            "decode_tree",
            format!(
                "pair key must carry exactly one value, got {}",
                key_field.values.len()
            ),
        ));
    }
    let key = match &key_field.values[0] {
        RawSyntaxDecodedValue::Node(string_node) if string_node.node_kind == STRING_RULE => {
            string_content_from_decoded_node(string_node)?
        }
        other => {
            return Err(TreeSitterJsonSubsetError::new(
                "decode_tree",
                format!("unexpected pair key value: {other:?}"),
            ));
        }
    };
    let value_field = node
        .field(PAIR_VALUE_FIELD)
        .ok_or_else(|| TreeSitterJsonSubsetError::new("decode_tree", "pair missing value field"))?;
    if value_field.values.len() != 1 {
        return Err(TreeSitterJsonSubsetError::new(
            "decode_tree",
            format!(
                "pair value must carry exactly one value, got {}",
                value_field.values.len()
            ),
        ));
    }
    let value = match &value_field.values[0] {
        RawSyntaxDecodedValue::Node(value_node) => value_from_decoded_value_node(value_node)?,
        other => {
            return Err(TreeSitterJsonSubsetError::new(
                "decode_tree",
                format!("unexpected pair value: {other:?}"),
            ));
        }
    };
    Ok(JsonSubsetPair { key, value })
}

fn string_content_from_decoded_node(
    child: &RawSyntaxDecodedNode,
) -> Result<String, TreeSitterJsonSubsetError> {
    let children = child.field(CHILDREN_FIELD_NAME).ok_or_else(|| {
        TreeSitterJsonSubsetError::new("decode_tree", "string missing children field")
    })?;
    if children.missing || children.repeated_empty || children.values.is_empty() {
        return Ok(String::new());
    }
    let mut content = String::new();
    for value in &children.values {
        match value {
            RawSyntaxDecodedValue::Terminal(terminal)
                if terminal.node_kind == STRING_CONTENT_KIND =>
            {
                content.push_str(&terminal.payload);
            }
            other => {
                return Err(TreeSitterJsonSubsetError::new(
                    "decode_tree",
                    format!("unexpected string child value: {other:?}"),
                ));
            }
        }
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subset_parse_render_null_bool_string_number_array_and_object() {
        assert_eq!(
            JsonSubsetValue::parse("null").unwrap(),
            JsonSubsetValue::Null
        );
        assert_eq!(
            JsonSubsetValue::parse("true").unwrap(),
            JsonSubsetValue::True
        );
        assert_eq!(
            JsonSubsetValue::parse("false").unwrap(),
            JsonSubsetValue::False
        );
        assert_eq!(
            JsonSubsetValue::parse("\"hi\"").unwrap(),
            JsonSubsetValue::String("hi".to_string())
        );
        assert_eq!(
            JsonSubsetValue::parse("0").unwrap(),
            JsonSubsetValue::Number(0)
        );
        assert_eq!(
            JsonSubsetValue::parse("42").unwrap(),
            JsonSubsetValue::Number(42)
        );
        assert_eq!(
            JsonSubsetValue::parse("255").unwrap(),
            JsonSubsetValue::Number(255)
        );
        assert_eq!(
            JsonSubsetValue::parse("[]").unwrap(),
            JsonSubsetValue::Array(Vec::new())
        );
        assert_eq!(
            JsonSubsetValue::parse("[null,true,1,\"a\"]").unwrap(),
            JsonSubsetValue::Array(vec![
                JsonSubsetValue::Null,
                JsonSubsetValue::True,
                JsonSubsetValue::Number(1),
                JsonSubsetValue::String("a".to_string()),
            ])
        );
        assert_eq!(
            JsonSubsetValue::parse("[[null],[]]").unwrap(),
            JsonSubsetValue::Array(vec![
                JsonSubsetValue::Array(vec![JsonSubsetValue::Null]),
                JsonSubsetValue::Array(Vec::new()),
            ])
        );
        assert_eq!(
            JsonSubsetValue::parse("{}").unwrap(),
            JsonSubsetValue::Object(Vec::new())
        );
        assert_eq!(
            JsonSubsetValue::parse("{\"a\":1}").unwrap(),
            JsonSubsetValue::Object(vec![JsonSubsetPair {
                key: "a".to_string(),
                value: JsonSubsetValue::Number(1),
            }])
        );
        assert_eq!(
            JsonSubsetValue::parse("{\"k\":null,\"nested\":{\"x\":true}}").unwrap(),
            JsonSubsetValue::Object(vec![
                JsonSubsetPair {
                    key: "k".to_string(),
                    value: JsonSubsetValue::Null,
                },
                JsonSubsetPair {
                    key: "nested".to_string(),
                    value: JsonSubsetValue::Object(vec![JsonSubsetPair {
                        key: "x".to_string(),
                        value: JsonSubsetValue::True,
                    }]),
                },
            ])
        );
        assert!(JsonSubsetValue::parse("256").is_err());
        assert!(JsonSubsetValue::parse("01").is_err());
        assert!(JsonSubsetValue::parse("-1").is_err());
        assert!(JsonSubsetValue::parse("1.5").is_err());
        assert!(JsonSubsetValue::parse("{a:1}").is_err());
        assert!(JsonSubsetValue::parse("{\"a\":1,}").is_err());
        assert!(JsonSubsetValue::parse("{\"a\": 1}").is_err());
        assert!(JsonSubsetValue::parse("[1,]").is_err());
        assert!(JsonSubsetValue::parse("[ 1]").is_err());
        assert_eq!(JsonSubsetValue::Null.render(), "null");
        assert_eq!(JsonSubsetValue::True.render(), "true");
        assert_eq!(JsonSubsetValue::False.render(), "false");
        assert_eq!(JsonSubsetValue::String("ab".to_string()).render(), "\"ab\"");
        assert_eq!(JsonSubsetValue::Number(7).render(), "7");
        assert_eq!(
            JsonSubsetValue::Array(vec![
                JsonSubsetValue::Number(1),
                JsonSubsetValue::Array(vec![JsonSubsetValue::Null])
            ])
            .render(),
            "[1,[null]]"
        );
        assert_eq!(
            JsonSubsetValue::Object(vec![
                JsonSubsetPair {
                    key: "a".to_string(),
                    value: JsonSubsetValue::Number(1),
                },
                JsonSubsetPair {
                    key: "".to_string(),
                    value: JsonSubsetValue::Object(Vec::new()),
                },
            ])
            .render(),
            "{\"a\":1,\"\":{}}"
        );
    }
}
