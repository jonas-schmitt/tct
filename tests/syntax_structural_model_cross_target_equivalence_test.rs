use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use tct::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use tct::go_syntax_metadata::GoSyntaxMetadataAdapter;
use tct::grammar_ir_structural_model::{
    ARTIFACT_KIND, GrammarStructuralModelSidecar, MODEL_VERSION,
    lower_grammar_ir_to_structural_model,
};
use tct::syntax_csharp_grammar_coverage::{
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_NEXT_READY_TASK, CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_READY,
    CSharpSyntaxGrammarCoverageReport,
};
use tct::syntax_generated_codec::{
    CSHARP_COLLECTION_INITIALIZER_METADATA_PATH, CSHARP_CONDITIONAL_ACCESS_METADATA_PATH,
    CSHARP_ELEMENT_ACCESS_METADATA_PATH, CSHARP_LOCAL_ASSIGNMENT_METADATA_PATH,
    CSHARP_LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH,
    CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH, CSHARP_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH, CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH,
    CSHARP_LOCAL_IF_ELSE_METADATA_PATH, CSHARP_LOCAL_IMPLICIT_DECLARATION_METADATA_PATH,
    CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH,
    CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_NULL_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH,
    CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH, CSHARP_LOCAL_SWITCH_METADATA_PATH,
    CSHARP_LOCAL_TYPED_DECLARATION_METADATA_PATH, CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH,
    CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH,
    CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH,
    CSHARP_METHOD_INVOCATION_RETURN_METADATA_PATH, CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH,
    CSHARP_RANGE_ELEMENT_ACCESS_METADATA_PATH, CSHARP_TWO_PARAMETER_METHOD_METADATA_PATH,
    CSHARP_WHILE_POSTFIX_LOOP_METADATA_PATH, GO_ARRAY_COMPOSITE_LITERAL_METADATA_PATH,
    GO_ESCAPED_LOCAL_ASSIGNMENT_METADATA_PATH, GO_INDEX_EXPRESSION_METADATA_PATH,
    GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH, GO_LOCAL_ASSIGNMENT_METADATA_PATH,
    GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
    GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH,
    GO_LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH, GO_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
    GO_LOCAL_FOR_CONDITION_METADATA_PATH, GO_LOCAL_IF_ELSE_METADATA_PATH,
    GO_LOCAL_INCREMENT_DECREMENT_METADATA_PATH, GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
    GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH,
    GO_LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH,
    GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
    GO_LOCAL_RANGE_STRING_METADATA_PATH, GO_LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH,
    GO_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH, GO_LOCAL_SWITCH_METADATA_PATH,
    GO_LOCAL_TYPED_DECLARATION_METADATA_PATH,
    GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH, GO_METADATA_PATH,
    GO_METHOD_DECLARATION_METADATA_PATH, GO_PACKAGE_TYPE_DECLARATION_METADATA_PATH,
    GO_PACKAGE_VAR_DECLARATION_METADATA_PATH, GO_SLICE_EXPRESSION_METADATA_PATH,
    GO_STRINGS_COMPARE_SOURCE_FILE_METADATA_PATH, GO_STRUCT_TYPE_METADATA_PATH,
    csharp_syntax_token_layout, go_syntax_token_layout,
};
use tct::syntax_production_route::syntax_production_route_construct;

const CSHARP_STATEMENTS_CONTROL_FLOW_CATEGORY_PARITY_READY: &str =
    "csharp_statements_control_flow_category_parity_ready";
const GO_CURRENT_PLUS_STATEMENTS_CONTROL_FLOW_DEPTH_TRIGGER: &str = "category_depth_ready";
const STATEMENTS_CONTROL_FLOW_GRAMMAR_CATEGORY: &str = "statements_control_flow";
const STALE_CSHARP_MEMBER_CONDITIONAL_SUCCESSOR_TASK: &str =
    "syntax backend C# member/conditional-access structural parity depth tranche";
const STALE_SOURCE_PROGRAM_SUCCESSOR_TASK: &str =
    "syntax backend source-program claim handoff gate";

#[test]
fn registered_csharp_syntax_metadata_reaches_raw_state_machine_grammar_coverage() {
    let coverage = CSharpSyntaxGrammarCoverageReport::from_registered_syntax_codec_entries()
        .expect("registered C# syntax codec entries should produce grammar coverage");

    coverage
        .validate()
        .expect("C# syntax grammar coverage report should validate");
    assert_eq!(coverage.status, CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_READY);
    assert_eq!(coverage.target_id, "csharp");
    assert_eq!(coverage.covered_entry_count, coverage.registry_entry_count);
    assert_eq!(coverage.blocked_entry_count, 0);
    assert!(
        coverage.registry_entry_count >= 30,
        "the C# gate should cover the maintained registered syntax codec surface"
    );
    assert!(
        coverage
            .entries
            .iter()
            .any(|entry| entry.grammar_id == "csharp_compilation_unit_source_file_syntax_v0"),
        "whole-source C# compilation-unit syntax codec entry should remain covered"
    );
    assert!(
        coverage.entries.iter().any(|entry| entry.grammar_id
            == "csharp_compilation_unit_logger_source_file_syntax_v0"),
        "Logger whole-source C# compilation-unit syntax codec entry should be covered"
    );
    for grammar_id in [
        "csharp_qualified_name_syntax_v0",
        "csharp_generic_name_syntax_v0",
        "csharp_while_postfix_loop_syntax_v0",
        "csharp_member_access_expression_syntax_v0",
        "csharp_conditional_access_syntax_v0",
        "csharp_array_rank_object_creation_syntax_v0",
        "csharp_this_expression_syntax_v0",
        "csharp_pattern_expression_syntax_v0",
        "csharp_compilation_unit_version_one_source_file_syntax_v0",
        "csharp_compilation_unit_iterator_methods_source_file_syntax_v0",
        "csharp_compilation_unit_loan_source_file_syntax_v0",
    ] {
        assert!(
            coverage
                .entries
                .iter()
                .any(|entry| entry.grammar_id == grammar_id),
            "remaining whole-source C# compilation-unit syntax codec entry {grammar_id} should be covered"
        );
    }
    assert_eq!(
        coverage.totals.grammar_ir_rule_count,
        coverage.totals.structural_model_record_count
    );
    assert_eq!(
        coverage.totals.grammar_ir_field_count,
        coverage.totals.structural_model_field_count
    );
    assert!(
        coverage.shared_token_layout.alternative_node_start_capacity
            >= coverage
                .shared_token_layout
                .max_multi_alternative_nonterminal_node_kind_count,
        "registered C# syntax entries should fit in the shared structural token layout"
    );

    for entry in &coverage.entries {
        assert_eq!(entry.target_id, "csharp");
        assert_eq!(
            entry.grammar_ir_rule_count,
            entry.structural_model_record_count
        );
        assert_eq!(
            entry.grammar_ir_field_count,
            entry.structural_model_field_count
        );
        assert!(entry.finite_domain_count > 0);
        assert!(entry.raw_state_machine.raw_state_machine_compile_ready);
        assert!(entry.raw_state_machine.initial_state_ready);
        assert!(
            entry
                .raw_state_machine
                .initial_state_admissible_raw_token_count
                > 0
        );
        assert!(entry.raw_state_machine.initial_state_contains_node_start);
        assert!(entry.first_blocker.is_none());
    }

    assert!(
        coverage
            .cross_target_framing
            .contains("Go AST grammar coverage")
    );
    assert!(
        coverage
            .cross_target_framing
            .contains("shared raw state-machine boundary")
    );
    assert!(!coverage.arena_ir_proof_used);
    assert!(!coverage.source_program_type_correctness_claim_allowed);
    assert!(!coverage.bpe_layer_defines_validity);
    assert!(!coverage.stable_backend_promotion_claim_allowed);
    assert!(!coverage.json_schema_backend_changed);
    assert_eq!(
        coverage.next_ready_task,
        CSHARP_SYNTAX_GRAMMAR_COVERAGE_NEXT_READY_TASK
    );
    assert!(
        !coverage.next_ready_task.contains("pattern-expression"),
        "C# grammar coverage next task should not point at completed pattern-expression work"
    );
    assert!(
        !coverage
            .next_ready_task
            .contains(STALE_CSHARP_MEMBER_CONDITIONAL_SUCCESSOR_TASK),
        "C# grammar coverage next task should not point at completed member/conditional-access work"
    );
    assert!(
        !coverage
            .next_ready_task
            .contains(STALE_SOURCE_PROGRAM_SUCCESSOR_TASK),
        "C# grammar coverage next task should not point at completed source-program handoff work"
    );
    assert!(
        coverage
            .next_ready_task
            .contains("Go Source-Record Raw-Token Generator Core"),
        "C# grammar coverage should route default implementation work back to the Go generator core"
    );
}

#[test]
fn matched_go_and_csharp_local_constructs_have_equivalent_structural_model_categories() {
    for case in equivalence_cases() {
        assert_equivalence_case_routes_are_registered(&case);

        let go = lower_go_sidecar(case.go_grammar_id, case.go_metadata_path);
        let csharp = lower_csharp_sidecar(case.csharp_grammar_id, case.csharp_metadata_path);

        assert_sidecar_stays_at_syntax_boundary(&go);
        assert_sidecar_stays_at_syntax_boundary(&csharp);

        let go_evidence = go_local_construct_evidence(case.kind, &go);
        let csharp_evidence = csharp_local_construct_evidence(case.kind, &csharp);

        assert_equivalent_local_constructs(case.name, &go_evidence, &csharp_evidence).unwrap();
        assert_field_profiles_are_preserved(case.name, case.kind, &go_evidence, &csharp_evidence);
    }
}

#[test]
fn matched_go_and_csharp_callable_wrapper_constructs_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_local_integer_return_literal_syntax_v0",
        GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_local_integer_return_literal_syntax_v0",
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_callable_wrapper_generated_codec_routes_are_registered();

    let go_evidence = go_callable_wrapper_evidence(&go);
    let csharp_evidence = csharp_callable_wrapper_evidence(&csharp);

    assert_equivalent_local_constructs("callable_wrapper", &go_evidence, &csharp_evidence).unwrap();
    assert_callable_wrapper_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_callable_wrapper_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn syntax_structural_model_go_csharp_float_return_literal_parity() {
    let case = float_return_literal_equivalence_case();
    assert_equivalence_case_routes_are_registered(&case);

    let go = lower_go_sidecar(case.go_grammar_id, case.go_metadata_path);
    let csharp = lower_csharp_sidecar(case.csharp_grammar_id, case.csharp_metadata_path);

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);

    let go_return_evidence = go_local_construct_evidence(case.kind, &go);
    let csharp_return_evidence = csharp_local_construct_evidence(case.kind, &csharp);

    assert_equivalent_local_constructs(
        "float_return_literal",
        &go_return_evidence,
        &csharp_return_evidence,
    )
    .unwrap();
    assert_field_profiles_are_preserved(
        "float_return_literal",
        case.kind,
        &go_return_evidence,
        &csharp_return_evidence,
    );

    let go_callable_evidence = go_callable_wrapper_return_literal_evidence(&go, "float_literal");
    let csharp_callable_evidence =
        csharp_callable_wrapper_return_literal_evidence(&csharp, "double_keyword", "float_literal");

    assert_equivalent_local_constructs(
        "float_return_literal_callable_wrapper",
        &go_callable_evidence,
        &csharp_callable_evidence,
    )
    .unwrap();
    assert_callable_wrapper_field_profiles_are_preserved(
        &go_callable_evidence,
        &csharp_callable_evidence,
    );
    assert_callable_wrapper_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_callable_evidence,
        &csharp_callable_evidence,
    );
}

#[test]
fn syntax_structural_model_go_csharp_boolean_return_parity() {
    let case = boolean_return_equivalence_case();
    assert_equivalence_case_routes_are_registered(&case);

    let go = lower_go_sidecar(case.go_grammar_id, case.go_metadata_path);
    let csharp = lower_csharp_sidecar(case.csharp_grammar_id, case.csharp_metadata_path);

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);

    let go_return_evidence = go_local_construct_evidence(case.kind, &go);
    let csharp_return_evidence = csharp_local_construct_evidence(case.kind, &csharp);

    assert_equivalent_local_constructs(
        "boolean_return",
        &go_return_evidence,
        &csharp_return_evidence,
    )
    .unwrap();
    assert_field_profiles_are_preserved(
        "boolean_return",
        case.kind,
        &go_return_evidence,
        &csharp_return_evidence,
    );

    let go_callable_evidence = go_callable_wrapper_return_boolean_evidence(&go);
    let csharp_callable_evidence =
        csharp_callable_wrapper_return_literal_evidence(&csharp, "bool_keyword", "boolean_literal");

    assert_equivalent_local_constructs(
        "boolean_return_callable_wrapper",
        &go_callable_evidence,
        &csharp_callable_evidence,
    )
    .unwrap();
    assert_callable_wrapper_field_profiles_are_preserved(
        &go_callable_evidence,
        &csharp_callable_evidence,
    );
    assert_callable_wrapper_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_callable_evidence,
        &csharp_callable_evidence,
    );
}

#[test]
fn syntax_structural_model_go_csharp_nil_null_return_parity() {
    let case = nil_null_return_equivalence_case();
    assert_equivalence_case_routes_are_registered(&case);

    let go = lower_go_sidecar(case.go_grammar_id, case.go_metadata_path);
    let csharp = lower_csharp_sidecar(case.csharp_grammar_id, case.csharp_metadata_path);

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);

    let go_return_evidence = go_local_construct_evidence(case.kind, &go);
    let csharp_return_evidence = csharp_local_construct_evidence(case.kind, &csharp);

    assert_equivalent_local_constructs(
        "nil_null_return",
        &go_return_evidence,
        &csharp_return_evidence,
    )
    .unwrap();
    assert_field_profiles_are_preserved(
        "nil_null_return",
        case.kind,
        &go_return_evidence,
        &csharp_return_evidence,
    );
    assert_nil_null_return_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_return_evidence,
        &csharp_return_evidence,
    );

    let go_callable_evidence = go_callable_wrapper_return_null_evidence(&go);
    let csharp_callable_evidence = csharp_callable_wrapper_return_null_evidence(&csharp);

    assert_equivalent_local_constructs(
        "nil_null_return_callable_wrapper",
        &go_callable_evidence,
        &csharp_callable_evidence,
    )
    .unwrap();
    assert_callable_wrapper_field_profiles_are_preserved(
        &go_callable_evidence,
        &csharp_callable_evidence,
    );
    assert_callable_wrapper_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_callable_evidence,
        &csharp_callable_evidence,
    );
    assert_nil_null_return_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_callable_evidence,
        &csharp_callable_evidence,
    );
}

