use std::collections::BTreeSet;
use std::path::Path;

use tct::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use tct::go_syntax_metadata::GoSyntaxMetadataAdapter;
use tct::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, GrammarStructuralTokenLayout, GrammarStructuralTokenRange,
    lower_grammar_ir_to_structural_model,
};
use tct::toy_grammar_frontend::ToyGrammarSpec;
use tct::tree_sitter_metadata::TreeSitterMetadataAdapter;

const TREE_SITTER_METADATA_PATH: &str =
    "tests/fixtures/tree_sitter_metadata/toy_assignment/node-types.json";
const CSHARP_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/minimal_class/syntax-metadata-v0.json";
const GO_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/minimal_function/syntax-metadata-v0.json";

#[test]
fn syntax_metadata_fixtures_share_decision_graph_categories() {
    let tree_sitter = summarize("tree_sitter_toy_assignment", lower_tree_sitter_sidecar());
    let csharp = summarize("csharp_minimal_class", lower_csharp_sidecar());
    let go = summarize("go_minimal_function", lower_go_sidecar());

    for summary in [&tree_sitter, &csharp, &go] {
        assert!(!summary.label.is_empty());
        assert!(summary.tokenizable);
        assert!(summary.has_stable_input_hash);
        assert!(!summary.source_kind.is_empty());
        assert!(summary.rules > 0);
        assert!(summary.fields > 0);
        assert!(summary.repetitions > 0);
        assert!(summary.references > 0);
        assert!(summary.payload_leaves > 0);
        assert!(summary.payload_boundary_edges > 0);
        assert!(summary.token_range_roles.contains("structural_decisions"));
        assert!(summary.token_range_roles.contains("utf8_byte_payload"));
        assert!(!summary.generated_json_schema_output_changed);
        assert!(!summary.core_compiler_path_changed);
        assert!(!summary.production_grammar_compiler_path_changed);
    }

    assert_eq!(tree_sitter.source_kind, "tree_sitter_node_types");
    assert_eq!(csharp.source_kind, "csharp_syntax_metadata");
    assert_eq!(go.source_kind, "go_syntax_metadata");
    assert!(!tree_sitter.has_parser_provenance);
    assert!(csharp.has_parser_provenance);
    assert!(go.has_parser_provenance);

    let shared_roles = tree_sitter
        .token_range_roles
        .intersection(&csharp.token_range_roles)
        .cloned()
        .collect::<BTreeSet<_>>();
    let shared_roles = shared_roles
        .intersection(&go.token_range_roles)
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        shared_roles,
        BTreeSet::from([
            "structural_decisions".to_string(),
            "utf8_byte_payload".to_string()
        ])
    );
    assert_eq!(
        tree_sitter.shared_categories, csharp.shared_categories,
        "syntax fixtures should exercise the same target-agnostic sidecar categories"
    );
    assert_eq!(
        tree_sitter.shared_categories, go.shared_categories,
        "Go parser-backed syntax metadata should exercise the shared sidecar categories"
    );
}

fn lower_tree_sitter_sidecar() -> GrammarStructuralModelSidecar {
    let adapter = TreeSitterMetadataAdapter::new("toy_assignment_tree_sitter");
    let grammar = adapter
        .lower_node_types_file(Path::new(TREE_SITTER_METADATA_PATH))
        .unwrap();
    let toy_layout = ToyGrammarSpec::assignment_language_v1().layout;
    lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&toy_layout),
    )
    .unwrap()
}

fn lower_csharp_sidecar() -> GrammarStructuralModelSidecar {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_minimal_class_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_METADATA_PATH))
        .unwrap();
    lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap()
}

