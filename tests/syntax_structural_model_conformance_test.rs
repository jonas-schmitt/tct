use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Value, json};
use tct::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use tct::go_syntax_metadata::GoSyntaxMetadataAdapter;
use tct::grammar_ir_structural_model::{
    ARTIFACT_KIND, GrammarIntegerValueRange, GrammarStructuralModelSidecar,
    GrammarStructuralTokenLayout, GrammarStructuralTokenRange, MODEL_VERSION,
    lower_grammar_ir_to_structural_model,
};
use tct::provenance::file_provenance;
use tct::toy_grammar_frontend::ToyGrammarSpec;

const CSHARP_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/minimal_class/syntax-metadata-v0.json";
const GO_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/minimal_function/syntax-metadata-v0.json";
const BOUNDARY_MANIFEST_PATH: &str =
    "tests/fixtures/syntax_structural_model/boundary-manifest-v1.json";
const BOUNDARY_MANIFEST_ARTIFACT_KIND: &str = "tct_syntax_structural_model_boundary_manifest";
const BOUNDARY_MANIFEST_VERSION: u32 = 1;

#[test]
fn syntax_structural_model_boundary_conformance_gate_proves_shared_profile() {
    let sidecars = [
        SyntaxSidecar {
            target: "toy",
            source_kind: "hand_written_toy_grammar",
            parser_tool: None,
            sidecar: lower_toy_sidecar(),
        },
        SyntaxSidecar {
            target: "csharp",
            source_kind: "csharp_syntax_metadata",
            parser_tool: Some("dotnet"),
            sidecar: lower_csharp_sidecar(),
        },
        SyntaxSidecar {
            target: "go",
            source_kind: "go_syntax_metadata",
            parser_tool: Some("go"),
            sidecar: lower_go_sidecar(),
        },
    ];

    let mut aggregate = BoundaryProfileAggregate::default();
    for sidecar in &sidecars {
        assert_structural_model_conformance(sidecar, &mut aggregate);
    }

    assert_eq!(
        sidecars
            .iter()
            .map(|sidecar| sidecar.target)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["toy", "csharp", "go"])
    );
    assert_eq!(
        aggregate.source_kinds,
        BTreeSet::from([
            "hand_written_toy_grammar".to_string(),
            "csharp_syntax_metadata".to_string(),
            "go_syntax_metadata".to_string(),
        ])
    );
    assert_eq!(
        aggregate.parser_tools,
        BTreeSet::from(["dotnet".to_string(), "go".to_string()])
    );

    for category in [
        "records",
        "finite_domains",
        "variants",
        "lists",
        "optionals",
        "references",
        "payload_leaves",
        "token_ranges",
        "renderer_metadata",
        "validation_hooks",
        "decision_graph_nodes",
        "decision_graph_edges",
        "source_provenance",
        "parser_provenance",
    ] {
        assert!(
            aggregate.categories.contains(category),
            "missing StructuralModel boundary category {category}"
        );
    }
}

#[test]
fn syntax_structural_model_boundary_manifest_matches_fixture() {
    let manifest = syntax_structural_model_boundary_manifest();
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(BOUNDARY_MANIFEST_PATH).unwrap()).unwrap();

    assert_eq!(manifest["artifact_kind"], BOUNDARY_MANIFEST_ARTIFACT_KIND);
    assert_eq!(manifest["version"], BOUNDARY_MANIFEST_VERSION);
    assert_eq!(
        manifest["compiler_boundary_claim"]["semantic_evidence"],
        "structured_structural_model_fields"
    );
    assert_eq!(
        manifest["targets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|target| target["target"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["toy", "csharp", "go"])
    );
    assert_eq!(manifest, fixture);
}

fn lower_toy_sidecar() -> GrammarStructuralModelSidecar {
    let spec = ToyGrammarSpec::assignment_language_v1();
    let grammar = spec.to_grammar_ir();
    lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&spec.layout),
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

