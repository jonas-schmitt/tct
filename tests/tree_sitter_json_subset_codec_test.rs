//! Workstream B1+: fixed tree-sitter-json subset codec smoke.
//!
//! Syntax-level only: pin → subset GrammarIR → StructuralModel → raw SM
//! encode/decode → render/reparse. Not JSON Schema; no Go layout pins.
//! Subset: null | true | false | string | number (u8 0..=255) | array | object.

use tct::grammar_ir_structural_model::lower_grammar_ir_to_structural_model;
use tct::shared_alphabet_layout::{
    SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
    shared_alphabet_choice_token_id,
};
use tct::syntax_state_machine::CompiledSyntaxModel;
use tct::tree_sitter_json_subset_codec::{
    CLAIM_BOUNDARY, JsonSubsetPair, JsonSubsetValue, SUBSET_GRAMMAR_ID, SUBSET_ID,
    TreeSitterJsonSubsetCodec, subset_token_layout, value_to_decoded_node,
};
use tct::tree_sitter_metadata::{CHILDREN_FIELD_NAME, SUBTYPE_FIELD_NAME, TreeSitterGrammarPin};

#[test]
fn tree_sitter_json_subset_codec_loads_pin_and_keeps_boundary() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();

    assert_eq!(codec.pin().pin_id, "tree_sitter_json_node_types_v0.24.8");
    assert_eq!(codec.pin().grammar_package, "tree-sitter-json");
    assert_eq!(codec.pin().grammar_version, "0.24.8");
    assert_eq!(
        codec.pin().node_types_sha256,
        "620b6f5d38676a97526e9d128972d4b156ddbdf6e130cd7e6b9c35e6823e943f"
    );
    assert!(
        TreeSitterGrammarPin::fixture_dir()
            .join("node-types.json")
            .is_file()
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
        Some("0.24.8")
    );

    let rule_names: Vec<_> = codec
        .grammar()
        .rules
        .iter()
        .map(|rule| rule.name.as_str())
        .collect();
    assert_eq!(
        rule_names,
        vec![
            "_value",
            "array",
            "false",
            "null",
            "number",
            "object",
            "pair",
            "string",
            "string_content",
            "true"
        ]
    );

    let value = codec.grammar().rule("_value").unwrap();
    assert_eq!(value.fields[0].name, SUBTYPE_FIELD_NAME);
    let kinds: Vec<_> = value.fields[0]
        .alternatives
        .iter()
        .map(|alternative| alternative.node_kind.as_str())
        .collect();
    assert_eq!(
        kinds,
        vec![
            "array", "false", "null", "number", "object", "string", "true"
        ]
    );
    assert!(
        value.fields[0]
            .alternatives
            .iter()
            .all(|alternative| alternative.terminal
                == tct::grammar_ir::GrammarTerminalKind::Nonterminal),
        "subtype alternatives must stay all-Nonterminal (no mixed payloads)"
    );

    let string = codec.grammar().rule("string").unwrap();
    assert_eq!(string.fields[0].name, CHILDREN_FIELD_NAME);
    assert_eq!(string.fields[0].alternatives[0].node_kind, "string_content");

    let number = codec.grammar().rule("number").unwrap();
    assert_eq!(number.fields[0].name, "value");
    assert_eq!(
        number.fields[0].alternatives[0].terminal,
        tct::grammar_ir::GrammarTerminalKind::IntegerPayload
    );

    let array = codec.grammar().rule("array").unwrap();
    assert_eq!(array.fields[0].name, CHILDREN_FIELD_NAME);
    assert!(array.fields[0].multiple);
    assert!(!array.fields[0].required);
    assert_eq!(array.fields[0].alternatives[0].node_kind, "_value");
    assert_eq!(
        array.fields[0].alternatives[0].terminal,
        tct::grammar_ir::GrammarTerminalKind::Nonterminal
    );

    let object = codec.grammar().rule("object").unwrap();
    assert_eq!(object.fields[0].name, CHILDREN_FIELD_NAME);
    assert!(object.fields[0].multiple);
    assert!(!object.fields[0].required);
    assert_eq!(object.fields[0].alternatives[0].node_kind, "pair");
    assert_eq!(
        object.fields[0].alternatives[0].terminal,
        tct::grammar_ir::GrammarTerminalKind::Nonterminal
    );

    let pair = codec.grammar().rule("pair").unwrap();
    assert_eq!(pair.fields.len(), 2);
    assert_eq!(pair.fields[0].name, "key");
    assert!(pair.fields[0].required);
    assert!(!pair.fields[0].multiple);
    assert_eq!(pair.fields[0].alternatives[0].node_kind, "string");
    assert_eq!(pair.fields[1].name, "value");
    assert!(pair.fields[1].required);
    assert!(!pair.fields[1].multiple);
    assert_eq!(pair.fields[1].alternatives[0].node_kind, "_value");

    for leaf in ["null", "true", "false"] {
        let rule = codec.grammar().rule(leaf).unwrap();
        assert!(
            rule.fields.is_empty(),
            "{leaf} must remain a field-less Nonterminal wrapper"
        );
    }

    let sidecar = codec.sidecar();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert_eq!(codec.claim_boundary(), CLAIM_BOUNDARY);
    assert_eq!(SUBSET_ID, SUBSET_GRAMMAR_ID);
}

