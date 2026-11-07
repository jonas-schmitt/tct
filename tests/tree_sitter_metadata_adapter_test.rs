use std::fs;
use std::path::Path;

use tct::grammar_ir::GrammarTerminalKind;
use tct::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
    GrammarStructuralTokenRange, lower_grammar_ir_to_structural_model,
};
use tct::shared_alphabet_layout::{
    SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
    shared_alphabet_choice_token_id,
};
use tct::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedTerminal,
    RawSyntaxDecodedValue,
};
use tct::toy_grammar_frontend::ToyGrammarSpec;
use tct::tree_sitter_metadata::{
    CHILDREN_FIELD_NAME, SUBTYPE_FIELD_NAME, TreeSitterGrammarPin, TreeSitterMetadataAdapter,
};

#[test]
fn tree_sitter_node_types_lower_to_minimal_grammar_ir() {
    let adapter = TreeSitterMetadataAdapter::new("toy_assignment_tree_sitter");
    let grammar = adapter
        .lower_node_types_file(Path::new(
            "tests/fixtures/tree_sitter_metadata/toy_assignment/node-types.json",
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "toy_assignment_tree_sitter");
    assert_eq!(grammar.provenance.source_kind, "tree_sitter_node_types");
    assert!(
        grammar
            .provenance
            .source_sha256
            .as_ref()
            .is_some_and(|sha| sha.len() == 64)
    );

    let assignment = grammar.rule("assignment").unwrap();
    assert_eq!(assignment.fields.len(), 2);
    let name = assignment
        .fields
        .iter()
        .find(|field| field.name == "name")
        .unwrap();
    assert!(name.required);
    assert!(!name.multiple);
    assert_eq!(name.alternatives[0].node_kind, "identifier");
    assert_eq!(
        name.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let value = assignment
        .fields
        .iter()
        .find(|field| field.name == "value")
        .unwrap();
    let terminals: Vec<_> = value
        .alternatives
        .iter()
        .map(|alternative| alternative.terminal)
        .collect();
    assert!(terminals.contains(&GrammarTerminalKind::IntegerPayload));
    assert!(terminals.contains(&GrammarTerminalKind::StringPayload));
    assert!(terminals.contains(&GrammarTerminalKind::BooleanPayload));

    let program = grammar.rule("program").unwrap();
    let statement = &program.fields[0];
    assert!(statement.multiple);
    assert_eq!(
        statement.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    let toy_layout = ToyGrammarSpec::assignment_language_v1().layout;
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&toy_layout),
    )
    .unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "tree_sitter_node_types"
    );
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 1);
    assert_eq!(
        sidecar.structural_model.references[0].target_rule,
        "assignment"
    );
    assert!(
        sidecar
            .structural_model
            .decision_graph
            .nodes
            .iter()
            .any(|node| node.id == "decision.rule.assignment.field.value"
                && node.decision_kind == "choice")
    );
    assert!(
        sidecar
            .structural_model
            .decision_graph
            .nodes
            .iter()
            .any(
                |node| node.id == "decision.rule.assignment.field.name.alternative.identifier"
                    && node.payload_kind.as_deref() == Some("identifier")
            )
    );
    assert_eq!(
        sidecar
            .proof
            .checks
            .get("decision_graph_payloads_match_payload_terminals")
            .copied(),
        Some(true)
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn tree_sitter_json_pin_lowers_to_grammar_ir_and_structural_model() {
    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_json_v0_24_8");
    let (grammar, pin) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::fixture_dir())
        .unwrap();

    assert_eq!(pin.pin_id, "tree_sitter_json_node_types_v0.24.8");
    assert_eq!(pin.grammar_package, "tree-sitter-json");
    assert_eq!(pin.grammar_version, "0.24.8");
    assert_eq!(pin.license, "MIT");
    assert_eq!(
        pin.node_types_sha256,
        "620b6f5d38676a97526e9d128972d4b156ddbdf6e130cd7e6b9c35e6823e943f"
    );
    assert!(
        TreeSitterGrammarPin::fixture_dir()
            .join(pin.license_file.as_deref().unwrap_or("LICENSE"))
            .is_file()
    );

    assert_eq!(grammar.grammar_id, "tree_sitter_json_v0_24_8");
    assert_eq!(grammar.provenance.source_kind, "tree_sitter_node_types");
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(pin.node_types_sha256.as_str())
    );
    assert!(
        grammar
            .provenance
            .metadata_path
            .as_ref()
            .is_some_and(|path| path.ends_with("pin.json"))
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "tree-sitter");
    assert_eq!(parser.package.as_deref(), Some("tree-sitter-json"));
    assert_eq!(parser.package_version.as_deref(), Some("0.24.8"));

    // Anonymous punctuation is dropped; every named export becomes a rule.
    let rule_names: Vec<_> = grammar
        .rules
        .iter()
        .map(|rule| rule.name.as_str())
        .collect();
    assert_eq!(
        rule_names,
        vec![
            "_value",
            "array",
            "comment",
            "document",
            "escape_sequence",
            "false",
            "null",
            "number",
            "object",
            "pair",
            "string",
            "string_content",
            "true",
        ]
    );

    // Supertype → synthetic subtype choice over concrete JSON values.
    let value = grammar.rule("_value").unwrap();
    assert_eq!(value.fields.len(), 1);
    assert_eq!(value.fields[0].name, SUBTYPE_FIELD_NAME);
    assert!(value.fields[0].required);
    assert!(!value.fields[0].multiple);
    let subtype_kinds: Vec<_> = value.fields[0]
        .alternatives
        .iter()
        .map(|alternative| alternative.node_kind.as_str())
        .collect();
    assert_eq!(
        subtype_kinds,
        vec![
            "array", "false", "null", "number", "object", "string", "true",
        ]
    );
    assert_eq!(
        value.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "object")
            .unwrap()
            .terminal,
        GrammarTerminalKind::Nonterminal
    );
    assert_eq!(
        value.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "number")
            .unwrap()
            .terminal,
        GrammarTerminalKind::IntegerPayload
    );
    assert_eq!(
        value.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "true")
            .unwrap()
            .terminal,
        GrammarTerminalKind::BooleanPayload
    );

    // Field-less children blocks become a synthetic list field.
    let document = grammar.rule("document").unwrap();
    assert_eq!(document.fields.len(), 1);
    assert_eq!(document.fields[0].name, CHILDREN_FIELD_NAME);
    assert!(!document.fields[0].required);
    assert!(document.fields[0].multiple);
    assert_eq!(document.fields[0].alternatives[0].node_kind, "_value");
    assert_eq!(
        document.fields[0].alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    let array = grammar.rule("array").unwrap();
    assert_eq!(array.fields[0].name, CHILDREN_FIELD_NAME);
    assert!(array.fields[0].multiple);
    assert_eq!(array.fields[0].alternatives[0].node_kind, "_value");

    let object = grammar.rule("object").unwrap();
    assert_eq!(object.fields[0].name, CHILDREN_FIELD_NAME);
    assert_eq!(object.fields[0].alternatives[0].node_kind, "pair");
    assert_eq!(
        object.fields[0].alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    // Named fields remain first-class; composite string stays a nonterminal.
    let pair = grammar.rule("pair").unwrap();
    assert_eq!(pair.fields.len(), 2);
    let key = pair
        .fields
        .iter()
        .find(|field| field.name == "key")
        .unwrap();
    assert!(key.required);
    assert_eq!(key.alternatives[0].node_kind, "string");
    assert_eq!(
        key.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let pair_value = pair
        .fields
        .iter()
        .find(|field| field.name == "value")
        .unwrap();
    assert_eq!(pair_value.alternatives[0].node_kind, "_value");
    assert_eq!(
        pair_value.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    let string = grammar.rule("string").unwrap();
    assert_eq!(string.fields[0].name, CHILDREN_FIELD_NAME);
    let string_child_kinds: Vec<_> = string.fields[0]
        .alternatives
        .iter()
        .map(|alternative| (alternative.node_kind.as_str(), alternative.terminal))
        .collect();
    assert!(string_child_kinds.contains(&("escape_sequence", GrammarTerminalKind::StringPayload)));
    assert!(string_child_kinds.contains(&("string_content", GrammarTerminalKind::StringPayload)));

    // Leaves keep empty field lists.
    assert!(grammar.rule("number").unwrap().fields.is_empty());
    assert!(grammar.rule("null").unwrap().fields.is_empty());
    assert!(grammar.rule("comment").unwrap().fields.is_empty());

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, tree_sitter_json_sidecar_token_layout())
            .unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(
        !sidecar
            .structural_model
            .boundary
            .production_grammar_compiler_path_changed
    );
    assert_eq!(
        sidecar
            .input
            .provenance
            .parser
            .as_ref()
            .unwrap()
            .package_version
            .as_deref(),
        Some("0.24.8")
    );

    assert!(
        sidecar
            .structural_model
            .lists
            .iter()
            .any(|list| list.owner_rule == "document" && list.field == CHILDREN_FIELD_NAME)
    );
    assert!(
        sidecar
            .structural_model
            .lists
            .iter()
            .any(|list| list.owner_rule == "array" && list.field == CHILDREN_FIELD_NAME)
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "document"
            && reference.field == CHILDREN_FIELD_NAME
            && reference.target_rule == "_value"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "pair"
            && reference.field == "key"
            && reference.target_rule == "string"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "_value"
            && reference.field == SUBTYPE_FIELD_NAME
            && reference.target_rule == "object"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "_value"
                    && payload.node_kind == "number"
                    && payload.payload_kind == "integer"
            })
    );
    assert!(
        sidecar
            .structural_model
            .decision_graph
            .nodes
            .iter()
            .any(|node| node.id == "decision.rule._value.field.subtype"
                && node.decision_kind == "choice")
    );
    assert_eq!(
        sidecar
            .proof
            .checks
            .get("decision_graph_payloads_match_payload_terminals")
            .copied(),
        Some(true)
    );
}