fn syntax_structural_model_boundary_manifest() -> Value {
    let sidecars = [
        SyntaxSidecar {
            target: "toy",
            source_kind: "hand_written_toy_grammar",
            parser_tool: None,
            sidecar: lower_toy_sidecar(),
        },
        SyntaxSidecar {
            target: "csharp",
            source_kind: "csharp_syntax_metadata",
            parser_tool: Some("dotnet"),
            sidecar: lower_csharp_sidecar(),
        },
        SyntaxSidecar {
            target: "go",
            source_kind: "go_syntax_metadata",
            parser_tool: Some("go"),
            sidecar: lower_go_sidecar(),
        },
    ];

    json!({
        "artifact_kind": BOUNDARY_MANIFEST_ARTIFACT_KIND,
        "version": BOUNDARY_MANIFEST_VERSION,
        "sidecar_boundary": "GrammarIR -> StructuralModel",
        "compiler_boundary_claim": {
            "sidecar_only": true,
            "semantic_evidence": "structured_structural_model_fields",
            "generated_rust_source_substrings_used": false,
            "tct_compiler_changed": false,
            "generated_json_schema_output_changed": false,
            "core_compiler_path_changed": false,
            "production_grammar_compiler_path_changed": false,
        },
        "targets": sidecars
            .iter()
            .map(syntax_structural_model_boundary_manifest_target)
            .collect::<Vec<_>>(),
    })
}

fn syntax_structural_model_boundary_manifest_target(sidecar: &SyntaxSidecar) -> Value {
    let model = &sidecar.sidecar.structural_model;
    let proof = &sidecar.sidecar.proof;
    let provenance = &sidecar.sidecar.input.provenance;
    let source_sha256 = provenance.source_sha256.clone().or_else(|| {
        provenance.source_path.as_ref().and_then(|path| {
            file_provenance(path)
                .ok()
                .map(|provenance| provenance.sha256)
        })
    });
    let token_range_roles = model
        .token_ranges
        .ranges
        .iter()
        .map(|range| range.role.as_str())
        .collect::<BTreeSet<_>>();
    let categories = structural_category_coverage(sidecar);

    json!({
        "target": sidecar.target,
        "grammar_id": sidecar.sidecar.input.grammar_id.clone(),
        "grammar_version": sidecar.sidecar.input.grammar_version,
        "source_kind": provenance.source_kind.clone(),
        "source_path": provenance.source_path.clone(),
        "source_sha256": source_sha256,
        "metadata_path": provenance.metadata_path.clone(),
        "metadata_sha256": provenance.metadata_sha256.clone(),
        "parser": provenance.parser.clone(),
        "artifact_kind": sidecar.sidecar.artifact_kind.clone(),
        "model_version": sidecar.sidecar.model_version.clone(),
        "boundary": model.boundary.clone(),
        "structural_category_coverage": categories,
        "structural_counts": proof.structural_model_counts.clone(),
        "grammar_shape": proof.grammar_shape.clone(),
        "token_range_roles": token_range_roles.into_iter().collect::<Vec<_>>(),
        "proof_checks": proof.checks.clone(),
    })
}

fn assert_structural_model_conformance(
    sidecar: &SyntaxSidecar,
    aggregate: &mut BoundaryProfileAggregate,
) {
    assert_eq!(sidecar.sidecar.artifact_kind, ARTIFACT_KIND);
    assert_eq!(sidecar.sidecar.model_version, MODEL_VERSION);
    assert!(sidecar.sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.sidecar.proof.checks.values().all(|check| *check));

    assert_sidecar_boundary(sidecar);
    assert_source_and_parser_provenance(sidecar, aggregate);
    assert_structural_categories(sidecar, aggregate);
    assert_token_layout_conformance(sidecar, aggregate);
    assert_decision_graph_conformance(sidecar, aggregate);
}

fn assert_sidecar_boundary(sidecar: &SyntaxSidecar) {
    let boundary = &sidecar.sidecar.structural_model.boundary;
    assert_eq!(boundary.role, "sidecar_evidence_only");
    assert!(!boundary.generated_json_schema_output_changed);
    assert!(!boundary.core_compiler_path_changed);
    assert!(!boundary.production_grammar_compiler_path_changed);
}