#[test]
fn tree_sitter_json_subset_null_round_trips_with_raw_sm() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    let report = codec.round_trip_source("null").unwrap();

    assert_eq!(report.rendered, "null");
    assert_eq!(report.claim_boundary, CLAIM_BOUNDARY);
    assert_eq!(report.subset_id, SUBSET_ID);
    assert_eq!(report.pin_id, "tree_sitter_json_node_types_v0.24.8");
    assert!(
        report.raw_token_count >= 5,
        "null stream too short: {}",
        report.raw_token_count
    );

    let tree = codec.decode_raw(&report.raw_token_ids).unwrap();
    assert!(tree.node_start_count() >= 2);
    assert_eq!(tree.root_rule(), Some("_value"));
    let decoded = codec.decode(&report.raw_token_ids).unwrap();
    assert_eq!(decoded, JsonSubsetValue::Null);
}

#[test]
fn tree_sitter_json_subset_true_false_round_trips_with_raw_sm() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    for (source, expected) in [
        ("true", JsonSubsetValue::True),
        ("false", JsonSubsetValue::False),
    ] {
        let report = codec.round_trip_source(source).unwrap();
        assert_eq!(report.rendered, source);
        assert_eq!(report.claim_boundary, CLAIM_BOUNDARY);
        assert_eq!(report.subset_id, SUBSET_ID);
        assert!(
            report.raw_token_count >= 5,
            "{source} stream too short: {}",
            report.raw_token_count
        );

        let decoded = codec.decode(&report.raw_token_ids).unwrap();
        assert_eq!(decoded, expected);
        assert_eq!(decoded.render(), source);

        let tree = codec.decode_raw(&report.raw_token_ids).unwrap();
        assert!(tree.node_start_count() >= 2);
        assert_eq!(tree.root_rule(), Some("_value"));
    }
}

#[test]
fn tree_sitter_json_subset_string_round_trips_with_raw_sm() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    for source in ["\"\"", "\"a\"", "\"hi\"", "\"json\""] {
        let report = codec.round_trip_source(source).unwrap();
        assert_eq!(report.rendered, source);
        assert_eq!(report.claim_boundary, CLAIM_BOUNDARY);

        let decoded = codec.decode(&report.raw_token_ids).unwrap();
        assert_eq!(decoded, JsonSubsetValue::parse(source).unwrap());
        assert_eq!(decoded.render(), source);

        let tree = codec.decode_raw(&report.raw_token_ids).unwrap();
        assert!(tree.node_start_count() >= 2);
        if source != "\"\"" {
            assert!(tree.payload_value_count() >= 1);
        }
    }
}

#[test]
fn tree_sitter_json_subset_number_round_trips_with_raw_sm() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    for source in ["0", "1", "7", "42", "255"] {
        let report = codec.round_trip_source(source).unwrap();
        assert_eq!(report.rendered, source);
        assert_eq!(report.claim_boundary, CLAIM_BOUNDARY);
        assert_eq!(report.subset_id, SUBSET_ID);

        let decoded = codec.decode(&report.raw_token_ids).unwrap();
        assert_eq!(decoded, JsonSubsetValue::parse(source).unwrap());
        assert_eq!(decoded.render(), source);

        let tree = codec.decode_raw(&report.raw_token_ids).unwrap();
        assert!(tree.node_start_count() >= 2);
        assert!(
            tree.payload_value_count() >= 1,
            "number {source} should materialize a payload value"
        );
    }
}