/// B′ contract: full-pin `_value.subtype` mixes Nonterminal, BooleanPayload,
/// and IntegerPayload. This is a lower-path fact only — not a full-pin codec
/// claim. See `docs/tree-sitter-json-mixed-payload.md`.
#[test]
fn tree_sitter_json_full_pin_value_subtype_has_mixed_payload_kinds() {
    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_json_mixed_payload_probe");
    let (grammar, pin) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::fixture_dir())
        .unwrap();
    assert_eq!(pin.pin_id, "tree_sitter_json_node_types_v0.24.8");

    let value = grammar.rule("_value").unwrap();
    assert_eq!(value.fields.len(), 1);
    assert_eq!(value.fields[0].name, SUBTYPE_FIELD_NAME);

    let expected: &[(&str, GrammarTerminalKind)] = &[
        ("array", GrammarTerminalKind::Nonterminal),
        ("false", GrammarTerminalKind::BooleanPayload),
        ("null", GrammarTerminalKind::Nonterminal),
        ("number", GrammarTerminalKind::IntegerPayload),
        ("object", GrammarTerminalKind::Nonterminal),
        ("string", GrammarTerminalKind::Nonterminal),
        ("true", GrammarTerminalKind::BooleanPayload),
    ];
    let actual: Vec<(&str, GrammarTerminalKind)> = value.fields[0]
        .alternatives
        .iter()
        .map(|alternative| (alternative.node_kind.as_str(), alternative.terminal))
        .collect();
    assert_eq!(actual, expected.to_vec());

    let terminals: Vec<_> = actual.iter().map(|(_, terminal)| *terminal).collect();
    assert!(
        terminals.contains(&GrammarTerminalKind::BooleanPayload)
            && terminals.contains(&GrammarTerminalKind::IntegerPayload)
            && terminals.contains(&GrammarTerminalKind::Nonterminal),
        "full pin must keep the three-way mixed-payload shape that forced the subset number wrapper: {terminals:?}"
    );
    assert!(
        !terminals.iter().any(|terminal| matches!(
            terminal,
            GrammarTerminalKind::StringPayload | GrammarTerminalKind::IdentifierPayload
        )),
        "unexpected payload class on full-pin _value.subtype: {terminals:?}"
    );

    // Bare pin `number` is a field-less IntegerPayload leaf, not a wrapper.
    assert!(grammar.rule("number").unwrap().fields.is_empty());

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, tree_sitter_json_sidecar_token_layout())
            .unwrap();
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "_value"
                    && payload.node_kind == "number"
                    && payload.payload_kind == "integer"
            }),
        "full pin records integer payload on _value.subtype/number (not under a number wrapper field)"
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "_value"
                    && payload.node_kind == "true"
                    && payload.payload_kind == "boolean"
            }),
        "full pin records boolean payload peers on the same subtype field"
    );
}

/// B′′ second-grammar pin: tree-sitter-c metadata lowers through the same
/// adapter path (subtypes, field maps, field-less children, composite
/// nonterminals). Metadata pin + adapter lower only — not a C codec.
#[test]
fn tree_sitter_c_pin_lowers_to_grammar_ir_and_structural_model() {
    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_c_v0_24_2");
    let (grammar, pin) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::c_fixture_dir())
        .unwrap();

    assert_eq!(pin.pin_id, "tree_sitter_c_node_types_v0.24.2");
    assert_eq!(pin.grammar_package, "tree-sitter-c");
    assert_eq!(pin.grammar_version, "0.24.2");
    assert_eq!(pin.license, "MIT");
    assert_eq!(
        pin.node_types_sha256,
        "23e819ef1eefd357bb6eba844f47f8492f6a88da3e5046b06ec4acd4da8a9fd6"
    );
    assert!(
        TreeSitterGrammarPin::c_fixture_dir()
            .join(pin.license_file.as_deref().unwrap_or("LICENSE"))
            .is_file()
    );
    assert!(
        pin.notes
            .iter()
            .any(|note| note.contains("second metadata pin")),
        "pin notes should document why C was chosen as the second grammar"
    );

    assert_eq!(grammar.grammar_id, "tree_sitter_c_v0_24_2");
    assert_eq!(grammar.provenance.source_kind, "tree_sitter_node_types");
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(pin.node_types_sha256.as_str())
    );
    assert!(
        grammar
            .provenance
            .metadata_path
            .as_ref()
            .is_some_and(|path| path.ends_with("pin.json"))
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "tree-sitter");
    assert_eq!(parser.package.as_deref(), Some("tree-sitter-c"));
    assert_eq!(parser.package_version.as_deref(), Some("0.24.2"));

    // Named export coverage is much larger than JSON; pin must not be empty.
    assert!(
        grammar.rules.len() > 100,
        "expected full named-node export, got {} rules",
        grammar.rules.len()
    );
    for required in [
        "translation_unit",
        "function_definition",
        "expression",
        "statement",
        "call_expression",
        "binary_expression",
        "identifier",
        "number_literal",
        "string_literal",
        "string_content",
        "true",
        "false",
    ] {
        assert!(
            grammar.rule(required).is_some(),
            "missing named rule {required}"
        );
    }

    // Supertype → synthetic subtype choice (expression / statement).
    let expression = grammar.rule("expression").unwrap();
    assert_eq!(expression.fields.len(), 1);
    assert_eq!(expression.fields[0].name, SUBTYPE_FIELD_NAME);
    assert!(expression.fields[0].required);
    assert!(!expression.fields[0].multiple);
    assert!(expression.fields[0].alternatives.len() >= 20);
    assert_eq!(
        expression.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "call_expression")
            .unwrap()
            .terminal,
        GrammarTerminalKind::Nonterminal
    );
    assert_eq!(
        expression.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "number_literal")
            .unwrap()
            .terminal,
        GrammarTerminalKind::IntegerPayload
    );
    assert_eq!(
        expression.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "true")
            .unwrap()
            .terminal,
        GrammarTerminalKind::BooleanPayload
    );
    assert_eq!(
        expression.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "identifier")
            .unwrap()
            .terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let statement = grammar.rule("statement").unwrap();
    assert_eq!(statement.fields[0].name, SUBTYPE_FIELD_NAME);
    assert!(
        statement.fields[0]
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "compound_statement"
                && alternative.terminal == GrammarTerminalKind::Nonterminal)
    );

    // Field-less children on the translation unit root.
    let translation_unit = grammar.rule("translation_unit").unwrap();
    assert_eq!(translation_unit.fields.len(), 1);
    assert_eq!(translation_unit.fields[0].name, CHILDREN_FIELD_NAME);
    assert!(!translation_unit.fields[0].required);
    assert!(translation_unit.fields[0].multiple);
    assert!(
        translation_unit.fields[0]
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "function_definition"
                && alternative.terminal == GrammarTerminalKind::Nonterminal)
    );

    // Named fields + optional field-less children on the same composite.
    let function_definition = grammar.rule("function_definition").unwrap();
    let field_names: Vec<_> = function_definition
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert!(field_names.contains(&"body"));
    assert!(field_names.contains(&"declarator"));
    assert!(field_names.contains(&"type"));
    assert!(field_names.contains(&CHILDREN_FIELD_NAME));
    let body = function_definition
        .fields
        .iter()
        .find(|field| field.name == "body")
        .unwrap();
    assert!(body.required);
    assert_eq!(body.alternatives[0].node_kind, "compound_statement");
    assert_eq!(
        body.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    let call = grammar.rule("call_expression").unwrap();
    let function_field = call
        .fields
        .iter()
        .find(|field| field.name == "function")
        .unwrap();
    assert!(function_field.required);
    assert_eq!(function_field.alternatives[0].node_kind, "expression");
    assert_eq!(
        function_field.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    // Leaves and string payload classification match the shared adapter map.
    assert!(grammar.rule("number_literal").unwrap().fields.is_empty());
    assert!(grammar.rule("identifier").unwrap().fields.is_empty());
    assert!(grammar.rule("true").unwrap().fields.is_empty());
    // Anonymous operators referenced from named fields become leaf rules.
    let star = grammar
        .rule("*")
        .expect("referenced anonymous `*` must materialize");
    assert!(star.fields.is_empty());
    let binary = grammar.rule("binary_expression").unwrap();
    let operator = binary
        .fields
        .iter()
        .find(|field| field.name == "operator")
        .unwrap();
    assert!(
        operator
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "*"
                && alternative.terminal == GrammarTerminalKind::Nonterminal)
    );
    let string_literal = grammar.rule("string_literal").unwrap();
    assert_eq!(string_literal.fields[0].name, CHILDREN_FIELD_NAME);
    let string_child_kinds: Vec<_> = string_literal.fields[0]
        .alternatives
        .iter()
        .map(|alternative| (alternative.node_kind.as_str(), alternative.terminal))
        .collect();
    assert!(string_child_kinds.contains(&("escape_sequence", GrammarTerminalKind::StringPayload)));
    assert!(string_child_kinds.contains(&("string_content", GrammarTerminalKind::StringPayload)));

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, tree_sitter_c_sidecar_token_layout())
            .unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(
        !sidecar
            .structural_model
            .boundary
            .production_grammar_compiler_path_changed
    );
    assert_eq!(
        sidecar
            .input
            .provenance
            .parser
            .as_ref()
            .unwrap()
            .package
            .as_deref(),
        Some("tree-sitter-c")
    );
    assert_eq!(
        sidecar
            .input
            .provenance
            .parser
            .as_ref()
            .unwrap()
            .package_version
            .as_deref(),
        Some("0.24.2")
    );

    assert!(
        sidecar
            .structural_model
            .lists
            .iter()
            .any(|list| list.owner_rule == "translation_unit" && list.field == CHILDREN_FIELD_NAME)
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "function_definition"
            && reference.field == "body"
            && reference.target_rule == "compound_statement"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "expression"
            && reference.field == SUBTYPE_FIELD_NAME
            && reference.target_rule == "call_expression"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "expression"
                    && payload.node_kind == "number_literal"
                    && payload.payload_kind == "integer"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "expression"
                    && payload.node_kind == "identifier"
                    && payload.payload_kind == "identifier"
            })
    );
    assert!(
        sidecar
            .structural_model
            .decision_graph
            .nodes
            .iter()
            .any(|node| node.id == "decision.rule.expression.field.subtype"
                && node.decision_kind == "choice")
    );
    assert_eq!(
        sidecar
            .proof
            .checks
            .get("decision_graph_payloads_match_payload_terminals")
            .copied(),
        Some(true)
    );
}