#[test]
fn matched_go_and_csharp_callable_parameter_lists_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar("go_minimal_function_syntax_v0", GO_METADATA_PATH);
    let csharp = lower_csharp_sidecar(
        "csharp_two_parameter_method_syntax_v0",
        CSHARP_TWO_PARAMETER_METHOD_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_callable_parameter_list_generated_codec_routes_are_registered();

    let go_evidence = go_callable_parameter_list_evidence(&go);
    let csharp_evidence = csharp_callable_parameter_list_evidence(&csharp);

    assert_equivalent_local_constructs("callable_parameter_list", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_callable_parameter_list_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_callable_parameter_list_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_member_list_containers_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_package_type_declaration_syntax_v0",
        GO_PACKAGE_TYPE_DECLARATION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_property_plus_method_syntax_v0",
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_member_list_container_generated_codec_routes_are_registered();

    let go_evidence = go_member_list_container_evidence(&go);
    let csharp_evidence = csharp_member_list_container_evidence(&csharp);

    assert_equivalent_local_constructs("member_list_container", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_member_list_container_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_member_list_container_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_data_member_fields_have_equivalent_structural_model_generated_codec_shape()
{
    let go = lower_go_sidecar("go_struct_type_syntax_v0", GO_STRUCT_TYPE_METADATA_PATH);
    let csharp = lower_csharp_sidecar(
        "csharp_property_plus_method_syntax_v0",
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_data_member_field_generated_codec_routes_are_registered();

    let go_evidence = go_data_member_field_evidence(&go);
    let csharp_evidence = csharp_data_member_field_evidence(&csharp);

    assert_equivalent_local_constructs("data_member_field", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_data_member_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_data_member_field_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_method_member_receivers_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_method_declaration_syntax_v0",
        GO_METHOD_DECLARATION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_property_plus_method_syntax_v0",
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_method_member_receiver_generated_codec_routes_are_registered();

    let go_evidence = go_method_member_receiver_evidence(&go);
    let csharp_evidence = csharp_method_member_receiver_evidence(&csharp);

    assert_equivalent_local_constructs("method_member_receiver", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_method_member_receiver_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_method_member_receiver_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_package_data_declarations_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_package_var_declaration_syntax_v0",
        GO_PACKAGE_VAR_DECLARATION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_property_plus_method_syntax_v0",
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_package_data_declaration_generated_codec_routes_are_registered();

    let go_evidence = go_package_data_declaration_evidence(&go);
    let csharp_evidence = csharp_package_data_declaration_evidence(&csharp);

    assert_equivalent_local_constructs("package_data_declaration", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_package_data_declaration_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_package_data_declaration_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_collection_initializers_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_array_composite_literal_syntax_v0",
        GO_ARRAY_COMPOSITE_LITERAL_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_collection_initializer_syntax_v0",
        CSHARP_COLLECTION_INITIALIZER_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_collection_initializer_generated_codec_routes_are_registered();

    let go_evidence = go_collection_initializer_evidence(&go);
    let csharp_evidence = csharp_collection_initializer_evidence(&csharp);

    assert_equivalent_local_constructs("collection_initializer", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_collection_initializer_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_collection_initializer_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_index_element_access_constructs_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_index_expression_syntax_v0",
        GO_INDEX_EXPRESSION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_element_access_syntax_v0",
        CSHARP_ELEMENT_ACCESS_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_index_element_access_generated_codec_routes_are_registered();

    let go_evidence = go_index_expression_evidence(&go);
    let csharp_evidence = csharp_element_access_evidence(&csharp);

    assert_equivalent_local_constructs("index_element_access", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_index_element_access_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_index_element_access_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_slice_range_constructs_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_slice_expression_syntax_v0",
        GO_SLICE_EXPRESSION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_range_element_access_syntax_v0",
        CSHARP_RANGE_ELEMENT_ACCESS_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_slice_range_generated_codec_routes_are_registered();

    let go_evidence = go_slice_expression_evidence(&go);
    let csharp_evidence = csharp_range_element_access_evidence(&csharp);

    assert_equivalent_local_constructs("slice_range", &go_evidence, &csharp_evidence).unwrap();
    assert_slice_range_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_slice_range_keeps_target_specific_evidence(&go, &csharp, &go_evidence, &csharp_evidence);
}

#[test]
fn matched_go_and_csharp_unary_integer_return_expressions_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_local_unary_integer_return_expression_syntax_v0",
        GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_local_unary_integer_return_syntax_v0",
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_unary_integer_return_expression_generated_codec_routes_are_registered();

    let go_evidence = go_unary_integer_return_expression_evidence(&go);
    let csharp_evidence = csharp_unary_integer_return_expression_evidence(&csharp);

    assert_equivalent_local_constructs(
        "unary_integer_return_expression",
        &go_evidence,
        &csharp_evidence,
    )
    .unwrap();
    assert_unary_integer_return_expression_field_profiles_are_preserved(
        &go_evidence,
        &csharp_evidence,
    );
    assert_unary_integer_return_expression_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_parenthesized_integer_return_expressions_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_local_parenthesized_integer_return_expression_syntax_v0",
        GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_local_parenthesized_integer_return_syntax_v0",
        CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_parenthesized_integer_return_expression_generated_codec_routes_are_registered();

    let go_evidence = go_parenthesized_integer_return_expression_evidence(&go);
    let csharp_evidence = csharp_parenthesized_integer_return_expression_evidence(&csharp);

    assert_equivalent_local_constructs(
        "parenthesized_integer_return_expression",
        &go_evidence,
        &csharp_evidence,
    )
    .unwrap();
    assert_parenthesized_integer_return_expression_field_profiles_are_preserved(
        &go_evidence,
        &csharp_evidence,
    );
    assert_parenthesized_integer_return_expression_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_binary_integer_return_expressions_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_local_binary_integer_return_expression_syntax_v0",
        GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_local_binary_integer_return_syntax_v0",
        CSHARP_LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_binary_integer_return_expression_generated_codec_routes_are_registered();

    let go_evidence = go_binary_integer_return_expression_evidence(&go);
    let csharp_evidence = csharp_binary_integer_return_expression_evidence(&csharp);

    assert_equivalent_local_constructs(
        "binary_integer_return_expression",
        &go_evidence,
        &csharp_evidence,
    )
    .unwrap();
    assert_binary_integer_return_expression_field_profiles_are_preserved(
        &go_evidence,
        &csharp_evidence,
    );
    assert_binary_integer_return_expression_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_call_expression_constructs_have_equivalent_structural_model_categories() {
    let go = lower_go_sidecar(
        "go_local_call_return_expression_syntax_v0",
        GO_LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_method_invocation_return_syntax_v0",
        CSHARP_METHOD_INVOCATION_RETURN_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_zero_argument_call_generated_codec_routes_are_registered();

    let go_evidence = go_call_expression_evidence(&go);
    let csharp_evidence = csharp_call_expression_evidence(&csharp);

    assert_equivalent_local_constructs("call_expression", &go_evidence, &csharp_evidence).unwrap();
    assert_call_expression_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_zero_argument_call_keeps_target_specific_evidence(&go, &csharp);
}

#[test]
fn matched_go_and_csharp_argument_call_expression_constructs_have_equivalent_structural_model_categories()
 {
    let go = lower_go_sidecar(
        "go_local_argument_call_return_expression_syntax_v0",
        GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_method_invocation_argument_return_syntax_v0",
        CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);

    let go_evidence = go_argument_call_expression_evidence(&go);
    let csharp_evidence = csharp_argument_call_expression_evidence(&csharp);

    assert_equivalent_local_constructs("argument_call_expression", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_argument_call_expression_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_argument_call_keeps_target_specific_evidence(&go, &csharp);
}

#[test]
fn matched_go_selector_and_csharp_member_conditional_access_have_equivalent_structural_model_shape()
{
    let go = lower_go_sidecar(
        "go_strings_compare_source_file_syntax_v0",
        GO_STRINGS_COMPARE_SOURCE_FILE_METADATA_PATH,
    );
    let csharp_member = lower_csharp_sidecar(
        "csharp_member_access_expression_syntax_v0",
        CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH,
    );
    let csharp_conditional = lower_csharp_sidecar(
        "csharp_conditional_access_syntax_v0",
        CSHARP_CONDITIONAL_ACCESS_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp_member);
    assert_sidecar_stays_at_syntax_boundary(&csharp_conditional);
    assert_member_conditional_access_generated_codec_routes_are_registered();

    let go_member_evidence = go_selector_member_access_evidence(&go);
    let csharp_member_evidence = csharp_member_access_expression_evidence(&csharp_member);

    assert_equivalent_local_constructs(
        "selector_member_access",
        &go_member_evidence,
        &csharp_member_evidence,
    )
    .unwrap();
    assert_member_access_field_profiles_are_preserved(
        "selector_member_access",
        &go_member_evidence,
        &csharp_member_evidence,
    );

    let go_conditional_evidence = go_selector_invocation_member_binding_evidence(&go);
    let csharp_conditional_evidence =
        csharp_conditional_access_member_binding_evidence(&csharp_conditional);

    assert_equivalent_local_constructs(
        "conditional_member_access",
        &go_conditional_evidence,
        &csharp_conditional_evidence,
    )
    .unwrap();
    assert_conditional_member_access_field_profiles_are_preserved(
        &go_conditional_evidence,
        &csharp_conditional_evidence,
    );
    assert_member_conditional_access_keeps_target_specific_evidence(
        &go,
        &csharp_member,
        &csharp_conditional,
        &go_member_evidence,
        &csharp_member_evidence,
    );
}

#[test]
fn matched_go_and_csharp_if_else_constructs_have_equivalent_structural_model_generated_codec_shape()
{
    let go = lower_go_sidecar("go_local_if_else_syntax_v0", GO_LOCAL_IF_ELSE_METADATA_PATH);
    let csharp = lower_csharp_sidecar(
        "csharp_local_if_else_syntax_v0",
        CSHARP_LOCAL_IF_ELSE_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_if_else_generated_codec_routes_are_registered();

    let go_evidence = go_if_else_evidence(&go);
    let csharp_evidence = csharp_if_else_evidence(&csharp);

    assert_equivalent_local_constructs("if_else_control_flow", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_if_else_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_if_else_keeps_target_specific_evidence(&go, &csharp, &go_evidence, &csharp_evidence);
}

#[test]
fn matched_go_and_csharp_condition_only_loop_constructs_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_local_for_condition_syntax_v0",
        GO_LOCAL_FOR_CONDITION_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_local_for_condition_syntax_v0",
        CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_for_condition_generated_codec_routes_are_registered();

    let go_evidence = go_for_condition_evidence(&go);
    let csharp_evidence = csharp_for_condition_evidence(&csharp);

    assert_equivalent_local_constructs("condition_only_loop", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_condition_only_loop_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_condition_only_loop_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_postfix_simple_statements_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_local_increment_decrement_syntax_v0",
        GO_LOCAL_INCREMENT_DECREMENT_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_while_postfix_loop_syntax_v0",
        CSHARP_WHILE_POSTFIX_LOOP_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_postfix_simple_statement_generated_codec_routes_are_registered();

    let go_evidence = go_postfix_simple_statement_evidence(&go);
    let csharp_evidence = csharp_postfix_simple_statement_evidence(&csharp);

    assert_equivalent_local_constructs("postfix_simple_statement", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_postfix_simple_statement_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_postfix_simple_statement_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn matched_go_and_csharp_single_case_switch_constructs_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar("go_local_switch_syntax_v0", GO_LOCAL_SWITCH_METADATA_PATH);
    let csharp = lower_csharp_sidecar(
        "csharp_local_switch_syntax_v0",
        CSHARP_LOCAL_SWITCH_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_switch_generated_codec_routes_are_registered();

    let go_evidence = go_switch_evidence(&go);
    let csharp_evidence = csharp_switch_evidence(&csharp);

    assert_equivalent_local_constructs("single_case_switch", &go_evidence, &csharp_evidence)
        .unwrap();
    assert_switch_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_switch_keeps_target_specific_evidence(&go, &csharp, &go_evidence, &csharp_evidence);
}

#[test]
fn matched_go_and_csharp_string_iteration_constructs_have_equivalent_structural_model_generated_codec_shape()
 {
    let go = lower_go_sidecar(
        "go_local_range_string_syntax_v0",
        GO_LOCAL_RANGE_STRING_METADATA_PATH,
    );
    let csharp = lower_csharp_sidecar(
        "csharp_local_foreach_string_syntax_v0",
        CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH,
    );

    assert_sidecar_stays_at_syntax_boundary(&go);
    assert_sidecar_stays_at_syntax_boundary(&csharp);
    assert_string_iteration_generated_codec_routes_are_registered();

    let go_evidence = go_string_iteration_evidence(&go);
    let csharp_evidence = csharp_string_iteration_evidence(&csharp);

    assert_equivalent_local_constructs("string_iteration", &go_evidence, &csharp_evidence).unwrap();
    assert_string_iteration_field_profiles_are_preserved(&go_evidence, &csharp_evidence);
    assert_string_iteration_keeps_target_specific_evidence(
        &go,
        &csharp,
        &go_evidence,
        &csharp_evidence,
    );
}

#[test]
fn csharp_statements_control_flow_category_parity_ready_for_go_current_plus_depth_trigger() {
    let cases = statements_control_flow_category_parity_cases();

    assert_eq!(
        CSHARP_STATEMENTS_CONTROL_FLOW_CATEGORY_PARITY_READY,
        "csharp_statements_control_flow_category_parity_ready"
    );
    assert_eq!(
        GO_CURRENT_PLUS_STATEMENTS_CONTROL_FLOW_DEPTH_TRIGGER,
        "category_depth_ready"
    );
    assert_eq!(
        STATEMENTS_CONTROL_FLOW_GRAMMAR_CATEGORY,
        "statements_control_flow"
    );
    assert_eq!(
        cases.len(),
        4,
        "statements/control-flow parity should aggregate the maintained branch, loop, switch, and iteration slices"
    );

    let mut covered_cases = BTreeSet::new();
    let mut covered_statement_kinds = BTreeSet::new();
    let mut covered_reference_roles = BTreeSet::new();
    let mut covered_payload_roles = BTreeSet::new();

    for case in cases {
        (case.assert_routes_registered)();

        let go = lower_go_sidecar(case.go_grammar_id, case.go_metadata_path);
        let csharp = lower_csharp_sidecar(case.csharp_grammar_id, case.csharp_metadata_path);

        assert_sidecar_stays_at_syntax_boundary(&go);
        assert_sidecar_stays_at_syntax_boundary(&csharp);

        let go_evidence = (case.go_evidence)(&go);
        let csharp_evidence = (case.csharp_evidence)(&csharp);

        assert_equivalent_local_constructs(case.name, &go_evidence, &csharp_evidence).unwrap();
        (case.assert_field_profiles)(&go_evidence, &csharp_evidence);
        (case.assert_target_specific)(&go, &csharp, &go_evidence, &csharp_evidence);

        covered_cases.insert(case.name);
        covered_statement_kinds.extend(go_evidence.comparable.statement_sequence.iter().copied());
        covered_statement_kinds.extend(
            csharp_evidence
                .comparable
                .statement_sequence
                .iter()
                .copied(),
        );
        covered_reference_roles.extend(
            go_evidence
                .comparable
                .semantic_references
                .iter()
                .map(|reference| reference.role.clone()),
        );
        covered_reference_roles.extend(
            csharp_evidence
                .comparable
                .semantic_references
                .iter()
                .map(|reference| reference.role.clone()),
        );
        covered_payload_roles.extend(
            go_evidence
                .comparable
                .semantic_payloads
                .iter()
                .map(|payload| payload.role.clone()),
        );
        covered_payload_roles.extend(
            csharp_evidence
                .comparable
                .semantic_payloads
                .iter()
                .map(|payload| payload.role.clone()),
        );
    }

    assert_eq!(
        covered_cases,
        BTreeSet::from([
            "if_else_control_flow",
            "condition_only_loop",
            "single_case_switch",
            "string_iteration",
        ])
    );
    for statement_kind in [
        "local_declaration",
        "if_else",
        "loop",
        "switch",
        "iteration_loop",
        "assignment",
        "case_body_assignment",
        "body_assignment",
        "return",
    ] {
        assert!(
            covered_statement_kinds.contains(statement_kind),
            "{CSHARP_STATEMENTS_CONTROL_FLOW_CATEGORY_PARITY_READY}: missing statement kind {statement_kind}"
        );
    }
    for role in [
        "block.statement.if_else",
        "block.statement.loop",
        "block.statement.switch",
        "block.statement.iteration_loop",
        "if.condition",
        "if.then_body",
        "if.else_body",
        "loop.condition",
        "loop.body",
        "switch.selector",
        "case.label",
        "case.body.assignment",
        "iteration.source",
        "iteration.body",
        "assignment.target",
        "assignment.value",
        "return.value",
    ] {
        assert!(
            covered_reference_roles.contains(role),
            "{CSHARP_STATEMENTS_CONTROL_FLOW_CATEGORY_PARITY_READY}: missing shared reference role {role}"
        );
    }
    for role in [
        "declaration.name",
        "declaration.initializer",
        "condition.left.identifier",
        "condition.right.literal",
        "switch.selector.identifier",
        "case.label.literal",
        "iteration.source.literal",
        "assignment.target",
        "assignment.value",
        "return.identifier",
    ] {
        assert!(
            covered_payload_roles.contains(role),
            "{CSHARP_STATEMENTS_CONTROL_FLOW_CATEGORY_PARITY_READY}: missing shared payload role {role}"
        );
    }
}

#[test]
fn cross_target_equivalence_comparison_detects_deliberate_divergence() {
    let case = equivalence_cases()
        .into_iter()
        .find(|case| case.name == "local_assignment")
        .unwrap();
    let go = lower_go_sidecar(case.go_grammar_id, case.go_metadata_path);
    let csharp = lower_csharp_sidecar(case.csharp_grammar_id, case.csharp_metadata_path);
    let go_evidence = go_local_construct_evidence(case.kind, &go);
    let mut diverged = csharp_local_construct_evidence(case.kind, &csharp);

    diverged
        .comparable
        .semantic_payloads
        .insert(SemanticPayload::new(
            "assignment.extra",
            "integer",
            "u8_offset",
        ));

    let error = assert_equivalent_local_constructs(case.name, &go_evidence, &diverged)
        .expect_err("comparison should reject an extra payload category");
    assert!(error.contains("semantic payloads"));
}

#[derive(Clone, Copy)]
struct EquivalenceCase {
    name: &'static str,
    kind: LocalConstructKind,
    go_grammar_id: &'static str,
    go_metadata_path: &'static str,
    go_round_trip_marker: &'static str,
    csharp_grammar_id: &'static str,
    csharp_metadata_path: &'static str,
    csharp_round_trip_marker: &'static str,
}

#[derive(Clone, Copy)]
struct ControlFlowParityCase {
    name: &'static str,
    go_grammar_id: &'static str,
    go_metadata_path: &'static str,
    csharp_grammar_id: &'static str,
    csharp_metadata_path: &'static str,
    assert_routes_registered: fn(),
    go_evidence: fn(&GrammarStructuralModelSidecar) -> LocalConstructBoundaryEvidence,
    csharp_evidence: fn(&GrammarStructuralModelSidecar) -> LocalConstructBoundaryEvidence,
    assert_field_profiles: fn(&LocalConstructBoundaryEvidence, &LocalConstructBoundaryEvidence),
    assert_target_specific: fn(
        &GrammarStructuralModelSidecar,
        &GrammarStructuralModelSidecar,
        &LocalConstructBoundaryEvidence,
        &LocalConstructBoundaryEvidence,
    ),
}

fn equivalence_cases() -> Vec<EquivalenceCase> {
    vec![
        EquivalenceCase {
            name: "local_string_return_literal",
            kind: LocalConstructKind::ReturnStringLiteral,
            go_grammar_id: "go_local_string_return_literal_syntax_v0",
            go_metadata_path: GO_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH,
            go_round_trip_marker: "generated_go_local_string_return_literal_round_trips",
            csharp_grammar_id: "csharp_local_string_return_literal_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH,
            csharp_round_trip_marker: "generated_csharp_local_string_return_literal_round_trips",
        },
        EquivalenceCase {
            name: "local_integer_return_literal",
            kind: LocalConstructKind::ReturnIntegerLiteral,
            go_grammar_id: "go_local_integer_return_literal_syntax_v0",
            go_metadata_path: GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
            go_round_trip_marker: "generated_go_local_integer_return_literal_round_trips",
            csharp_grammar_id: "csharp_local_integer_return_literal_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
            csharp_round_trip_marker: "generated_csharp_local_integer_return_literal_round_trips",
        },
        float_return_literal_equivalence_case(),
        boolean_return_equivalence_case(),
        EquivalenceCase {
            name: "local_typed_declaration",
            kind: LocalConstructKind::TypedDeclaration,
            go_grammar_id: "go_local_typed_declaration_syntax_v0",
            go_metadata_path: GO_LOCAL_TYPED_DECLARATION_METADATA_PATH,
            go_round_trip_marker: "generated_go_local_typed_declaration_round_trips",
            csharp_grammar_id: "csharp_local_typed_declaration_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_TYPED_DECLARATION_METADATA_PATH,
            csharp_round_trip_marker: "generated_csharp_local_typed_declaration_round_trips",
        },
        EquivalenceCase {
            name: "local_implicit_declaration",
            kind: LocalConstructKind::ImplicitDeclaration,
            go_grammar_id: "go_local_short_var_declaration_syntax_v0",
            go_metadata_path: GO_LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH,
            go_round_trip_marker: "generated_go_local_short_var_declaration_round_trips",
            csharp_grammar_id: "csharp_local_implicit_declaration_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_IMPLICIT_DECLARATION_METADATA_PATH,
            csharp_round_trip_marker: "generated_csharp_local_implicit_declaration_round_trips",
        },
        EquivalenceCase {
            name: "local_integer_implicit_declaration",
            kind: LocalConstructKind::ImplicitIntegerDeclaration,
            go_grammar_id: "go_local_integer_short_var_declaration_syntax_v0",
            go_metadata_path: GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH,
            go_round_trip_marker: "generated_go_local_integer_short_var_declaration_round_trips",
            csharp_grammar_id: "csharp_local_integer_implicit_declaration_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH,
            csharp_round_trip_marker: "generated_csharp_local_integer_implicit_declaration_round_trips",
        },
        EquivalenceCase {
            name: "local_assignment",
            kind: LocalConstructKind::Assignment,
            go_grammar_id: "go_local_assignment_syntax_v0",
            go_metadata_path: GO_LOCAL_ASSIGNMENT_METADATA_PATH,
            go_round_trip_marker: "generated_go_local_assignment_round_trips",
            csharp_grammar_id: "csharp_local_assignment_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_ASSIGNMENT_METADATA_PATH,
            csharp_round_trip_marker: "generated_csharp_local_assignment_round_trips",
        },
        EquivalenceCase {
            name: "local_escaped_assignment",
            kind: LocalConstructKind::Assignment,
            go_grammar_id: "go_escaped_local_assignment_syntax_v0",
            go_metadata_path: GO_ESCAPED_LOCAL_ASSIGNMENT_METADATA_PATH,
            go_round_trip_marker: "generated_go_escaped_local_assignment_round_trips",
            csharp_grammar_id: "csharp_local_escaped_assignment_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH,
            csharp_round_trip_marker: "generated_csharp_local_escaped_assignment_round_trips",
        },
    ]
}

fn statements_control_flow_category_parity_cases() -> Vec<ControlFlowParityCase> {
    vec![
        ControlFlowParityCase {
            name: "if_else_control_flow",
            go_grammar_id: "go_local_if_else_syntax_v0",
            go_metadata_path: GO_LOCAL_IF_ELSE_METADATA_PATH,
            csharp_grammar_id: "csharp_local_if_else_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_IF_ELSE_METADATA_PATH,
            assert_routes_registered: assert_if_else_generated_codec_routes_are_registered,
            go_evidence: go_if_else_evidence,
            csharp_evidence: csharp_if_else_evidence,
            assert_field_profiles: assert_if_else_field_profiles_are_preserved,
            assert_target_specific: assert_if_else_keeps_target_specific_evidence,
        },
        ControlFlowParityCase {
            name: "condition_only_loop",
            go_grammar_id: "go_local_for_condition_syntax_v0",
            go_metadata_path: GO_LOCAL_FOR_CONDITION_METADATA_PATH,
            csharp_grammar_id: "csharp_local_for_condition_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH,
            assert_routes_registered: assert_for_condition_generated_codec_routes_are_registered,
            go_evidence: go_for_condition_evidence,
            csharp_evidence: csharp_for_condition_evidence,
            assert_field_profiles: assert_condition_only_loop_field_profiles_are_preserved,
            assert_target_specific: assert_condition_only_loop_keeps_target_specific_evidence,
        },
        ControlFlowParityCase {
            name: "single_case_switch",
            go_grammar_id: "go_local_switch_syntax_v0",
            go_metadata_path: GO_LOCAL_SWITCH_METADATA_PATH,
            csharp_grammar_id: "csharp_local_switch_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_SWITCH_METADATA_PATH,
            assert_routes_registered: assert_switch_generated_codec_routes_are_registered,
            go_evidence: go_switch_evidence,
            csharp_evidence: csharp_switch_evidence,
            assert_field_profiles: assert_switch_field_profiles_are_preserved,
            assert_target_specific: assert_switch_keeps_target_specific_evidence,
        },
        ControlFlowParityCase {
            name: "string_iteration",
            go_grammar_id: "go_local_range_string_syntax_v0",
            go_metadata_path: GO_LOCAL_RANGE_STRING_METADATA_PATH,
            csharp_grammar_id: "csharp_local_foreach_string_syntax_v0",
            csharp_metadata_path: CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH,
            assert_routes_registered: assert_string_iteration_generated_codec_routes_are_registered,
            go_evidence: go_string_iteration_evidence,
            csharp_evidence: csharp_string_iteration_evidence,
            assert_field_profiles: assert_string_iteration_field_profiles_are_preserved,
            assert_target_specific: assert_string_iteration_keeps_target_specific_evidence,
        },
    ]
}

fn float_return_literal_equivalence_case() -> EquivalenceCase {
    EquivalenceCase {
        name: "local_float_return_literal",
        kind: LocalConstructKind::ReturnFloatLiteral,
        go_grammar_id: "go_local_float_return_literal_syntax_v0",
        go_metadata_path: GO_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
        go_round_trip_marker: "generated_go_local_float_return_literal_round_trips",
        csharp_grammar_id: "csharp_local_float_return_literal_syntax_v0",
        csharp_metadata_path: CSHARP_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
        csharp_round_trip_marker: "generated_csharp_local_float_return_literal_round_trips",
    }
}

fn boolean_return_equivalence_case() -> EquivalenceCase {
    EquivalenceCase {
        name: "local_boolean_return",
        kind: LocalConstructKind::ReturnBooleanLiteral,
        go_grammar_id: "go_local_boolean_return_identifier_syntax_v0",
        go_metadata_path: GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH,
        go_round_trip_marker: "generated_go_local_boolean_return_identifier_round_trips",
        csharp_grammar_id: "csharp_local_boolean_return_literal_syntax_v0",
        csharp_metadata_path: CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH,
        csharp_round_trip_marker: "generated_csharp_local_boolean_return_literal_round_trips",
    }
}

fn nil_null_return_equivalence_case() -> EquivalenceCase {
    EquivalenceCase {
        name: "local_nil_null_return",
        kind: LocalConstructKind::ReturnNullLiteral,
        go_grammar_id: "go_local_nil_return_identifier_syntax_v0",
        go_metadata_path: GO_LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH,
        go_round_trip_marker: "generated_go_local_nil_return_identifier_round_trips",
        csharp_grammar_id: "csharp_local_null_return_literal_syntax_v0",
        csharp_metadata_path: CSHARP_LOCAL_NULL_RETURN_LITERAL_METADATA_PATH,
        csharp_round_trip_marker: "generated_csharp_local_null_return_literal_round_trips",
    }
}

fn assert_equivalence_case_routes_are_registered(case: &EquivalenceCase) {
    let go = syntax_production_route_construct(case.go_grammar_id).unwrap_or_else(|| {
        panic!(
            "{}: Go route {} must stay registered",
            case.name, case.go_grammar_id
        )
    });
    let csharp = syntax_production_route_construct(case.csharp_grammar_id).unwrap_or_else(|| {
        panic!(
            "{}: C# route {} must stay registered",
            case.name, case.csharp_grammar_id
        )
    });

    assert_eq!(
        go.metadata_path, case.go_metadata_path,
        "{}: Go route metadata path drifted",
        case.name
    );
    assert_eq!(
        csharp.metadata_path, case.csharp_metadata_path,
        "{}: C# route metadata path drifted",
        case.name
    );
    assert!(
        go.generated_test_markers
            .contains(&case.go_round_trip_marker),
        "{}: Go route must expose generated round-trip marker {}",
        case.name,
        case.go_round_trip_marker
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&case.csharp_round_trip_marker),
        "{}: C# route must expose generated round-trip marker {}",
        case.name,
        case.csharp_round_trip_marker
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalConstructKind {
    ReturnStringLiteral,
    ReturnIntegerLiteral,
    ReturnFloatLiteral,
    ReturnBooleanLiteral,
    ReturnNullLiteral,
    TypedDeclaration,
    ImplicitDeclaration,
    ImplicitIntegerDeclaration,
    Assignment,
}

#[derive(Debug, PartialEq, Eq)]
struct LocalConstructBoundaryEvidence {
    comparable: CanonicalLocalConstruct,
    field_profiles: BTreeMap<String, FieldCardinality>,
    target_specific_payloads: BTreeSet<SemanticPayload>,
}

#[derive(Debug, PartialEq, Eq)]
struct CanonicalLocalConstruct {
    statement_sequence: Vec<&'static str>,
    semantic_payloads: BTreeSet<SemanticPayload>,
    semantic_references: BTreeSet<SemanticReference>,
    semantic_decision_edges: BTreeSet<SemanticDecisionEdge>,
    token_range_roles: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticPayload {
    role: String,
    payload_kind: String,
    codec: String,
}

impl SemanticPayload {
    fn new(role: &str, payload_kind: &str, codec: &str) -> Self {
        Self {
            role: role.to_string(),
            payload_kind: payload_kind.to_string(),
            codec: codec.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticReference {
    role: String,
    model_kind: String,
}

impl SemanticReference {
    fn new(role: &str, model_kind: &str) -> Self {
        Self {
            role: role.to_string(),
            model_kind: model_kind.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticDecisionEdge {
    role: String,
    crosses_payload_boundary: bool,
}

impl SemanticDecisionEdge {
    fn new(role: &str, crosses_payload_boundary: bool) -> Self {
        Self {
            role: role.to_string(),
            crosses_payload_boundary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FieldCardinality {
    required: bool,
    multiple: bool,
}

fn lower_go_sidecar(grammar_id: &str, metadata_path: &str) -> GrammarStructuralModelSidecar {
    let grammar = GoSyntaxMetadataAdapter::new(grammar_id)
        .lower_metadata_file(Path::new(metadata_path))
        .unwrap();
    lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap()
}

fn lower_csharp_sidecar(grammar_id: &str, metadata_path: &str) -> GrammarStructuralModelSidecar {
    let grammar = CSharpSyntaxMetadataAdapter::new(grammar_id)
        .lower_metadata_file(Path::new(metadata_path))
        .unwrap();
    lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap()
}

fn assert_sidecar_stays_at_syntax_boundary(sidecar: &GrammarStructuralModelSidecar) {
    assert_eq!(sidecar.artifact_kind, ARTIFACT_KIND);
    assert_eq!(sidecar.model_version, MODEL_VERSION);
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.proof.checks.values().all(|check| *check));
    assert_eq!(
        sidecar.structural_model.boundary.role,
        "sidecar_evidence_only"
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
}

fn go_local_construct_evidence(
    kind: LocalConstructKind,
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = go_statement_sequence(kind, sidecar);
    let mut evidence = empty_evidence(statement_sequence.clone());

    if statement_sequence.contains(&"local_declaration") {
        let target_rule = if matches!(
            kind,
            LocalConstructKind::ImplicitDeclaration
                | LocalConstructKind::ImplicitIntegerDeclaration
        ) {
            "AssignStmt"
        } else {
            "DeclStmt"
        };
        add_reference(
            &mut evidence,
            sidecar,
            "block.statement.local_declaration",
            "BlockStmt",
            "List",
            target_rule,
        );
    }
    if statement_sequence.contains(&"assignment") {
        add_reference(
            &mut evidence,
            sidecar,
            "block.statement.assignment",
            "BlockStmt",
            "List",
            "AssignStmt",
        );
    }
    if statement_sequence.contains(&"return") {
        add_reference(
            &mut evidence,
            sidecar,
            "block.statement.return",
            "BlockStmt",
            "List",
            "ReturnStmt",
        );
    }
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );

    match kind {
        LocalConstructKind::ReturnStringLiteral
        | LocalConstructKind::ReturnIntegerLiteral
        | LocalConstructKind::ReturnFloatLiteral => {
            add_reference(
                &mut evidence,
                sidecar,
                "return.value",
                "ReturnStmt",
                "Results",
                "BasicLit",
            );
            add_payload(
                &mut evidence,
                sidecar,
                "return.literal",
                "BasicLit",
                "Value",
                return_literal_node_kind(kind),
            );
            record_field(
                &mut evidence,
                sidecar,
                "return.value",
                "ReturnStmt",
                "Results",
            );
        }
        LocalConstructKind::ReturnBooleanLiteral | LocalConstructKind::ReturnNullLiteral => {
            add_reference(
                &mut evidence,
                sidecar,
                "return.value",
                "ReturnStmt",
                "Results",
                "Ident",
            );
            add_payload(
                &mut evidence,
                sidecar,
                "return.literal",
                "Ident",
                "Name",
                "identifier",
            );
            record_field(
                &mut evidence,
                sidecar,
                "return.value",
                "ReturnStmt",
                "Results",
            );
            if matches!(kind, LocalConstructKind::ReturnNullLiteral) {
                add_target_specific_payload(
                    &mut evidence,
                    sidecar,
                    "go.return_nil.identifier",
                    "Ident",
                    "Name",
                    "identifier",
                );
            }
        }
        LocalConstructKind::TypedDeclaration => {
            add_go_declaration(&mut evidence, sidecar, true);
            add_go_return_identifier(&mut evidence, sidecar);
        }
        LocalConstructKind::ImplicitDeclaration => {
            add_go_short_var_declaration(&mut evidence, sidecar, "string_literal");
            add_go_return_identifier(&mut evidence, sidecar);
        }
        LocalConstructKind::ImplicitIntegerDeclaration => {
            add_go_short_var_declaration(&mut evidence, sidecar, "integer_literal");
            add_go_return_identifier(&mut evidence, sidecar);
        }
        LocalConstructKind::Assignment => {
            add_go_declaration(&mut evidence, sidecar, false);
            add_go_assignment(&mut evidence, sidecar);
            add_go_return_identifier(&mut evidence, sidecar);
        }
    }

    evidence
}

fn csharp_local_construct_evidence(
    kind: LocalConstructKind,
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = csharp_statement_sequence(kind, sidecar);
    let mut evidence = empty_evidence(statement_sequence.clone());

    if statement_sequence.contains(&"local_declaration") {
        add_reference(
            &mut evidence,
            sidecar,
            "block.statement.local_declaration",
            "BlockSyntax",
            "Statements",
            "LocalDeclarationStatementSyntax",
        );
    }
    if statement_sequence.contains(&"assignment") {
        add_reference(
            &mut evidence,
            sidecar,
            "block.statement.assignment",
            "BlockSyntax",
            "Statements",
            "ExpressionStatementSyntax",
        );
    }
    if statement_sequence.contains(&"return") {
        add_reference(
            &mut evidence,
            sidecar,
            "block.statement.return",
            "BlockSyntax",
            "Statements",
            "ReturnStatementSyntax",
        );
    }
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );

    match kind {
        LocalConstructKind::ReturnStringLiteral
        | LocalConstructKind::ReturnIntegerLiteral
        | LocalConstructKind::ReturnFloatLiteral
        | LocalConstructKind::ReturnBooleanLiteral
        | LocalConstructKind::ReturnNullLiteral => {
            add_reference(
                &mut evidence,
                sidecar,
                "return.value",
                "ReturnStatementSyntax",
                "Expression",
                "LiteralExpressionSyntax",
            );
            add_payload(
                &mut evidence,
                sidecar,
                "return.literal",
                "LiteralExpressionSyntax",
                "Token",
                return_literal_node_kind(kind),
            );
            record_field(
                &mut evidence,
                sidecar,
                "return.value",
                "ReturnStatementSyntax",
                "Expression",
            );
            if matches!(kind, LocalConstructKind::ReturnNullLiteral) {
                add_target_specific_payload(
                    &mut evidence,
                    sidecar,
                    "csharp.return_null.literal",
                    "LiteralExpressionSyntax",
                    "Token",
                    "null_literal",
                );
            }
        }
        LocalConstructKind::TypedDeclaration => {
            add_csharp_declaration(&mut evidence, sidecar, true);
            add_csharp_return_identifier(&mut evidence, sidecar);
        }
        LocalConstructKind::ImplicitDeclaration => {
            add_csharp_implicit_declaration(&mut evidence, sidecar, "string_literal");
            add_csharp_return_identifier(&mut evidence, sidecar);
        }
        LocalConstructKind::ImplicitIntegerDeclaration => {
            add_csharp_implicit_declaration(&mut evidence, sidecar, "integer_literal");
            add_csharp_return_identifier(&mut evidence, sidecar);
        }
        LocalConstructKind::Assignment => {
            add_csharp_declaration(&mut evidence, sidecar, false);
            add_csharp_assignment(&mut evidence, sidecar);
            add_csharp_return_identifier(&mut evidence, sidecar);
        }
    }

    evidence
}

fn go_callable_wrapper_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    go_callable_wrapper_return_literal_evidence(sidecar, "integer_literal")
}

fn go_callable_wrapper_return_literal_evidence(
    sidecar: &GrammarStructuralModelSidecar,
    literal_node_kind: &'static str,
) -> LocalConstructBoundaryEvidence {
    go_callable_wrapper_return_value_evidence(sidecar, "BasicLit", "Value", literal_node_kind)
}

fn go_callable_wrapper_return_boolean_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    go_callable_wrapper_return_value_evidence(sidecar, "Ident", "Name", "identifier")
}

fn go_callable_wrapper_return_null_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence =
        go_callable_wrapper_return_value_evidence(sidecar, "Ident", "Name", "identifier");
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.return_nil.identifier",
        "Ident",
        "Name",
        "identifier",
    );
    evidence
}

fn go_callable_wrapper_return_value_evidence(
    sidecar: &GrammarStructuralModelSidecar,
    return_value_rule: &'static str,
    return_payload_field: &'static str,
    return_payload_node_kind: &'static str,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.declaration",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "callable.name",
        "FuncDecl",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.return_type",
        "Field",
        "Type",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "callable.return_type",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "FuncDecl",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.value",
        "ReturnStmt",
        "Results",
        return_value_rule,
    );
    add_payload(
        &mut evidence,
        sidecar,
        "return.literal",
        return_value_rule,
        return_payload_field,
        return_payload_node_kind,
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.declaration",
        "File",
        "Decls",
    );
    record_field(&mut evidence, sidecar, "callable.name", "FuncDecl", "Name");
    record_field(
        &mut evidence,
        sidecar,
        "callable.return_type",
        "Field",
        "Type",
    );
    record_field(&mut evidence, sidecar, "callable.body", "FuncDecl", "Body");
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.value",
        "ReturnStmt",
        "Results",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.return_type.identifier",
        "Ident",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_callable_wrapper_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    csharp_callable_wrapper_return_literal_evidence(sidecar, "int_keyword", "integer_literal")
}

fn csharp_callable_wrapper_return_literal_evidence(
    sidecar: &GrammarStructuralModelSidecar,
    return_type_node_kind: &'static str,
    literal_node_kind: &'static str,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.declaration",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "callable.name",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.return_type",
        "MethodDeclarationSyntax",
        "ReturnType",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "callable.return_type",
        "PredefinedTypeSyntax",
        "Keyword",
        return_type_node_kind,
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.value",
        "ReturnStatementSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "return.literal",
        "LiteralExpressionSyntax",
        "Token",
        literal_node_kind,
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.declaration",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.name",
        "MethodDeclarationSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.return_type",
        "MethodDeclarationSyntax",
        "ReturnType",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.value",
        "ReturnStatementSyntax",
        "Expression",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.return_type.keyword",
        "PredefinedTypeSyntax",
        "Keyword",
        return_type_node_kind,
    );

    evidence
}

fn csharp_callable_wrapper_return_null_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence =
        csharp_callable_wrapper_return_literal_evidence(sidecar, "object_keyword", "null_literal");
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.return_null.literal",
        "LiteralExpressionSyntax",
        "Token",
        "null_literal",
    );
    evidence
}

fn go_callable_parameter_list_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(Vec::new());

    add_reference(
        &mut evidence,
        sidecar,
        "callable.parameter_list",
        "FuncType",
        "Params",
        "FieldList",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.binding",
        "FieldList",
        "List",
        "Field",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.type",
        "Field",
        "Type",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.type",
        "Ident",
        "Name",
        "identifier",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.parameter_list",
        "FuncType",
        "Params",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.binding",
        "FieldList",
        "List",
    );
    record_field(&mut evidence, sidecar, "parameter.name", "Field", "Names");
    record_field(&mut evidence, sidecar, "parameter.type", "Field", "Type");

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.import.keyword",
        "GenDecl",
        "Tok",
        "import_keyword",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.import.path",
        "ImportSpec",
        "Path",
        "string_literal",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.parameter.type.identifier",
        "Ident",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_callable_parameter_list_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(Vec::new());

    add_reference(
        &mut evidence,
        sidecar,
        "callable.parameter_list",
        "MethodDeclarationSyntax",
        "ParameterList",
        "ParameterListSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.binding",
        "ParameterListSyntax",
        "Parameters",
        "ParameterSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.name",
        "ParameterSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.type",
        "ParameterSyntax",
        "Type",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.type",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.parameter_list",
        "MethodDeclarationSyntax",
        "ParameterList",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.binding",
        "ParameterListSyntax",
        "Parameters",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.name",
        "ParameterSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.type",
        "ParameterSyntax",
        "Type",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.return_type.keyword",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.parameter.type.keyword",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );

    evidence
}

fn go_member_list_container_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(Vec::new());

    add_reference(
        &mut evidence,
        sidecar,
        "member.data_declaration",
        "File",
        "Decls",
        "GenDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "member.callable_declaration",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "member.data.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "member.data.type",
        "TypeSpec",
        "Type",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "member.data.type",
        "Ident",
        "Name",
        "identifier",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "member.callable.name",
        "FuncDecl",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "member.callable.return_type",
        "Field",
        "Type",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "member.callable.return_type",
        "Ident",
        "Name",
        "identifier",
    );

    record_field(&mut evidence, sidecar, "member.list", "File", "Decls");
    record_field(
        &mut evidence,
        sidecar,
        "member.data.name",
        "TypeSpec",
        "Name",
    );
    record_field(
        &mut evidence,
        sidecar,
        "member.data.type",
        "TypeSpec",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "member.callable.name",
        "FuncDecl",
        "Name",
    );
    record_field(
        &mut evidence,
        sidecar,
        "member.callable.return_type",
        "Field",
        "Type",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.type.keyword",
        "GenDecl",
        "Tok",
        "type_keyword",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.return.literal",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    evidence
}

fn csharp_member_list_container_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(Vec::new());

    add_reference(
        &mut evidence,
        sidecar,
        "member.data_declaration",
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "member.callable_declaration",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "member.data.name",
        "PropertyDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "member.data.type",
        "PropertyDeclarationSyntax",
        "Type",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "member.data.type",
        "PredefinedTypeSyntax",
        "Keyword",
        "string_keyword",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "member.callable.name",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "member.callable.return_type",
        "MethodDeclarationSyntax",
        "ReturnType",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "member.callable.return_type",
        "PredefinedTypeSyntax",
        "Keyword",
        "string_keyword",
    );

    record_field(
        &mut evidence,
        sidecar,
        "member.list",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "member.data.name",
        "PropertyDeclarationSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "member.data.type",
        "PropertyDeclarationSyntax",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "member.callable.name",
        "MethodDeclarationSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "member.callable.return_type",
        "MethodDeclarationSyntax",
        "ReturnType",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.property.initializer",
        "PropertyDeclarationSyntax",
        "Initializer",
        "StringLiteralExpressionSyntax",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.return.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );

    evidence
}

fn go_data_member_field_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(Vec::new());

    add_reference(
        &mut evidence,
        sidecar,
        "data_member.container",
        "StructType",
        "Fields",
        "FieldList",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "data_member.binding",
        "FieldList",
        "List",
        "Field",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "data_member.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "data_member.type",
        "Field",
        "Type",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "data_member.type",
        "Ident",
        "Name",
        "identifier",
    );

    record_field(
        &mut evidence,
        sidecar,
        "data_member.container",
        "StructType",
        "Fields",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_member.binding",
        "FieldList",
        "List",
    );
    record_field(&mut evidence, sidecar, "data_member.name", "Field", "Names");
    record_field(&mut evidence, sidecar, "data_member.type", "Field", "Type");

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.type.keyword",
        "GenDecl",
        "Tok",
        "type_keyword",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.callable.name",
        "FuncDecl",
        "Name",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.callable.return.literal",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    evidence
}

fn csharp_data_member_field_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(Vec::new());

    add_reference(
        &mut evidence,
        sidecar,
        "data_member.container",
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "data_member.binding",
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "data_member.name",
        "PropertyDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "data_member.type",
        "PropertyDeclarationSyntax",
        "Type",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "data_member.type",
        "PredefinedTypeSyntax",
        "Keyword",
        "string_keyword",
    );

    record_field(
        &mut evidence,
        sidecar,
        "data_member.container",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_member.binding",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_member.name",
        "PropertyDeclarationSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_member.type",
        "PropertyDeclarationSyntax",
        "Type",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.property.initializer",
        "PropertyDeclarationSyntax",
        "Initializer",
        "StringLiteralExpressionSyntax",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.return.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );

    evidence
}

fn go_method_member_receiver_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "method_member.declaration",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "method_member.name",
        "FuncDecl",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "method_member.return_type",
        "Field",
        "Type",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "method_member.return_type",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "method_member.body",
        "FuncDecl",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.value",
        "ReturnStmt",
        "Results",
        "BasicLit",
    );

    record_field(
        &mut evidence,
        sidecar,
        "method_member.declaration",
        "File",
        "Decls",
    );
    record_field(
        &mut evidence,
        sidecar,
        "method_member.name",
        "FuncDecl",
        "Name",
    );
    record_field(
        &mut evidence,
        sidecar,
        "method_member.return_type",
        "Field",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "method_member.body",
        "FuncDecl",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.value",
        "ReturnStmt",
        "Results",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.type.keyword",
        "GenDecl",
        "Tok",
        "type_keyword",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.receiver.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.receiver.type",
        "Ident",
        "Name",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.method.return.literal",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    evidence
}

fn csharp_method_member_receiver_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "method_member.declaration",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "method_member.name",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "method_member.return_type",
        "MethodDeclarationSyntax",
        "ReturnType",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "method_member.return_type",
        "PredefinedTypeSyntax",
        "Keyword",
        "string_keyword",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "method_member.body",
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.value",
        "ReturnStatementSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );

    record_field(
        &mut evidence,
        sidecar,
        "method_member.declaration",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "method_member.name",
        "MethodDeclarationSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "method_member.return_type",
        "MethodDeclarationSyntax",
        "ReturnType",
    );
    record_field(
        &mut evidence,
        sidecar,
        "method_member.body",
        "MethodDeclarationSyntax",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.value",
        "ReturnStatementSyntax",
        "Expression",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.property.initializer",
        "PropertyDeclarationSyntax",
        "Initializer",
        "StringLiteralExpressionSyntax",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.return.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );

    evidence
}

fn go_package_data_declaration_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "data_declaration.container",
        "File",
        "Decls",
        "GenDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "data_declaration.binding",
        "GenDecl",
        "Specs",
        "ValueSpec",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "data_declaration.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "data_declaration.type",
        "ValueSpec",
        "Type",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "data_declaration.type",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "companion_member.callable",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "companion_member.return",
        "ReturnStmt",
        "Results",
        "Ident",
    );

    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.container",
        "File",
        "Decls",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.binding",
        "GenDecl",
        "Specs",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.name",
        "ValueSpec",
        "Names",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.type",
        "ValueSpec",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.initializer",
        "ValueSpec",
        "Values",
    );
    record_field(
        &mut evidence,
        sidecar,
        "companion_member.callable",
        "File",
        "Decls",
    );
    record_field(
        &mut evidence,
        sidecar,
        "companion_member.return",
        "ReturnStmt",
        "Results",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.var.keyword",
        "GenDecl",
        "Tok",
        "var_keyword",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.data.initializer.literal",
        "BasicLit",
        "Value",
        "integer_literal",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.companion.return.identifier",
        "Ident",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_package_data_declaration_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "data_declaration.container",
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "data_declaration.binding",
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "data_declaration.name",
        "PropertyDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "data_declaration.type",
        "PropertyDeclarationSyntax",
        "Type",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "data_declaration.type",
        "PredefinedTypeSyntax",
        "Keyword",
        "string_keyword",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "companion_member.callable",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "companion_member.return",
        "ReturnStatementSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );

    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.container",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.binding",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.name",
        "PropertyDeclarationSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.type",
        "PropertyDeclarationSyntax",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "data_declaration.initializer",
        "PropertyDeclarationSyntax",
        "Initializer",
    );
    record_field(
        &mut evidence,
        sidecar,
        "companion_member.callable",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "companion_member.return",
        "ReturnStatementSyntax",
        "Expression",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.data.initializer.string",
        "PropertyDeclarationSyntax",
        "Initializer",
        "StringLiteralExpressionSyntax",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.return.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );

    evidence
}

fn go_collection_initializer_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "FuncDecl",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.return.collection_type",
        "Field",
        "Type",
        "ArrayType",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.return.element_type",
        "ArrayType",
        "Elt",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "callable.return.element_type",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "collection.creation",
        "ReturnStmt",
        "Results",
        "CompositeLit",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "collection.creation.collection_type",
        "CompositeLit",
        "Type",
        "ArrayType",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "collection.creation.element_type",
        "ArrayType",
        "Elt",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "collection.creation.element_type",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "collection.initializer.element",
        "CompositeLit",
        "Elts",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "collection.initializer.integer",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
    );
    record_field(&mut evidence, sidecar, "callable.body", "FuncDecl", "Body");
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.return.collection_type",
        "Field",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.return.element_type",
        "ArrayType",
        "Elt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.creation",
        "ReturnStmt",
        "Results",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.creation.collection_type",
        "CompositeLit",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.creation.element_type",
        "ArrayType",
        "Elt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.initializer.element",
        "CompositeLit",
        "Elts",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.initializer.integer",
        "BasicLit",
        "Value",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.function.name",
        "FuncDecl",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_collection_initializer_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.return.collection_type",
        "MethodDeclarationSyntax",
        "ReturnType",
        "ArrayTypeSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.return.element_type",
        "ArrayTypeSyntax",
        "ElementType",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "callable.return.element_type",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "collection.creation",
        "ReturnStatementSyntax",
        "Expression",
        "ArrayCreationExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "collection.creation.collection_type",
        "ArrayCreationExpressionSyntax",
        "Type",
        "ArrayTypeSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "collection.creation.element_type",
        "ArrayTypeSyntax",
        "ElementType",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "collection.creation.element_type",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "collection.initializer.element",
        "InitializerExpressionSyntax",
        "Expressions",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "collection.initializer.integer",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.return.collection_type",
        "MethodDeclarationSyntax",
        "ReturnType",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.return.element_type",
        "ArrayTypeSyntax",
        "ElementType",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.creation",
        "ReturnStatementSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.creation.collection_type",
        "ArrayCreationExpressionSyntax",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.creation.element_type",
        "ArrayTypeSyntax",
        "ElementType",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.initializer.element",
        "InitializerExpressionSyntax",
        "Expressions",
    );
    record_field(
        &mut evidence,
        sidecar,
        "collection.initializer.integer",
        "LiteralExpressionSyntax",
        "Token",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.identifier",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );

    evidence
}

fn go_index_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "FuncDecl",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.array_type",
        "Field",
        "Type",
        "ArrayType",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "ArrayType",
        "Elt",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "Ident",
        "Name",
        "identifier",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "index.expression",
        "ReturnStmt",
        "Results",
        "IndexExpr",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "index.target",
        "IndexExpr",
        "X",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "index.target",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "index.argument",
        "IndexExpr",
        "Index",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "index.argument.integer",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
    );
    record_field(&mut evidence, sidecar, "callable.body", "FuncDecl", "Body");
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.array_type",
        "Field",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "ArrayType",
        "Elt",
    );
    record_field(&mut evidence, sidecar, "parameter.name", "Field", "Names");
    record_field(
        &mut evidence,
        sidecar,
        "index.expression",
        "ReturnStmt",
        "Results",
    );
    record_field(&mut evidence, sidecar, "index.target", "IndexExpr", "X");
    record_field(
        &mut evidence,
        sidecar,
        "index.argument",
        "IndexExpr",
        "Index",
    );
    record_field(
        &mut evidence,
        sidecar,
        "index.argument.integer",
        "BasicLit",
        "Value",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.function.name",
        "FuncDecl",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_element_access_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.array_type",
        "ParameterSyntax",
        "Type",
        "ArrayTypeSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "ArrayTypeSyntax",
        "ElementType",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.name",
        "ParameterSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "index.expression",
        "ReturnStatementSyntax",
        "Expression",
        "ElementAccessExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "index.target",
        "ElementAccessExpressionSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "index.target",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "index.argument",
        "BracketedArgumentListSyntax",
        "Arguments",
        "ArgumentSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "index.argument.integer",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.array_type",
        "ParameterSyntax",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "ArrayTypeSyntax",
        "ElementType",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.name",
        "ParameterSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "index.expression",
        "ReturnStatementSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "index.target",
        "ElementAccessExpressionSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "index.argument",
        "BracketedArgumentListSyntax",
        "Arguments",
    );
    record_field(
        &mut evidence,
        sidecar,
        "index.argument.integer",
        "LiteralExpressionSyntax",
        "Token",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.identifier",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.return_type",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );

    evidence
}

fn go_slice_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "FuncDecl",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.array_type",
        "Field",
        "Type",
        "ArrayType",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "ArrayType",
        "Elt",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "Ident",
        "Name",
        "identifier",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "range.expression",
        "ReturnStmt",
        "Results",
        "SliceExpr",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "range.target",
        "SliceExpr",
        "X",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "range.target",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "range.start",
        "SliceExpr",
        "Low",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "range.start.integer",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
    );
    record_field(&mut evidence, sidecar, "callable.body", "FuncDecl", "Body");
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.array_type",
        "Field",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "ArrayType",
        "Elt",
    );
    record_field(&mut evidence, sidecar, "parameter.name", "Field", "Names");
    record_field(
        &mut evidence,
        sidecar,
        "range.expression",
        "ReturnStmt",
        "Results",
    );
    record_field(&mut evidence, sidecar, "range.target", "SliceExpr", "X");
    record_field(&mut evidence, sidecar, "range.start", "SliceExpr", "Low");
    record_field(
        &mut evidence,
        sidecar,
        "range.start.integer",
        "BasicLit",
        "Value",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.function.name",
        "FuncDecl",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_range_element_access_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.array_type",
        "ParameterSyntax",
        "Type",
        "ArrayTypeSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "ArrayTypeSyntax",
        "ElementType",
        "PredefinedTypeSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parameter.name",
        "ParameterSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "range.expression",
        "ReturnStatementSyntax",
        "Expression",
        "ElementAccessExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "range.target",
        "ElementAccessExpressionSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "range.target",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "range.start",
        "RangeExpressionSyntax",
        "LeftOperand",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "range.start.integer",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.array_type",
        "ParameterSyntax",
        "Type",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.element_type",
        "ArrayTypeSyntax",
        "ElementType",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parameter.name",
        "ParameterSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "range.expression",
        "ReturnStatementSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "range.target",
        "ElementAccessExpressionSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "range.start",
        "RangeExpressionSyntax",
        "LeftOperand",
    );
    record_field(
        &mut evidence,
        sidecar,
        "range.start.integer",
        "LiteralExpressionSyntax",
        "Token",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.identifier",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    evidence
}