#[test]
fn tree_sitter_json_subset_array_round_trips_with_raw_sm() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    for source in [
        "[]",
        "[null]",
        "[true,false]",
        "[0,1,255]",
        "[\"a\",\"\"]",
        "[null,true,1,\"x\"]",
        "[[]]",
        "[[null],[true,2]]",
        "[[],[[]],null]",
        "[{}]",
        "[{\"a\":1},null]",
    ] {
        let report = codec.round_trip_source(source).unwrap();
        assert_eq!(report.rendered, source, "render mismatch for {source}");
        assert_eq!(report.claim_boundary, CLAIM_BOUNDARY);
        assert_eq!(report.subset_id, SUBSET_ID);

        let decoded = codec.decode(&report.raw_token_ids).unwrap();
        assert_eq!(decoded, JsonSubsetValue::parse(source).unwrap());
        assert_eq!(decoded.render(), source);

        let tree = codec.decode_raw(&report.raw_token_ids).unwrap();
        assert_eq!(tree.root_rule(), Some("_value"));
        // Empty array still opens _value + array nodes.
        assert!(
            tree.node_start_count() >= 2,
            "array {source} node_start_count={}",
            tree.node_start_count()
        );
        if source != "[]" {
            assert!(
                tree.node_start_count() >= 3,
                "non-empty array {source} should open nested _value nodes"
            );
        }
    }
}

#[test]
fn tree_sitter_json_subset_object_round_trips_with_raw_sm() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    for source in [
        "{}",
        "{\"a\":1}",
        "{\"\":null}",
        "{\"k\":true,\"v\":false}",
        "{\"n\":255,\"s\":\"hi\",\"z\":[]}",
        "{\"nested\":{\"x\":0}}",
        "{\"arr\":[1,{\"inner\":null}]}",
        "{\"a\":1,\"a\":2}",
    ] {
        let report = codec
            .round_trip_source(source)
            .unwrap_or_else(|error| panic!("round-trip failed for {source}: {error}"));
        assert_eq!(report.rendered, source, "render mismatch for {source}");
        assert_eq!(report.claim_boundary, CLAIM_BOUNDARY);
        assert_eq!(report.subset_id, SUBSET_ID);

        let decoded = codec.decode(&report.raw_token_ids).unwrap();
        assert_eq!(decoded, JsonSubsetValue::parse(source).unwrap());
        assert_eq!(decoded.render(), source);

        let tree = codec.decode_raw(&report.raw_token_ids).unwrap();
        assert_eq!(tree.root_rule(), Some("_value"));
        assert!(
            tree.node_start_count() >= 2,
            "object {source} node_start_count={}",
            tree.node_start_count()
        );
        if source != "{}" {
            assert!(
                tree.node_start_count() >= 4,
                "non-empty object {source} should open object + pair (+ key string) nodes, got {}",
                tree.node_start_count()
            );
        }
    }

    // Explicit AST shape for a multi-pair object with nesting.
    let nested = JsonSubsetValue::Object(vec![
        JsonSubsetPair {
            key: "outer".to_string(),
            value: JsonSubsetValue::Object(vec![JsonSubsetPair {
                key: "inner".to_string(),
                value: JsonSubsetValue::Array(vec![
                    JsonSubsetValue::Number(1),
                    JsonSubsetValue::True,
                ]),
            }]),
        },
        JsonSubsetPair {
            key: "tail".to_string(),
            value: JsonSubsetValue::Null,
        },
    ]);
    assert_eq!(
        nested.render(),
        "{\"outer\":{\"inner\":[1,true]},\"tail\":null}"
    );
    let tokens = codec.encode(&nested).unwrap();
    assert_eq!(codec.decode(&tokens).unwrap(), nested);
}