#[test]
fn tree_sitter_pinned_grammar_rejects_sha256_mismatch() {
    let adapter = TreeSitterMetadataAdapter::new("bad_pin");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("node-types.json"),
        r#"[{"type":"program","named":true,"fields":{}}]"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("pin.json"),
        r#"{
          "pin_id": "bad",
          "grammar_package": "tree-sitter-json",
          "grammar_version": "0.0.0",
          "license": "MIT",
          "node_types_file": "node-types.json",
          "node_types_sha256": "0000000000000000000000000000000000000000000000000000000000000000"
        }"#,
    )
    .unwrap();

    let error = adapter.lower_pinned_grammar_dir(dir.path()).unwrap_err();
    assert_eq!(error.phase, "provenance");
    assert!(error.message.contains("sha256 mismatch"));
    assert!(
        error.message.contains("pin expects") && error.message.contains("file has"),
        "mismatch error should name both digests: {}",
        error.message
    );
}

/// MG-3: multi-pin smoke — all three real metadata pins lower independently
/// with distinct pin identity and matching node-types digests. Metadata only.
#[test]
fn tree_sitter_multi_pin_smoke_json_c_javascript() {
    let pins: &[(&str, std::path::PathBuf, &str, &str)] = &[
        (
            "tree_sitter_json_v0_24_8",
            TreeSitterGrammarPin::json_fixture_dir(),
            "tree_sitter_json_node_types_v0.24.8",
            "tree-sitter-json",
        ),
        (
            "tree_sitter_c_v0_24_2",
            TreeSitterGrammarPin::c_fixture_dir(),
            "tree_sitter_c_node_types_v0.24.2",
            "tree-sitter-c",
        ),
        (
            "tree_sitter_javascript_v0_25_0",
            TreeSitterGrammarPin::javascript_fixture_dir(),
            "tree_sitter_javascript_node_types_v0.25.0",
            "tree-sitter-javascript",
        ),
    ];

    let mut seen_pin_ids = std::collections::BTreeSet::new();
    let mut seen_packages = std::collections::BTreeSet::new();
    let mut seen_sha = std::collections::BTreeSet::new();
    let mut seen_grammar_ids = std::collections::BTreeSet::new();

    for (grammar_id, dir, expected_pin_id, expected_package) in pins {
        assert!(
            dir.join("pin.json").is_file(),
            "missing pin.json under {}",
            dir.display()
        );
        assert!(
            dir.join("node-types.json").is_file(),
            "missing node-types.json under {}",
            dir.display()
        );
        assert!(
            dir.join("LICENSE").is_file(),
            "missing LICENSE under {}",
            dir.display()
        );

        let adapter = TreeSitterMetadataAdapter::new(*grammar_id);
        let (grammar, pin) = adapter
            .lower_pinned_grammar_dir(dir)
            .unwrap_or_else(|error| {
                panic!(
                    "multi-pin smoke failed for {grammar_id} at {}: {error}",
                    dir.display()
                )
            });

        assert_eq!(pin.pin_id, *expected_pin_id);
        assert_eq!(pin.grammar_package, *expected_package);
        assert_eq!(pin.license, "MIT");
        assert_eq!(pin.node_types_sha256.len(), 64);
        assert_eq!(grammar.grammar_id, *grammar_id);
        assert_eq!(
            grammar.provenance.source_sha256.as_deref(),
            Some(pin.node_types_sha256.as_str())
        );
        assert_eq!(
            grammar
                .provenance
                .parser
                .as_ref()
                .and_then(|parser| parser.package.as_deref()),
            Some(*expected_package)
        );
        assert!(
            !grammar.rules.is_empty(),
            "{grammar_id} must emit at least one rule"
        );

        assert!(
            seen_pin_ids.insert(pin.pin_id.clone()),
            "duplicate pin_id {}",
            pin.pin_id
        );
        assert!(
            seen_packages.insert(pin.grammar_package.clone()),
            "duplicate package {}",
            pin.grammar_package
        );
        assert!(
            seen_sha.insert(pin.node_types_sha256.clone()),
            "duplicate node_types_sha256 {}",
            pin.node_types_sha256
        );
        assert!(
            seen_grammar_ids.insert(grammar.grammar_id.clone()),
            "duplicate grammar_id {}",
            grammar.grammar_id
        );
    }

    assert_eq!(seen_pin_ids.len(), 3);
    assert_eq!(seen_packages.len(), 3);
    assert_eq!(seen_sha.len(), 3);
    assert_eq!(seen_grammar_ids.len(), 3);
}

#[test]
fn tree_sitter_pinned_grammar_rejects_missing_pin_json() {
    let adapter = TreeSitterMetadataAdapter::new("missing_pin");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("node-types.json"),
        r#"[{"type":"program","named":true,"fields":{}}]"#,
    )
    .unwrap();

    let error = adapter.lower_pinned_grammar_dir(dir.path()).unwrap_err();
    assert_eq!(error.phase, "read");
    assert!(
        error.message.contains("pin.json"),
        "missing pin should name pin.json: {}",
        error.message
    );
}

#[test]
fn tree_sitter_pinned_grammar_rejects_missing_node_types_file() {
    let adapter = TreeSitterMetadataAdapter::new("missing_node_types");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("pin.json"),
        r#"{
          "pin_id": "missing_nodes",
          "grammar_package": "tree-sitter-json",
          "grammar_version": "0.0.0",
          "license": "MIT",
          "node_types_file": "node-types.json",
          "node_types_sha256": "0000000000000000000000000000000000000000000000000000000000000000"
        }"#,
    )
    .unwrap();

    let error = adapter.lower_pinned_grammar_dir(dir.path()).unwrap_err();
    assert_eq!(error.phase, "read");
    assert!(
        error.message.contains("node-types.json"),
        "missing node-types should name the file: {}",
        error.message
    );
}

#[test]
fn tree_sitter_pinned_grammar_rejects_invalid_pin_json() {
    let adapter = TreeSitterMetadataAdapter::new("invalid_pin");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("pin.json"), r#"{ "pin_id": 123 }"#).unwrap();
    std::fs::write(
        dir.path().join("node-types.json"),
        r#"[{"type":"program","named":true,"fields":{}}]"#,
    )
    .unwrap();

    let error = adapter.lower_pinned_grammar_dir(dir.path()).unwrap_err();
    assert_eq!(error.phase, "parse");
    assert!(
        error.message.contains("pin.json is invalid"),
        "invalid pin should be parse-phase: {}",
        error.message
    );
}

#[test]
fn tree_sitter_metadata_adapter_rejects_empty_field_types_with_context() {
    let adapter = TreeSitterMetadataAdapter::new("bad");
    let error = adapter
        .lower_node_types_str(
            r#"[{"type":"node","named":true,"fields":{"child":{"required":true,"types":[]}}}]"#,
            tct::grammar_ir::GrammarIrProvenance {
                source_kind: "inline".to_string(),
                source_path: None,
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        )
        .unwrap_err();

    assert_eq!(error.phase, "lower");
    assert_eq!(error.node_type.as_deref(), Some("node"));
    assert_eq!(error.field.as_deref(), Some("child"));
}

#[test]
fn tree_sitter_metadata_adapter_lowers_children_and_subtypes() {
    let adapter = TreeSitterMetadataAdapter::new("inline_children_subtypes");
    let grammar = adapter
        .lower_node_types_str(
            r#"[
              {
                "type": "_expr",
                "named": true,
                "subtypes": [
                  {"type": "number", "named": true},
                  {"type": "list", "named": true}
                ]
              },
              {
                "type": "list",
                "named": true,
                "fields": {},
                "children": {
                  "multiple": true,
                  "required": false,
                  "types": [{"type": "_expr", "named": true}]
                }
              },
              {
                "type": "number",
                "named": true
              }
            ]"#,
            tct::grammar_ir::GrammarIrProvenance {
                source_kind: "inline".to_string(),
                source_path: None,
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        )
        .unwrap();

    let expr = grammar.rule("_expr").unwrap();
    assert_eq!(expr.fields[0].name, SUBTYPE_FIELD_NAME);
    assert_eq!(
        expr.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "list")
            .unwrap()
            .terminal,
        GrammarTerminalKind::Nonterminal
    );
    assert_eq!(
        expr.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "number")
            .unwrap()
            .terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let list = grammar.rule("list").unwrap();
    assert_eq!(list.fields[0].name, CHILDREN_FIELD_NAME);
    assert!(list.fields[0].multiple);
    assert_eq!(list.fields[0].alternatives[0].node_kind, "_expr");
}