fn go_unary_integer_return_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "FuncDecl",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.unary_expression",
        "ReturnStmt",
        "Results",
        "UnaryExpr",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "unary.operator",
        "UnaryExpr",
        "Op",
        "unary_operator",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "unary.operand",
        "UnaryExpr",
        "X",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "unary.operand.integer",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
    );
    record_field(&mut evidence, sidecar, "callable.body", "FuncDecl", "Body");
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.unary_expression",
        "ReturnStmt",
        "Results",
    );
    record_field(&mut evidence, sidecar, "unary.operator", "UnaryExpr", "Op");
    record_field(&mut evidence, sidecar, "unary.operand", "UnaryExpr", "X");
    record_field(
        &mut evidence,
        sidecar,
        "unary.operand.integer",
        "BasicLit",
        "Value",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.function.name",
        "FuncDecl",
        "Name",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.return_type.identifier",
        "Ident",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_unary_integer_return_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.unary_expression",
        "ReturnStatementSyntax",
        "Expression",
        "PrefixUnaryExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "unary.operator",
        "PrefixUnaryExpressionSyntax",
        "OperatorToken",
        "unary_operator",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "unary.operand",
        "PrefixUnaryExpressionSyntax",
        "Operand",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "unary.operand.integer",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.unary_expression",
        "ReturnStatementSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "unary.operator",
        "PrefixUnaryExpressionSyntax",
        "OperatorToken",
    );
    record_field(
        &mut evidence,
        sidecar,
        "unary.operand",
        "PrefixUnaryExpressionSyntax",
        "Operand",
    );
    record_field(
        &mut evidence,
        sidecar,
        "unary.operand.integer",
        "LiteralExpressionSyntax",
        "Token",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.identifier",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.return_type.keyword",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );

    evidence
}