fn lower_go_sidecar() -> GrammarStructuralModelSidecar {
    let adapter = GoSyntaxMetadataAdapter::new("go_minimal_function_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_METADATA_PATH))
        .unwrap();
    lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap()
}

fn summarize(label: &'static str, sidecar: GrammarStructuralModelSidecar) -> SyntaxDecisionSummary {
    let token_range_roles = sidecar
        .structural_model
        .token_ranges
        .ranges
        .iter()
        .map(|range| range.role.clone())
        .collect::<BTreeSet<_>>();
    let payload_boundary_edges = sidecar
        .structural_model
        .decision_graph
        .edges
        .iter()
        .filter(|edge| edge.crosses_payload_boundary)
        .count();
    let source_sha256_present = sidecar
        .input
        .provenance
        .source_sha256
        .as_ref()
        .is_some_and(|sha| sha.len() == 64);
    let metadata_sha256_present = sidecar
        .input
        .provenance
        .metadata_sha256
        .as_ref()
        .is_some_and(|sha| sha.len() == 64);
    let has_stable_input_hash = source_sha256_present || metadata_sha256_present;
    let rules = sidecar.proof.grammar_shape.rules;
    let fields = sidecar.proof.grammar_shape.fields;
    let repetitions = sidecar.proof.grammar_shape.repeated_fields;
    let references = sidecar.structural_model.references.len();
    let payload_leaves = sidecar.structural_model.payload_leaves.len();
    let boundary = &sidecar.structural_model.boundary;
    let mut shared_categories = BTreeSet::new();
    if !sidecar.structural_model.decision_graph.nodes.is_empty()
        && !sidecar.structural_model.decision_graph.edges.is_empty()
    {
        shared_categories.insert("decision_graph".to_string());
    }
    if payload_boundary_edges > 0 {
        shared_categories.insert("payload_boundary_edges".to_string());
    }
    if payload_leaves > 0 {
        shared_categories.insert("payload_leaves".to_string());
    }
    if references > 0 {
        shared_categories.insert("references".to_string());
    }
    if repetitions > 0 {
        shared_categories.insert("repetitions".to_string());
    }
    if rules > 0 && fields > 0 {
        shared_categories.insert("rules".to_string());
    }
    if has_stable_input_hash {
        shared_categories.insert("source_hash_provenance".to_string());
    }
    if token_range_roles.contains("structural_decisions")
        && token_range_roles.contains("utf8_byte_payload")
    {
        shared_categories.insert("token_range_roles".to_string());
    }

    SyntaxDecisionSummary {
        label,
        source_kind: sidecar.input.provenance.source_kind.clone(),
        has_stable_input_hash,
        has_parser_provenance: sidecar.input.provenance.parser.is_some(),
        rules,
        fields,
        repetitions,
        references,
        payload_leaves,
        payload_boundary_edges,
        token_range_roles,
        generated_json_schema_output_changed: boundary.generated_json_schema_output_changed,
        core_compiler_path_changed: boundary.core_compiler_path_changed,
        production_grammar_compiler_path_changed: boundary.production_grammar_compiler_path_changed,
        tokenizable: sidecar.proof.tokenizable_decisions_match
            && sidecar.proof.checks.values().all(|check| *check),
        shared_categories,
    }
}

fn csharp_syntax_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 321,
        pad_token: 320,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "csharp.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 64,
            },
            GrammarStructuralTokenRange {
                id: "csharp.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 64,
                end_exclusive: 320,
            },
        ],
        integer_value_range: None,
    }
}

fn go_syntax_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 321,
        pad_token: 320,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "go.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 64,
            },
            GrammarStructuralTokenRange {
                id: "go.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 64,
                end_exclusive: 320,
            },
        ],
        integer_value_range: None,
    }
}

#[derive(Debug)]
struct SyntaxDecisionSummary {
    label: &'static str,
    source_kind: String,
    has_stable_input_hash: bool,
    has_parser_provenance: bool,
    rules: usize,
    fields: usize,
    repetitions: usize,
    references: usize,
    payload_leaves: usize,
    payload_boundary_edges: usize,
    token_range_roles: BTreeSet<String>,
    generated_json_schema_output_changed: bool,
    core_compiler_path_changed: bool,
    production_grammar_compiler_path_changed: bool,
    tokenizable: bool,
    shared_categories: BTreeSet<String>,
}