/// CPU-5 T3: optional per-pin `terminal_classes.json` overrides leaf
/// classification while keeping the built-in default map for existing names.
/// Composite kinds stay Nonterminal even when a sidecar entry exists.
#[test]
fn tree_sitter_metadata_terminal_classes_sidecar_overrides_default_map() {
    let dir = tempfile::tempdir().unwrap();
    let node_types = r#"[
      {
        "type": "wrapper",
        "named": true,
        "fields": {
          "child": {
            "multiple": false,
            "required": true,
            "types": [
              {"type": "property_identifier", "named": true},
              {"type": "identifier", "named": true},
              {"type": "composite_kind", "named": true}
            ]
          }
        }
      },
      {"type": "property_identifier", "named": true},
      {"type": "identifier", "named": true},
      {
        "type": "composite_kind",
        "named": true,
        "fields": {
          "inner": {
            "multiple": false,
            "required": true,
            "types": [{"type": "identifier", "named": true}]
          }
        }
      }
    ]"#;
    std::fs::write(dir.path().join("node-types.json"), node_types).unwrap();
    let node_sha = tct::provenance::sha256_hex(node_types.as_bytes());
    let terminal_classes = r#"{
      "version": 1,
      "classes": {
        "property_identifier": "identifier",
        "composite_kind": "integer"
      }
    }"#;
    std::fs::write(dir.path().join("terminal_classes.json"), terminal_classes).unwrap();
    let classes_sha = tct::provenance::sha256_hex(terminal_classes.as_bytes());
    let pin = format!(
        r#"{{
      "pin_id": "terminal_classes_sidecar_test",
      "grammar_package": "test",
      "grammar_version": "0.0.0",
      "license": "MIT",
      "node_types_file": "node-types.json",
      "node_types_sha256": "{node_sha}",
      "terminal_classes_file": "terminal_classes.json",
      "terminal_classes_sha256": "{classes_sha}"
    }}"#
    );
    std::fs::write(dir.path().join("pin.json"), pin).unwrap();

    let adapter = TreeSitterMetadataAdapter::new("terminal_classes_sidecar");
    let (grammar, loaded_pin) = adapter.lower_pinned_grammar_dir(dir.path()).unwrap();
    assert_eq!(
        loaded_pin.terminal_classes_file.as_deref(),
        Some("terminal_classes.json")
    );

    let wrapper = grammar.rule("wrapper").unwrap();
    let child = &wrapper.fields[0];
    let class_of = |kind: &str| {
        child
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == kind)
            .unwrap()
            .terminal
    };
    // Sidecar promotes property_identifier (not in the default map).
    assert_eq!(
        class_of("property_identifier"),
        GrammarTerminalKind::IdentifierPayload
    );
    // Default map still classifies identifier.
    assert_eq!(
        class_of("identifier"),
        GrammarTerminalKind::IdentifierPayload
    );
    // Structural composite derivation wins over a sidecar integer label.
    assert_eq!(class_of("composite_kind"), GrammarTerminalKind::Nonterminal);
}

#[test]
fn tree_sitter_metadata_terminal_classes_sidecar_sha_mismatch_is_provenance_error() {
    let dir = tempfile::tempdir().unwrap();
    let node_types = r#"[{"type":"program","named":true,"fields":{}}]"#;
    std::fs::write(dir.path().join("node-types.json"), node_types).unwrap();
    let node_sha = tct::provenance::sha256_hex(node_types.as_bytes());
    std::fs::write(
        dir.path().join("terminal_classes.json"),
        r#"{"version":1,"classes":{}}"#,
    )
    .unwrap();
    let pin = format!(
        r#"{{
      "pin_id": "bad_classes_sha",
      "grammar_package": "test",
      "grammar_version": "0.0.0",
      "license": "MIT",
      "node_types_file": "node-types.json",
      "node_types_sha256": "{node_sha}",
      "terminal_classes_file": "terminal_classes.json",
      "terminal_classes_sha256": "0000000000000000000000000000000000000000000000000000000000000000"
    }}"#
    );
    std::fs::write(dir.path().join("pin.json"), pin).unwrap();

    let adapter = TreeSitterMetadataAdapter::new("bad_classes_sha");
    let error = adapter.lower_pinned_grammar_dir(dir.path()).unwrap_err();
    assert_eq!(error.phase, "provenance");
    assert!(
        error.message.contains("terminal_classes sha256 mismatch"),
        "expected classes sha mismatch, got: {}",
        error.message
    );
}

/// CPU-5 T2: a named node with no fields/children/subtypes is a structural
/// leaf. Emit a field-less rule so field alternatives that reference it
/// resolve through `collect_references` (design-note step 4.3: rule identity
/// carries meaning for atoms like `this` / `null` / `empty_statement`).
#[test]
fn tree_sitter_metadata_emits_fieldless_rules_for_named_structural_leaves() {
    let adapter = TreeSitterMetadataAdapter::new("structural_leaves");
    let grammar = adapter
        .lower_node_types_str(
            r#"[
              {
                "type": "statement",
                "named": true,
                "subtypes": [
                  {"type": "empty_statement", "named": true},
                  {"type": "debugger_statement", "named": true},
                  {"type": "expr_statement", "named": true}
                ]
              },
              {
                "type": "empty_statement",
                "named": true
              },
              {
                "type": "debugger_statement",
                "named": true
              },
              {
                "type": "expr_statement",
                "named": true,
                "fields": {
                  "expression": {
                    "multiple": false,
                    "required": true,
                    "types": [
                      {"type": "this", "named": true},
                      {"type": "null", "named": true},
                      {"type": "identifier", "named": true}
                    ]
                  }
                }
              },
              {
                "type": "this",
                "named": true
              },
              {
                "type": "null",
                "named": true
              },
              {
                "type": "identifier",
                "named": true
              }
            ]"#,
            tct::grammar_ir::GrammarIrProvenance {
                source_kind: "inline".to_string(),
                source_path: None,
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        )
        .unwrap();

    for leaf in [
        "empty_statement",
        "debugger_statement",
        "this",
        "null",
        "identifier",
    ] {
        let rule = grammar
            .rule(leaf)
            .unwrap_or_else(|| panic!("structural leaf {leaf} must emit a rule"));
        assert!(
            rule.fields.is_empty(),
            "structural leaf {leaf} must be field-less (rule identity carries meaning)"
        );
    }

    let expr_statement = grammar.rule("expr_statement").unwrap();
    let expression = &expr_statement.fields[0];
    assert_eq!(expression.name, "expression");
    for (kind, terminal) in [
        ("this", GrammarTerminalKind::Nonterminal),
        ("null", GrammarTerminalKind::Nonterminal),
        ("identifier", GrammarTerminalKind::IdentifierPayload),
    ] {
        let alternative = expression
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == kind)
            .unwrap_or_else(|| panic!("missing alternative {kind}"));
        assert_eq!(alternative.terminal, terminal);
    }

    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout {
            vocab_size: 64,
            pad_token: 63,
            ranges: vec![
                GrammarStructuralTokenRange {
                    id: "structural_leaves.structural".to_string(),
                    role: "structural_decisions".to_string(),
                    start: 0,
                    end_exclusive: 32,
                },
                GrammarStructuralTokenRange {
                    id: "structural_leaves.utf8".to_string(),
                    role: "utf8_byte_payload".to_string(),
                    start: 32,
                    end_exclusive: 63,
                },
            ],
            integer_value_range: None,
        },
    )
    .expect("structural-leaf rules must resolve every Nonterminal reference");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 7);
}

/// CPU-5 T2 acceptance on the JS pin: every field/children/subtype reference
/// resolves after named structural leaves emit field-less rules.
#[test]
fn tree_sitter_javascript_pin_structural_leaves_resolve_all_references() {
    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_javascript_v0_25_0");
    let (grammar, _) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::javascript_fixture_dir())
        .unwrap();

    // Representative zero-payload atoms and payload-classified leaves.
    for leaf in [
        "this",
        "super",
        "null",
        "undefined",
        "import",
        "optional_chain",
        "debugger_statement",
        "empty_statement",
        "meta_property",
        "hash_bang_line",
        "property_identifier",
        "identifier",
        "true",
        "false",
        "number",
    ] {
        let rule = grammar
            .rule(leaf)
            .unwrap_or_else(|| panic!("JS pin must emit structural/payload leaf rule {leaf}"));
        assert!(
            rule.fields.is_empty(),
            "JS pin leaf {leaf} must be field-less"
        );
    }

    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        tree_sitter_javascript_sidecar_token_layout(),
    )
    .expect("JS pin collect_references must resolve every Nonterminal after T2");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.structural_model_counts.references, 361);
}

/// CPU-5 T1: when a kind appears once named and once anonymous (JS `class`,
/// `import`), rule identity prefers the named node. The anonymous keyword is
/// a terminal literal, not a second rule with the same name.
#[test]
fn tree_sitter_metadata_prefers_named_rule_when_anonymous_keyword_collides() {
    let adapter = TreeSitterMetadataAdapter::new("named_anonymous_collision");
    let grammar = adapter
        .lower_node_types_str(
            r#"[
              {
                "type": "primary",
                "named": true,
                "subtypes": [
                  {"type": "class", "named": true},
                  {"type": "import", "named": true}
                ]
              },
              {
                "type": "class",
                "named": true,
                "fields": {
                  "name": {
                    "multiple": false,
                    "required": true,
                    "types": [{"type": "identifier", "named": true}]
                  }
                }
              },
              {
                "type": "class",
                "named": false
              },
              {
                "type": "import",
                "named": true
              },
              {
                "type": "import",
                "named": false
              },
              {
                "type": "identifier",
                "named": true
              },
              {
                "type": "+",
                "named": false
              },
              {
                "type": "binary",
                "named": true,
                "fields": {
                  "operator": {
                    "multiple": false,
                    "required": true,
                    "types": [{"type": "+", "named": false}]
                  }
                }
              }
            ]"#,
            tct::grammar_ir::GrammarIrProvenance {
                source_kind: "inline".to_string(),
                source_path: None,
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        )
        .unwrap();

    let rule_names: Vec<_> = grammar
        .rules
        .iter()
        .map(|rule| rule.name.as_str())
        .collect();
    let mut sorted_names = rule_names.clone();
    sorted_names.sort_unstable();
    let mut unique_names = sorted_names.clone();
    unique_names.dedup();
    assert_eq!(
        sorted_names, unique_names,
        "duplicate rule names after named/anonymous collision: {rule_names:?}"
    );

    assert_eq!(
        grammar
            .rules
            .iter()
            .filter(|rule| rule.name == "class")
            .count(),
        1,
        "prefer named class rule; do not emit anonymous keyword as second rule"
    );
    assert_eq!(
        grammar
            .rules
            .iter()
            .filter(|rule| rule.name == "import")
            .count(),
        1,
        "prefer named import rule; do not emit anonymous keyword as second rule"
    );

    // Named class keeps structural fields; named import stays a field-less leaf.
    let class = grammar.rule("class").expect("named class rule");
    assert_eq!(class.fields.len(), 1);
    assert_eq!(class.fields[0].name, "name");
    assert!(
        grammar
            .rule("import")
            .expect("named import rule")
            .fields
            .is_empty()
    );

    // Unrelated anonymous operators still materialize when referenced (C-style).
    let plus = grammar
        .rule("+")
        .expect("referenced anonymous operator must still materialize");
    assert!(plus.fields.is_empty());

    // StructuralModel uniqueness accepts the disambiguated rule set.
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout {
            vocab_size: 64,
            pad_token: 63,
            ranges: vec![GrammarStructuralTokenRange {
                id: "collision.structural".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 32,
            }],
            integer_value_range: None,
        },
    )
    .expect("disambiguated rules must lower without duplicate rule name");
    assert!(sidecar.proof.tokenizable_decisions_match);
}