fn go_parenthesized_integer_return_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "FuncDecl",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.parenthesized_expression",
        "ReturnStmt",
        "Results",
        "ParenExpr",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parenthesized.operand",
        "ParenExpr",
        "X",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parenthesized.operand.integer",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
    );
    record_field(&mut evidence, sidecar, "callable.body", "FuncDecl", "Body");
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.parenthesized_expression",
        "ReturnStmt",
        "Results",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parenthesized.operand",
        "ParenExpr",
        "X",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parenthesized.operand.integer",
        "BasicLit",
        "Value",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.function.name",
        "FuncDecl",
        "Name",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.return_type.identifier",
        "Ident",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_parenthesized_integer_return_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.parenthesized_expression",
        "ReturnStatementSyntax",
        "Expression",
        "ParenthesizedExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "parenthesized.operand",
        "ParenthesizedExpressionSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "parenthesized.operand.integer",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.parenthesized_expression",
        "ReturnStatementSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parenthesized.operand",
        "ParenthesizedExpressionSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "parenthesized.operand.integer",
        "LiteralExpressionSyntax",
        "Token",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.identifier",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.return_type.keyword",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );

    evidence
}

fn go_binary_integer_return_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
        "FuncDecl",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "FuncDecl",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.binary_expression",
        "ReturnStmt",
        "Results",
        "BinaryExpr",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "binary.operator",
        "BinaryExpr",
        "Op",
        "binary_operator",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "binary.left_operand",
        "BinaryExpr",
        "X",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "binary.left_operand.integer",
        "BasicLit",
        "Value",
        "integer_literal",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "binary.right_operand",
        "BinaryExpr",
        "Y",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "binary.right_operand.integer",
        "BasicLit",
        "Value",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "File",
        "Decls",
    );
    record_field(&mut evidence, sidecar, "callable.body", "FuncDecl", "Body");
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.binary_expression",
        "ReturnStmt",
        "Results",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.operator",
        "BinaryExpr",
        "Op",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.left_operand",
        "BinaryExpr",
        "X",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.left_operand.integer",
        "BasicLit",
        "Value",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.right_operand",
        "BinaryExpr",
        "Y",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.right_operand.integer",
        "BasicLit",
        "Value",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.function.name",
        "FuncDecl",
        "Name",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "go.return_type.identifier",
        "Ident",
        "Name",
        "identifier",
    );

    evidence
}

fn csharp_binary_integer_return_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.binary_expression",
        "ReturnStatementSyntax",
        "Expression",
        "BinaryExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "binary.operator",
        "BinaryExpressionSyntax",
        "OperatorToken",
        "binary_operator",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "binary.left_operand",
        "BinaryExpressionSyntax",
        "Left",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "binary.left_operand.integer",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "binary.right_operand",
        "BinaryExpressionSyntax",
        "Right",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "binary.right_operand.integer",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );

    record_field(
        &mut evidence,
        sidecar,
        "callable.member_container",
        "ClassDeclarationSyntax",
        "Members",
    );
    record_field(
        &mut evidence,
        sidecar,
        "callable.body",
        "MethodDeclarationSyntax",
        "Body",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.binary_expression",
        "ReturnStatementSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.operator",
        "BinaryExpressionSyntax",
        "OperatorToken",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.left_operand",
        "BinaryExpressionSyntax",
        "Left",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.left_operand.integer",
        "LiteralExpressionSyntax",
        "Token",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.right_operand",
        "BinaryExpressionSyntax",
        "Right",
    );
    record_field(
        &mut evidence,
        sidecar,
        "binary.right_operand.integer",
        "LiteralExpressionSyntax",
        "Token",
    );

    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.namespace.name",
        "FileScopedNamespaceDeclarationSyntax",
        "Name",
        "qualified_identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.class.identifier",
        "ClassDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.method.identifier",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.return_type.keyword",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );

    evidence
}

fn go_call_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.call",
        "ReturnStmt",
        "Results",
        "CallExpr",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "call.callee",
        "CallExpr",
        "Fun",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "call.callee.identifier",
        "Ident",
        "Name",
        "identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.call",
        "ReturnStmt",
        "Results",
    );
    record_field(&mut evidence, sidecar, "call.callee", "CallExpr", "Fun");

    evidence
}

fn csharp_call_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "return.call",
        "ReturnStatementSyntax",
        "Expression",
        "InvocationExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "call.callee",
        "InvocationExpressionSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "call.callee.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    record_field(
        &mut evidence,
        sidecar,
        "return.call",
        "ReturnStatementSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "call.callee",
        "InvocationExpressionSyntax",
        "Expression",
    );

    evidence
}

fn go_argument_call_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = go_call_expression_evidence(sidecar);

    add_reference(
        &mut evidence,
        sidecar,
        "call.argument",
        "CallExpr",
        "Args",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "call.argument.literal",
        "BasicLit",
        "Value",
        "string_literal",
    );
    record_field(&mut evidence, sidecar, "call.argument", "CallExpr", "Args");

    evidence
}

fn csharp_argument_call_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = csharp_call_expression_evidence(sidecar);

    add_reference(
        &mut evidence,
        sidecar,
        "call.argument",
        "ArgumentSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "call.argument.literal",
        "LiteralExpressionSyntax",
        "Token",
        "string_literal",
    );
    record_field(
        &mut evidence,
        sidecar,
        "call.argument",
        "ArgumentSyntax",
        "Expression",
    );

    evidence
}

fn go_selector_member_access_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(vec!["member_access"]);

    add_reference(
        &mut evidence,
        sidecar,
        "access.expression",
        "CallExpr",
        "Fun",
        "SelectorExpr",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "access.receiver",
        "SelectorExpr",
        "X",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "access.receiver.identifier",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "access.member",
        "SelectorExpr",
        "Sel",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "access.member.identifier",
        "Ident",
        "Name",
        "identifier",
    );

    record_field(
        &mut evidence,
        sidecar,
        "access.expression",
        "CallExpr",
        "Fun",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.receiver",
        "SelectorExpr",
        "X",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.member",
        "SelectorExpr",
        "Sel",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.receiver.identifier",
        "Ident",
        "Name",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.member.identifier",
        "Ident",
        "Name",
    );

    evidence
}

fn csharp_member_access_expression_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(vec!["member_access"]);

    add_reference(
        &mut evidence,
        sidecar,
        "access.expression",
        "ReturnStatementSyntax",
        "Expression",
        "MemberAccessExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "access.receiver",
        "MemberAccessExpressionSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "access.receiver.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "access.member",
        "MemberAccessExpressionSyntax",
        "Name",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "access.member.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );

    record_field(
        &mut evidence,
        sidecar,
        "access.expression",
        "ReturnStatementSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.receiver",
        "MemberAccessExpressionSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.member",
        "MemberAccessExpressionSyntax",
        "Name",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.receiver.identifier",
        "IdentifierNameSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.member.identifier",
        "IdentifierNameSyntax",
        "Identifier",
    );

    evidence
}

fn go_selector_invocation_member_binding_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(vec!["conditional_member_access"]);

    add_reference(
        &mut evidence,
        sidecar,
        "access.member_binding",
        "CallExpr",
        "Fun",
        "SelectorExpr",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "access.receiver",
        "SelectorExpr",
        "X",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "access.receiver.identifier",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "access.member",
        "SelectorExpr",
        "Sel",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "access.member.identifier",
        "Ident",
        "Name",
        "identifier",
    );

    record_field(
        &mut evidence,
        sidecar,
        "access.member_binding",
        "CallExpr",
        "Fun",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.receiver",
        "SelectorExpr",
        "X",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.member",
        "SelectorExpr",
        "Sel",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.receiver.identifier",
        "Ident",
        "Name",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.member.identifier",
        "Ident",
        "Name",
    );

    evidence
}

fn csharp_conditional_access_member_binding_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let mut evidence = empty_evidence(vec!["conditional_member_access"]);

    add_reference(
        &mut evidence,
        sidecar,
        "access.member_binding",
        "InvocationExpressionSyntax",
        "Expression",
        "MemberBindingExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "access.receiver",
        "ConditionalAccessExpressionSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "access.receiver.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "access.member",
        "MemberBindingExpressionSyntax",
        "Name",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "access.member.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );

    record_field(
        &mut evidence,
        sidecar,
        "access.member_binding",
        "InvocationExpressionSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.receiver",
        "ConditionalAccessExpressionSyntax",
        "Expression",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.member",
        "MemberBindingExpressionSyntax",
        "Name",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.receiver.identifier",
        "IdentifierNameSyntax",
        "Identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "access.member.identifier",
        "IdentifierNameSyntax",
        "Identifier",
    );

    evidence
}

fn go_if_else_evidence(sidecar: &GrammarStructuralModelSidecar) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "DeclStmt" => Some("local_declaration"),
            "IfStmt" => Some("if_else"),
            "AssignStmt" => Some("assignment"),
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["local_declaration", "if_else", "assignment", "return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.local_declaration",
        "BlockStmt",
        "List",
        "DeclStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.if_else",
        "BlockStmt",
        "List",
        "IfStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.assignment",
        "BlockStmt",
        "List",
        "AssignStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );

    add_go_integer_declaration(&mut evidence, sidecar);
    add_reference(
        &mut evidence,
        sidecar,
        "if.condition",
        "IfStmt",
        "Cond",
        "BinaryExpr",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "if.then_body",
        "IfStmt",
        "Body",
        "BlockStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "if.else_body",
        "IfStmt",
        "Else",
        "BlockStmt",
    );
    record_field(&mut evidence, sidecar, "if.condition", "IfStmt", "Cond");
    record_field(&mut evidence, sidecar, "if.then_body", "IfStmt", "Body");
    record_field(&mut evidence, sidecar, "if.else_body", "IfStmt", "Else");

    add_go_integer_condition(&mut evidence, sidecar);
    add_go_integer_assignment(&mut evidence, sidecar);
    add_go_return_identifier(&mut evidence, sidecar);

    evidence
}

fn csharp_if_else_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "LocalDeclarationStatementSyntax" => Some("local_declaration"),
            "IfStatementSyntax" => Some("if_else"),
            "ExpressionStatementSyntax" => Some("assignment"),
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["local_declaration", "if_else", "assignment", "return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.local_declaration",
        "BlockSyntax",
        "Statements",
        "LocalDeclarationStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.if_else",
        "BlockSyntax",
        "Statements",
        "IfStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.assignment",
        "BlockSyntax",
        "Statements",
        "ExpressionStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );

    add_csharp_integer_declaration(&mut evidence, sidecar);
    add_reference(
        &mut evidence,
        sidecar,
        "if.condition",
        "IfStatementSyntax",
        "Condition",
        "BinaryExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "if.then_body",
        "IfStatementSyntax",
        "Statement",
        "BlockSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "if.else_body",
        "ElseClauseSyntax",
        "Statement",
        "BlockSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "if.condition",
        "IfStatementSyntax",
        "Condition",
    );
    record_field(
        &mut evidence,
        sidecar,
        "if.then_body",
        "IfStatementSyntax",
        "Statement",
    );
    record_field(
        &mut evidence,
        sidecar,
        "if.else_body",
        "ElseClauseSyntax",
        "Statement",
    );

    add_csharp_integer_condition(&mut evidence, sidecar);
    add_csharp_integer_assignment(&mut evidence, sidecar);
    add_csharp_return_identifier(&mut evidence, sidecar);

    evidence
}

