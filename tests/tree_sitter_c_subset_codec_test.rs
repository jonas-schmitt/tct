//! MG-6 tree-sitter-C construct-subset codec tests.
//!
//! Pin inventory, subset GrammarIR / StructuralModel projection, and raw SM
//! encode/decode/render round-trip for the minimal free-function construct set.
//!
//! Claim boundary: `syntax_level_tree_sitter_c_subset_only` only.

use tct::grammar_ir::GrammarTerminalKind;
use tct::grammar_ir_structural_model::lower_grammar_ir_to_structural_model;
use tct::shared_alphabet_layout::{
    SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
    shared_alphabet_choice_token_id,
};
use tct::syntax_state_machine::CompiledSyntaxModel;
use tct::tree_sitter_c_subset_codec::{
    CLAIM_BOUNDARY, CSubsetPrimitiveType, CSubsetProgram, MIXED_PAYLOAD_RISKS, SUBSET_GRAMMAR_ID,
    SUBSET_ID, SUBSET_PIN_RULE_NAMES, TreeSitterCSubsetCodec, inventory_pinned_c_subset_fixture,
    program_to_decoded_node, subset_token_layout,
};
use tct::tree_sitter_metadata::{CHILDREN_FIELD_NAME, SUBTYPE_FIELD_NAME, TreeSitterGrammarPin};

#[test]
fn tree_sitter_c_subset_inventory_matches_pin_and_claim_boundary() {
    let inventory = inventory_pinned_c_subset_fixture().expect("C subset pin inventory");

    assert_eq!(inventory.pin_id, "tree_sitter_c_node_types_v0.24.2");
    assert_eq!(inventory.grammar_package, "tree-sitter-c");
    assert_eq!(inventory.grammar_version, "0.24.2");
    assert_eq!(
        inventory.node_types_sha256,
        "23e819ef1eefd357bb6eba844f47f8492f6a88da3e5046b06ec4acd4da8a9fd6"
    );
    assert!(
        TreeSitterGrammarPin::c_fixture_dir()
            .join("node-types.json")
            .is_file()
    );
    assert!(
        inventory.full_rule_count > 100,
        "full C pin must stay large; inventory is not a full-language codec"
    );

    assert_eq!(inventory.subset_id, SUBSET_ID);
    assert_eq!(inventory.subset_id, SUBSET_GRAMMAR_ID);
    assert_eq!(inventory.claim_boundary, CLAIM_BOUNDARY);
    assert_eq!(CLAIM_BOUNDARY, "syntax_level_tree_sitter_c_subset_only");
    assert_eq!(SUBSET_ID, "tree_sitter_c_minimal_function_subset_v0");

    assert_eq!(
        inventory.subset_pin_rules_present,
        SUBSET_PIN_RULE_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>()
    );
    for required in [
        "translation_unit",
        "function_definition",
        "primitive_type",
        "function_declarator",
        "identifier",
        "parameter_list",
        "parameter_declaration",
        "compound_statement",
        "return_statement",
        "number_literal",
    ] {
        assert!(
            inventory
                .subset_pin_rules_present
                .iter()
                .any(|name| name == required),
            "missing subset pin rule {required}"
        );
    }
}

#[test]
fn tree_sitter_c_subset_inventory_records_mixed_payload_risks() {
    let inventory = inventory_pinned_c_subset_fixture().expect("C subset pin inventory");

    assert_eq!(MIXED_PAYLOAD_RISKS.len(), 2);
    assert_eq!(
        MIXED_PAYLOAD_RISKS[0].owner_rule, "_declarator",
        "identifier is IdentifierPayload on _declarator.subtype in the full pin"
    );
    assert_eq!(MIXED_PAYLOAD_RISKS[0].owner_field, SUBTYPE_FIELD_NAME);
    assert_eq!(
        MIXED_PAYLOAD_RISKS[1].owner_rule, "expression",
        "number_literal is IntegerPayload on expression.subtype in the full pin"
    );

    assert_eq!(inventory.mixed_payload_risks_observed.len(), 2);
    assert!(
        inventory.mixed_payload_risks_observed[0].contains("_declarator"),
        "unexpected risk note: {}",
        inventory.mixed_payload_risks_observed[0]
    );
    assert!(
        inventory.mixed_payload_risks_observed[1].contains("expression"),
        "unexpected risk note: {}",
        inventory.mixed_payload_risks_observed[1]
    );
}

