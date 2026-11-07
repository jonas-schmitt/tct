use tct::grammar_ir::GrammarTerminalKind;
use tct::grammar_ir_structural_model::{
    ARTIFACT_KIND, GrammarStructuralTokenLayout, GrammarStructuralTokenRange, MODEL_VERSION,
    lower_grammar_ir_to_structural_model,
};
use tct::toy_grammar_frontend::{
    BYTE_TOKEN_BASE, BYTE_TOKEN_END, INTEGER_TOKEN_BASE, INTEGER_TOKEN_END, PAD_TOKEN,
    PROGRAM_END_TOKEN, STRUCTURAL_TOKEN_END, TOY_GRAMMAR_LANGUAGE_ID, ToyGrammarSpec, ToyValue,
    VOCAB_SIZE,
};

#[test]
fn toy_grammar_round_trips_parse_tokens_and_render() {
    let tokenizer = ToyGrammarSpec::assignment_language_v1().compile_tokenizer();
    let source = r#"
        let alpha = 7;
        let enabled = true;
        let message = "hello\nworld";
    "#;

    let parsed = tokenizer.parse_source(source).unwrap();
    let tokens = tokenizer.encode_program(&parsed).unwrap();
    assert_eq!(tokens.first().copied(), Some(0));
    assert_eq!(tokens.last().copied(), Some(PROGRAM_END_TOKEN));
    assert!(tokens.iter().all(|token| *token < PAD_TOKEN));
    assert!(tokens.iter().any(|token| {
        (BYTE_TOKEN_BASE..BYTE_TOKEN_END).contains(token)
            || (INTEGER_TOKEN_BASE..INTEGER_TOKEN_END).contains(token)
    }));

    let decoded = tokenizer.decode_tokens(&tokens).unwrap();
    assert_eq!(decoded, parsed);
    let rendered = tokenizer.render_program(&decoded);
    assert_eq!(
        rendered,
        "let alpha = 7;\nlet enabled = true;\nlet message = \"hello\\nworld\";\n"
    );
    assert_eq!(tokenizer.parse_source(&rendered).unwrap(), parsed);
}

#[test]
fn toy_grammar_reports_token_layout_and_vocab_invariants() {
    let spec = ToyGrammarSpec::assignment_language_v1();
    assert_eq!(spec.language_id, TOY_GRAMMAR_LANGUAGE_ID);
    assert_eq!(spec.layout.vocab_size, VOCAB_SIZE);
    assert_eq!(spec.layout.pad_token, PAD_TOKEN);
    assert_eq!(spec.layout.structural_tokens.start, 0);
    assert_eq!(
        spec.layout.structural_tokens.end_exclusive,
        STRUCTURAL_TOKEN_END
    );
    assert_eq!(spec.layout.utf8_byte_tokens.start, BYTE_TOKEN_BASE);
    assert_eq!(spec.layout.utf8_byte_tokens.end_exclusive, BYTE_TOKEN_END);
    assert_eq!(spec.layout.u8_integer_tokens.start, INTEGER_TOKEN_BASE);
    assert_eq!(
        spec.layout.u8_integer_tokens.end_exclusive,
        INTEGER_TOKEN_END
    );
    assert!(spec.layout.pad_token + 1 == spec.layout.vocab_size);
}