fn go_for_condition_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "DeclStmt" => Some("local_declaration"),
            "ForStmt" => Some("loop"),
            "AssignStmt" => Some("assignment"),
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec!["local_declaration", "loop", "assignment", "return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.local_declaration",
        "BlockStmt",
        "List",
        "DeclStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.loop",
        "BlockStmt",
        "List",
        "ForStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.assignment",
        "BlockStmt",
        "List",
        "AssignStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );

    add_go_integer_declaration(&mut evidence, sidecar);
    add_reference(
        &mut evidence,
        sidecar,
        "loop.condition",
        "ForStmt",
        "Cond",
        "BinaryExpr",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "loop.body",
        "ForStmt",
        "Body",
        "BlockStmt",
    );
    record_field(&mut evidence, sidecar, "loop.condition", "ForStmt", "Cond");
    record_field(&mut evidence, sidecar, "loop.body", "ForStmt", "Body");

    add_go_integer_condition(&mut evidence, sidecar);
    add_go_integer_assignment(&mut evidence, sidecar);
    add_go_return_identifier(&mut evidence, sidecar);

    evidence
}

fn csharp_for_condition_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "LocalDeclarationStatementSyntax" => Some("local_declaration"),
            "ForStatementSyntax" => Some("loop"),
            "ExpressionStatementSyntax" => Some("assignment"),
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec!["local_declaration", "loop", "assignment", "return"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.local_declaration",
        "BlockSyntax",
        "Statements",
        "LocalDeclarationStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.loop",
        "BlockSyntax",
        "Statements",
        "ForStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.assignment",
        "BlockSyntax",
        "Statements",
        "ExpressionStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );

    add_csharp_integer_declaration(&mut evidence, sidecar);
    add_reference(
        &mut evidence,
        sidecar,
        "loop.condition",
        "ForStatementSyntax",
        "Condition",
        "BinaryExpressionSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "loop.body",
        "ForStatementSyntax",
        "Statement",
        "BlockSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "loop.condition",
        "ForStatementSyntax",
        "Condition",
    );
    record_field(
        &mut evidence,
        sidecar,
        "loop.body",
        "ForStatementSyntax",
        "Statement",
    );

    add_csharp_integer_condition_with_operator(&mut evidence, sidecar, "less_than_token");
    add_csharp_integer_assignment(&mut evidence, sidecar);
    add_csharp_return_identifier(&mut evidence, sidecar);

    evidence
}

fn go_postfix_simple_statement_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "IncDecStmt" => Some("simple_statement_update"),
            _ => None,
        },
        vec!["simple_statement_update"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.simple_statement_update",
        "BlockStmt",
        "List",
        "IncDecStmt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "update.target",
        "IncDecStmt",
        "X",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "update.target",
        "Ident",
        "Name",
        "identifier",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "update.operator",
        "IncDecStmt",
        "Tok",
        "increment_operator",
    );
    record_field(&mut evidence, sidecar, "update.target", "IncDecStmt", "X");
    record_field(
        &mut evidence,
        sidecar,
        "update.operator",
        "IncDecStmt",
        "Tok",
    );

    evidence
}

fn csharp_postfix_simple_statement_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "ExpressionStatementSyntax" => Some("simple_statement_update"),
            _ => None,
        },
        vec!["simple_statement_update"],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.simple_statement_update",
        "BlockSyntax",
        "Statements",
        "ExpressionStatementSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "update.target",
        "PostfixUnaryExpressionSyntax",
        "Operand",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "update.target",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "update.operator",
        "PostfixUnaryExpressionSyntax",
        "OperatorToken",
        "postfix_operator",
    );
    record_field(
        &mut evidence,
        sidecar,
        "update.target",
        "PostfixUnaryExpressionSyntax",
        "Operand",
    );
    record_field(
        &mut evidence,
        sidecar,
        "update.operator",
        "PostfixUnaryExpressionSyntax",
        "OperatorToken",
    );

    evidence
}

fn go_switch_evidence(sidecar: &GrammarStructuralModelSidecar) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "DeclStmt" => Some("local_declaration"),
            "SwitchStmt" => Some("switch"),
            "AssignStmt" => Some("case_body_assignment"),
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec![
            "local_declaration",
            "switch",
            "case_body_assignment",
            "return",
        ],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.local_declaration",
        "BlockStmt",
        "List",
        "DeclStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.switch",
        "BlockStmt",
        "List",
        "SwitchStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.case_body_assignment",
        "BlockStmt",
        "List",
        "AssignStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );

    add_go_integer_declaration(&mut evidence, sidecar);
    add_reference(
        &mut evidence,
        sidecar,
        "switch.selector",
        "SwitchStmt",
        "Tag",
        "Ident",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "switch.selector.identifier",
        "Ident",
        "Name",
        "identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "switch.selector",
        "SwitchStmt",
        "Tag",
    );

    add_reference(
        &mut evidence,
        sidecar,
        "case.label",
        "CaseClause",
        "List",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "case.label.literal",
        "BasicLit",
        "Value",
        "integer_literal",
    );
    record_field(&mut evidence, sidecar, "case.label", "CaseClause", "List");

    add_reference(
        &mut evidence,
        sidecar,
        "case.body.assignment",
        "CaseClause",
        "Body",
        "AssignStmt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "case.body.assignment",
        "CaseClause",
        "Body",
    );

    add_go_integer_assignment(&mut evidence, sidecar);
    add_go_return_identifier(&mut evidence, sidecar);

    evidence
}

fn csharp_switch_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "LocalDeclarationStatementSyntax" => Some("local_declaration"),
            "SwitchStatementSyntax" => Some("switch"),
            "ExpressionStatementSyntax" => Some("case_body_assignment"),
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec![
            "local_declaration",
            "switch",
            "case_body_assignment",
            "return",
        ],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.local_declaration",
        "BlockSyntax",
        "Statements",
        "LocalDeclarationStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.switch",
        "BlockSyntax",
        "Statements",
        "SwitchStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.case_body_assignment",
        "BlockSyntax",
        "Statements",
        "ExpressionStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );

    add_csharp_integer_declaration(&mut evidence, sidecar);
    add_reference(
        &mut evidence,
        sidecar,
        "switch.selector",
        "SwitchStatementSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "switch.selector.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    record_field(
        &mut evidence,
        sidecar,
        "switch.selector",
        "SwitchStatementSyntax",
        "Expression",
    );

    add_reference(
        &mut evidence,
        sidecar,
        "case.label",
        "CaseSwitchLabelSyntax",
        "Value",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "case.label.literal",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );
    record_field(
        &mut evidence,
        sidecar,
        "case.label",
        "CaseSwitchLabelSyntax",
        "Value",
    );

    add_reference(
        &mut evidence,
        sidecar,
        "case.body.assignment",
        "SwitchSectionSyntax",
        "Statements",
        "ExpressionStatementSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "case.body.assignment",
        "SwitchSectionSyntax",
        "Statements",
    );

    add_csharp_integer_assignment(&mut evidence, sidecar);
    add_csharp_return_identifier(&mut evidence, sidecar);

    evidence
}

fn go_string_iteration_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "DeclStmt" => Some("local_declaration"),
            "RangeStmt" => Some("iteration_loop"),
            "AssignStmt" => Some("body_assignment"),
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        vec![
            "local_declaration",
            "iteration_loop",
            "body_assignment",
            "return",
        ],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.local_declaration",
        "BlockStmt",
        "List",
        "DeclStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.iteration_loop",
        "BlockStmt",
        "List",
        "RangeStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.body_assignment",
        "BlockStmt",
        "List",
        "AssignStmt",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockStmt",
        "List",
        "ReturnStmt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockStmt",
        "List",
    );

    add_go_integer_declaration(&mut evidence, sidecar);
    add_reference(
        &mut evidence,
        sidecar,
        "iteration.source",
        "RangeStmt",
        "X",
        "BasicLit",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "iteration.source.literal",
        "BasicLit",
        "Value",
        "string_literal",
    );
    record_field(&mut evidence, sidecar, "iteration.source", "RangeStmt", "X");
    add_reference(
        &mut evidence,
        sidecar,
        "iteration.body",
        "RangeStmt",
        "Body",
        "BlockStmt",
    );
    record_field(
        &mut evidence,
        sidecar,
        "iteration.body",
        "RangeStmt",
        "Body",
    );

    add_go_integer_assignment(&mut evidence, sidecar);
    add_go_return_identifier(&mut evidence, sidecar);

    evidence
}

fn csharp_string_iteration_evidence(
    sidecar: &GrammarStructuralModelSidecar,
) -> LocalConstructBoundaryEvidence {
    let statement_sequence = statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "LocalDeclarationStatementSyntax" => Some("local_declaration"),
            "ForEachStatementSyntax" => Some("iteration_loop"),
            "ExpressionStatementSyntax" => Some("body_assignment"),
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        vec![
            "local_declaration",
            "iteration_loop",
            "body_assignment",
            "return",
        ],
    );
    let mut evidence = empty_evidence(statement_sequence);

    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.local_declaration",
        "BlockSyntax",
        "Statements",
        "LocalDeclarationStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.iteration_loop",
        "BlockSyntax",
        "Statements",
        "ForEachStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.body_assignment",
        "BlockSyntax",
        "Statements",
        "ExpressionStatementSyntax",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "block.statement.return",
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "block.statements",
        "BlockSyntax",
        "Statements",
    );

    add_csharp_integer_declaration(&mut evidence, sidecar);
    add_reference(
        &mut evidence,
        sidecar,
        "iteration.source",
        "ForEachStatementSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    add_payload(
        &mut evidence,
        sidecar,
        "iteration.source.literal",
        "LiteralExpressionSyntax",
        "Token",
        "string_literal",
    );
    record_field(
        &mut evidence,
        sidecar,
        "iteration.source",
        "ForEachStatementSyntax",
        "Expression",
    );
    add_reference(
        &mut evidence,
        sidecar,
        "iteration.body",
        "ForEachStatementSyntax",
        "Statement",
        "BlockSyntax",
    );
    record_field(
        &mut evidence,
        sidecar,
        "iteration.body",
        "ForEachStatementSyntax",
        "Statement",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.iteration.type_keyword",
        "PredefinedTypeSyntax",
        "Keyword",
        "char_keyword",
    );
    add_target_specific_payload(
        &mut evidence,
        sidecar,
        "csharp.iteration.variable",
        "ForEachStatementSyntax",
        "Identifier",
        "identifier",
    );

    add_csharp_integer_assignment(&mut evidence, sidecar);
    add_csharp_return_identifier(&mut evidence, sidecar);

    evidence
}

fn empty_evidence(statement_sequence: Vec<&'static str>) -> LocalConstructBoundaryEvidence {
    LocalConstructBoundaryEvidence {
        comparable: CanonicalLocalConstruct {
            statement_sequence,
            semantic_payloads: BTreeSet::new(),
            semantic_references: BTreeSet::new(),
            semantic_decision_edges: BTreeSet::new(),
            token_range_roles: BTreeSet::new(),
        },
        field_profiles: BTreeMap::new(),
        target_specific_payloads: BTreeSet::new(),
    }
}

fn add_go_declaration(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    typed: bool,
) {
    add_reference(
        evidence,
        sidecar,
        "declaration.statement",
        "DeclStmt",
        "Decl",
        "GenDecl",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.binding_list",
        "GenDecl",
        "Specs",
        "ValueSpec",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.name",
        "ValueSpec",
        "Names",
        "Ident",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.initializer",
        "ValueSpec",
        "Values",
        "BasicLit",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.initializer",
        "BasicLit",
        "Value",
        "string_literal",
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "go.declaration.keyword",
        "GenDecl",
        "Tok",
        "var_keyword",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.binding_list",
        "GenDecl",
        "Specs",
    );
    record_field(evidence, sidecar, "declaration.name", "ValueSpec", "Names");
    record_field(
        evidence,
        sidecar,
        "declaration.initializer",
        "ValueSpec",
        "Values",
    );

    if typed {
        add_reference(
            evidence,
            sidecar,
            "declaration.type",
            "ValueSpec",
            "Type",
            "Ident",
        );
        add_payload(
            evidence,
            sidecar,
            "declaration.type",
            "Ident",
            "Name",
            "identifier",
        );
        record_field(evidence, sidecar, "declaration.type", "ValueSpec", "Type");
    }
}

fn add_go_short_var_declaration(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    initializer_node_kind: &'static str,
) {
    add_reference(
        evidence,
        sidecar,
        "declaration.name",
        "AssignStmt",
        "Lhs",
        "Ident",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.initializer",
        "AssignStmt",
        "Rhs",
        "BasicLit",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.initializer",
        "BasicLit",
        "Value",
        initializer_node_kind,
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "go.declaration.define_operator",
        "AssignStmt",
        "Tok",
        "define_operator",
    );
    record_field(evidence, sidecar, "declaration.name", "AssignStmt", "Lhs");
    record_field(
        evidence,
        sidecar,
        "declaration.initializer",
        "AssignStmt",
        "Rhs",
    );
}

fn add_csharp_declaration(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    typed_equivalence_role: bool,
) {
    add_reference(
        evidence,
        sidecar,
        "declaration.statement",
        "LocalDeclarationStatementSyntax",
        "Declaration",
        "VariableDeclarationSyntax",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.binding_list",
        "VariableDeclarationSyntax",
        "Variables",
        "VariableDeclaratorSyntax",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclarationSyntax",
        "Variables",
        "VariableDeclaratorSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclaratorSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.initializer",
        "EqualsValueClauseSyntax",
        "Value",
        "LiteralExpressionSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.initializer",
        "LiteralExpressionSyntax",
        "Token",
        "string_literal",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.binding_list",
        "VariableDeclarationSyntax",
        "Variables",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclaratorSyntax",
        "Identifier",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.initializer",
        "EqualsValueClauseSyntax",
        "Value",
    );

    if typed_equivalence_role {
        add_reference(
            evidence,
            sidecar,
            "declaration.type",
            "VariableDeclarationSyntax",
            "Type",
            "PredefinedTypeSyntax",
        );
        add_payload(
            evidence,
            sidecar,
            "declaration.type",
            "PredefinedTypeSyntax",
            "Keyword",
            "string_keyword",
        );
        record_field(
            evidence,
            sidecar,
            "declaration.type",
            "VariableDeclarationSyntax",
            "Type",
        );
    } else {
        add_target_specific_payload(
            evidence,
            sidecar,
            "csharp.declaration.required_type_keyword",
            "PredefinedTypeSyntax",
            "Keyword",
            "string_keyword",
        );
    }
}

fn add_csharp_implicit_declaration(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    initializer_node_kind: &'static str,
) {
    add_reference(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclarationSyntax",
        "Variables",
        "VariableDeclaratorSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclaratorSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.initializer",
        "EqualsValueClauseSyntax",
        "Value",
        "LiteralExpressionSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.initializer",
        "LiteralExpressionSyntax",
        "Token",
        initializer_node_kind,
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "csharp.declaration.implicit_type_keyword",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclaratorSyntax",
        "Identifier",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.initializer",
        "EqualsValueClauseSyntax",
        "Value",
    );
}

fn add_go_integer_declaration(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "declaration.statement",
        "DeclStmt",
        "Decl",
        "GenDecl",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.binding_list",
        "GenDecl",
        "Specs",
        "ValueSpec",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.name",
        "ValueSpec",
        "Names",
        "Ident",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.name",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.initializer",
        "ValueSpec",
        "Values",
        "BasicLit",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.initializer",
        "BasicLit",
        "Value",
        "integer_literal",
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "go.declaration.keyword",
        "GenDecl",
        "Tok",
        "var_keyword",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.binding_list",
        "GenDecl",
        "Specs",
    );
    record_field(evidence, sidecar, "declaration.name", "ValueSpec", "Names");
    record_field(
        evidence,
        sidecar,
        "declaration.initializer",
        "ValueSpec",
        "Values",
    );
}

fn add_csharp_integer_declaration(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "declaration.statement",
        "LocalDeclarationStatementSyntax",
        "Declaration",
        "VariableDeclarationSyntax",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.binding_list",
        "VariableDeclarationSyntax",
        "Variables",
        "VariableDeclaratorSyntax",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclarationSyntax",
        "Variables",
        "VariableDeclaratorSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclaratorSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "declaration.initializer",
        "EqualsValueClauseSyntax",
        "Value",
        "LiteralExpressionSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "declaration.initializer",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "csharp.declaration.required_type_keyword",
        "PredefinedTypeSyntax",
        "Keyword",
        "int_keyword",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.binding_list",
        "VariableDeclarationSyntax",
        "Variables",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.name",
        "VariableDeclaratorSyntax",
        "Identifier",
    );
    record_field(
        evidence,
        sidecar,
        "declaration.initializer",
        "EqualsValueClauseSyntax",
        "Value",
    );
}

fn add_go_integer_condition(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "condition.left",
        "BinaryExpr",
        "X",
        "Ident",
    );
    add_payload(
        evidence,
        sidecar,
        "condition.left.identifier",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "condition.right",
        "BinaryExpr",
        "Y",
        "BasicLit",
    );
    add_payload(
        evidence,
        sidecar,
        "condition.right.literal",
        "BasicLit",
        "Value",
        "integer_literal",
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "go.condition.operator",
        "BinaryExpr",
        "Op",
        "binary_operator",
    );
    record_field(evidence, sidecar, "condition.left", "BinaryExpr", "X");
    record_field(evidence, sidecar, "condition.right", "BinaryExpr", "Y");
}

fn add_csharp_integer_condition(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_csharp_integer_condition_with_operator(evidence, sidecar, "greater_than_token");
}

fn add_csharp_integer_condition_with_operator(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    operator_node_kind: &'static str,
) {
    add_reference(
        evidence,
        sidecar,
        "condition.left",
        "BinaryExpressionSyntax",
        "Left",
        "IdentifierNameSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "condition.left.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "condition.right",
        "BinaryExpressionSyntax",
        "Right",
        "LiteralExpressionSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "condition.right.literal",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "csharp.condition.operator",
        "BinaryExpressionSyntax",
        "OperatorToken",
        operator_node_kind,
    );
    record_field(
        evidence,
        sidecar,
        "condition.left",
        "BinaryExpressionSyntax",
        "Left",
    );
    record_field(
        evidence,
        sidecar,
        "condition.right",
        "BinaryExpressionSyntax",
        "Right",
    );
}

fn add_go_integer_assignment(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "assignment.target",
        "AssignStmt",
        "Lhs",
        "Ident",
    );
    add_payload(
        evidence,
        sidecar,
        "assignment.target",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "assignment.value",
        "AssignStmt",
        "Rhs",
        "BasicLit",
    );
    add_payload(
        evidence,
        sidecar,
        "assignment.value",
        "BasicLit",
        "Value",
        "integer_literal",
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "go.assignment.operator",
        "AssignStmt",
        "Tok",
        "assign_operator",
    );
    record_field(evidence, sidecar, "assignment.target", "AssignStmt", "Lhs");
    record_field(evidence, sidecar, "assignment.value", "AssignStmt", "Rhs");
}

fn add_csharp_integer_assignment(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "assignment.target",
        "AssignmentExpressionSyntax",
        "Left",
        "IdentifierNameSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "assignment.target",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "assignment.value",
        "AssignmentExpressionSyntax",
        "Right",
        "LiteralExpressionSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "assignment.value",
        "LiteralExpressionSyntax",
        "Token",
        "integer_literal",
    );
    record_field(
        evidence,
        sidecar,
        "assignment.target",
        "AssignmentExpressionSyntax",
        "Left",
    );
    record_field(
        evidence,
        sidecar,
        "assignment.value",
        "AssignmentExpressionSyntax",
        "Right",
    );
}

fn add_go_assignment(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "assignment.target",
        "AssignStmt",
        "Lhs",
        "Ident",
    );
    add_payload(
        evidence,
        sidecar,
        "assignment.target",
        "Ident",
        "Name",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "assignment.value",
        "AssignStmt",
        "Rhs",
        "BasicLit",
    );
    add_payload(
        evidence,
        sidecar,
        "assignment.value",
        "BasicLit",
        "Value",
        "string_literal",
    );
    add_target_specific_payload(
        evidence,
        sidecar,
        "go.assignment.operator",
        "AssignStmt",
        "Tok",
        "assign_operator",
    );
    record_field(evidence, sidecar, "assignment.target", "AssignStmt", "Lhs");
    record_field(evidence, sidecar, "assignment.value", "AssignStmt", "Rhs");
}

fn add_csharp_assignment(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "assignment.target",
        "AssignmentExpressionSyntax",
        "Left",
        "IdentifierNameSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "assignment.target",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    add_reference(
        evidence,
        sidecar,
        "assignment.value",
        "AssignmentExpressionSyntax",
        "Right",
        "LiteralExpressionSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "assignment.value",
        "LiteralExpressionSyntax",
        "Token",
        "string_literal",
    );
    record_field(
        evidence,
        sidecar,
        "assignment.target",
        "AssignmentExpressionSyntax",
        "Left",
    );
    record_field(
        evidence,
        sidecar,
        "assignment.value",
        "AssignmentExpressionSyntax",
        "Right",
    );
}