/// Sidecar-only layout for the JSON text grammar pin (not a Go/layout pin).
fn tree_sitter_json_sidecar_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 577,
        pad_token: 576,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "tree_sitter_json.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 64,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_json.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 64,
                end_exclusive: 320,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_json.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 320,
                end_exclusive: 576,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "tree_sitter_json.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

/// Sidecar-only layout for the C grammar metadata pin (not a Go/layout pin or
/// C codec). Structural range is oversized for the larger named-node set.
fn tree_sitter_c_sidecar_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 1281,
        pad_token: 1280,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "tree_sitter_c.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 768,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_c.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 768,
                end_exclusive: 1024,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_c.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 1024,
                end_exclusive: 1280,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "tree_sitter_c.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

/// B′′ second-grammar pin: tree-sitter-javascript v0.25.0 metadata lower.
/// Claim boundary: pin + adapter GrammarIR + sidecar StructuralModel lower
/// only (not a full JS codec). CPU-5 T1 rule identity clears the former
/// `duplicate rule name` residual for named/anonymous `class`/`import`.
#[test]
fn tree_sitter_javascript_pin_lowers_to_grammar_ir() {
    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_javascript_v0_25_0");
    let (grammar, pin) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::javascript_fixture_dir())
        .unwrap();

    assert_eq!(pin.pin_id, "tree_sitter_javascript_node_types_v0.25.0");
    assert_eq!(pin.grammar_package, "tree-sitter-javascript");
    assert_eq!(pin.grammar_version, "0.25.0");
    assert_eq!(pin.license, "MIT");
    assert_eq!(
        pin.node_types_sha256,
        "188a8baa97018edaf63bf7ece372c07bc0d74d1301004bc8f211eb7dee5a6e70"
    );
    assert!(
        TreeSitterGrammarPin::javascript_fixture_dir()
            .join(pin.license_file.as_deref().unwrap_or("LICENSE"))
            .is_file()
    );

    assert_eq!(grammar.grammar_id, "tree_sitter_javascript_v0_25_0");
    assert_eq!(grammar.provenance.source_kind, "tree_sitter_node_types");
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(pin.node_types_sha256.as_str())
    );
    assert!(
        grammar
            .provenance
            .metadata_path
            .as_ref()
            .is_some_and(|path| path.ends_with("pin.json"))
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "tree-sitter");
    assert_eq!(parser.package.as_deref(), Some("tree-sitter-javascript"));
    assert_eq!(parser.package_version.as_deref(), Some("0.25.0"));

    // Named exports plus materialized anonymous operator leaves (same adapter
    // policy as the C pin) — metadata lower only, not a JS codec claim.
    // CPU-5 T1: named/anonymous collisions (`class`, `import`) emit one rule
    // each (prefer named), so 176 − 2 = 174.
    assert_eq!(grammar.rules.len(), 174);
    let rule_names: Vec<_> = grammar
        .rules
        .iter()
        .map(|rule| rule.name.as_str())
        .collect();
    let mut sorted_names = rule_names.clone();
    sorted_names.sort_unstable();
    let mut unique_names = sorted_names.clone();
    unique_names.dedup();
    assert_eq!(
        sorted_names, unique_names,
        "JS pin must not emit duplicate rule names after T1"
    );
    assert_eq!(
        grammar
            .rules
            .iter()
            .filter(|rule| rule.name == "class" || rule.name == "import")
            .count(),
        2,
        "class and import each appear exactly once"
    );
    // Named `class` keeps structural fields; named `import` is the field-less leaf.
    assert!(
        !grammar
            .rule("class")
            .expect("named class")
            .fields
            .is_empty(),
        "named class rule must retain structural fields"
    );
    assert!(
        grammar
            .rule("import")
            .expect("named import")
            .fields
            .is_empty(),
        "named import leaf must remain field-less"
    );
    assert!(grammar.rule("program").is_some());
    assert!(grammar.rule("statement").is_some());
    assert!(grammar.rule("expression").is_some());
    assert!(grammar.rule("function_declaration").is_some());
    assert!(grammar.rule("binary_expression").is_some());
    assert!(grammar.rule("identifier").is_some());
    // Anonymous operator kinds may materialize so StructuralModel refs resolve.
    assert!(grammar.rule("+").is_some() || grammar.rule("!=").is_some());

    // Supertype → synthetic subtype choice (structurally richer than JSON).
    let statement = grammar.rule("statement").unwrap();
    assert_eq!(statement.fields.len(), 1);
    assert_eq!(statement.fields[0].name, SUBTYPE_FIELD_NAME);
    assert!(statement.fields[0].required);
    assert!(!statement.fields[0].multiple);
    let statement_subtypes: Vec<_> = statement.fields[0]
        .alternatives
        .iter()
        .map(|alternative| alternative.node_kind.as_str())
        .collect();
    assert_eq!(
        statement_subtypes,
        vec![
            "break_statement",
            "continue_statement",
            "debugger_statement",
            "declaration",
            "do_statement",
            "empty_statement",
            "export_statement",
            "expression_statement",
            "for_in_statement",
            "for_statement",
            "if_statement",
            "import_statement",
            "labeled_statement",
            "return_statement",
            "statement_block",
            "switch_statement",
            "throw_statement",
            "try_statement",
            "while_statement",
            "with_statement",
        ]
    );
    assert!(
        statement.fields[0]
            .alternatives
            .iter()
            .all(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal)
    );

    // Field-less children on the root (hash_bang_line | statement list).
    let program = grammar.rule("program").unwrap();
    assert_eq!(program.fields.len(), 1);
    assert_eq!(program.fields[0].name, CHILDREN_FIELD_NAME);
    assert!(!program.fields[0].required);
    assert!(program.fields[0].multiple);
    let program_child_kinds: Vec<_> = program.fields[0]
        .alternatives
        .iter()
        .map(|alternative| alternative.node_kind.as_str())
        .collect();
    assert_eq!(program_child_kinds, vec!["hash_bang_line", "statement"]);
    assert_eq!(
        program.fields[0]
            .alternatives
            .iter()
            .find(|alternative| alternative.node_kind == "statement")
            .unwrap()
            .terminal,
        GrammarTerminalKind::Nonterminal
    );

    // Named fields on a declaration form.
    let function_declaration = grammar.rule("function_declaration").unwrap();
    assert_eq!(function_declaration.fields.len(), 3);
    let body = function_declaration
        .fields
        .iter()
        .find(|field| field.name == "body")
        .unwrap();
    assert!(body.required);
    assert_eq!(body.alternatives[0].node_kind, "statement_block");
    assert_eq!(
        body.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let name = function_declaration
        .fields
        .iter()
        .find(|field| field.name == "name")
        .unwrap();
    assert_eq!(name.alternatives[0].node_kind, "identifier");
    assert_eq!(
        name.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let parameters = function_declaration
        .fields
        .iter()
        .find(|field| field.name == "parameters")
        .unwrap();
    assert_eq!(parameters.alternatives[0].node_kind, "formal_parameters");
    assert_eq!(
        parameters.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    // binary_expression keeps named fields; operator alternatives include
    // anonymous punctuation kinds that are not separate rules.
    let binary = grammar.rule("binary_expression").unwrap();
    let left = binary
        .fields
        .iter()
        .find(|field| field.name == "left")
        .unwrap();
    assert!(left.required);
    assert!(
        left.alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "expression"
                && alternative.terminal == GrammarTerminalKind::Nonterminal)
    );
    let operator = binary
        .fields
        .iter()
        .find(|field| field.name == "operator")
        .unwrap();
    assert!(operator.required);
    assert!(
        operator
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "!=")
    );
    assert!(
        operator
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "+")
    );

    // Payload leaves stay field-less; composite string stays a nonterminal rule.
    assert!(grammar.rule("number").unwrap().fields.is_empty());
    assert!(grammar.rule("identifier").unwrap().fields.is_empty());
    assert!(grammar.rule("true").unwrap().fields.is_empty());
    let string = grammar.rule("string").unwrap();
    assert_eq!(string.fields[0].name, CHILDREN_FIELD_NAME);

    // CPU-5 T1: the only full-pin StructuralModel residual was
    // `duplicate rule name` for named/anonymous `class` and `import`. Preferring
    // the named rule clears that residual; named leaves were already emitted as
    // field-less rules, so references resolve without a separate T2 code path.
    // (Pinned negative residual intentionally inverted — SM lower now succeeds.)
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        tree_sitter_javascript_sidecar_token_layout(),
    )
    .expect("JS full-pin StructuralModel lower after T1 rule-identity fix");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
    assert_eq!(sidecar.proof.grammar_shape.rules, 174);
}