#[test]
fn toy_grammar_ir_lowers_to_structural_model_sidecar() {
    let spec = ToyGrammarSpec::assignment_language_v1();
    let grammar = spec.to_grammar_ir();
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&spec.layout),
    )
    .unwrap();

    assert_eq!(sidecar.artifact_kind, ARTIFACT_KIND);
    assert_eq!(sidecar.model_version, MODEL_VERSION);
    assert_eq!(sidecar.input.grammar_id, TOY_GRAMMAR_LANGUAGE_ID);
    assert_eq!(
        sidecar.structural_model.frontend_id,
        TOY_GRAMMAR_LANGUAGE_ID
    );
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
    assert!(
        !sidecar
            .structural_model
            .boundary
            .production_grammar_compiler_path_changed
    );

    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(
        sidecar
            .proof
            .checks
            .get("token_ranges_do_not_overlap")
            .copied(),
        Some(true)
    );
    assert_eq!(sidecar.proof.grammar_shape.rules, 2);
    assert_eq!(sidecar.proof.grammar_shape.fields, 3);
    assert_eq!(sidecar.proof.grammar_shape.alternatives, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 1);

    let program = sidecar
        .structural_model
        .records
        .iter()
        .find(|record| record.grammar_rule == "program")
        .unwrap();
    assert_eq!(program.required_fields, vec!["statements"]);
    let statements = program
        .fields
        .iter()
        .find(|field| field.name == "statements")
        .unwrap();
    assert_eq!(statements.field_kind, "List");
    assert_eq!(
        statements.target.as_deref(),
        Some("grammar.rule.assignment")
    );

    let value_domain = sidecar
        .structural_model
        .finite_domains
        .iter()
        .find(|domain| domain.owner_rule == "assignment" && domain.owner_field == "value")
        .unwrap();
    assert_eq!(value_domain.model_kind, "Variant");
    let terminals: Vec<_> = value_domain
        .alternatives
        .iter()
        .map(|alternative| alternative.terminal)
        .collect();
    assert!(terminals.contains(&GrammarTerminalKind::IntegerPayload));
    assert!(terminals.contains(&GrammarTerminalKind::StringPayload));
    assert!(terminals.contains(&GrammarTerminalKind::BooleanPayload));
    assert_eq!(sidecar.structural_model.payload_leaves.len(), 4);
    assert_eq!(sidecar.structural_model.references.len(), 1);

    let decision_graph = &sidecar.structural_model.decision_graph;
    assert_eq!(
        decision_graph
            .nodes
            .iter()
            .filter(|node| node.node_kind == "rule")
            .count(),
        2
    );
    assert_eq!(
        decision_graph
            .nodes
            .iter()
            .filter(|node| node.node_kind == "field")
            .count(),
        3
    );
    let value_decision = decision_graph
        .nodes
        .iter()
        .find(|node| node.id == "decision.rule.assignment.field.value")
        .unwrap();
    assert_eq!(value_decision.decision_kind, "choice");
    assert_eq!(
        value_decision.token_range.as_deref(),
        Some("toy.structural_tokens")
    );
    let integer_payload = decision_graph
        .nodes
        .iter()
        .find(|node| node.id == "decision.rule.assignment.field.value.alternative.integer")
        .unwrap();
    assert_eq!(integer_payload.node_kind, "alternative_payload");
    assert_eq!(
        integer_payload.terminal,
        Some(GrammarTerminalKind::IntegerPayload)
    );
    assert_eq!(integer_payload.payload_kind.as_deref(), Some("integer"));
    assert_eq!(
        integer_payload.token_range.as_deref(),
        Some("toy.u8_integer_payload")
    );
    assert!(decision_graph.edges.iter().any(|edge| {
        edge.source == "decision.rule.assignment.field.value"
            && edge.target == "decision.rule.assignment.field.value.alternative.integer"
            && edge.crosses_payload_boundary
    }));
    assert_eq!(
        sidecar
            .proof
            .checks
            .get("decision_graph_token_ranges_known")
            .copied(),
        Some(true)
    );

    assert_eq!(sidecar.structural_model.token_ranges.pad_token, PAD_TOKEN);
    assert!(
        sidecar
            .structural_model
            .token_ranges
            .ranges
            .iter()
            .all(|range| range.end_exclusive <= VOCAB_SIZE)
    );
}

#[test]
fn toy_grammar_structural_model_rejects_overlapping_token_ranges() {
    let spec = ToyGrammarSpec::assignment_language_v1();
    let grammar = spec.to_grammar_ir();
    let mut layout = GrammarStructuralTokenLayout::from_toy_assignment(&spec.layout);
    layout.ranges.push(GrammarStructuralTokenRange {
        id: "toy.invalid_overlap".to_string(),
        role: "invalid_overlap".to_string(),
        start: 1,
        end_exclusive: 4,
    });

    let error = lower_grammar_ir_to_structural_model(&grammar, layout).unwrap_err();
    assert!(error.to_string().contains("overlaps"));
}

#[test]
fn toy_grammar_rejects_unsupported_source_with_rule_context() {
    let tokenizer = ToyGrammarSpec::assignment_language_v1().compile_tokenizer();
    let error = tokenizer.parse_source("const alpha = 1;").unwrap_err();
    assert_eq!(error.phase, "parse");
    assert_eq!(error.rule, "let_statement");
    assert_eq!(error.position, Some(0));

    let error = tokenizer.parse_source("let alpha = 999;").unwrap_err();
    assert_eq!(error.rule, "integer");
    assert!(error.message.contains("0..=255"));
}

#[test]
fn toy_grammar_strict_decode_rejects_bad_tokens_and_trailing_pad() {
    let tokenizer = ToyGrammarSpec::assignment_language_v1().compile_tokenizer();
    let mut tokens = tokenizer.encode_source("let alpha = false;").unwrap();
    tokens.push(PAD_TOKEN);
    let error = tokenizer.decode_tokens(&tokens).unwrap_err();
    assert_eq!(error.phase, "decode");
    assert_eq!(error.rule, "trailing_tokens");

    let mut bad_value_tag = tokenizer.encode_source("let alpha = 1;").unwrap();
    let integer_tag_index = bad_value_tag
        .iter()
        .position(|token| *token == INTEGER_TOKEN_BASE + 1)
        .expect("integer payload should be present")
        - 1;
    bad_value_tag[integer_tag_index] = 900;
    let error = tokenizer.decode_tokens(&bad_value_tag).unwrap_err();
    assert_eq!(error.rule, "value_tag");

    let decoded = tokenizer
        .decode_tokens(&tokenizer.encode_source("let answer = 42;").unwrap())
        .unwrap();
    assert_eq!(decoded.statements[0].value, ToyValue::Integer(42));
}