fn add_go_return_identifier(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "return.value",
        "ReturnStmt",
        "Results",
        "Ident",
    );
    add_payload(
        evidence,
        sidecar,
        "return.identifier",
        "Ident",
        "Name",
        "identifier",
    );
    record_field(evidence, sidecar, "return.value", "ReturnStmt", "Results");
}

fn add_csharp_return_identifier(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
) {
    add_reference(
        evidence,
        sidecar,
        "return.value",
        "ReturnStatementSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    add_payload(
        evidence,
        sidecar,
        "return.identifier",
        "IdentifierNameSyntax",
        "Identifier",
        "identifier",
    );
    record_field(
        evidence,
        sidecar,
        "return.value",
        "ReturnStatementSyntax",
        "Expression",
    );
}

fn add_reference(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    role: &str,
    owner_rule: &str,
    field: &str,
    target_rule: &str,
) {
    let reference = sidecar
        .structural_model
        .references
        .iter()
        .find(|reference| {
            reference.owner_rule == owner_rule
                && reference.field == field
                && reference.target_rule == target_rule
        })
        .unwrap_or_else(|| panic!("missing reference {owner_rule}.{field} -> {target_rule}"));

    evidence
        .comparable
        .semantic_references
        .insert(SemanticReference::new(role, &reference.model_kind));
    add_decision_edge(
        evidence,
        sidecar,
        role,
        owner_rule,
        field,
        target_rule,
        false,
    );
}

fn add_payload(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    role: &str,
    owner_rule: &str,
    field: &str,
    node_kind: &str,
) {
    let payload = sidecar
        .structural_model
        .payload_leaves
        .iter()
        .find(|payload| {
            payload.owner_rule == owner_rule
                && payload.field == field
                && payload.node_kind == node_kind
        })
        .unwrap_or_else(|| panic!("missing payload {owner_rule}.{field} -> {node_kind}"));

    evidence
        .comparable
        .semantic_payloads
        .insert(SemanticPayload::new(
            role,
            &payload.payload_kind,
            &payload.codec,
        ));
    add_decision_edge(evidence, sidecar, role, owner_rule, field, node_kind, true);
}

fn add_target_specific_payload(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    role: &str,
    owner_rule: &str,
    field: &str,
    node_kind: &str,
) {
    let payload = sidecar
        .structural_model
        .payload_leaves
        .iter()
        .find(|payload| {
            payload.owner_rule == owner_rule
                && payload.field == field
                && payload.node_kind == node_kind
        })
        .unwrap_or_else(|| {
            panic!("missing target-specific payload {owner_rule}.{field} -> {node_kind}")
        });

    evidence
        .target_specific_payloads
        .insert(SemanticPayload::new(
            role,
            &payload.payload_kind,
            &payload.codec,
        ));
}

fn assert_reference_exists(
    sidecar: &GrammarStructuralModelSidecar,
    owner_rule: &str,
    field: &str,
    target_rule: &str,
) {
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == owner_rule
                && reference.field == field
                && reference.target_rule == target_rule),
        "missing reference {owner_rule}.{field} -> {target_rule}"
    );
}

fn assert_field_absent(sidecar: &GrammarStructuralModelSidecar, owner_rule: &str, field: &str) {
    let record = sidecar
        .structural_model
        .records
        .iter()
        .find(|record| record.grammar_rule == owner_rule)
        .unwrap_or_else(|| panic!("missing structural record {owner_rule}"));
    assert!(
        !record
            .fields
            .iter()
            .any(|candidate| candidate.name == field),
        "unexpected field {owner_rule}.{field}"
    );
}

fn assert_field_cardinality(
    sidecar: &GrammarStructuralModelSidecar,
    owner_rule: &str,
    field_name: &str,
    expected: FieldCardinality,
) {
    let record = sidecar
        .structural_model
        .records
        .iter()
        .find(|record| record.grammar_rule == owner_rule)
        .unwrap_or_else(|| panic!("missing structural record {owner_rule}"));
    let field = record
        .fields
        .iter()
        .find(|candidate| candidate.name == field_name)
        .unwrap_or_else(|| panic!("missing field {owner_rule}.{field_name}"));
    assert_eq!(
        FieldCardinality {
            required: field.required,
            multiple: field.multiple,
        },
        expected,
        "{owner_rule}.{field_name} cardinality changed"
    );
}

fn add_decision_edge(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    role: &str,
    owner_rule: &str,
    field: &str,
    node_kind: &str,
    crosses_payload_boundary: bool,
) {
    let source = format!("decision.rule.{owner_rule}.field.{field}");
    let target = format!("decision.rule.{owner_rule}.field.{field}.alternative.{node_kind}");
    let edge = sidecar
        .structural_model
        .decision_graph
        .edges
        .iter()
        .find(|edge| edge.source == source && edge.target == target)
        .unwrap_or_else(|| panic!("missing decision edge {source} -> {target}"));
    assert_eq!(edge.crosses_payload_boundary, crosses_payload_boundary);

    evidence
        .comparable
        .semantic_decision_edges
        .insert(SemanticDecisionEdge::new(role, crosses_payload_boundary));
    insert_decision_node_range_role(evidence, sidecar, &source);
    insert_decision_node_range_role(evidence, sidecar, &target);
}

fn insert_decision_node_range_role(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    node_id: &str,
) {
    let Some(token_range_id) = sidecar
        .structural_model
        .decision_graph
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .and_then(|node| node.token_range.as_ref())
    else {
        return;
    };
    let range = sidecar
        .structural_model
        .token_ranges
        .ranges
        .iter()
        .find(|range| range.id == *token_range_id)
        .unwrap_or_else(|| panic!("missing token range {token_range_id}"));
    evidence
        .comparable
        .token_range_roles
        .insert(range.role.clone());
}

fn record_field(
    evidence: &mut LocalConstructBoundaryEvidence,
    sidecar: &GrammarStructuralModelSidecar,
    role: &str,
    owner_rule: &str,
    field: &str,
) {
    let record = sidecar
        .structural_model
        .records
        .iter()
        .find(|record| record.grammar_rule == owner_rule)
        .unwrap_or_else(|| panic!("missing structural record {owner_rule}"));
    let field = record
        .fields
        .iter()
        .find(|candidate| candidate.name == field)
        .unwrap_or_else(|| panic!("missing field {owner_rule}.{field}"));
    evidence.field_profiles.insert(
        role.to_string(),
        FieldCardinality {
            required: field.required,
            multiple: field.multiple,
        },
    );
}

fn go_statement_sequence(
    kind: LocalConstructKind,
    sidecar: &GrammarStructuralModelSidecar,
) -> Vec<&'static str> {
    let assign_stmt_role = if matches!(
        kind,
        LocalConstructKind::ImplicitDeclaration | LocalConstructKind::ImplicitIntegerDeclaration
    ) {
        "local_declaration"
    } else {
        "assignment"
    };
    statement_sequence(
        sidecar,
        "BlockStmt",
        "List",
        |node_kind| match node_kind {
            "DeclStmt" => Some("local_declaration"),
            "AssignStmt" => Some(assign_stmt_role),
            "ReturnStmt" => Some("return"),
            _ => None,
        },
        expected_statement_sequence(kind),
    )
}

fn csharp_statement_sequence(
    kind: LocalConstructKind,
    sidecar: &GrammarStructuralModelSidecar,
) -> Vec<&'static str> {
    statement_sequence(
        sidecar,
        "BlockSyntax",
        "Statements",
        |node_kind| match node_kind {
            "LocalDeclarationStatementSyntax" => Some("local_declaration"),
            "ExpressionStatementSyntax" => Some("assignment"),
            "ReturnStatementSyntax" => Some("return"),
            _ => None,
        },
        expected_statement_sequence(kind),
    )
}

fn expected_statement_sequence(kind: LocalConstructKind) -> Vec<&'static str> {
    match kind {
        LocalConstructKind::ReturnStringLiteral
        | LocalConstructKind::ReturnIntegerLiteral
        | LocalConstructKind::ReturnFloatLiteral
        | LocalConstructKind::ReturnBooleanLiteral
        | LocalConstructKind::ReturnNullLiteral => {
            vec!["return"]
        }
        LocalConstructKind::TypedDeclaration => vec!["local_declaration", "return"],
        LocalConstructKind::ImplicitDeclaration
        | LocalConstructKind::ImplicitIntegerDeclaration => {
            vec!["local_declaration", "return"]
        }
        LocalConstructKind::Assignment => vec!["local_declaration", "assignment", "return"],
    }
}

fn return_literal_node_kind(kind: LocalConstructKind) -> &'static str {
    match kind {
        LocalConstructKind::ReturnStringLiteral => "string_literal",
        LocalConstructKind::ReturnIntegerLiteral => "integer_literal",
        LocalConstructKind::ReturnFloatLiteral => "float_literal",
        LocalConstructKind::ReturnBooleanLiteral => "boolean_literal",
        LocalConstructKind::ReturnNullLiteral => "null_literal",
        other => panic!("construct {other:?} does not have a return literal payload"),
    }
}

fn statement_sequence(
    sidecar: &GrammarStructuralModelSidecar,
    owner_rule: &str,
    owner_field: &str,
    map_statement: impl Fn(&str) -> Option<&'static str>,
    expected: Vec<&'static str>,
) -> Vec<&'static str> {
    let domain = sidecar
        .structural_model
        .finite_domains
        .iter()
        .find(|domain| domain.owner_rule == owner_rule && domain.owner_field == owner_field)
        .unwrap_or_else(|| panic!("missing statement domain {owner_rule}.{owner_field}"));
    let actual = domain
        .alternatives
        .iter()
        .filter_map(|alternative| map_statement(&alternative.node_kind))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    actual
}

fn assert_equivalent_local_constructs(
    case_name: &str,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) -> Result<(), String> {
    if go.comparable.statement_sequence != csharp.comparable.statement_sequence {
        return Err(format!("{case_name}: statement sequence mismatch"));
    }
    if go.comparable.semantic_payloads != csharp.comparable.semantic_payloads {
        return Err(format!("{case_name}: semantic payloads mismatch"));
    }
    if go.comparable.semantic_references != csharp.comparable.semantic_references {
        return Err(format!("{case_name}: semantic references mismatch"));
    }
    if go.comparable.semantic_decision_edges != csharp.comparable.semantic_decision_edges {
        return Err(format!("{case_name}: decision edge mismatch"));
    }
    if go.comparable.token_range_roles != csharp.comparable.token_range_roles {
        return Err(format!("{case_name}: token range role mismatch"));
    }
    if go.field_profiles.keys().collect::<Vec<_>>()
        != csharp.field_profiles.keys().collect::<Vec<_>>()
    {
        return Err(format!("{case_name}: field profile role mismatch"));
    }
    Ok(())
}

fn assert_field_profiles_are_preserved(
    case_name: &str,
    kind: LocalConstructKind,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "{case_name}: Go block statement cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "{case_name}: C# block statement cardinality changed"
    );

    match kind {
        LocalConstructKind::ReturnStringLiteral
        | LocalConstructKind::ReturnIntegerLiteral
        | LocalConstructKind::ReturnFloatLiteral
        | LocalConstructKind::ReturnBooleanLiteral
        | LocalConstructKind::ReturnNullLiteral => {
            assert_go_slice_vs_csharp_single(case_name, "return.value", go, csharp);
        }
        LocalConstructKind::TypedDeclaration => {
            assert_eq!(
                go.field_profiles["declaration.binding_list"],
                FieldCardinality {
                    required: true,
                    multiple: true
                }
            );
            assert_eq!(
                csharp.field_profiles["declaration.binding_list"],
                FieldCardinality {
                    required: true,
                    multiple: true
                }
            );
            assert_eq!(
                go.field_profiles["declaration.type"],
                csharp.field_profiles["declaration.type"]
            );
            assert_go_slice_vs_csharp_single(case_name, "declaration.name", go, csharp);
            assert_go_slice_vs_csharp_single(case_name, "declaration.initializer", go, csharp);
            assert_go_slice_vs_csharp_single(case_name, "return.value", go, csharp);
        }
        LocalConstructKind::ImplicitDeclaration
        | LocalConstructKind::ImplicitIntegerDeclaration => {
            assert_go_slice_vs_csharp_single(case_name, "declaration.name", go, csharp);
            assert_go_slice_vs_csharp_single(case_name, "declaration.initializer", go, csharp);
            assert_go_slice_vs_csharp_single(case_name, "return.value", go, csharp);
            assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
                "go.declaration.define_operator",
                "string",
                "utf8_string"
            )));
            assert!(
                csharp
                    .target_specific_payloads
                    .contains(&SemanticPayload::new(
                        "csharp.declaration.implicit_type_keyword",
                        "identifier",
                        "utf8_identifier"
                    ))
            );
        }
        LocalConstructKind::Assignment => {
            assert_eq!(
                go.field_profiles["declaration.binding_list"],
                csharp.field_profiles["declaration.binding_list"]
            );
            assert_go_slice_vs_csharp_single(case_name, "declaration.name", go, csharp);
            assert_go_slice_vs_csharp_single(case_name, "declaration.initializer", go, csharp);
            assert_go_slice_vs_csharp_single(case_name, "assignment.target", go, csharp);
            assert_go_slice_vs_csharp_single(case_name, "assignment.value", go, csharp);
            assert_go_slice_vs_csharp_single(case_name, "return.value", go, csharp);
            assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
                "go.assignment.operator",
                "string",
                "utf8_string"
            )));
            assert!(
                csharp
                    .target_specific_payloads
                    .contains(&SemanticPayload::new(
                        "csharp.declaration.required_type_keyword",
                        "identifier",
                        "utf8_identifier"
                    ))
            );
        }
    }
}

fn assert_call_expression_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "call_expression: Go block statement cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "call_expression: C# block statement cardinality changed"
    );
    assert_go_slice_vs_csharp_single("call_expression", "return.call", go, csharp);
    assert_eq!(
        go.field_profiles["call.callee"], csharp.field_profiles["call.callee"],
        "call_expression: callee cardinality changed"
    );
}

fn assert_argument_call_expression_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_call_expression_field_profiles_are_preserved(go, csharp);
    assert_go_slice_vs_csharp_single("argument_call_expression", "call.argument", go, csharp);
}

fn assert_member_access_field_profiles_are_preserved(
    case_name: &str,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    for role in [
        "access.expression",
        "access.receiver",
        "access.member",
        "access.receiver.identifier",
        "access.member.identifier",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "{case_name}: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "{case_name}: {role} must stay a required singleton"
        );
    }
}

fn assert_conditional_member_access_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    for role in [
        "access.member_binding",
        "access.receiver",
        "access.member",
        "access.receiver.identifier",
        "access.member.identifier",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "conditional_member_access: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "conditional_member_access: {role} must stay a required singleton"
        );
    }
}

fn assert_callable_wrapper_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["callable.declaration"], csharp.field_profiles["callable.declaration"],
        "callable_wrapper: callable declaration cardinality changed"
    );
    assert_eq!(
        go.field_profiles["callable.declaration"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "callable_wrapper: callable declaration must stay a required list"
    );
    for role in ["callable.name", "callable.return_type", "callable.body"] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "callable_wrapper: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "callable_wrapper: {role} must stay a required singleton"
        );
    }
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "callable_wrapper: body statement list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "callable_wrapper: body statement list must stay required and repeated"
    );
    assert_go_slice_vs_csharp_single("callable_wrapper", "return.value", go, csharp);
}

fn assert_callable_parameter_list_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    for role in ["callable.parameter_list", "parameter.type"] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "callable_parameter_list: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "callable_parameter_list: {role} must stay a required singleton"
        );
    }
    assert_eq!(
        go.field_profiles["parameter.binding"], csharp.field_profiles["parameter.binding"],
        "callable_parameter_list: parameter binding cardinality changed"
    );
    assert_eq!(
        go.field_profiles["parameter.binding"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "callable_parameter_list: parameter binding must stay required and repeated"
    );
    assert_eq!(
        go.field_profiles["parameter.name"],
        FieldCardinality {
            required: false,
            multiple: true
        },
        "callable_parameter_list: Go parameter names remain optional and repeated"
    );
    assert_eq!(
        csharp.field_profiles["parameter.name"],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "callable_parameter_list: C# parameter names remain required singletons"
    );
}

fn assert_member_list_container_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["member.list"], csharp.field_profiles["member.list"],
        "member_list_container: member list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["member.list"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "member_list_container: member list must stay required and repeated"
    );
    for role in [
        "member.data.name",
        "member.data.type",
        "member.callable.name",
        "member.callable.return_type",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "member_list_container: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "member_list_container: {role} must stay a required singleton"
        );
    }
}

fn assert_data_member_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["data_member.container"],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "data_member_field: Go struct fields container cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles["data_member.container"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "data_member_field: C# class member container cardinality changed"
    );
    assert_eq!(
        go.field_profiles["data_member.binding"],
        FieldCardinality {
            required: false,
            multiple: true
        },
        "data_member_field: Go field binding cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles["data_member.binding"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "data_member_field: C# property binding cardinality changed"
    );
    assert_eq!(
        go.field_profiles["data_member.name"],
        FieldCardinality {
            required: false,
            multiple: true
        },
        "data_member_field: Go field names remain optional and repeated"
    );
    assert_eq!(
        csharp.field_profiles["data_member.name"],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "data_member_field: C# property names remain required singletons"
    );
    assert_eq!(
        go.field_profiles["data_member.type"], csharp.field_profiles["data_member.type"],
        "data_member_field: member type cardinality changed"
    );
    assert_eq!(
        go.field_profiles["data_member.type"],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "data_member_field: member type must stay a required singleton"
    );
}

fn assert_method_member_receiver_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["method_member.declaration"],
        csharp.field_profiles["method_member.declaration"],
        "method_member_receiver: method declaration container cardinality changed"
    );
    assert_eq!(
        go.field_profiles["method_member.declaration"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "method_member_receiver: method declaration container must stay required and repeated"
    );
    for role in [
        "method_member.name",
        "method_member.return_type",
        "method_member.body",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "method_member_receiver: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "method_member_receiver: {role} must stay a required singleton"
        );
    }
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "method_member_receiver: body statement list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "method_member_receiver: body statement list must stay required and repeated"
    );
    assert_go_slice_vs_csharp_single("method_member_receiver", "return.value", go, csharp);
}

fn assert_package_data_declaration_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    for role in [
        "data_declaration.container",
        "data_declaration.binding",
        "companion_member.callable",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "package_data_declaration: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: true
            },
            "package_data_declaration: {role} must stay required and repeated"
        );
    }
    {
        let role = "data_declaration.type";
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "package_data_declaration: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "package_data_declaration: {role} must stay a required singleton"
        );
    }
    assert_go_slice_vs_csharp_single(
        "package_data_declaration",
        "data_declaration.name",
        go,
        csharp,
    );
    assert_go_slice_vs_csharp_single(
        "package_data_declaration",
        "data_declaration.initializer",
        go,
        csharp,
    );
    assert_go_slice_vs_csharp_single(
        "package_data_declaration",
        "companion_member.return",
        go,
        csharp,
    );
}

fn assert_collection_initializer_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["callable.member_container"],
        csharp.field_profiles["callable.member_container"],
        "collection_initializer: callable/member container cardinality changed"
    );
    assert_eq!(
        go.field_profiles["callable.member_container"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "collection_initializer: callable/member container must stay required and repeated"
    );
    for role in [
        "callable.body",
        "callable.return.collection_type",
        "callable.return.element_type",
        "collection.creation.collection_type",
        "collection.creation.element_type",
        "collection.initializer.integer",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "collection_initializer: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "collection_initializer: {role} must stay a required singleton"
        );
    }
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "collection_initializer: body statement list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "collection_initializer: body statement list must stay required and repeated"
    );
    assert_eq!(
        go.field_profiles["collection.initializer.element"],
        csharp.field_profiles["collection.initializer.element"],
        "collection_initializer: repeated initializer element list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["collection.initializer.element"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "collection_initializer: initializer element list must stay required and repeated"
    );
    assert_go_slice_vs_csharp_single("collection_initializer", "collection.creation", go, csharp);
}

fn assert_index_element_access_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["callable.member_container"],
        csharp.field_profiles["callable.member_container"],
        "index_element_access: callable/member container cardinality changed"
    );
    assert_eq!(
        go.field_profiles["callable.member_container"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "index_element_access: callable/member container must stay required and repeated"
    );
    for role in [
        "callable.body",
        "parameter.array_type",
        "parameter.element_type",
        "index.target",
        "index.argument.integer",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "index_element_access: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "index_element_access: {role} must stay a required singleton"
        );
    }
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "index_element_access: body statement list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "index_element_access: body statement list must stay required and repeated"
    );
    assert_go_slice_vs_csharp_single("index_element_access", "index.expression", go, csharp);
    assert_eq!(
        go.field_profiles["parameter.name"],
        FieldCardinality {
            required: false,
            multiple: true
        },
        "index_element_access: Go parameter names remain optional and repeated"
    );
    assert_eq!(
        csharp.field_profiles["parameter.name"],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "index_element_access: C# parameter name remains a required singleton"
    );
    assert_eq!(
        go.field_profiles["index.argument"],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "index_element_access: Go index argument remains a required singleton"
    );
    assert_eq!(
        csharp.field_profiles["index.argument"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "index_element_access: C# bracketed argument list remains required and repeated"
    );
}