/// Sidecar-only layout for the JavaScript pin (B′′). Larger structural band
/// than JSON; not a production JS tokenizer layout or Go pin.
fn tree_sitter_javascript_sidecar_token_layout() -> GrammarStructuralTokenLayout {
    // Mirror the JSON sidecar shape: payload bands then PAD at VOCAB_SIZE-1.
    GrammarStructuralTokenLayout {
        vocab_size: 2049,
        pad_token: 2048,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "tree_sitter_javascript.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 1536,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_javascript.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 1536,
                end_exclusive: 1792,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_javascript.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 1792,
                end_exclusive: 2048,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "tree_sitter_javascript.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

/// CPU-5 T5 held-out pin proof: tree-sitter-rust metadata lower with **zero**
/// language-keyed Rust classification branches and **zero** terminal_classes
/// sidecar entries (Tier A). Fixture-path plumbing is allowed; a match arm or
/// kind table entry keyed to this grammar is a Failure, not something to patch.
#[test]
fn tree_sitter_rust_held_out_pin_lowers_without_language_keyed_rust() {
    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_rust_v0_24_0");
    let (grammar, pin) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::rust_fixture_dir())
        .expect(
            "Tier A: held-out rust pin must lower to GrammarIR without new classification Rust",
        );

    assert_eq!(pin.pin_id, "tree_sitter_rust_node_types_v0.24.0");
    assert_eq!(pin.grammar_package, "tree-sitter-rust");
    assert_eq!(pin.grammar_version, "0.24.0");
    assert_eq!(
        pin.node_types_sha256,
        "7aa048da68d64014402935bec749d57e05b1fd0d7fee1d111edfffc537342e1b"
    );
    // Tier A requires zero terminal_classes sidecar entries on the pin.
    assert!(
        pin.terminal_classes_file.is_none() && pin.terminal_classes_sha256.is_none(),
        "Tier A forbids terminal_classes sidecar on the held-out pin"
    );
    assert_eq!(grammar.grammar_id, "tree_sitter_rust_v0_24_0");
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(pin.node_types_sha256.as_str())
    );

    // Named/anonymous collisions (`block`, `lifetime`) must not produce
    // duplicate rule names (T1 policy is language-agnostic).
    let rule_names: Vec<_> = grammar
        .rules
        .iter()
        .map(|rule| rule.name.as_str())
        .collect();
    let mut sorted_names = rule_names.clone();
    sorted_names.sort_unstable();
    let mut unique_names = sorted_names.clone();
    unique_names.dedup();
    assert_eq!(
        sorted_names, unique_names,
        "held-out pin must not emit duplicate rule names: {rule_names:?}"
    );
    assert!(
        grammar.rules.len() > 100,
        "expected full named-node export, got {} rules",
        grammar.rules.len()
    );
    assert!(grammar.rule("source_file").is_some() || grammar.rule("function_item").is_some());

    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        tree_sitter_rust_held_out_sidecar_token_layout(),
    )
    .expect(
        "Tier A: held-out pin must lower GrammarIR → StructuralModel with zero sidecar class \
         entries and zero language-keyed classification Rust",
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
    assert_eq!(
        sidecar.proof.grammar_shape.rules,
        grammar.rules.len(),
        "StructuralModel record count must match held-out GrammarIR rules"
    );
}

/// Oversized structural band for the held-out rust pin only — token-layout
/// plumbing for SM lower, not a production layout or classification table.
fn tree_sitter_rust_held_out_sidecar_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 4097,
        pad_token: 4096,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 3072,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 3072,
                end_exclusive: 3328,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 3328,
                end_exclusive: 4096,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "tree_sitter_rust.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

/// Claim boundary for
/// `generic_codec_layer_held_out_rust_constructed_tree_v0` (tranche 10 of
/// `docs/generic-codec-layer-contract.md`). Constructed-tree encode/decode only;
/// not parse, render, reparse, source round-trip, production route, rust layout
/// pin stability, or any paper metric.
const GENERIC_CODEC_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_ID: &str =
    "generic_codec_layer_held_out_rust_constructed_tree_v0";
const GENERIC_CODEC_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_BOUNDARY: &str =
    "syntax_level_generic_encode_decode_constructed_tree_only";

/// Rust-specific kind/rule names that must **not** appear as language-keyed
/// tables in the generic encode implementation. The proof test may reference
/// rust fixture **paths** only; encode modules must stay language-neutral.
const GENERIC_ENCODE_FORBIDDEN_RUST_RULE_MARKERS: &[&str] = &[
    "function_item",
    "source_file",
    "impl_item",
    "let_declaration",
    "struct_item",
    "tree_sitter_rust",
    "boolean_literal",
    "empty_statement",
    "unit_expression",
    "unit_type",
];

/// `ModelFacts` / `CompiledSyntaxModel` root the first StructuralModel record.
/// The held-out full pin's first record is a field-less operator leaf, so
/// exercising fields/payloads/choices requires re-rooting a present record to
/// index 0. This is test plumbing only — not language-keyed encode Rust and not
/// a production root policy.
fn reroot_structural_model_sidecar(
    mut sidecar: GrammarStructuralModelSidecar,
    root_rule: &str,
) -> GrammarStructuralModelSidecar {
    let idx = sidecar
        .structural_model
        .records
        .iter()
        .position(|record| record.grammar_rule == root_rule)
        .unwrap_or_else(|| {
            panic!("held-out StructuralModel must contain record `{root_rule}` for re-root")
        });
    let record = sidecar.structural_model.records.remove(idx);
    sidecar.structural_model.records.insert(0, record);
    sidecar
}

/// Stream ↔ tree inverse identities for a constructed held-out tree under the
/// generic encode/decode claim boundary.
fn assert_held_out_constructed_tree_inverses(
    model: &CompiledSyntaxModel,
    tree: &RawSyntaxDecodedNode,
    witness: &str,
) {
    let tokens = model.encode_complete_tree(tree).unwrap_or_else(|error| {
        panic!(
            "encode_complete_tree must succeed for constructed held-out tree \
             ({witness}, rule={}, node_kind={}): {error:?}",
            tree.rule_name, tree.node_kind
        )
    });
    assert!(
        !tokens.is_empty(),
        "encoded held-out stream must be non-empty ({witness})"
    );

    let decoded = model.decode_complete_tree(&tokens).unwrap_or_else(|error| {
        panic!("decode_complete_tree must accept encoded held-out stream ({witness}): {error:?}")
    });
    let tree2 = decoded.decoded_node_tree().unwrap_or_else(|error| {
        panic!("decoded_node_tree must materialize for held-out stream ({witness}): {error:?}")
    });
    assert_eq!(
        tree2, *tree,
        "decode(encode(tree)) must equal the constructed tree ({witness}; \
         {GENERIC_CODEC_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_BOUNDARY})"
    );

    let reencoded = model.encode_complete_tree(&tree2).unwrap_or_else(|error| {
        panic!("re-encode of decoded held-out tree must succeed ({witness}): {error:?}")
    });
    assert_eq!(
        reencoded, tokens,
        "encode(decode(tokens)) must equal tokens ({witness}; \
         {GENERIC_CODEC_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_BOUNDARY})"
    );
}