#[test]
fn tree_sitter_json_subset_rejects_out_of_subset_source() {
    assert!(JsonSubsetValue::parse("256").is_err());
    assert!(JsonSubsetValue::parse("01").is_err());
    assert!(JsonSubsetValue::parse("-1").is_err());
    assert!(JsonSubsetValue::parse("1.5").is_err());
    assert!(JsonSubsetValue::parse("{a:1}").is_err());
    assert!(JsonSubsetValue::parse("{\"a\":1,}").is_err());
    assert!(JsonSubsetValue::parse("{\"a\": 1}").is_err());
    assert!(JsonSubsetValue::parse("{\"a\":1.5}").is_err());
    assert!(JsonSubsetValue::parse("\"a\\nb\"").is_err());
    assert!(JsonSubsetValue::parse("True").is_err());
    assert!(JsonSubsetValue::parse("FALSE").is_err());
    assert!(JsonSubsetValue::parse("[1,]").is_err());
    assert!(JsonSubsetValue::parse("[ 1]").is_err());
    assert!(JsonSubsetValue::parse("[null, 1]").is_err());
}

#[test]
fn tree_sitter_json_subset_structural_model_uses_sidecar_layout() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    let layout = subset_token_layout();
    let again = lower_grammar_ir_to_structural_model(codec.grammar(), layout.clone()).unwrap();
    assert_eq!(
        again.structural_model.token_ranges.vocab_size,
        layout.vocab_size
    );
    assert_eq!(
        again.structural_model.token_ranges.pad_token,
        layout.pad_token
    );
    for target in [
        "null", "true", "false", "string", "number", "array", "object",
    ] {
        assert!(
            again.structural_model.references.iter().any(|reference| {
                reference.owner_rule == "_value"
                    && reference.field == SUBTYPE_FIELD_NAME
                    && reference.target_rule == target
            }),
            "missing subtype reference to {target}"
        );
    }
    assert!(
        again.structural_model.references.iter().any(|reference| {
            reference.owner_rule == "array"
                && reference.field == CHILDREN_FIELD_NAME
                && reference.target_rule == "_value"
        }),
        "array.children must reference recursive _value"
    );
    assert!(
        again.structural_model.references.iter().any(|reference| {
            reference.owner_rule == "object"
                && reference.field == CHILDREN_FIELD_NAME
                && reference.target_rule == "pair"
        }),
        "object.children must reference pair"
    );
    assert!(
        again.structural_model.references.iter().any(|reference| {
            reference.owner_rule == "pair"
                && reference.field == "key"
                && reference.target_rule == "string"
        }),
        "pair.key must reference string"
    );
    assert!(
        again.structural_model.references.iter().any(|reference| {
            reference.owner_rule == "pair"
                && reference.field == "value"
                && reference.target_rule == "_value"
        }),
        "pair.value must reference recursive _value"
    );
    assert!(
        again
            .structural_model
            .bounded_integers
            .iter()
            .any(|bounded| bounded.owner_rule == "number" && bounded.field == "value"),
        "number.value must register as a bounded integer for the u8 layout"
    );
}