fn assert_slice_range_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["callable.member_container"],
        csharp.field_profiles["callable.member_container"],
        "slice_range: callable/member container cardinality changed"
    );
    assert_eq!(
        go.field_profiles["callable.member_container"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "slice_range: callable/member container must stay required and repeated"
    );
    for role in [
        "callable.body",
        "parameter.array_type",
        "parameter.element_type",
        "range.target",
        "range.start",
        "range.start.integer",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "slice_range: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "slice_range: {role} must stay a required singleton"
        );
    }
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "slice_range: body statement list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "slice_range: body statement list must stay required and repeated"
    );
    assert_go_slice_vs_csharp_single("slice_range", "range.expression", go, csharp);
    assert_eq!(
        go.field_profiles["parameter.name"],
        FieldCardinality {
            required: false,
            multiple: true
        },
        "slice_range: Go parameter names remain optional and repeated"
    );
    assert_eq!(
        csharp.field_profiles["parameter.name"],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "slice_range: C# parameter name remains a required singleton"
    );
}

fn assert_unary_integer_return_expression_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["callable.member_container"],
        csharp.field_profiles["callable.member_container"],
        "unary_integer_return_expression: callable/member container cardinality changed"
    );
    assert_eq!(
        go.field_profiles["callable.member_container"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "unary_integer_return_expression: callable/member container must stay required and repeated"
    );
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "unary_integer_return_expression: body statement list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "unary_integer_return_expression: body statement list must stay required and repeated"
    );
    for role in [
        "callable.body",
        "unary.operator",
        "unary.operand",
        "unary.operand.integer",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "unary_integer_return_expression: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "unary_integer_return_expression: {role} must stay a required singleton"
        );
    }
    assert_go_slice_vs_csharp_single(
        "unary_integer_return_expression",
        "return.unary_expression",
        go,
        csharp,
    );
}

fn assert_parenthesized_integer_return_expression_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["callable.member_container"],
        csharp.field_profiles["callable.member_container"],
        "parenthesized_integer_return_expression: callable/member container cardinality changed"
    );
    assert_eq!(
        go.field_profiles["callable.member_container"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "parenthesized_integer_return_expression: callable/member container must stay required and repeated"
    );
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "parenthesized_integer_return_expression: body statement list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "parenthesized_integer_return_expression: body statement list must stay required and repeated"
    );
    for role in [
        "callable.body",
        "parenthesized.operand",
        "parenthesized.operand.integer",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "parenthesized_integer_return_expression: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "parenthesized_integer_return_expression: {role} must stay a required singleton"
        );
    }
    assert_go_slice_vs_csharp_single(
        "parenthesized_integer_return_expression",
        "return.parenthesized_expression",
        go,
        csharp,
    );
}

fn assert_binary_integer_return_expression_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["callable.member_container"],
        csharp.field_profiles["callable.member_container"],
        "binary_integer_return_expression: callable/member container cardinality changed"
    );
    assert_eq!(
        go.field_profiles["callable.member_container"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "binary_integer_return_expression: callable/member container must stay required and repeated"
    );
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "binary_integer_return_expression: body statement list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "binary_integer_return_expression: body statement list must stay required and repeated"
    );
    for role in [
        "callable.body",
        "binary.operator",
        "binary.left_operand",
        "binary.left_operand.integer",
        "binary.right_operand",
        "binary.right_operand.integer",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "binary_integer_return_expression: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "binary_integer_return_expression: {role} must stay a required singleton"
        );
    }
    assert_go_slice_vs_csharp_single(
        "binary_integer_return_expression",
        "return.binary_expression",
        go,
        csharp,
    );
}

fn assert_callable_wrapper_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_local_integer_return_literal_syntax_v0")
        .expect("Go integer-return callable wrapper route must stay registered");
    let csharp = syntax_production_route_construct("csharp_local_integer_return_literal_syntax_v0")
        .expect("C# integer-return callable wrapper route must stay registered");

    assert_eq!(
        go.metadata_path,
        GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH
    );
    assert_eq!(
        csharp.metadata_path,
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_integer_return_literal_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_local_integer_return_literal_round_trips")
    );
}

fn assert_callable_parameter_list_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_minimal_function_syntax_v0")
        .expect("Go minimal function route must stay registered");
    let csharp = syntax_production_route_construct("csharp_two_parameter_method_syntax_v0")
        .expect("C# two-parameter method route must stay registered");

    assert_eq!(go.metadata_path, GO_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_TWO_PARAMETER_METHOD_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_minimal_function_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_two_parameter_method_round_trips")
    );
}

fn assert_member_list_container_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_package_type_declaration_syntax_v0")
        .expect("Go package type declaration route must stay registered");
    let csharp = syntax_production_route_construct("csharp_property_plus_method_syntax_v0")
        .expect("C# property-plus-method route must stay registered");

    assert_eq!(go.metadata_path, GO_PACKAGE_TYPE_DECLARATION_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_package_type_declaration_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_property_plus_method_round_trips")
    );
}

fn assert_data_member_field_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_struct_type_syntax_v0")
        .expect("Go struct type route must stay registered");
    let csharp = syntax_production_route_construct("csharp_property_plus_method_syntax_v0")
        .expect("C# property-plus-method route must stay registered");

    assert_eq!(go.metadata_path, GO_STRUCT_TYPE_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_struct_type_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_property_plus_method_round_trips")
    );
}

fn assert_method_member_receiver_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_method_declaration_syntax_v0")
        .expect("Go method declaration route must stay registered");
    let csharp = syntax_production_route_construct("csharp_property_plus_method_syntax_v0")
        .expect("C# property-plus-method route must stay registered");

    assert_eq!(go.metadata_path, GO_METHOD_DECLARATION_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_method_declaration_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_property_plus_method_round_trips")
    );
}

fn assert_package_data_declaration_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_package_var_declaration_syntax_v0")
        .expect("Go package var declaration route must stay registered");
    let csharp = syntax_production_route_construct("csharp_property_plus_method_syntax_v0")
        .expect("C# property-plus-method route must stay registered");

    assert_eq!(go.metadata_path, GO_PACKAGE_VAR_DECLARATION_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_package_var_declaration_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_property_plus_method_round_trips")
    );
}

fn assert_collection_initializer_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_array_composite_literal_syntax_v0")
        .expect("Go array composite literal route must stay registered");
    let csharp = syntax_production_route_construct("csharp_collection_initializer_syntax_v0")
        .expect("C# collection initializer route must stay registered");

    assert_eq!(go.metadata_path, GO_ARRAY_COMPOSITE_LITERAL_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_COLLECTION_INITIALIZER_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_array_composite_literal_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_collection_initializer_round_trips")
    );
}

fn assert_index_element_access_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_index_expression_syntax_v0")
        .expect("Go index expression route must stay registered");
    let csharp = syntax_production_route_construct("csharp_element_access_syntax_v0")
        .expect("C# element-access route must stay registered");

    assert_eq!(go.metadata_path, GO_INDEX_EXPRESSION_METADATA_PATH);
    assert_eq!(csharp.metadata_path, CSHARP_ELEMENT_ACCESS_METADATA_PATH);
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_index_expression_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_element_access_round_trips")
    );
}

fn assert_slice_range_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_slice_expression_syntax_v0")
        .expect("Go slice expression route must stay registered");
    let csharp = syntax_production_route_construct("csharp_range_element_access_syntax_v0")
        .expect("C# range element-access route must stay registered");

    assert_eq!(go.metadata_path, GO_SLICE_EXPRESSION_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_RANGE_ELEMENT_ACCESS_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_slice_expression_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_range_element_access_round_trips")
    );
}

fn assert_unary_integer_return_expression_generated_codec_routes_are_registered() {
    let go =
        syntax_production_route_construct("go_local_unary_integer_return_expression_syntax_v0")
            .expect("Go unary integer return route must stay registered");
    let csharp = syntax_production_route_construct("csharp_local_unary_integer_return_syntax_v0")
        .expect("C# prefix-unary integer return route must stay registered");

    assert_eq!(
        go.metadata_path,
        GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH
    );
    assert_eq!(
        csharp.metadata_path,
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_unary_integer_return_expression_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_local_unary_integer_return_round_trips")
    );
}

fn assert_parenthesized_integer_return_expression_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct(
        "go_local_parenthesized_integer_return_expression_syntax_v0",
    )
    .expect("Go parenthesized integer return route must stay registered");
    let csharp =
        syntax_production_route_construct("csharp_local_parenthesized_integer_return_syntax_v0")
            .expect("C# parenthesized integer return route must stay registered");

    assert_eq!(
        go.metadata_path,
        GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH
    );
    assert_eq!(
        csharp.metadata_path,
        CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_parenthesized_integer_return_expression_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_local_parenthesized_integer_return_round_trips")
    );
}

fn assert_binary_integer_return_expression_generated_codec_routes_are_registered() {
    let go =
        syntax_production_route_construct("go_local_binary_integer_return_expression_syntax_v0")
            .expect("Go binary integer return route must stay registered");
    let csharp = syntax_production_route_construct("csharp_local_binary_integer_return_syntax_v0")
        .expect("C# binary integer return route must stay registered");

    assert_eq!(
        go.metadata_path,
        GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH
    );
    assert_eq!(
        csharp.metadata_path,
        CSHARP_LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_binary_integer_return_expression_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_local_binary_integer_return_round_trips")
    );
}

fn assert_zero_argument_call_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_local_call_return_expression_syntax_v0")
        .expect("Go zero-argument call route must stay registered");
    let csharp = syntax_production_route_construct("csharp_method_invocation_return_syntax_v0")
        .expect("C# zero-argument invocation route must stay registered");

    assert_eq!(
        go.metadata_path,
        GO_LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH
    );
    assert_eq!(
        csharp.metadata_path,
        CSHARP_METHOD_INVOCATION_RETURN_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_call_return_expression_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_method_invocation_return_round_trips")
    );
}

fn assert_if_else_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_local_if_else_syntax_v0")
        .expect("Go if/else route must stay registered");
    let csharp = syntax_production_route_construct("csharp_local_if_else_syntax_v0")
        .expect("C# if/else route must stay registered");

    assert_eq!(go.metadata_path, GO_LOCAL_IF_ELSE_METADATA_PATH);
    assert_eq!(csharp.metadata_path, CSHARP_LOCAL_IF_ELSE_METADATA_PATH);
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_if_else_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_local_if_else_round_trips")
    );
}

fn assert_for_condition_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_local_for_condition_syntax_v0")
        .expect("Go condition-only for route must stay registered");
    let csharp = syntax_production_route_construct("csharp_local_for_condition_syntax_v0")
        .expect("C# condition-only for route must stay registered");

    assert_eq!(go.metadata_path, GO_LOCAL_FOR_CONDITION_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_for_condition_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_local_for_condition_round_trips")
    );
}

fn assert_postfix_simple_statement_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_local_increment_decrement_syntax_v0")
        .expect("Go increment/decrement route must stay registered");
    let csharp = syntax_production_route_construct("csharp_while_postfix_loop_syntax_v0")
        .expect("C# while/postfix loop route must stay registered");

    assert_eq!(go.metadata_path, GO_LOCAL_INCREMENT_DECREMENT_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_WHILE_POSTFIX_LOOP_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_increment_decrement_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_while_postfix_loop_round_trips")
    );
}

fn assert_switch_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_local_switch_syntax_v0")
        .expect("Go single-case switch route must stay registered");
    let csharp = syntax_production_route_construct("csharp_local_switch_syntax_v0")
        .expect("C# single-case switch route must stay registered");

    assert_eq!(go.metadata_path, GO_LOCAL_SWITCH_METADATA_PATH);
    assert_eq!(csharp.metadata_path, CSHARP_LOCAL_SWITCH_METADATA_PATH);
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_switch_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_local_switch_round_trips")
    );
}

fn assert_string_iteration_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_local_range_string_syntax_v0")
        .expect("Go string range route must stay registered");
    let csharp = syntax_production_route_construct("csharp_local_foreach_string_syntax_v0")
        .expect("C# foreach string route must stay registered");

    assert_eq!(go.metadata_path, GO_LOCAL_RANGE_STRING_METADATA_PATH);
    assert_eq!(
        csharp.metadata_path,
        CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_local_range_string_round_trips")
    );
    assert!(
        csharp
            .generated_test_markers
            .contains(&"generated_csharp_local_foreach_string_round_trips")
    );
}

fn assert_postfix_simple_statement_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["block.statements"], csharp.field_profiles["block.statements"],
        "postfix_simple_statement: block statement cardinality changed"
    );
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "postfix_simple_statement: block statement list must stay required and repeated"
    );
    for role in ["update.target", "update.operator"] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "postfix_simple_statement: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "postfix_simple_statement: {role} must stay a required singleton"
        );
    }
}

fn assert_if_else_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "if_else_control_flow: Go block statement cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "if_else_control_flow: C# block statement cardinality changed"
    );
    assert_go_slice_vs_csharp_single("if_else_control_flow", "declaration.name", go, csharp);
    assert_go_slice_vs_csharp_single(
        "if_else_control_flow",
        "declaration.initializer",
        go,
        csharp,
    );
    assert_go_slice_vs_csharp_single("if_else_control_flow", "assignment.target", go, csharp);
    assert_go_slice_vs_csharp_single("if_else_control_flow", "assignment.value", go, csharp);
    assert_go_slice_vs_csharp_single("if_else_control_flow", "return.value", go, csharp);

    assert_eq!(
        go.field_profiles["declaration.binding_list"],
        csharp.field_profiles["declaration.binding_list"],
        "if_else_control_flow: declaration list cardinality changed"
    );
    for role in [
        "if.condition",
        "if.then_body",
        "if.else_body",
        "condition.left",
        "condition.right",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "if_else_control_flow: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "if_else_control_flow: {role} must stay a required singleton"
        );
    }
}

fn assert_condition_only_loop_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "condition_only_loop: Go block statement cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "condition_only_loop: C# block statement cardinality changed"
    );
    assert_go_slice_vs_csharp_single("condition_only_loop", "declaration.name", go, csharp);
    assert_go_slice_vs_csharp_single("condition_only_loop", "declaration.initializer", go, csharp);
    assert_go_slice_vs_csharp_single("condition_only_loop", "assignment.target", go, csharp);
    assert_go_slice_vs_csharp_single("condition_only_loop", "assignment.value", go, csharp);
    assert_go_slice_vs_csharp_single("condition_only_loop", "return.value", go, csharp);

    assert_eq!(
        go.field_profiles["declaration.binding_list"],
        csharp.field_profiles["declaration.binding_list"],
        "condition_only_loop: declaration list cardinality changed"
    );
    for role in [
        "loop.condition",
        "loop.body",
        "condition.left",
        "condition.right",
    ] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "condition_only_loop: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "condition_only_loop: {role} must stay a required singleton"
        );
    }
}

fn assert_switch_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "single_case_switch: Go block statement cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "single_case_switch: C# block statement cardinality changed"
    );
    assert_go_slice_vs_csharp_single("single_case_switch", "declaration.name", go, csharp);
    assert_go_slice_vs_csharp_single("single_case_switch", "declaration.initializer", go, csharp);
    assert_go_slice_vs_csharp_single("single_case_switch", "case.label", go, csharp);
    assert_go_slice_vs_csharp_single("single_case_switch", "assignment.target", go, csharp);
    assert_go_slice_vs_csharp_single("single_case_switch", "assignment.value", go, csharp);
    assert_go_slice_vs_csharp_single("single_case_switch", "return.value", go, csharp);

    assert_eq!(
        go.field_profiles["declaration.binding_list"],
        csharp.field_profiles["declaration.binding_list"],
        "single_case_switch: declaration list cardinality changed"
    );
    assert_eq!(
        go.field_profiles["switch.selector"], csharp.field_profiles["switch.selector"],
        "single_case_switch: switch.selector cardinality changed"
    );
    assert_eq!(
        go.field_profiles["switch.selector"],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "single_case_switch: switch.selector must stay a required singleton"
    );
    assert_eq!(
        go.field_profiles["case.body.assignment"], csharp.field_profiles["case.body.assignment"],
        "single_case_switch: case.body.assignment cardinality changed"
    );
    assert_eq!(
        go.field_profiles["case.body.assignment"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "single_case_switch: case.body.assignment must stay a required statement list"
    );
}

fn assert_string_iteration_field_profiles_are_preserved(
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "string_iteration: Go block statement cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles["block.statements"],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "string_iteration: C# block statement cardinality changed"
    );
    assert_go_slice_vs_csharp_single("string_iteration", "declaration.name", go, csharp);
    assert_go_slice_vs_csharp_single("string_iteration", "declaration.initializer", go, csharp);
    assert_go_slice_vs_csharp_single("string_iteration", "assignment.target", go, csharp);
    assert_go_slice_vs_csharp_single("string_iteration", "assignment.value", go, csharp);
    assert_go_slice_vs_csharp_single("string_iteration", "return.value", go, csharp);

    assert_eq!(
        go.field_profiles["declaration.binding_list"],
        csharp.field_profiles["declaration.binding_list"],
        "string_iteration: declaration list cardinality changed"
    );
    for role in ["iteration.source", "iteration.body"] {
        assert_eq!(
            go.field_profiles[role], csharp.field_profiles[role],
            "string_iteration: {role} cardinality changed"
        );
        assert_eq!(
            go.field_profiles[role],
            FieldCardinality {
                required: true,
                multiple: false
            },
            "string_iteration: {role} must stay a required singleton"
        );
    }
}