fn assert_source_and_parser_provenance(
    sidecar: &SyntaxSidecar,
    aggregate: &mut BoundaryProfileAggregate,
) {
    let provenance = &sidecar.sidecar.input.provenance;
    assert_eq!(provenance.source_kind, sidecar.source_kind);
    aggregate
        .source_kinds
        .insert(provenance.source_kind.clone());

    match sidecar.parser_tool {
        Some(expected_tool) => {
            let parser = provenance
                .parser
                .as_ref()
                .expect("parser-backed syntax sidecar should record parser provenance");
            assert_eq!(parser.tool, expected_tool);
            assert!(!parser.version.is_empty());
            assert!(
                parser
                    .package
                    .as_ref()
                    .is_some_and(|value| !value.is_empty())
            );
            assert!(
                parser
                    .package_version
                    .as_ref()
                    .is_some_and(|value| !value.is_empty())
            );
            aggregate.parser_tools.insert(parser.tool.clone());
            aggregate.categories.insert("parser_provenance");
        }
        None => assert!(provenance.parser.is_none()),
    }

    assert!(provenance.source_path.is_some() || provenance.metadata_path.is_some());
    assert!(
        provenance
            .source_sha256
            .as_ref()
            .is_some_and(|sha| sha.len() == 64)
            || provenance
                .metadata_sha256
                .as_ref()
                .is_some_and(|sha| sha.len() == 64)
            || sidecar.target == "toy"
    );
    aggregate.categories.insert("source_provenance");
}

fn assert_structural_categories(sidecar: &SyntaxSidecar, aggregate: &mut BoundaryProfileAggregate) {
    let model = &sidecar.sidecar.structural_model;
    let counts = &sidecar.sidecar.proof.structural_model_counts;
    let grammar_shape = &sidecar.sidecar.proof.grammar_shape;

    assert_eq!(counts.records, model.records.len());
    assert_eq!(counts.finite_domains, model.finite_domains.len());
    assert_eq!(counts.lists, model.lists.len());
    assert_eq!(counts.optionals, model.optionals.len());
    assert_eq!(counts.references, model.references.len());
    assert_eq!(counts.payload_leaves, model.payload_leaves.len());
    assert_eq!(counts.records, grammar_shape.rules);
    assert_eq!(counts.finite_domains, grammar_shape.fields);
    assert_eq!(counts.lists, grammar_shape.repeated_fields);
    assert_eq!(
        counts.optionals,
        grammar_shape.fields - grammar_shape.required_fields
    );
    assert_eq!(counts.references, grammar_shape.nonterminal_references);
    assert_eq!(counts.payload_leaves, grammar_shape.payload_terminals);

    assert!(!model.records.is_empty());
    assert!(
        model
            .records
            .iter()
            .all(|record| record.model_kind == "Record")
    );
    assert!(!model.finite_domains.is_empty());
    assert!(
        model
            .finite_domains
            .iter()
            .all(|domain| !domain.alternatives.is_empty())
    );
    assert!(!model.lists.is_empty());
    assert!(model.lists.iter().all(|list| list.model_kind == "List"));
    assert!(!model.references.is_empty());
    assert!(
        model
            .references
            .iter()
            .all(|reference| reference.model_kind == "Reference")
    );
    assert!(!model.payload_leaves.is_empty());
    assert!(
        model
            .payload_leaves
            .iter()
            .all(|payload| payload.model_kind == "Payload")
    );
    assert!(!model.renderer_metadata.id.is_empty());
    assert!(!model.validation_hooks.is_empty());
    assert!(
        model
            .validation_hooks
            .iter()
            .all(|hook| !hook.runs_in_generated_hot_path)
    );

    aggregate.categories.insert("records");
    aggregate.categories.insert("finite_domains");
    aggregate.categories.insert("lists");
    aggregate.categories.insert("references");
    aggregate.categories.insert("payload_leaves");
    aggregate.categories.insert("renderer_metadata");
    aggregate.categories.insert("validation_hooks");

    if model
        .finite_domains
        .iter()
        .any(|domain| domain.model_kind == "Variant")
    {
        aggregate.categories.insert("variants");
    }
    if !model.optionals.is_empty() {
        assert!(
            model
                .optionals
                .iter()
                .all(|optional| optional.model_kind == "Optional")
        );
        aggregate.categories.insert("optionals");
    }
}