#[test]
fn tree_sitter_c_subset_v0_construct_set_is_closed() {
    for forbidden in [
        "pointer_declarator",
        "struct_specifier",
        "if_statement",
        "for_statement",
        "while_statement",
        "call_expression",
        "string_literal",
        "preproc_include",
        "binary_expression",
        "array_declarator",
    ] {
        assert!(
            !SUBSET_PIN_RULE_NAMES.contains(&forbidden),
            "{forbidden} is out of v0 and must not be in SUBSET_PIN_RULE_NAMES"
        );
    }
}

#[test]
fn tree_sitter_c_subset_grammar_and_structural_model_project() {
    let codec = TreeSitterCSubsetCodec::from_pinned_fixture().expect("C subset codec");

    assert_eq!(codec.pin().pin_id, "tree_sitter_c_node_types_v0.24.2");
    assert_eq!(
        codec.pin().node_types_sha256,
        "23e819ef1eefd357bb6eba844f47f8492f6a88da3e5046b06ec4acd4da8a9fd6"
    );
    assert_eq!(codec.grammar().grammar_id, SUBSET_GRAMMAR_ID);
    assert_eq!(
        codec.grammar().provenance.source_kind,
        "tree_sitter_node_types_subset"
    );
    assert_eq!(
        codec
            .grammar()
            .provenance
            .parser
            .as_ref()
            .unwrap()
            .package_version
            .as_deref(),
        Some("0.24.2")
    );

    let rule_names: Vec<_> = codec
        .grammar()
        .rules
        .iter()
        .map(|rule| rule.name.as_str())
        .collect();
    // Root must be first (ModelFacts::root_rule uses records.first()).
    assert_eq!(rule_names[0], "translation_unit");
    assert!(rule_names.contains(&"function_definition"));
    assert!(rule_names.contains(&"_declarator"));
    assert!(rule_names.contains(&"number_literal"));
    assert!(!rule_names.contains(&"pointer_declarator"));
    assert!(!rule_names.contains(&"if_statement"));

    let translation_unit = codec.grammar().rule("translation_unit").unwrap();
    assert_eq!(translation_unit.fields[0].name, CHILDREN_FIELD_NAME);
    assert!(translation_unit.fields[0].multiple);
    assert!(!translation_unit.fields[0].required);
    assert_eq!(
        translation_unit.fields[0].alternatives[0].node_kind,
        "function_definition"
    );

    let function_definition = codec.grammar().rule("function_definition").unwrap();
    let field_names: Vec<_> = function_definition
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(field_names, vec!["body", "declarator", "type"]);

    let declarator = codec.grammar().rule("_declarator").unwrap();
    let subtypes: Vec<_> = declarator.fields[0]
        .alternatives
        .iter()
        .map(|alternative| (alternative.node_kind.as_str(), alternative.terminal))
        .collect();
    assert_eq!(
        subtypes,
        vec![
            ("function_declarator", GrammarTerminalKind::Nonterminal),
            ("identifier", GrammarTerminalKind::Nonterminal),
        ]
    );

    let identifier = codec.grammar().rule("identifier").unwrap();
    assert_eq!(identifier.fields[0].name, "name");
    assert_eq!(
        identifier.fields[0].alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let number = codec.grammar().rule("number_literal").unwrap();
    assert_eq!(number.fields[0].name, "value");
    assert_eq!(
        number.fields[0].alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let expression = codec.grammar().rule("expression").unwrap();
    assert!(
        expression.fields[0]
            .alternatives
            .iter()
            .all(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal),
        "expression.subtype must stay all-Nonterminal after number_literal wrap"
    );

    let sidecar = codec.sidecar();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert_eq!(
        sidecar.structural_model.records[0].grammar_rule,
        "translation_unit"
    );

    // Sidecar layout is subset-scoped, not a Go pin.
    let layout = subset_token_layout();
    assert_eq!(layout.vocab_size, 577);
    assert_eq!(layout.pad_token, 576);
    let re_lower = lower_grammar_ir_to_structural_model(codec.grammar(), layout).unwrap();
    assert!(re_lower.proof.tokenizable_decisions_match);

    assert_eq!(codec.claim_boundary(), CLAIM_BOUNDARY);
    assert_eq!(SUBSET_ID, SUBSET_GRAMMAR_ID);
}

#[test]
fn tree_sitter_c_subset_main_return_zero_round_trips_with_raw_sm() {
    let codec = TreeSitterCSubsetCodec::from_pinned_fixture().expect("C subset codec");
    let source = "int main(void) {\n  return 0;\n}";
    let report = codec.round_trip_source(source).expect("round-trip");

    assert_eq!(report.rendered, source);
    assert_eq!(report.claim_boundary, CLAIM_BOUNDARY);
    assert_eq!(report.subset_id, SUBSET_ID);
    assert_eq!(report.pin_id, "tree_sitter_c_node_types_v0.24.2");
    assert!(
        report.raw_token_count >= 20,
        "main stream too short: {}",
        report.raw_token_count
    );

    let tree = codec.decode_raw(&report.raw_token_ids).unwrap();
    assert_eq!(tree.root_rule(), Some("translation_unit"));
    let decoded = codec.decode(&report.raw_token_ids).unwrap();
    assert_eq!(decoded.functions.len(), 1);
    assert_eq!(decoded.functions[0].name, "main");
    assert_eq!(decoded.functions[0].return_type, CSubsetPrimitiveType::Int);
    assert_eq!(
        decoded.functions[0].parameters[0].ty,
        CSubsetPrimitiveType::Void
    );
    assert_eq!(decoded.functions[0].returns[0].value, 0);
    assert_eq!(decoded.render(), source);
}

#[test]
fn tree_sitter_c_subset_second_fixture_and_whitespace_round_trip() {
    let codec = TreeSitterCSubsetCodec::from_pinned_fixture().expect("C subset codec");
    let source = "int f(void){return 42;}";
    let report = codec.round_trip_source(source).expect("round-trip");
    assert_eq!(report.rendered, "int f(void) {\n  return 42;\n}");
    let decoded = codec.decode(&report.raw_token_ids).unwrap();
    assert_eq!(decoded.functions[0].name, "f");
    assert_eq!(decoded.functions[0].returns[0].value, 42);
}

#[test]
fn tree_sitter_c_subset_rejects_out_of_subset_source() {
    assert!(CSubsetProgram::parse("int main(void) { return 256; }").is_err());
    assert!(CSubsetProgram::parse("int main(void) { return -1; }").is_err());
    assert!(CSubsetProgram::parse("void main(void) { return 0; }").is_err());
    assert!(CSubsetProgram::parse("int main(int x) { return 0; }").is_err());
    assert!(CSubsetProgram::parse("int *main(void) { return 0; }").is_err());
    assert!(CSubsetProgram::parse("int main(void) { if (1) return 0; }").is_err());
    assert!(CSubsetProgram::parse("#include <stdio.h>\nint main(void) { return 0; }").is_err());
    assert!(CSubsetProgram::parse("int main(void) { return 0; }\n").is_ok());
}

/// Assert absolute raw-token-ID pin + identity metadata + render round-trip.
///
/// Vectors were captured from a live `round_trip_source` run of this codec;
/// they are not hand-computed.
fn assert_c_absolute_token_pin(
    source_label: &str,
    report: &tct::tree_sitter_c_subset_codec::TreeSitterCSubsetRoundTrip,
    expected: &[u32],
    expected_render: &str,
) {
    assert_eq!(
        report.pin_id, "tree_sitter_c_node_types_v0.24.2",
        "{source_label}: pin_id drift"
    );
    assert_eq!(
        report.subset_id, SUBSET_ID,
        "{source_label}: subset_id drift"
    );
    assert_eq!(
        report.claim_boundary, CLAIM_BOUNDARY,
        "{source_label}: claim_boundary drift"
    );
    assert_eq!(
        report.raw_token_count,
        expected.len(),
        "{source_label}: raw_token_count mismatch"
    );
    assert_eq!(
        report.raw_token_ids.as_slice(),
        expected,
        "{source_label}: absolute raw_token_ids pin broken"
    );
    assert_eq!(
        report.rendered, expected_render,
        "{source_label}: render must match canonical form"
    );
}

/// Absolute token-ID golden pins for the C subset codec.
///
/// These pin ABSOLUTE token IDs (not mere encode/decode self-consistency).
/// Changing any expected vector is a `tct-token-diff` breaking change for any
/// downstream raw-token corpus or BPE artifact, and must not be updated merely
/// to make a refactor pass. See DEVELOPMENT.md ("Schema Evolution Reports")
/// for the layout-diff guidance (treat non-`exact_reuse` layout shifts like breaking changes).
#[test]
fn tree_sitter_c_subset_absolute_token_id_golden_pins() {
    let codec = TreeSitterCSubsetCodec::from_pinned_fixture().expect("C subset codec");

    // --- minimal free function: int f(void) { return 0; } ---
    // Pins ABSOLUTE token IDs; changing this is a tct-token-diff breaking change
    // for downstream raw-token corpora / BPE artifacts (see DEVELOPMENT.md ("Schema Evolution Reports")).
    const MINIMAL_GOLDEN: &[u32] = &[
        0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 6, 320, 7, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 2, 0,
        2, 10, 2, 0, 2, 11, 2, 6, 166, 7, 3, 1, 3, 1, 3, 2, 0, 2, 0, 2, 0, 2, 0, 2, 6, 182, 175,
        169, 164, 7, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 2, 0, 2, 0, 2, 6, 169, 174, 180, 7, 3,
        1, 3, 1, 3, 1, 3, 1,
    ];
    {
        let report = codec
            .round_trip_source("int f(void) { return 0; }")
            .expect("minimal round-trip");
        assert_c_absolute_token_pin(
            "minimal f/return-0",
            &report,
            MINIMAL_GOLDEN,
            "int f(void) {\n  return 0;\n}",
        );
        assert_eq!(
            codec.decode(&report.raw_token_ids).unwrap().render(),
            report.rendered
        );
    }

    // --- multiple return statements in one compound body ---
    // Pins ABSOLUTE token IDs; changing this is a tct-token-diff breaking change
    // for downstream raw-token corpora / BPE artifacts (see DEVELOPMENT.md ("Schema Evolution Reports")).
    const MULTI_RETURN_GOLDEN: &[u32] = &[
        0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 6, 321, 7, 3, 1, 3, 1, 3, 1, 3, 1, 0, 2, 0, 2, 0,
        2, 0, 2, 6, 322, 7, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 2, 0, 2, 10, 2, 0, 2, 11, 2, 6, 166,
        7, 3, 1, 3, 1, 3, 2, 0, 2, 0, 2, 0, 2, 0, 2, 6, 182, 175, 169, 164, 7, 3, 1, 3, 1, 3, 1, 3,
        1, 3, 1, 3, 1, 3, 2, 0, 2, 0, 2, 6, 169, 174, 180, 7, 3, 1, 3, 1, 3, 1, 3, 1,
    ];
    {
        let source = "int f(void) {\n  return 1;\n  return 2;\n}";
        let report = codec
            .round_trip_source(source)
            .expect("multi-return round-trip");
        assert_c_absolute_token_pin("multi-return f", &report, MULTI_RETURN_GOLDEN, source);
        let decoded = codec.decode(&report.raw_token_ids).unwrap();
        assert_eq!(decoded.functions[0].returns.len(), 2);
        assert_eq!(decoded.render(), source);
    }

    // --- two functions, different identifier lengths (main vs foo) ---
    // Pins ABSOLUTE token IDs; changing this is a tct-token-diff breaking change
    // for downstream raw-token corpora / BPE artifacts (see DEVELOPMENT.md ("Schema Evolution Reports")).
    const TWO_FUNCS_GOLDEN: &[u32] = &[
        0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 6, 320, 7, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 2, 0,
        2, 10, 2, 0, 2, 11, 2, 6, 173, 161, 169, 174, 7, 3, 1, 3, 1, 3, 2, 0, 2, 0, 2, 0, 2, 0, 2,
        6, 182, 175, 169, 164, 7, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 2, 0, 2, 0, 2, 6, 169,
        174, 180, 7, 3, 1, 3, 1, 3, 1, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 6, 327, 7, 3, 1, 3, 1,
        3, 1, 3, 1, 3, 1, 3, 2, 0, 2, 10, 2, 0, 2, 11, 2, 6, 166, 175, 175, 7, 3, 1, 3, 1, 3, 2, 0,
        2, 0, 2, 0, 2, 0, 2, 6, 182, 175, 169, 164, 7, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 2, 0,
        2, 0, 2, 6, 169, 174, 180, 7, 3, 1, 3, 1, 3, 1, 3, 1,
    ];
    {
        let source = "int main(void) {\n  return 0;\n}\nint foo(void) {\n  return 7;\n}";
        // Canonical render inserts a blank line between free functions.
        let expected_render = "int main(void) {\n  return 0;\n}\n\nint foo(void) {\n  return 7;\n}";
        let report = codec
            .round_trip_source(source)
            .expect("two-function round-trip");
        assert_c_absolute_token_pin("main+foo", &report, TWO_FUNCS_GOLDEN, expected_render);
        let decoded = codec.decode(&report.raw_token_ids).unwrap();
        assert_eq!(decoded.functions.len(), 2);
        assert_eq!(decoded.functions[0].name, "main");
        assert_eq!(decoded.functions[1].name, "foo");
        assert_eq!(decoded.render(), expected_render);
    }
}

#[test]
fn tree_sitter_c_subset_number_literal_terminal_is_exact_integer_payload() {
    // Empirical shape of the accepted encode→decode path: the strict decode
    // helper matches this exact pair, not a substring of payload kind.
    use tct::syntax_state_machine::RawSyntaxDecodedValue;

    let codec = TreeSitterCSubsetCodec::from_pinned_fixture().expect("C subset codec");
    let program = CSubsetProgram::parse("int main(void) { return 42; }").expect("parse");
    let tokens = codec.encode(&program).expect("encode");
    let tree = codec.decode_raw(&tokens).expect("decode_raw");
    let root = tree.decoded_node_tree().expect("node tree");

    fn find_number_literal(
        node: &tct::syntax_state_machine::RawSyntaxDecodedNode,
    ) -> Option<&tct::syntax_state_machine::RawSyntaxDecodedNode> {
        if node.node_kind == "number_literal" {
            return Some(node);
        }
        for field in &node.fields {
            for value in &field.values {
                if let RawSyntaxDecodedValue::Node(child) = value
                    && let Some(found) = find_number_literal(child)
                {
                    return Some(found);
                }
            }
        }
        None
    }

    let number = find_number_literal(&root).expect("number_literal node");
    assert_eq!(number.rule_name, "number_literal");
    assert_eq!(number.node_kind, "number_literal");
    let value_field = number.field("value").expect("value field");
    assert_eq!(value_field.values.len(), 1);
    match &value_field.values[0] {
        RawSyntaxDecodedValue::Terminal(terminal) => {
            assert_eq!(terminal.node_kind, "number_literal");
            assert_eq!(terminal.terminal, "integer");
            assert_eq!(terminal.payload, "42");
        }
        other => panic!("expected Terminal payload, got {other:?}"),
    }
    // rule_name and node_kind stay equal on every node for this subset stream.
    fn assert_rule_equals_kind(node: &tct::syntax_state_machine::RawSyntaxDecodedNode) {
        assert_eq!(
            node.rule_name, node.node_kind,
            "subset projection keeps rule_name == node_kind"
        );
        for field in &node.fields {
            for value in &field.values {
                if let RawSyntaxDecodedValue::Node(child) = value {
                    assert_rule_equals_kind(child);
                }
            }
        }
    }
    assert_rule_equals_kind(&root);
    // Public token path still round-trips after the tightened predicate.
    assert_eq!(
        codec.decode(&tokens).expect("decode").functions[0].returns[0].value,
        42
    );
}

/// I5 checklist tranche 9: C subset under SharedAlphabet (opt-in).
///
/// Program → decoded node → encode/decode inverse under the shared
/// decision-token alphabet. Default PerGrammar codec and absolute golden pins
/// stay on the historical path (Decision 4 — second selectable layout mode).
/// Claim boundary: `compiler_shared_alphabet_layout_only`.
#[test]
fn tree_sitter_c_subset_shared_alphabet_program_round_trip() {
    assert_eq!(
        SHARED_ALPHABET_CLAIM_BOUNDARY,
        "compiler_shared_alphabet_layout_only"
    );

    let codec = TreeSitterCSubsetCodec::from_pinned_fixture().expect("C subset codec");
    assert_eq!(
        codec.model().layout_mode(),
        SyntaxTokenLayoutMode::PerGrammar,
        "default C subset codec must remain PerGrammar"
    );

    let shared = CompiledSyntaxModel::from_sidecar_with_mode(
        codec.sidecar(),
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect("C subset should compile under SharedAlphabet");
    assert_eq!(shared.layout_mode(), SyntaxTokenLayoutMode::SharedAlphabet);
    let roles = shared.token_roles();
    assert_eq!(roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
    assert_eq!(roles.pad_token, SHARED_ALPHABET_VOCAB_SIZE - 1);

    // Domain-stable order for `_declarator.subtype` (arity 2):
    // function_declarator, identifier → function_declarator is ordinal 0.
    let function_declarator_choice = roles
        .alternative_token_for_field("_declarator", SUBTYPE_FIELD_NAME, "function_declarator")
        .expect("SharedAlphabet should assign CHOICE for function_declarator");
    let expected_function_declarator =
        shared_alphabet_choice_token_id(roles.structural_range.start, 2, 0)
            .expect("CHOICE(2,0) must pack");
    assert_eq!(
        function_declarator_choice, expected_function_declarator,
        "function_declarator must be CHOICE(2,0) under SharedAlphabet"
    );
    let identifier_choice = roles
        .alternative_token_for_field("_declarator", SUBTYPE_FIELD_NAME, "identifier")
        .expect("SharedAlphabet should assign CHOICE for identifier");
    let expected_identifier = shared_alphabet_choice_token_id(roles.structural_range.start, 2, 1)
        .expect("CHOICE(2,1) must pack");
    assert_eq!(
        identifier_choice, expected_identifier,
        "identifier must be CHOICE(2,1) under SharedAlphabet"
    );

    let sources = [
        "int f(void) { return 0; }",
        "int main(void) { return 42; }",
        "int f(void) {\n  return 1;\n  return 2;\n}",
        "int main(void) {\n  return 0;\n}\nint foo(void) {\n  return 7;\n}",
    ];

    for source in sources {
        let program = CSubsetProgram::parse(source)
            .unwrap_or_else(|error| panic!("parse failed for {source:?}: {error:?}"));
        let root = program_to_decoded_node(&program).unwrap_or_else(|error| {
            panic!("program_to_decoded_node failed for {source:?}: {error:?}")
        });
        let tokens = shared.encode_complete_tree(&root).unwrap_or_else(|error| {
            panic!("SharedAlphabet encode failed for {source:?}: {error:?}")
        });
        for &token in &tokens {
            assert!(
                token < roles.pad_token && token < roles.vocab_size,
                "token {token} must stay below PAD/VOCAB under shared layout for {source:?}"
            );
        }
        let decoded = shared
            .decode_complete_tree(&tokens)
            .unwrap_or_else(|error| {
                panic!("SharedAlphabet decode failed for {source:?}: {error:?}")
            });
        let tree2 = decoded
            .decoded_node_tree()
            .expect("decoded node tree should materialize under SharedAlphabet");
        assert_eq!(
            &tree2, &root,
            "decode(encode(tree)) must equal tree under SharedAlphabet for {source:?}"
        );
        let reencoded = shared
            .encode_complete_tree(&tree2)
            .expect("re-encode under SharedAlphabet should succeed");
        assert_eq!(
            reencoded, tokens,
            "encode(decode(tokens)) must equal tokens under SharedAlphabet for {source:?}"
        );
        assert!(
            tokens.contains(&function_declarator_choice),
            "stream should include CHOICE(2,0)={function_declarator_choice} for {source:?}: {tokens:?}"
        );
        assert!(
            tokens.contains(&identifier_choice),
            "stream should include CHOICE(2,1)={identifier_choice} for {source:?}: {tokens:?}"
        );
    }

    // Shared mode is a second layout: absolute streams must not match the
    // default PerGrammar golden for multi-alt declarators (free-slot 10/11).
    const MINIMAL_PER_GRAMMAR_GOLDEN: &[u32] = &[
        0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 6, 320, 7, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 2, 0,
        2, 10, 2, 0, 2, 11, 2, 6, 166, 7, 3, 1, 3, 1, 3, 2, 0, 2, 0, 2, 0, 2, 0, 2, 6, 182, 175,
        169, 164, 7, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 2, 0, 2, 0, 2, 6, 169, 174, 180, 7, 3,
        1, 3, 1, 3, 1, 3, 1,
    ];
    let minimal = CSubsetProgram::parse("int f(void) { return 0; }").expect("parse minimal");
    let minimal_shared = shared
        .encode_complete_tree(&program_to_decoded_node(&minimal).expect("lower minimal"))
        .expect("minimal under SharedAlphabet");
    assert_ne!(
        minimal_shared.as_slice(),
        MINIMAL_PER_GRAMMAR_GOLDEN,
        "SharedAlphabet minimal stream must differ from PerGrammar absolute golden"
    );
    assert!(
        minimal_shared.contains(&function_declarator_choice),
        "minimal SharedAlphabet stream should include CHOICE(2,0)={function_declarator_choice}: {minimal_shared:?}"
    );
    assert!(
        minimal_shared.contains(&identifier_choice),
        "minimal SharedAlphabet stream should include CHOICE(2,1)={identifier_choice}: {minimal_shared:?}"
    );

    // Default PerGrammar path remains bit-identical (Decision 4 pin safety).
    let minimal_per_grammar = codec.encode(&minimal).expect("PerGrammar encode");
    assert_eq!(
        minimal_per_grammar.as_slice(),
        MINIMAL_PER_GRAMMAR_GOLDEN,
        "PerGrammar minimal absolute pin must stay bit-identical without fixture edits"
    );
}