fn assert_callable_wrapper_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_field_cardinality(
        go_sidecar,
        "FuncDecl",
        "Type",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "FuncType",
        "Params",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "FuncType",
        "Results",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "FieldList",
        "List",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.return_type.identifier",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.return_type.keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_nil_null_return_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "ReturnStmt", "Results", "Ident");
    assert_reference_exists(
        csharp_sidecar,
        "ReturnStatementSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.return_nil.identifier",
        "identifier",
        "utf8_identifier"
    )));
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.return_null.literal",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_callable_parameter_list_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "GenDecl");
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "GenDecl", "Specs", "ImportSpec");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Names", "Ident");
    assert_reference_exists(go_sidecar, "Field", "Type", "Ident");
    assert_field_cardinality(
        go_sidecar,
        "Field",
        "Names",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.import.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.import.path",
        "string",
        "utf8_string"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.parameter.type.identifier",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "MethodDeclarationSyntax",
        "ParameterList",
        "ParameterListSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ParameterListSyntax",
        "Parameters",
        "ParameterSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ParameterSyntax",
        "Type",
        "PredefinedTypeSyntax",
    );
    assert_field_cardinality(
        csharp_sidecar,
        "ParameterSyntax",
        "Identifier",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.return_type.keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.parameter.type.keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_member_list_container_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "GenDecl");
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "GenDecl", "Specs", "TypeSpec");
    assert_reference_exists(go_sidecar, "TypeSpec", "Name", "Ident");
    assert_reference_exists(go_sidecar, "TypeSpec", "Type", "Ident");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "BlockStmt", "List", "ReturnStmt");
    assert_reference_exists(go_sidecar, "ReturnStmt", "Results", "BasicLit");
    assert_field_cardinality(
        go_sidecar,
        "GenDecl",
        "Specs",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "FieldList",
        "List",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.type.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.return.literal",
        "integer",
        "u8_integer"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ReturnStatementSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert_field_cardinality(
        csharp_sidecar,
        "PropertyDeclarationSyntax",
        "Initializer",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.property.initializer",
                "string",
                "utf8_string"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.return.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_data_member_field_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "GenDecl");
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "GenDecl", "Specs", "TypeSpec");
    assert_reference_exists(go_sidecar, "TypeSpec", "Name", "Ident");
    assert_reference_exists(go_sidecar, "TypeSpec", "Type", "StructType");
    assert_reference_exists(go_sidecar, "StructType", "Fields", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Names", "Ident");
    assert_reference_exists(go_sidecar, "Field", "Type", "Ident");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "BlockStmt", "List", "ReturnStmt");
    assert_reference_exists(go_sidecar, "ReturnStmt", "Results", "BasicLit");
    assert_field_cardinality(
        go_sidecar,
        "StructType",
        "Fields",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "FieldList",
        "List",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "Field",
        "Names",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.type.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.callable.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.callable.return.literal",
        "integer",
        "u8_integer"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "PropertyDeclarationSyntax",
        "Type",
        "PredefinedTypeSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ReturnStatementSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    assert_field_cardinality(
        csharp_sidecar,
        "PropertyDeclarationSyntax",
        "Initializer",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.property.initializer",
                "string",
                "utf8_string"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.return.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_method_member_receiver_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "GenDecl");
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "GenDecl", "Specs", "TypeSpec");
    assert_reference_exists(go_sidecar, "TypeSpec", "Name", "Ident");
    assert_reference_exists(go_sidecar, "TypeSpec", "Type", "Ident");
    assert_reference_exists(go_sidecar, "FuncDecl", "Recv", "FieldList");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncDecl", "Body", "BlockStmt");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Names", "Ident");
    assert_reference_exists(go_sidecar, "Field", "Type", "Ident");
    assert_reference_exists(go_sidecar, "ReturnStmt", "Results", "BasicLit");
    assert_field_cardinality(
        go_sidecar,
        "FuncDecl",
        "Recv",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "FieldList",
        "List",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "Field",
        "Names",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.type.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.receiver.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.receiver.type",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.method.return.literal",
        "integer",
        "u8_integer"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ReturnStatementSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert_field_cardinality(
        csharp_sidecar,
        "PropertyDeclarationSyntax",
        "Initializer",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.property.initializer",
                "string",
                "utf8_string"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.return.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_package_data_declaration_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "GenDecl");
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "GenDecl", "Specs", "ValueSpec");
    assert_reference_exists(go_sidecar, "ValueSpec", "Names", "Ident");
    assert_reference_exists(go_sidecar, "ValueSpec", "Type", "Ident");
    assert_reference_exists(go_sidecar, "ValueSpec", "Values", "BasicLit");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncDecl", "Body", "BlockStmt");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Type", "Ident");
    assert_reference_exists(go_sidecar, "BlockStmt", "List", "ReturnStmt");
    assert_reference_exists(go_sidecar, "ReturnStmt", "Results", "Ident");
    assert_field_cardinality(
        go_sidecar,
        "ValueSpec",
        "Names",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "ValueSpec",
        "Values",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "ReturnStmt",
        "Results",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.var.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.data.initializer.literal",
        "integer",
        "u8_integer"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.companion.return.identifier",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "PropertyDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "PropertyDeclarationSyntax",
        "Type",
        "PredefinedTypeSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "MethodDeclarationSyntax",
        "ReturnType",
        "PredefinedTypeSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "MethodDeclarationSyntax",
        "Body",
        "BlockSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "BlockSyntax",
        "Statements",
        "ReturnStatementSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ReturnStatementSyntax",
        "Expression",
        "IdentifierNameSyntax",
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert_field_cardinality(
        csharp_sidecar,
        "PropertyDeclarationSyntax",
        "Initializer",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.data.initializer.string",
                "string",
                "utf8_string"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.return.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_collection_initializer_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Type", "ArrayType");
    assert_reference_exists(go_sidecar, "CompositeLit", "Type", "ArrayType");
    assert_reference_exists(go_sidecar, "CompositeLit", "Elts", "BasicLit");
    assert_field_cardinality(
        go_sidecar,
        "ReturnStmt",
        "Results",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.function.name",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ArrayCreationExpressionSyntax",
        "Initializer",
        "InitializerExpressionSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "InitializerExpressionSyntax",
        "Expressions",
        "LiteralExpressionSyntax",
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert_field_cardinality(
        csharp_sidecar,
        "ArrayCreationExpressionSyntax",
        "Initializer",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_index_element_access_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Names", "Ident");
    assert_reference_exists(go_sidecar, "Field", "Type", "ArrayType");
    assert_reference_exists(go_sidecar, "IndexExpr", "Index", "BasicLit");
    assert_field_cardinality(
        go_sidecar,
        "Field",
        "Names",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "ReturnStmt",
        "Results",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.function.name",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "MethodDeclarationSyntax",
        "ParameterList",
        "ParameterListSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ParameterListSyntax",
        "Parameters",
        "ParameterSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ElementAccessExpressionSyntax",
        "ArgumentList",
        "BracketedArgumentListSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ArgumentSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    assert_field_cardinality(
        csharp_sidecar,
        "ElementAccessExpressionSyntax",
        "ArgumentList",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "BracketedArgumentListSyntax",
        "Arguments",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.return_type",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_slice_range_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Names", "Ident");
    assert_reference_exists(go_sidecar, "Field", "Type", "ArrayType");
    assert_reference_exists(go_sidecar, "SliceExpr", "Low", "BasicLit");
    assert_field_absent(go_sidecar, "SliceExpr", "High");
    assert_field_cardinality(
        go_sidecar,
        "Field",
        "Names",
        FieldCardinality {
            required: false,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "ReturnStmt",
        "Results",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.function.name",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ElementAccessExpressionSyntax",
        "ArgumentList",
        "BracketedArgumentListSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "BracketedArgumentListSyntax",
        "Arguments",
        "ArgumentSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ArgumentSyntax",
        "Expression",
        "RangeExpressionSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "RangeExpressionSyntax",
        "RightOperand",
        "LiteralExpressionSyntax",
    );
    assert_field_cardinality(
        csharp_sidecar,
        "ElementAccessExpressionSyntax",
        "ArgumentList",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "BracketedArgumentListSyntax",
        "Arguments",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "ArgumentSyntax",
        "Expression",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "RangeExpressionSyntax",
        "RightOperand",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_unary_integer_return_expression_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Type", "Ident");
    assert_reference_exists(go_sidecar, "UnaryExpr", "X", "BasicLit");
    assert_field_cardinality(
        go_sidecar,
        "ReturnStmt",
        "Results",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "UnaryExpr",
        "Op",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "UnaryExpr",
        "X",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.function.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.return_type.identifier",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "MethodDeclarationSyntax",
        "ReturnType",
        "PredefinedTypeSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "PrefixUnaryExpressionSyntax",
        "Operand",
        "LiteralExpressionSyntax",
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert_field_cardinality(
        csharp_sidecar,
        "ReturnStatementSyntax",
        "Expression",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "PrefixUnaryExpressionSyntax",
        "OperatorToken",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "PrefixUnaryExpressionSyntax",
        "Operand",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.return_type.keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_parenthesized_integer_return_expression_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Type", "Ident");
    assert_reference_exists(go_sidecar, "ParenExpr", "X", "BasicLit");
    assert_field_cardinality(
        go_sidecar,
        "ReturnStmt",
        "Results",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "ParenExpr",
        "X",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.function.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.return_type.identifier",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "MethodDeclarationSyntax",
        "ReturnType",
        "PredefinedTypeSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ParenthesizedExpressionSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert_field_cardinality(
        csharp_sidecar,
        "ReturnStatementSyntax",
        "Expression",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "ParenthesizedExpressionSyntax",
        "Expression",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.return_type.keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_binary_integer_return_expression_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "File", "Decls", "FuncDecl");
    assert_reference_exists(go_sidecar, "FuncDecl", "Type", "FuncType");
    assert_reference_exists(go_sidecar, "FuncType", "Params", "FieldList");
    assert_reference_exists(go_sidecar, "FuncType", "Results", "FieldList");
    assert_reference_exists(go_sidecar, "FieldList", "List", "Field");
    assert_reference_exists(go_sidecar, "Field", "Type", "Ident");
    assert_reference_exists(go_sidecar, "BinaryExpr", "X", "BasicLit");
    assert_reference_exists(go_sidecar, "BinaryExpr", "Y", "BasicLit");
    assert_field_cardinality(
        go_sidecar,
        "ReturnStmt",
        "Results",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "BinaryExpr",
        "Op",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "BinaryExpr",
        "X",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        go_sidecar,
        "BinaryExpr",
        "Y",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.package.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.function.name",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.return_type.identifier",
        "identifier",
        "utf8_identifier"
    )));

    assert_reference_exists(
        csharp_sidecar,
        "CompilationUnitSyntax",
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "FileScopedNamespaceDeclarationSyntax",
        "Members",
        "ClassDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ClassDeclarationSyntax",
        "Members",
        "MethodDeclarationSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "MethodDeclarationSyntax",
        "ReturnType",
        "PredefinedTypeSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "BinaryExpressionSyntax",
        "Left",
        "LiteralExpressionSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "BinaryExpressionSyntax",
        "Right",
        "LiteralExpressionSyntax",
    );
    assert_field_absent(csharp_sidecar, "MethodDeclarationSyntax", "ParameterList");
    assert_field_cardinality(
        csharp_sidecar,
        "ReturnStatementSyntax",
        "Expression",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "BinaryExpressionSyntax",
        "OperatorToken",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "BinaryExpressionSyntax",
        "Left",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "BinaryExpressionSyntax",
        "Right",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.namespace.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.class.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.method.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.return_type.keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_if_else_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(
        csharp_sidecar,
        "IfStatementSyntax",
        "Else",
        "ElseClauseSyntax",
    );
    assert_field_cardinality(
        csharp_sidecar,
        "IfStatementSyntax",
        "Else",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_reference_exists(go_sidecar, "IfStmt", "Else", "BlockStmt");
    assert_reference_exists(
        csharp_sidecar,
        "ElseClauseSyntax",
        "Statement",
        "BlockSyntax",
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.declaration.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.condition.operator",
        "string",
        "utf8_string"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.assignment.operator",
        "string",
        "utf8_string"
    )));
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.declaration.required_type_keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.condition.operator",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_condition_only_loop_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "ForStmt", "Cond", "BinaryExpr");
    assert_reference_exists(go_sidecar, "ForStmt", "Body", "BlockStmt");
    assert_reference_exists(
        csharp_sidecar,
        "ForStatementSyntax",
        "Condition",
        "BinaryExpressionSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ForStatementSyntax",
        "Statement",
        "BlockSyntax",
    );
    assert_field_absent(go_sidecar, "ForStmt", "Init");
    assert_field_absent(go_sidecar, "ForStmt", "Post");
    assert_field_absent(csharp_sidecar, "ForStatementSyntax", "Declaration");
    assert_field_absent(csharp_sidecar, "ForStatementSyntax", "Initializers");
    assert_field_absent(csharp_sidecar, "ForStatementSyntax", "Incrementors");
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.declaration.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.condition.operator",
        "string",
        "utf8_string"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.assignment.operator",
        "string",
        "utf8_string"
    )));
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.declaration.required_type_keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.condition.operator",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_postfix_simple_statement_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "BlockStmt", "List", "IncDecStmt");
    assert_reference_exists(
        csharp_sidecar,
        "BlockSyntax",
        "Statements",
        "ExpressionStatementSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ExpressionStatementSyntax",
        "Expression",
        "PostfixUnaryExpressionSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "WhileStatementSyntax",
        "Statement",
        "BlockSyntax",
    );
    assert_field_cardinality(
        csharp_sidecar,
        "ExpressionStatementSyntax",
        "Expression",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(
        go.comparable
            .semantic_payloads
            .contains(&SemanticPayload::new(
                "update.operator",
                "string",
                "utf8_string"
            )),
        "postfix_simple_statement: Go update operator payload should remain explicit"
    );
    assert!(
        csharp
            .comparable
            .semantic_payloads
            .contains(&SemanticPayload::new(
                "update.operator",
                "string",
                "utf8_string"
            )),
        "postfix_simple_statement: C# postfix operator payload should remain explicit"
    );
}

fn assert_switch_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "SwitchStmt", "Body", "BlockStmt");
    assert_reference_exists(
        csharp_sidecar,
        "SwitchStatementSyntax",
        "Sections",
        "SwitchSectionSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "SwitchSectionSyntax",
        "Labels",
        "CaseSwitchLabelSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "SwitchSectionSyntax",
        "Statements",
        "BreakStatementSyntax",
    );
    assert_field_cardinality(
        go_sidecar,
        "SwitchStmt",
        "Body",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "SwitchStatementSyntax",
        "Sections",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "SwitchSectionSyntax",
        "Labels",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "SwitchSectionSyntax",
        "Statements",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.declaration.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.assignment.operator",
        "string",
        "utf8_string"
    )));
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.declaration.required_type_keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_string_iteration_keeps_target_specific_evidence(
    go_sidecar: &GrammarStructuralModelSidecar,
    csharp_sidecar: &GrammarStructuralModelSidecar,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go_sidecar, "RangeStmt", "Body", "BlockStmt");
    assert_field_absent(go_sidecar, "RangeStmt", "Key");
    assert_field_absent(go_sidecar, "RangeStmt", "Value");
    assert_reference_exists(
        csharp_sidecar,
        "ForEachStatementSyntax",
        "Type",
        "PredefinedTypeSyntax",
    );
    assert_reference_exists(
        csharp_sidecar,
        "ForEachStatementSyntax",
        "Statement",
        "BlockSyntax",
    );
    assert_field_cardinality(
        csharp_sidecar,
        "ForEachStatementSyntax",
        "Type",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_sidecar,
        "ForEachStatementSyntax",
        "Identifier",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.declaration.keyword",
        "identifier",
        "utf8_identifier"
    )));
    assert!(go.target_specific_payloads.contains(&SemanticPayload::new(
        "go.assignment.operator",
        "string",
        "utf8_string"
    )));
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.declaration.required_type_keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.iteration.type_keyword",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.iteration.variable",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_zero_argument_call_keeps_target_specific_evidence(
    go: &GrammarStructuralModelSidecar,
    csharp: &GrammarStructuralModelSidecar,
) {
    assert_field_absent(go, "CallExpr", "Args");

    assert_reference_exists(
        csharp,
        "InvocationExpressionSyntax",
        "ArgumentList",
        "ArgumentListSyntax",
    );
    assert_field_cardinality(
        csharp,
        "InvocationExpressionSyntax",
        "ArgumentList",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_absent(csharp, "ArgumentListSyntax", "Arguments");
    assert_field_absent(csharp, "MethodDeclarationSyntax", "ParameterList");
    assert_reference_exists(
        csharp,
        "ReturnStatementSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );

    let mut csharp_specific = empty_evidence(Vec::new());
    add_target_specific_payload(
        &mut csharp_specific,
        csharp,
        "csharp.helper.return.literal",
        "LiteralExpressionSyntax",
        "Token",
        "string_literal",
    );
    add_target_specific_payload(
        &mut csharp_specific,
        csharp,
        "csharp.helper.method.identifier",
        "MethodDeclarationSyntax",
        "Identifier",
        "identifier",
    );

    assert!(
        csharp_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.helper.return.literal",
                "string",
                "utf8_string"
            ))
    );
    assert!(
        csharp_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.helper.method.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_argument_call_keeps_target_specific_evidence(
    go: &GrammarStructuralModelSidecar,
    csharp: &GrammarStructuralModelSidecar,
) {
    assert_reference_exists(go, "CallExpr", "Args", "BasicLit");
    assert_reference_exists(
        csharp,
        "InvocationExpressionSyntax",
        "ArgumentList",
        "ArgumentListSyntax",
    );
    assert_reference_exists(csharp, "ArgumentListSyntax", "Arguments", "ArgumentSyntax");
    assert_reference_exists(
        csharp,
        "ArgumentSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    assert_reference_exists(
        csharp,
        "MethodDeclarationSyntax",
        "ParameterList",
        "ParameterListSyntax",
    );
    assert_reference_exists(
        csharp,
        "ParameterListSyntax",
        "Parameters",
        "ParameterSyntax",
    );
    assert_reference_exists(csharp, "ParameterSyntax", "Type", "PredefinedTypeSyntax");

    assert_field_cardinality(
        go,
        "CallExpr",
        "Args",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        csharp,
        "InvocationExpressionSyntax",
        "ArgumentList",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp,
        "ArgumentListSyntax",
        "Arguments",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        csharp,
        "ParameterListSyntax",
        "Parameters",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );

    let mut csharp_specific = empty_evidence(Vec::new());
    add_target_specific_payload(
        &mut csharp_specific,
        csharp,
        "csharp.call.parameter.identifier",
        "ParameterSyntax",
        "Identifier",
        "identifier",
    );
    add_target_specific_payload(
        &mut csharp_specific,
        csharp,
        "csharp.call.parameter.type",
        "PredefinedTypeSyntax",
        "Keyword",
        "string_keyword",
    );

    assert!(
        csharp_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.call.parameter.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.call.parameter.type",
                "identifier",
                "utf8_identifier"
            ))
    );
}

fn assert_member_conditional_access_generated_codec_routes_are_registered() {
    let go = syntax_production_route_construct("go_strings_compare_source_file_syntax_v0")
        .expect("Go strings/compare selector route must stay registered");
    let csharp_member =
        syntax_production_route_construct("csharp_member_access_expression_syntax_v0")
            .expect("C# member-access route must stay registered");
    let csharp_conditional =
        syntax_production_route_construct("csharp_conditional_access_syntax_v0")
            .expect("C# conditional-access route must stay registered");

    assert_eq!(
        go.metadata_path,
        GO_STRINGS_COMPARE_SOURCE_FILE_METADATA_PATH
    );
    assert_eq!(
        csharp_member.metadata_path,
        CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH
    );
    assert_eq!(
        csharp_conditional.metadata_path,
        CSHARP_CONDITIONAL_ACCESS_METADATA_PATH
    );
    assert!(
        go.generated_test_markers
            .contains(&"generated_go_strings_compare_source_file_round_trips")
    );
    assert!(
        csharp_member
            .generated_test_markers
            .contains(&"generated_csharp_member_access_expression_round_trips")
    );
    assert!(
        csharp_conditional
            .generated_test_markers
            .contains(&"generated_csharp_conditional_access_round_trips")
    );
}

fn assert_member_conditional_access_keeps_target_specific_evidence(
    go: &GrammarStructuralModelSidecar,
    csharp_member: &GrammarStructuralModelSidecar,
    csharp_conditional: &GrammarStructuralModelSidecar,
    go_member: &LocalConstructBoundaryEvidence,
    csharp_member_evidence: &LocalConstructBoundaryEvidence,
) {
    assert_reference_exists(go, "ReturnStmt", "Results", "CallExpr");
    assert_reference_exists(go, "CallExpr", "Args", "Ident");
    assert_field_cardinality(
        go,
        "ReturnStmt",
        "Results",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        go,
        "CallExpr",
        "Args",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );

    let mut go_specific = empty_evidence(Vec::new());
    add_target_specific_payload(
        &mut go_specific,
        go,
        "go.package.name",
        "File",
        "Name",
        "package_identifier",
    );
    add_target_specific_payload(
        &mut go_specific,
        go,
        "go.function.name",
        "FuncDecl",
        "Name",
        "identifier",
    );
    add_target_specific_payload(
        &mut go_specific,
        go,
        "go.import.path",
        "ImportSpec",
        "Path",
        "string_literal",
    );
    assert!(
        go_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "go.package.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        go_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "go.function.name",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        go_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "go.import.path",
                "string",
                "utf8_string"
            ))
    );

    assert_reference_exists(
        csharp_member,
        "ReturnStatementSyntax",
        "Expression",
        "MemberAccessExpressionSyntax",
    );
    assert!(
        go_member
            .comparable
            .semantic_payloads
            .contains(&SemanticPayload::new(
                "access.member.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );
    assert!(
        csharp_member_evidence
            .comparable
            .semantic_payloads
            .contains(&SemanticPayload::new(
                "access.member.identifier",
                "identifier",
                "utf8_identifier"
            ))
    );

    assert_reference_exists(
        csharp_conditional,
        "ExpressionStatementSyntax",
        "Expression",
        "ConditionalAccessExpressionSyntax",
    );
    assert_reference_exists(
        csharp_conditional,
        "ConditionalAccessExpressionSyntax",
        "WhenNotNull",
        "InvocationExpressionSyntax",
    );
    assert_reference_exists(
        csharp_conditional,
        "InvocationExpressionSyntax",
        "ArgumentList",
        "ArgumentListSyntax",
    );
    assert_reference_exists(
        csharp_conditional,
        "ArgumentListSyntax",
        "Arguments",
        "ArgumentSyntax",
    );
    assert_reference_exists(
        csharp_conditional,
        "ArgumentSyntax",
        "Expression",
        "LiteralExpressionSyntax",
    );
    assert_field_cardinality(
        csharp_conditional,
        "BlockSyntax",
        "Statements",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );
    assert_field_cardinality(
        csharp_conditional,
        "ConditionalAccessExpressionSyntax",
        "WhenNotNull",
        FieldCardinality {
            required: true,
            multiple: false,
        },
    );
    assert_field_cardinality(
        csharp_conditional,
        "ArgumentListSyntax",
        "Arguments",
        FieldCardinality {
            required: true,
            multiple: true,
        },
    );

    let mut csharp_specific = empty_evidence(Vec::new());
    add_target_specific_payload(
        &mut csharp_specific,
        csharp_conditional,
        "csharp.conditional.argument.literal",
        "LiteralExpressionSyntax",
        "Token",
        "string_literal",
    );
    add_target_specific_payload(
        &mut csharp_specific,
        csharp_conditional,
        "csharp.conditional.return.literal",
        "LiteralExpressionSyntax",
        "Token",
        "string_literal",
    );
    assert!(
        csharp_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.conditional.argument.literal",
                "string",
                "utf8_string"
            ))
    );
    assert!(
        csharp_specific
            .target_specific_payloads
            .contains(&SemanticPayload::new(
                "csharp.conditional.return.literal",
                "string",
                "utf8_string"
            ))
    );
}

fn assert_go_slice_vs_csharp_single(
    case_name: &str,
    role: &str,
    go: &LocalConstructBoundaryEvidence,
    csharp: &LocalConstructBoundaryEvidence,
) {
    assert_eq!(
        go.field_profiles[role],
        FieldCardinality {
            required: true,
            multiple: true
        },
        "{case_name}: Go {role} cardinality changed"
    );
    assert_eq!(
        csharp.field_profiles[role],
        FieldCardinality {
            required: true,
            multiple: false
        },
        "{case_name}: C# {role} cardinality changed"
    );
}