/// B′ contract: subset homogenizes `_value.subtype` to all-Nonterminal and
/// relocates IntegerPayload under `number.value`. Contrasts with full-pin
/// mixed payload kinds (`docs/tree-sitter-json-mixed-payload.md`). Not a
/// full-pin codec claim.
#[test]
fn tree_sitter_json_subset_mixed_payload_contract_homogenizes_value_subtype() {
    use tct::tree_sitter_metadata::TreeSitterMetadataAdapter;

    // Full-pin facts (lower path only).
    let (pin_grammar, pin) =
        TreeSitterMetadataAdapter::new("tree_sitter_json_mixed_payload_pin_probe")
            .lower_pinned_grammar_dir(&TreeSitterGrammarPin::fixture_dir())
            .unwrap();
    assert_eq!(pin.pin_id, "tree_sitter_json_node_types_v0.24.8");
    let pin_value = pin_grammar.rule("_value").unwrap();
    let pin_number_terminal = pin_value.fields[0]
        .alternatives
        .iter()
        .find(|alternative| alternative.node_kind == "number")
        .map(|alternative| alternative.terminal);
    assert_eq!(
        pin_number_terminal,
        Some(tct::grammar_ir::GrammarTerminalKind::IntegerPayload),
        "pin still classifies bare number as IntegerPayload on _value.subtype"
    );
    let pin_terminals: Vec<_> = pin_value.fields[0]
        .alternatives
        .iter()
        .map(|alternative| alternative.terminal)
        .collect();
    assert!(
        pin_terminals.contains(&tct::grammar_ir::GrammarTerminalKind::IntegerPayload)
            && pin_terminals.contains(&tct::grammar_ir::GrammarTerminalKind::BooleanPayload)
            && pin_terminals.contains(&tct::grammar_ir::GrammarTerminalKind::Nonterminal),
        "pin _value.subtype must stay mixed-payload (Integer/Boolean/Nonterminal): {pin_terminals:?}"
    );

    // Subset projection: homogeneous Nonterminal subtypes + wrapper field.
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    assert_eq!(codec.claim_boundary(), CLAIM_BOUNDARY);
    assert_eq!(CLAIM_BOUNDARY, "syntax_level_tree_sitter_json_subset_only");

    let subset_value = codec.grammar().rule("_value").unwrap();
    assert!(
        subset_value.fields[0]
            .alternatives
            .iter()
            .all(|alternative| {
                alternative.terminal == tct::grammar_ir::GrammarTerminalKind::Nonterminal
            }),
        "subset _value.subtype must not mix payload kinds"
    );
    let number = codec.grammar().rule("number").unwrap();
    assert_eq!(number.fields.len(), 1);
    assert_eq!(number.fields[0].name, "value");
    assert!(number.fields[0].required);
    assert!(!number.fields[0].multiple);
    assert_eq!(
        number.fields[0].alternatives[0].terminal,
        tct::grammar_ir::GrammarTerminalKind::IntegerPayload
    );

    let sidecar = codec.sidecar();
    assert!(
        !sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "_value"),
        "subset must not attach payload leaves directly to _value"
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "number"
                    && payload.field == "value"
                    && payload.payload_kind == "integer"
            }),
        "subset integer payload must live under number.value"
    );
    assert!(
        sidecar
            .structural_model
            .bounded_integers
            .iter()
            .any(|bounded| bounded.owner_rule == "number" && bounded.field == "value"),
        "number.value must remain the bounded u8 integer site"
    );

    // Round-trip still within the subset claim boundary (null + number peers).
    for source in ["null", "true", "false", "0", "255", "[]", "{}"] {
        let report = codec.round_trip_source(source).unwrap();
        assert_eq!(report.claim_boundary, CLAIM_BOUNDARY);
        assert_eq!(report.subset_id, SUBSET_ID);
    }
}

/// Assert absolute raw-token-ID pin + identity metadata + render from a
/// `round_trip_source` report.
///
/// Vectors were captured from a live codec run; they are not hand-computed.
fn assert_json_report_absolute_pin(
    label: &str,
    report: &tct::tree_sitter_json_subset_codec::TreeSitterJsonSubsetRoundTrip,
    expected: &[u32],
    expected_render: &str,
) {
    // Identity metadata pins so a renumber cannot be laundered as a fixture update.
    assert_eq!(
        report.pin_id, "tree_sitter_json_node_types_v0.24.8",
        "{label}: pin_id drift"
    );
    assert_eq!(report.subset_id, SUBSET_ID, "{label}: subset_id drift");
    assert_eq!(
        report.claim_boundary, CLAIM_BOUNDARY,
        "{label}: claim_boundary drift"
    );
    assert_eq!(
        report.raw_token_count,
        expected.len(),
        "{label}: raw_token_count mismatch"
    );
    assert_eq!(
        report.raw_token_ids.as_slice(),
        expected,
        "{label}: absolute raw_token_ids pin broken"
    );
    assert_eq!(
        report.rendered, expected_render,
        "{label}: render must match canonical form"
    );
}