fn structural_category_coverage(sidecar: &SyntaxSidecar) -> Value {
    let model = &sidecar.sidecar.structural_model;
    json!({
        "records": !model.records.is_empty(),
        "finite_domains": !model.finite_domains.is_empty(),
        "variants": model
            .finite_domains
            .iter()
            .any(|domain| domain.model_kind == "Variant"),
        "lists": !model.lists.is_empty(),
        "optionals": !model.optionals.is_empty(),
        "references": !model.references.is_empty(),
        "payload_leaves": !model.payload_leaves.is_empty(),
        "token_ranges": !model.token_ranges.ranges.is_empty(),
        "renderer_metadata": !model.renderer_metadata.id.is_empty(),
        "validation_hooks": !model.validation_hooks.is_empty(),
        "decision_graph_nodes": !model.decision_graph.nodes.is_empty(),
        "decision_graph_edges": !model.decision_graph.edges.is_empty(),
        "source_provenance": sidecar.sidecar.input.provenance.source_path.is_some()
            || sidecar.sidecar.input.provenance.metadata_path.is_some(),
        "parser_provenance": sidecar.sidecar.input.provenance.parser.is_some(),
    })
}

fn assert_token_layout_conformance(
    sidecar: &SyntaxSidecar,
    aggregate: &mut BoundaryProfileAggregate,
) {
    let token_layout = &sidecar.sidecar.structural_model.token_ranges;
    let roles = token_layout
        .ranges
        .iter()
        .map(|range| range.role.as_str())
        .collect::<BTreeSet<_>>();
    assert!(roles.contains("structural_decisions"));
    assert!(roles.contains("utf8_byte_payload"));
    assert!(token_layout.pad_token < token_layout.vocab_size);
    assert!(
        token_layout
            .ranges
            .iter()
            .all(|range| range.start < range.end_exclusive
                && range.end_exclusive <= token_layout.vocab_size
                && !(range.start..range.end_exclusive).contains(&token_layout.pad_token))
    );
    assert_token_ranges_do_not_overlap(&token_layout.ranges);
    aggregate.categories.insert("token_ranges");
}

fn assert_decision_graph_conformance(
    sidecar: &SyntaxSidecar,
    aggregate: &mut BoundaryProfileAggregate,
) {
    let model = &sidecar.sidecar.structural_model;
    let graph = &model.decision_graph;
    assert_eq!(
        sidecar
            .sidecar
            .proof
            .structural_model_counts
            .decision_graph_nodes,
        graph.nodes.len()
    );
    assert_eq!(
        sidecar
            .sidecar
            .proof
            .structural_model_counts
            .decision_graph_edges,
        graph.edges.len()
    );
    assert!(!graph.nodes.is_empty());
    assert!(!graph.edges.is_empty());
    assert!(graph.nodes.iter().any(|node| node.node_kind == "rule"));
    assert!(graph.nodes.iter().any(|node| node.node_kind == "field"));
    assert!(graph.edges.iter().any(|edge| edge.crosses_payload_boundary));

    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    let token_range_ids = model
        .token_ranges
        .ranges
        .iter()
        .map(|range| range.id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(graph.edges.iter().all(|edge| {
        node_ids.contains(edge.source.as_str()) && node_ids.contains(edge.target.as_str())
    }));
    assert!(graph.nodes.iter().all(|node| {
        node.token_range
            .as_ref()
            .is_none_or(|range| token_range_ids.contains(range.as_str()))
    }));

    aggregate.categories.insert("decision_graph_nodes");
    aggregate.categories.insert("decision_graph_edges");
}

fn assert_token_ranges_do_not_overlap(ranges: &[GrammarStructuralTokenRange]) {
    let mut occupied = BTreeMap::new();
    for range in ranges {
        for token in range.start..range.end_exclusive {
            let previous = occupied.insert(token, range.id.as_str());
            assert!(
                previous.is_none(),
                "token {token} is covered by both {} and {}",
                previous.unwrap_or("<none>"),
                range.id
            );
        }
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
        vocab_size: 641,
        pad_token: 640,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "go.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 128,
            },
            GrammarStructuralTokenRange {
                id: "go.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 128,
                end_exclusive: 384,
            },
            GrammarStructuralTokenRange {
                id: "go.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 384,
                end_exclusive: 640,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "go.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

struct SyntaxSidecar {
    target: &'static str,
    source_kind: &'static str,
    parser_tool: Option<&'static str>,
    sidecar: GrammarStructuralModelSidecar,
}

#[derive(Default)]
struct BoundaryProfileAggregate {
    categories: BTreeSet<&'static str>,
    source_kinds: BTreeSet<String>,
    parser_tools: BTreeSet<String>,
}