/// Held-out proof (tranche 10, strengthened post-11): pin → GrammarIR →
/// StructuralModel → `CompiledSyntaxModel`, then construct `RawSyntaxDecodedNode`
/// trees (fixture data — **not** parsed source) that exercise fields, multi-alt
/// choice, nesting, and homogeneous payload on the held-out grammar. Assert
/// generic encode/decode inverse identities. Zero language-keyed encode Rust.
#[test]
fn tree_sitter_rust_held_out_constructed_tree_generic_encode_decode() {
    assert_eq!(
        GENERIC_CODEC_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_ID,
        "generic_codec_layer_held_out_rust_constructed_tree_v0"
    );
    assert_eq!(
        GENERIC_CODEC_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_BOUNDARY,
        "syntax_level_generic_encode_decode_constructed_tree_only"
    );

    // Architecture guard: generic encode implementation modules contain no
    // rust-specific rule-name tables (fixture paths and rule names in this test
    // are allowed; they select present lowered records, not encode tables).
    assert_generic_encode_has_no_rust_rule_name_tables();

    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_rust_v0_24_0");
    let (grammar, pin) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::rust_fixture_dir())
        .expect("held-out rust pin must lower to GrammarIR without language-keyed classification");
    assert_eq!(pin.pin_id, "tree_sitter_rust_node_types_v0.24.0");
    assert!(
        pin.terminal_classes_file.is_none() && pin.terminal_classes_sha256.is_none(),
        "Tier A forbids terminal_classes sidecar on the held-out pin"
    );

    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        tree_sitter_rust_held_out_sidecar_token_layout(),
    )
    .expect("held-out rust GrammarIR must lower to StructuralModel");
    assert!(sidecar.proof.tokenizable_decisions_match);

    // Default pin root (first record) remains a field-less leaf — keep a minimal
    // encode/decode identity so the default root path stays covered.
    let default_model = CompiledSyntaxModel::from_sidecar(&sidecar).expect(
        "held-out rust StructuralModel must compile to CompiledSyntaxModel with zero \
         language-keyed encode/decode Rust",
    );
    let default_root = sidecar
        .structural_model
        .records
        .first()
        .expect("StructuralModel must contain at least one record");
    assert!(
        default_root.fields.is_empty(),
        "default held-out root is expected to stay field-less (operator leaf after \
         adapter sort); root was rule={} node_kind={} fields={:?}",
        default_root.grammar_rule,
        default_root.record_type,
        default_root
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
    );
    let field_less_tree = RawSyntaxDecodedNode {
        rule_name: default_root.grammar_rule.clone(),
        node_kind: default_root.record_type.clone(),
        fields: vec![],
    };
    assert_held_out_constructed_tree_inverses(
        &default_model,
        &field_less_tree,
        "default_field_less_root",
    );
    assert_eq!(
        default_model
            .encode_complete_tree(&field_less_tree)
            .expect("field-less encode"),
        vec![
            default_model.token_roles().node_start,
            default_model.token_roles().node_end
        ],
        "field-less constructed root must emit plain node_start/node_end only"
    );

    // --- Strengthened constructed trees (not parsed source) ---
    // Each witness re-roots a present lowered record so encode's root check
    // accepts a non-field-less fragment. Rule names here are fixture selection
    // only; encode modules remain language-neutral (architecture guard above).

    // 1) Nested multi-alt Nonterminal choice:
    //    source_file.children → _declaration_statement.subtype → empty_statement
    {
        let model = CompiledSyntaxModel::from_sidecar(&reroot_structural_model_sidecar(
            sidecar.clone(),
            "source_file",
        ))
        .expect("re-rooted source_file model must compile");
        let tree = RawSyntaxDecodedNode {
            rule_name: "source_file".to_string(),
            node_kind: "source_file".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "children".to_string(),
                values: vec![RawSyntaxDecodedValue::Node(Box::new(
                    RawSyntaxDecodedNode {
                        rule_name: "_declaration_statement".to_string(),
                        node_kind: "_declaration_statement".to_string(),
                        fields: vec![RawSyntaxDecodedField {
                            name: "subtype".to_string(),
                            values: vec![RawSyntaxDecodedValue::Node(Box::new(
                                RawSyntaxDecodedNode {
                                    rule_name: "empty_statement".to_string(),
                                    node_kind: "empty_statement".to_string(),
                                    fields: vec![],
                                },
                            ))],
                            missing: false,
                            repeated_empty: false,
                        }],
                    },
                ))],
                missing: false,
                repeated_empty: false,
            }],
        };
        assert_held_out_constructed_tree_inverses(
            &model,
            &tree,
            "nested_multi_alt_source_file_empty_statement",
        );
        // Stream must be strictly richer than node_start/node_end (fields + alts).
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("nested multi-alt encode");
        assert!(
            tokens.len() > 2,
            "nested multi-alt held-out tree must emit more than bare node brackets; \
             got {tokens:?}"
        );
        assert!(
            tokens.contains(&model.token_roles().field_start)
                && tokens.contains(&model.token_roles().field_end),
            "nested multi-alt held-out tree must emit field brackets; got {tokens:?}"
        );
    }

    // 2) Homogeneous multi-alt Nonterminal field with field-less leaf child:
    //    negative_literal.children → integer_literal
    {
        let model = CompiledSyntaxModel::from_sidecar(&reroot_structural_model_sidecar(
            sidecar.clone(),
            "negative_literal",
        ))
        .expect("re-rooted negative_literal model must compile");
        let tree = RawSyntaxDecodedNode {
            rule_name: "negative_literal".to_string(),
            node_kind: "negative_literal".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "children".to_string(),
                values: vec![RawSyntaxDecodedValue::Node(Box::new(
                    RawSyntaxDecodedNode {
                        rule_name: "integer_literal".to_string(),
                        node_kind: "integer_literal".to_string(),
                        fields: vec![],
                    },
                ))],
                missing: false,
                repeated_empty: false,
            }],
        };
        assert_held_out_constructed_tree_inverses(
            &model,
            &tree,
            "multi_alt_negative_literal_integer_literal",
        );
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("negative_literal encode");
        // Child open must use an alt-specific node_start (not plain node_start
        // for the leaf), then leaf node_end, field_end, root node_end.
        assert_eq!(
            tokens.len(),
            6,
            "negative_literal → integer_literal stream shape: root open, field_start, \
             alt child open, child end, field_end, root end; got {tokens:?}"
        );
        assert_ne!(
            tokens[2],
            model.token_roles().node_start,
            "multi-alt child open must use an alternative node_start token, not plain \
             node_start; tokens={tokens:?}"
        );
    }

    // 3) Homogeneous IdentifierPayload field on a re-rooted record:
    //    lifetime.children → identifier payload "a"
    {
        let model = CompiledSyntaxModel::from_sidecar(&reroot_structural_model_sidecar(
            sidecar.clone(),
            "lifetime",
        ))
        .expect("re-rooted lifetime model must compile");
        let tree = RawSyntaxDecodedNode {
            rule_name: "lifetime".to_string(),
            node_kind: "lifetime".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "children".to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "identifier".to_string(),
                    terminal: "identifier".to_string(),
                    payload: "a".to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        assert_held_out_constructed_tree_inverses(
            &model,
            &tree,
            "homogeneous_identifier_payload_lifetime",
        );
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("lifetime payload encode");
        let roles = model.token_roles();
        assert!(
            tokens.contains(&roles.terminal) && tokens.contains(&roles.payload_end),
            "lifetime payload witness must emit homogeneous terminal + payload_end; \
             got {tokens:?}"
        );
        assert!(
            tokens.iter().any(|token| {
                *token >= roles.utf8_range.start && *token < roles.utf8_range.end_exclusive
            }),
            "lifetime payload witness must emit utf8 payload bytes; got {tokens:?}"
        );
    }
}

/// Claim boundary for I5 checklist tranche 10
/// (`i5_shared_alphabet_held_out_rust_constructed_tree_v0`).
/// Constructed-tree encode/decode under SharedAlphabet only; not parse, render,
/// rust codec, corpus, production route, default pin edits, or paper metrics.
const I5_SHARED_ALPHABET_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_ID: &str =
    "i5_shared_alphabet_held_out_rust_constructed_tree_v0";

/// Stream ↔ tree inverse under SharedAlphabet for a constructed held-out tree.
/// Every emitted token must stay strictly below PAD and VOCAB (Decision 1).
fn assert_shared_alphabet_held_out_constructed_tree_inverses(
    model: &CompiledSyntaxModel,
    tree: &RawSyntaxDecodedNode,
    witness: &str,
) {
    assert_eq!(
        model.layout_mode(),
        SyntaxTokenLayoutMode::SharedAlphabet,
        "held-out SharedAlphabet proof requires SharedAlphabet layout mode ({witness})"
    );
    let roles = model.token_roles();
    let tokens = model.encode_complete_tree(tree).unwrap_or_else(|error| {
        panic!(
            "SharedAlphabet encode_complete_tree must succeed for constructed held-out tree \
             ({witness}, rule={}, node_kind={}): {error:?}",
            tree.rule_name, tree.node_kind
        )
    });
    assert!(
        !tokens.is_empty(),
        "encoded SharedAlphabet held-out stream must be non-empty ({witness})"
    );
    for &token in &tokens {
        assert!(
            token < roles.pad_token && token < roles.vocab_size,
            "token {token} must stay below PAD={} / VOCAB={} under shared layout ({witness})",
            roles.pad_token,
            roles.vocab_size
        );
    }

    let decoded = model.decode_complete_tree(&tokens).unwrap_or_else(|error| {
        panic!(
            "SharedAlphabet decode_complete_tree must accept encoded held-out stream \
             ({witness}): {error:?}"
        )
    });
    let tree2 = decoded.decoded_node_tree().unwrap_or_else(|error| {
        panic!(
            "decoded_node_tree must materialize for SharedAlphabet held-out stream \
             ({witness}): {error:?}"
        )
    });
    assert_eq!(
        tree2, *tree,
        "decode(encode(tree)) must equal the constructed tree under SharedAlphabet ({witness}; \
         {SHARED_ALPHABET_CLAIM_BOUNDARY})"
    );

    let reencoded = model.encode_complete_tree(&tree2).unwrap_or_else(|error| {
        panic!(
            "SharedAlphabet re-encode of decoded held-out tree must succeed ({witness}): {error:?}"
        )
    });
    assert_eq!(
        reencoded, tokens,
        "encode(decode(tokens)) must equal tokens under SharedAlphabet ({witness}; \
         {SHARED_ALPHABET_CLAIM_BOUNDARY})"
    );
}