/// Absolute token-ID golden pins for the JSON subset codec.
///
/// These pin ABSOLUTE token IDs (not mere encode/decode self-consistency).
/// Changing any expected vector is a `tct-token-diff` breaking change for any
/// downstream raw-token corpus or BPE artifact, and must not be updated merely
/// to make a refactor pass. See DEVELOPMENT.md ("Schema Evolution Reports")
/// for the layout-diff guidance (treat non-`exact_reuse` layout shifts like breaking changes).
#[test]
fn tree_sitter_json_subset_absolute_token_id_golden_pins() {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();

    // --- scalar leaf: `null` (field-less Nonterminal alternative) ---
    // Pins ABSOLUTE token IDs; changing this is a tct-token-diff breaking change
    // for downstream raw-token corpora / BPE artifacts (see DEVELOPMENT.md ("Schema Evolution Reports")).
    const NULL_GOLDEN: &[u32] = &[0, 2, 12, 1, 3, 1];
    {
        let report = codec.round_trip_source("null").unwrap();
        assert_json_report_absolute_pin("null", &report, NULL_GOLDEN, "null");
        assert_eq!(
            codec.decode(&report.raw_token_ids).unwrap().render(),
            "null"
        );
    }

    // --- nested object + array: exercises pair/key/string, recursive _value ---
    // Pins ABSOLUTE token IDs; changing this is a tct-token-diff breaking change
    // for downstream raw-token corpora / BPE artifacts (see DEVELOPMENT.md ("Schema Evolution Reports")).
    const NESTED_GOLDEN: &[u32] = &[
        0, 2, 14, 2, 0, 2, 0, 2, 6, 161, 7, 3, 1, 3, 2, 0, 2, 10, 2, 0, 2, 13, 2, 6, 321, 7, 3, 1,
        3, 1, 0, 2, 12, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1,
    ];
    {
        let source = "{\"a\":[1,null]}";
        let report = codec.round_trip_source(source).unwrap();
        assert_json_report_absolute_pin(source, &report, NESTED_GOLDEN, source);
        assert_eq!(
            codec.decode(&report.raw_token_ids).unwrap().render(),
            source
        );
    }

    // --- multi-byte UTF-8 string payload (café: U+00E9 → bytes C3 A9) ---
    // Source-text parse rejects non-ASCII; pin goes through encode/decode/render
    // on the AST so utf8_byte_payload absolute IDs are still locked.
    // Pins ABSOLUTE token IDs; changing this is a tct-token-diff breaking change
    // for downstream raw-token corpora / BPE artifacts (see DEVELOPMENT.md ("Schema Evolution Reports")).
    const UTF8_GOLDEN: &[u32] = &[0, 2, 15, 2, 6, 163, 161, 166, 259, 233, 7, 3, 1, 3, 1];
    {
        let value = JsonSubsetValue::String("café".to_string());
        let tokens = codec.encode(&value).unwrap();
        let decoded = codec.decode(&tokens).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(codec.pin().pin_id, "tree_sitter_json_node_types_v0.24.8");
        assert_eq!(
            SUBSET_ID,
            "tree_sitter_json_null_bool_string_number_array_object_subset_v0"
        );
        assert_eq!(CLAIM_BOUNDARY, "syntax_level_tree_sitter_json_subset_only");
        assert_eq!(tokens.len(), UTF8_GOLDEN.len());
        assert_eq!(
            tokens.as_slice(),
            UTF8_GOLDEN,
            "utf8-café: absolute raw_token_ids pin broken"
        );
        assert_eq!(decoded.render(), "\"café\"");
    }
}

/// I5 checklist tranche 8: JSON subset under SharedAlphabet (opt-in).
///
/// Language value → decoded node → encode/decode inverse under the shared
/// decision-token alphabet. Default PerGrammar codec and absolute golden pins
/// stay on the historical path (Decision 4 — second selectable layout mode).
/// Claim boundary: `compiler_shared_alphabet_layout_only`.
#[test]
fn tree_sitter_json_subset_shared_alphabet_language_value_round_trip() {
    assert_eq!(
        SHARED_ALPHABET_CLAIM_BOUNDARY,
        "compiler_shared_alphabet_layout_only"
    );

    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().unwrap();
    assert_eq!(
        codec.model().layout_mode(),
        SyntaxTokenLayoutMode::PerGrammar,
        "default JSON subset codec must remain PerGrammar"
    );

    let shared = CompiledSyntaxModel::from_sidecar_with_mode(
        codec.sidecar(),
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect("JSON subset should compile under SharedAlphabet");
    assert_eq!(shared.layout_mode(), SyntaxTokenLayoutMode::SharedAlphabet);
    let roles = shared.token_roles();
    assert_eq!(roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
    assert_eq!(roles.pad_token, SHARED_ALPHABET_VOCAB_SIZE - 1);

    // Domain-stable order for `_value.subtype` (arity 7):
    // array, false, null, number, object, string, true → null is ordinal 2.
    let null_choice = roles
        .alternative_token_for_field("_value", SUBTYPE_FIELD_NAME, "null")
        .expect("SharedAlphabet should assign CHOICE for null");
    let expected_null_choice = shared_alphabet_choice_token_id(roles.structural_range.start, 7, 2)
        .expect("CHOICE(7,2) must pack");
    assert_eq!(
        null_choice, expected_null_choice,
        "null must be CHOICE(7,2) under SharedAlphabet"
    );

    let cases = [
        JsonSubsetValue::Null,
        JsonSubsetValue::True,
        JsonSubsetValue::False,
        JsonSubsetValue::Number(0),
        JsonSubsetValue::Number(255),
        JsonSubsetValue::String(String::new()),
        JsonSubsetValue::String("hi".to_string()),
        JsonSubsetValue::String("café".to_string()),
        JsonSubsetValue::Array(vec![]),
        JsonSubsetValue::Array(vec![
            JsonSubsetValue::Null,
            JsonSubsetValue::True,
            JsonSubsetValue::Number(1),
            JsonSubsetValue::String("x".to_string()),
        ]),
        JsonSubsetValue::Object(vec![]),
        JsonSubsetValue::Object(vec![JsonSubsetPair {
            key: "a".to_string(),
            value: JsonSubsetValue::Array(vec![JsonSubsetValue::Number(1), JsonSubsetValue::Null]),
        }]),
        JsonSubsetValue::Object(vec![
            JsonSubsetPair {
                key: "outer".to_string(),
                value: JsonSubsetValue::Object(vec![JsonSubsetPair {
                    key: "inner".to_string(),
                    value: JsonSubsetValue::Array(vec![
                        JsonSubsetValue::Number(1),
                        JsonSubsetValue::True,
                    ]),
                }]),
            },
            JsonSubsetPair {
                key: "tail".to_string(),
                value: JsonSubsetValue::Null,
            },
        ]),
    ];

    for value in &cases {
        let root = value_to_decoded_node(value);
        let tokens = shared.encode_complete_tree(&root).unwrap_or_else(|error| {
            panic!("SharedAlphabet encode failed for {value:?}: {error:?}")
        });
        for &token in &tokens {
            assert!(
                token < roles.pad_token && token < roles.vocab_size,
                "token {token} must stay below PAD/VOCAB under shared layout for {value:?}"
            );
        }
        let decoded = shared
            .decode_complete_tree(&tokens)
            .unwrap_or_else(|error| {
                panic!("SharedAlphabet decode failed for {value:?}: {error:?}")
            });
        let tree2 = decoded
            .decoded_node_tree()
            .expect("decoded node tree should materialize under SharedAlphabet");
        assert_eq!(
            &tree2, &root,
            "decode(encode(tree)) must equal tree under SharedAlphabet for {value:?}"
        );
        let reencoded = shared
            .encode_complete_tree(&tree2)
            .expect("re-encode under SharedAlphabet should succeed");
        assert_eq!(
            reencoded, tokens,
            "encode(decode(tokens)) must equal tokens under SharedAlphabet for {value:?}"
        );
    }

    // Shared mode is a second layout: absolute streams must not match the
    // default PerGrammar golden for multi-alt null (token 12 free-slot).
    const NULL_PER_GRAMMAR_GOLDEN: &[u32] = &[0, 2, 12, 1, 3, 1];
    let null_shared = shared
        .encode_complete_tree(&value_to_decoded_node(&JsonSubsetValue::Null))
        .expect("null under SharedAlphabet");
    assert_ne!(
        null_shared.as_slice(),
        NULL_PER_GRAMMAR_GOLDEN,
        "SharedAlphabet null stream must differ from PerGrammar absolute golden"
    );
    assert!(
        null_shared.contains(&null_choice),
        "null SharedAlphabet stream should include CHOICE(7,2)={null_choice}: {null_shared:?}"
    );

    // Default PerGrammar path remains bit-identical (Decision 4 pin safety).
    let null_per_grammar = codec.encode(&JsonSubsetValue::Null).unwrap();
    assert_eq!(
        null_per_grammar.as_slice(),
        NULL_PER_GRAMMAR_GOLDEN,
        "PerGrammar null absolute pin must stay bit-identical without fixture edits"
    );
}