/// I5 checklist tranche 10: held-out tree-sitter-rust constructed trees under
/// SharedAlphabet (opt-in). Reuses the generic-codec held-out fixture path and
/// constructed witnesses; compiles with `SyntaxTokenLayoutMode::SharedAlphabet`.
/// No rust codec, no corpus, no default absolute pin edits (Decision 4).
/// Claim id: `i5_shared_alphabet_held_out_rust_constructed_tree_v0`.
/// Boundary: `compiler_shared_alphabet_layout_only`.
#[test]
fn tree_sitter_rust_held_out_constructed_tree_shared_alphabet_encode_decode() {
    assert_eq!(
        I5_SHARED_ALPHABET_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_ID,
        "i5_shared_alphabet_held_out_rust_constructed_tree_v0"
    );
    assert_eq!(
        SHARED_ALPHABET_CLAIM_BOUNDARY,
        "compiler_shared_alphabet_layout_only"
    );
    assert_generic_encode_has_no_rust_rule_name_tables();

    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_rust_v0_24_0");
    let (grammar, pin) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::rust_fixture_dir())
        .expect("held-out rust pin must lower to GrammarIR without language-keyed classification");
    assert_eq!(pin.pin_id, "tree_sitter_rust_node_types_v0.24.0");
    assert!(
        pin.terminal_classes_file.is_none() && pin.terminal_classes_sha256.is_none(),
        "Tier A forbids terminal_classes sidecar on the held-out pin"
    );

    // Sidecar still carries the oversized PerGrammar plumbing layout used for
    // SM lower; SharedAlphabet role assignment replaces ranges at compile time.
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        tree_sitter_rust_held_out_sidecar_token_layout(),
    )
    .expect("held-out rust GrammarIR must lower to StructuralModel");
    assert!(sidecar.proof.tokenizable_decisions_match);

    let default_model = CompiledSyntaxModel::from_sidecar_with_mode(
        &sidecar,
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect(
        "held-out rust StructuralModel must compile under SharedAlphabet with zero \
         language-keyed encode/decode Rust",
    );
    assert_eq!(
        default_model.layout_mode(),
        SyntaxTokenLayoutMode::SharedAlphabet
    );
    let roles = default_model.token_roles();
    assert_eq!(roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
    assert_eq!(roles.pad_token, SHARED_ALPHABET_VOCAB_SIZE - 1);

    // Default pin root remains a field-less leaf under SharedAlphabet.
    let default_root = sidecar
        .structural_model
        .records
        .first()
        .expect("StructuralModel must contain at least one record");
    assert!(
        default_root.fields.is_empty(),
        "default held-out root is expected to stay field-less (operator leaf after \
         adapter sort); root was rule={} node_kind={} fields={:?}",
        default_root.grammar_rule,
        default_root.record_type,
        default_root
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
    );
    let field_less_tree = RawSyntaxDecodedNode {
        rule_name: default_root.grammar_rule.clone(),
        node_kind: default_root.record_type.clone(),
        fields: vec![],
    };
    assert_shared_alphabet_held_out_constructed_tree_inverses(
        &default_model,
        &field_less_tree,
        "default_field_less_root_shared_alphabet",
    );
    assert_eq!(
        default_model
            .encode_complete_tree(&field_less_tree)
            .expect("field-less SharedAlphabet encode"),
        vec![roles.node_start, roles.node_end],
        "field-less constructed root under SharedAlphabet must emit plain node_start/node_end only"
    );

    // 1) Nested multi-alt Nonterminal choice under SharedAlphabet:
    //    source_file.children → _declaration_statement.subtype → empty_statement
    {
        let model = CompiledSyntaxModel::from_sidecar_with_mode(
            &reroot_structural_model_sidecar(sidecar.clone(), "source_file"),
            SyntaxTokenLayoutMode::SharedAlphabet,
        )
        .expect("re-rooted source_file model must compile under SharedAlphabet");
        let tree = RawSyntaxDecodedNode {
            rule_name: "source_file".to_string(),
            node_kind: "source_file".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "children".to_string(),
                values: vec![RawSyntaxDecodedValue::Node(Box::new(
                    RawSyntaxDecodedNode {
                        rule_name: "_declaration_statement".to_string(),
                        node_kind: "_declaration_statement".to_string(),
                        fields: vec![RawSyntaxDecodedField {
                            name: "subtype".to_string(),
                            values: vec![RawSyntaxDecodedValue::Node(Box::new(
                                RawSyntaxDecodedNode {
                                    rule_name: "empty_statement".to_string(),
                                    node_kind: "empty_statement".to_string(),
                                    fields: vec![],
                                },
                            ))],
                            missing: false,
                            repeated_empty: false,
                        }],
                    },
                ))],
                missing: false,
                repeated_empty: false,
            }],
        };
        assert_shared_alphabet_held_out_constructed_tree_inverses(
            &model,
            &tree,
            "nested_multi_alt_source_file_empty_statement_shared_alphabet",
        );
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("nested multi-alt SharedAlphabet encode");
        assert!(
            tokens.len() > 2,
            "nested multi-alt held-out tree under SharedAlphabet must emit more than bare \
             node brackets; got {tokens:?}"
        );
        assert!(
            tokens.contains(&model.token_roles().field_start)
                && tokens.contains(&model.token_roles().field_end),
            "nested multi-alt held-out tree under SharedAlphabet must emit field brackets; \
             got {tokens:?}"
        );
        // source_file.children → _declaration_statement uses a multi-alt CHOICE.
        let children_choice = model
            .token_roles()
            .alternative_token_for_field("source_file", "children", "_declaration_statement")
            .expect(
                "SharedAlphabet must assign CHOICE for source_file.children/_declaration_statement",
            );
        assert!(
            tokens.contains(&children_choice),
            "source_file.children multi-alt open must emit field-local CHOICE token \
             {children_choice}; tokens={tokens:?}"
        );
        let empty_choice = model
            .token_roles()
            .alternative_token_for_field("_declaration_statement", "subtype", "empty_statement")
            .expect("SharedAlphabet must assign CHOICE for empty_statement subtype");
        assert!(
            tokens.contains(&empty_choice),
            "empty_statement multi-alt open must emit field-local CHOICE token \
             {empty_choice}; tokens={tokens:?}"
        );
    }

    // 2) Homogeneous multi-alt field: negative_literal.children → integer_literal
    {
        let model = CompiledSyntaxModel::from_sidecar_with_mode(
            &reroot_structural_model_sidecar(sidecar.clone(), "negative_literal"),
            SyntaxTokenLayoutMode::SharedAlphabet,
        )
        .expect("re-rooted negative_literal model must compile under SharedAlphabet");
        let tree = RawSyntaxDecodedNode {
            rule_name: "negative_literal".to_string(),
            node_kind: "negative_literal".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "children".to_string(),
                values: vec![RawSyntaxDecodedValue::Node(Box::new(
                    RawSyntaxDecodedNode {
                        rule_name: "integer_literal".to_string(),
                        node_kind: "integer_literal".to_string(),
                        fields: vec![],
                    },
                ))],
                missing: false,
                repeated_empty: false,
            }],
        };
        assert_shared_alphabet_held_out_constructed_tree_inverses(
            &model,
            &tree,
            "multi_alt_negative_literal_integer_literal_shared_alphabet",
        );
        let roles = model.token_roles();
        let integer_choice = roles
            .alternative_token_for_field("negative_literal", "children", "integer_literal")
            .expect("SharedAlphabet must assign CHOICE for integer_literal child");
        // Domain-stable order for negative_literal.children (arity 2):
        // float_literal, integer_literal → integer_literal is ordinal 1.
        let expected = shared_alphabet_choice_token_id(roles.structural_range.start, 2, 1)
            .expect("CHOICE(2,1) must pack");
        assert_eq!(
            integer_choice, expected,
            "integer_literal under negative_literal.children must be CHOICE(2,1)"
        );
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("negative_literal SharedAlphabet encode");
        assert_eq!(
            tokens.len(),
            6,
            "negative_literal → integer_literal SharedAlphabet stream shape: root open, \
             field_start, CHOICE child open, child end, field_end, root end; got {tokens:?}"
        );
        assert_eq!(
            tokens[2], integer_choice,
            "multi-alt child open under SharedAlphabet must emit CHOICE(2,1)={integer_choice}; \
             tokens={tokens:?}"
        );
        assert_ne!(
            tokens[2], roles.node_start,
            "multi-alt child open must not use plain node_start under SharedAlphabet; \
             tokens={tokens:?}"
        );

        // Shared mode is a second layout: absolute stream must not match the
        // PerGrammar free-slot encoding of the same constructed tree.
        let per_grammar = CompiledSyntaxModel::from_sidecar(&reroot_structural_model_sidecar(
            sidecar.clone(),
            "negative_literal",
        ))
        .expect("PerGrammar negative_literal model must still compile");
        let per_grammar_tokens = per_grammar
            .encode_complete_tree(&tree)
            .expect("PerGrammar negative_literal encode");
        assert_ne!(
            tokens.as_slice(),
            per_grammar_tokens.as_slice(),
            "SharedAlphabet multi-alt stream must differ from PerGrammar absolute tokens \
             (Decision 4 second mode; no pin edit)"
        );
    }

    // 3) Homogeneous IdentifierPayload under shared utf8 band:
    //    lifetime.children → identifier payload "a"
    {
        let model = CompiledSyntaxModel::from_sidecar_with_mode(
            &reroot_structural_model_sidecar(sidecar.clone(), "lifetime"),
            SyntaxTokenLayoutMode::SharedAlphabet,
        )
        .expect("re-rooted lifetime model must compile under SharedAlphabet");
        let tree = RawSyntaxDecodedNode {
            rule_name: "lifetime".to_string(),
            node_kind: "lifetime".to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: "children".to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "identifier".to_string(),
                    terminal: "identifier".to_string(),
                    payload: "a".to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        assert_shared_alphabet_held_out_constructed_tree_inverses(
            &model,
            &tree,
            "homogeneous_identifier_payload_lifetime_shared_alphabet",
        );
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("lifetime payload SharedAlphabet encode");
        let roles = model.token_roles();
        assert!(
            tokens.contains(&roles.terminal) && tokens.contains(&roles.payload_end),
            "lifetime payload witness under SharedAlphabet must emit terminal + payload_end; \
             got {tokens:?}"
        );
        assert!(
            tokens.iter().any(|token| {
                *token >= roles.utf8_range.start && *token < roles.utf8_range.end_exclusive
            }),
            "lifetime payload witness under SharedAlphabet must emit utf8 payload bytes in the \
             shared band [{}, {}); got {tokens:?}",
            roles.utf8_range.start,
            roles.utf8_range.end_exclusive
        );
        // Shared utf8 band starts after the structural CHOICE band (not the
        // oversized PerGrammar held-out band at 3072).
        assert_eq!(
            roles.utf8_range.start, roles.structural_range.end_exclusive,
            "SharedAlphabet utf8 band must abut structural band"
        );
        assert!(
            roles.utf8_range.start < 3072,
            "SharedAlphabet utf8 base must not use the PerGrammar held-out band start 3072; \
             got start={}",
            roles.utf8_range.start
        );
    }

    // Default PerGrammar compile path remains available and is not SharedAlphabet
    // (Decision 4 pin safety — no silent rewrite of default mode).
    let per_grammar_default = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("held-out rust StructuralModel must still compile under default PerGrammar mode");
    assert_eq!(
        per_grammar_default.layout_mode(),
        SyntaxTokenLayoutMode::PerGrammar,
        "default from_sidecar must remain PerGrammar (Decision 4)"
    );
}

/// Grep-style architecture guard: production generic-encode modules must not
/// embed rust rule-name allowlists/tables. Unit-test modules after `#[cfg(test)]`
/// are excluded so synthetic fixtures do not false-positive.
fn assert_generic_encode_has_no_rust_rule_name_tables() {
    const ENCODE_MODULES: &[&str] = &[
        "src/syntax_state_machine/encode.rs",
        "src/syntax_state_machine.rs",
        "src/tree_sitter_subset_framework.rs",
    ];
    let mut violations = Vec::new();
    for path in ENCODE_MODULES {
        let source = fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("architecture guard failed to read {path}: {error}");
        });
        let production = source
            .split_once("#[cfg(test)]")
            .map_or(source.as_str(), |(prod, _tests)| prod);
        for marker in GENERIC_ENCODE_FORBIDDEN_RUST_RULE_MARKERS {
            if production.contains(marker) {
                violations.push(format!(
                    "{path} production code contains rust-specific marker `{marker}`"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "generic encode must have zero rust-specific rule-name tables \
         ({GENERIC_CODEC_HELD_OUT_RUST_CONSTRUCTED_TREE_CLAIM_ID}):\n{}",
        violations.join("\n")
    );
}
