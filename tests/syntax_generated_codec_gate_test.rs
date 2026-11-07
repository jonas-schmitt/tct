#![recursion_limit = "256"]

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};
use tct::bpe::encoder::{BpeDecoder, BpeEncoder};
use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
use tct::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use tct::generated_crate::{
    GENERATED_EXTERNAL_BUILD_ENV, GeneratedCargoToml, configure_generated_cargo_command,
    copy_workspace_cargo_lock,
};
use tct::go_ast_schema_inventory::{
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE,
};
use tct::go_syntax_metadata::GoSyntaxMetadataAdapter;
use tct::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
    GrammarTerminalKind,
};
use tct::grammar_ir_structural_model::{
    GrammarStructuralBpeBoundary, GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
    lower_grammar_ir_to_structural_model,
};
use tct::provenance::{json_sha256, sha256_hex};
use tct::syntax_generated_codec::{
    CSHARP_ARRAY_RANK_OBJECT_CREATION_METADATA_PATH, CSHARP_ARROW_EXPRESSION_CLAUSE_METADATA_PATH,
    CSHARP_ATTRIBUTE_SYNTAX_METADATA_PATH, CSHARP_BASE_LIST_METADATA_PATH,
    CSHARP_BLOCK_SCOPED_NAMESPACE_METADATA_PATH, CSHARP_COLLECTION_INITIALIZER_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH,
    CSHARP_CONDITIONAL_ACCESS_METADATA_PATH, CSHARP_CONSTRUCTOR_DECLARATION_METADATA_PATH,
    CSHARP_ELEMENT_ACCESS_METADATA_PATH, CSHARP_ENUM_DECLARATION_METADATA_PATH,
    CSHARP_EVENT_DECLARATION_METADATA_PATH, CSHARP_FIELD_DECLARATION_METADATA_PATH,
    CSHARP_GENERIC_NAME_METADATA_PATH, CSHARP_INTERPOLATED_STRING_METADATA_PATH,
    CSHARP_LOCAL_ASSIGNMENT_METADATA_PATH, CSHARP_LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH,
    CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH, CSHARP_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH, CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH,
    CSHARP_LOCAL_IF_ELSE_METADATA_PATH, CSHARP_LOCAL_IMPLICIT_DECLARATION_METADATA_PATH,
    CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH,
    CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_NULL_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_NULLABLE_DECLARATION_METADATA_PATH,
    CSHARP_LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH,
    CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH,
    CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH, CSHARP_LOCAL_SWITCH_METADATA_PATH,
    CSHARP_LOCAL_TYPED_DECLARATION_METADATA_PATH, CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH,
    CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH, CSHARP_METADATA_PATH,
    CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH,
    CSHARP_METHOD_INVOCATION_RETURN_METADATA_PATH,
    CSHARP_NULLABLE_PARAMETER_DECLARATION_METADATA_PATH,
    CSHARP_NULLABLE_PROPERTY_DECLARATION_METADATA_PATH, CSHARP_PATTERN_EXPRESSION_METADATA_PATH,
    CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH, CSHARP_QUALIFIED_NAME_METADATA_PATH,
    CSHARP_RANGE_ELEMENT_ACCESS_METADATA_PATH, CSHARP_STRUCT_DECLARATION_METADATA_PATH,
    CSHARP_THIS_EXPRESSION_METADATA_PATH, CSHARP_TWO_PARAMETER_METHOD_METADATA_PATH,
    CSHARP_USING_DIRECTIVE_METADATA_PATH, CSHARP_WHILE_POSTFIX_LOOP_METADATA_PATH,
    CSHARP_YIELD_STATEMENT_METADATA_PATH, GENERATED_ARTIFACT_ENVELOPE_FIXTURE,
    GENERATED_ARTIFACT_ENVELOPE_PATH, GENERATED_ARTIFACT_REPLAY_FIXTURE,
    GENERATED_ARTIFACT_REPLAY_PATH, GO_ARRAY_COMPOSITE_LITERAL_METADATA_PATH,
    GO_COMPOSITE_LITERAL_METADATA_PATH, GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH,
    GO_CRYPTO_SUBTLE_XOR_SOURCE_FILE_METADATA_PATH,
    GO_CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_FILE_METADATA_PATH,
    GO_ESCAPED_LOCAL_ASSIGNMENT_METADATA_PATH, GO_FUNCTION_LITERAL_METADATA_PATH,
    GO_IMPORT_SPEC_COMMENT_METADATA_PATH, GO_IMPORT_SPEC_NAME_METADATA_PATH,
    GO_INDEX_EXPRESSION_METADATA_PATH, GO_INTERNAL_COVERAGE_ULEB128_SOURCE_FILE_METADATA_PATH,
    GO_KEY_VALUE_COMPOSITE_LITERAL_METADATA_PATH,
    GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH, GO_LOCAL_ASSIGNMENT_METADATA_PATH,
    GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
    GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH, GO_LOCAL_BRANCH_STATEMENT_METADATA_PATH,
    GO_LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH, GO_LOCAL_COMPOUND_ASSIGNMENT_METADATA_PATH,
    GO_LOCAL_DECLARATION_METADATA_PATH, GO_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
    GO_LOCAL_FOR_CONDITION_METADATA_PATH, GO_LOCAL_GENERIC_FUNCTION_METADATA_PATH,
    GO_LOCAL_IF_ELSE_METADATA_PATH, GO_LOCAL_INCREMENT_DECREMENT_METADATA_PATH,
    GO_LOCAL_INTEGER_ASSIGNMENT_METADATA_PATH, GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
    GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH,
    GO_LOCAL_INTEGER_TYPED_DECLARATION_METADATA_PATH, GO_LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH,
    GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
    GO_LOCAL_RANGE_STRING_METADATA_PATH, GO_LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH,
    GO_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH, GO_LOCAL_SWITCH_INIT_METADATA_PATH,
    GO_LOCAL_SWITCH_METADATA_PATH, GO_LOCAL_TYPE_SWITCH_METADATA_PATH,
    GO_LOCAL_TYPED_DECLARATION_METADATA_PATH,
    GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
    GO_LOCAL_UNTYPED_INTEGER_DECLARATION_METADATA_PATH, GO_LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH,
    GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_METADATA_PATH, GO_METADATA_PATH,
    GO_METHOD_DECLARATION_METADATA_PATH, GO_OS_SIGNAL_NOTIFY_CONTEXT_SOURCE_SPAN_METADATA_PATH,
    GO_PACKAGE_CONST_DECLARATION_METADATA_PATH, GO_PACKAGE_TYPE_DECLARATION_METADATA_PATH,
    GO_PACKAGE_VAR_DECLARATION_METADATA_PATH, GO_PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_METADATA_PATH,
    GO_PATH_MATCH_SOURCE_FILE_METADATA_PATH, GO_SLICE_EXPRESSION_METADATA_PATH,
    GO_SLICE_EXPRESSION_THREE_INDEX_METADATA_PATH,
    GO_SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_METADATA_PATH,
    GO_SORT_INTERFACE_SOURCE_FILE_METADATA_PATH,
    GO_SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_METADATA_PATH,
    GO_SORT_SEARCH_SOURCE_FILE_METADATA_PATH, GO_STAR_EXPRESSION_METADATA_PATH,
    GO_STRINGS_CLONE_SOURCE_FILE_METADATA_PATH, GO_STRINGS_COMPARE_SOURCE_FILE_METADATA_PATH,
    GO_STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_METADATA_PATH,
    GO_STRINGS_SEARCH_SOURCE_FILE_METADATA_PATH, GO_STRUCT_TYPE_METADATA_PATH,
    GO_TYPE_SPEC_ASSIGN_METADATA_PATH, GO_TYPE_SPEC_COMMENT_METADATA_PATH,
    GO_TYPE_SPEC_DOC_METADATA_PATH, GO_TYPE_SPEC_TYPE_PARAMS_METADATA_PATH,
    GO_VALUE_SPEC_COMMENT_METADATA_PATH, INPUT_DIAGNOSTICS_CONTRACT_FIXTURE,
    INPUT_DIAGNOSTICS_CONTRACT_PATH, INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE,
    INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE_PATH, STRUCTURAL_MODEL_BOUNDARY_MANIFEST_PATH,
    SyntaxGeneratedCodecArtifact, VALIDATION_HOOK_CONTRACT_FIXTURE, VALIDATION_HOOK_CONTRACT_PATH,
    VALIDATION_HOOK_SIDECAR_EVIDENCE_FIXTURE, VALIDATION_HOOK_SIDECAR_EVIDENCE_PATH,
    build_csharp_array_rank_object_creation_syntax_generated_codec_artifact,
    build_csharp_arrow_expression_clause_syntax_generated_codec_artifact,
    build_csharp_attribute_syntax_generated_codec_artifact,
    build_csharp_base_list_syntax_generated_codec_artifact,
    build_csharp_block_scoped_namespace_syntax_generated_codec_artifact,
    build_csharp_collection_initializer_syntax_generated_codec_artifact,
    build_csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_artifact,
    build_csharp_compilation_unit_loan_source_file_syntax_generated_codec_artifact,
    build_csharp_compilation_unit_logger_source_file_syntax_generated_codec_artifact,
    build_csharp_compilation_unit_version_one_source_file_syntax_generated_codec_artifact,
    build_csharp_conditional_access_syntax_generated_codec_artifact,
    build_csharp_constructor_declaration_syntax_generated_codec_artifact,
    build_csharp_element_access_syntax_generated_codec_artifact,
    build_csharp_enum_declaration_syntax_generated_codec_artifact,
    build_csharp_event_declaration_syntax_generated_codec_artifact,
    build_csharp_field_declaration_syntax_generated_codec_artifact,
    build_csharp_generic_name_syntax_generated_codec_artifact,
    build_csharp_interpolated_string_syntax_generated_codec_artifact,
    build_csharp_local_assignment_syntax_generated_codec_artifact,
    build_csharp_local_binary_integer_return_syntax_generated_codec_artifact,
    build_csharp_local_boolean_return_literal_syntax_generated_codec_artifact,
    build_csharp_local_escaped_assignment_syntax_generated_codec_artifact,
    build_csharp_local_float_return_literal_syntax_generated_codec_artifact,
    build_csharp_local_for_condition_syntax_generated_codec_artifact,
    build_csharp_local_foreach_string_syntax_generated_codec_artifact,
    build_csharp_local_if_else_syntax_generated_codec_artifact,
    build_csharp_local_implicit_declaration_syntax_generated_codec_artifact,
    build_csharp_local_integer_implicit_declaration_syntax_generated_codec_artifact,
    build_csharp_local_integer_return_literal_syntax_generated_codec_artifact,
    build_csharp_local_null_return_literal_syntax_generated_codec_artifact,
    build_csharp_local_nullable_declaration_syntax_generated_codec_artifact,
    build_csharp_local_nullable_return_type_syntax_generated_codec_artifact,
    build_csharp_local_parenthesized_integer_return_syntax_generated_codec_artifact,
    build_csharp_local_switch_syntax_generated_codec_artifact,
    build_csharp_local_typed_declaration_syntax_generated_codec_artifact,
    build_csharp_local_unary_integer_return_syntax_generated_codec_artifact,
    build_csharp_member_access_expression_syntax_generated_codec_artifact,
    build_csharp_method_invocation_argument_return_syntax_generated_codec_artifact,
    build_csharp_method_invocation_return_syntax_generated_codec_artifact,
    build_csharp_nullable_parameter_declaration_syntax_generated_codec_artifact,
    build_csharp_nullable_property_declaration_syntax_generated_codec_artifact,
    build_csharp_pattern_expression_syntax_generated_codec_artifact,
    build_csharp_property_plus_method_syntax_generated_codec_artifact,
    build_csharp_qualified_name_syntax_generated_codec_artifact,
    build_csharp_range_element_access_syntax_generated_codec_artifact,
    build_csharp_struct_declaration_syntax_generated_codec_artifact,
    build_csharp_this_expression_syntax_generated_codec_artifact,
    build_csharp_two_parameter_method_syntax_generated_codec_artifact,
    build_csharp_using_directive_syntax_generated_codec_artifact,
    build_csharp_while_postfix_loop_syntax_generated_codec_artifact,
    build_csharp_yield_statement_syntax_generated_codec_artifact,
    build_go_array_composite_literal_syntax_generated_codec_artifact,
    build_go_composite_literal_syntax_generated_codec_artifact,
    build_go_concurrency_map_interface_type_assertion_syntax_generated_codec_artifact,
    build_go_crypto_subtle_xor_source_file_syntax_generated_codec_artifact,
    build_go_crypto_sysrand_rand_windows_source_file_syntax_generated_codec_artifact,
    build_go_escaped_local_assignment_syntax_generated_codec_artifact,
    build_go_function_literal_syntax_generated_codec_artifact,
    build_go_index_expression_syntax_generated_codec_artifact,
    build_go_internal_coverage_uleb128_source_file_syntax_generated_codec_artifact,
    build_go_key_value_composite_literal_syntax_generated_codec_artifact,
    build_go_local_argument_call_return_expression_syntax_generated_codec_artifact,
    build_go_local_assignment_syntax_generated_codec_artifact,
    build_go_local_binary_integer_return_expression_syntax_generated_codec_artifact,
    build_go_local_boolean_return_identifier_syntax_generated_codec_artifact,
    build_go_local_branch_statement_syntax_generated_codec_artifact,
    build_go_local_call_return_expression_syntax_generated_codec_artifact,
    build_go_local_compound_assignment_syntax_generated_codec_artifact,
    build_go_local_float_return_literal_syntax_generated_codec_artifact,
    build_go_local_for_condition_syntax_generated_codec_artifact,
    build_go_local_generic_function_syntax_generated_codec_artifact,
    build_go_local_if_else_syntax_generated_codec_artifact,
    build_go_local_increment_decrement_syntax_generated_codec_artifact,
    build_go_local_integer_assignment_syntax_generated_codec_artifact,
    build_go_local_integer_return_literal_syntax_generated_codec_artifact,
    build_go_local_integer_short_var_declaration_syntax_generated_codec_artifact,
    build_go_local_integer_typed_declaration_syntax_generated_codec_artifact,
    build_go_local_nil_return_identifier_syntax_generated_codec_artifact,
    build_go_local_parenthesized_integer_return_expression_syntax_generated_codec_artifact,
    build_go_local_range_string_syntax_generated_codec_artifact,
    build_go_local_short_var_declaration_syntax_generated_codec_artifact,
    build_go_local_string_return_literal_syntax_generated_codec_artifact,
    build_go_local_switch_init_syntax_generated_codec_artifact,
    build_go_local_switch_syntax_generated_codec_artifact,
    build_go_local_type_switch_syntax_generated_codec_artifact,
    build_go_local_typed_declaration_syntax_generated_codec_artifact,
    build_go_local_unary_integer_return_expression_syntax_generated_codec_artifact,
    build_go_local_untyped_integer_declaration_syntax_generated_codec_artifact,
    build_go_local_variadic_ellipsis_syntax_generated_codec_artifact,
    build_go_local_zero_value_typed_declaration_syntax_generated_codec_artifact,
    build_go_method_declaration_syntax_generated_codec_artifact,
    build_go_os_signal_notify_context_source_span_syntax_generated_codec_artifact,
    build_go_package_const_declaration_syntax_generated_codec_artifact,
    build_go_package_type_declaration_syntax_generated_codec_artifact,
    build_go_package_var_declaration_syntax_generated_codec_artifact,
    build_go_path_match_source_file_match_prefix_syntax_generated_codec_artifact,
    build_go_path_match_source_file_syntax_generated_codec_artifact,
    build_go_slice_expression_syntax_generated_codec_artifact,
    build_go_slice_expression_three_index_syntax_generated_codec_artifact,
    build_go_sort_interface_source_file_declaration_prefix_syntax_generated_codec_artifact,
    build_go_sort_interface_source_file_syntax_generated_codec_artifact,
    build_go_sort_search_source_file_example_prefix_syntax_generated_codec_artifact,
    build_go_sort_search_source_file_syntax_generated_codec_artifact,
    build_go_star_expression_syntax_generated_codec_artifact,
    build_go_strings_clone_source_file_syntax_generated_codec_artifact,
    build_go_strings_compare_source_file_syntax_generated_codec_artifact,
    build_go_strings_search_source_file_finder_prefix_syntax_generated_codec_artifact,
    build_go_strings_search_source_file_syntax_generated_codec_artifact,
    build_go_struct_type_syntax_generated_codec_artifact,
    build_go_type_spec_assign_syntax_generated_codec_artifact,
    build_go_type_spec_comment_syntax_generated_codec_artifact,
    build_go_type_spec_doc_syntax_generated_codec_artifact,
    build_go_type_spec_type_params_syntax_generated_codec_artifact,
    build_registered_parser_backed_syntax_generated_codec_artifact,
    build_toy_syntax_generated_codec_artifact, csharp_syntax_token_layout,
    generated_codec_prefix_state_api_metadata, go_syntax_token_layout,
    render_csharp_syntax_codec_fixture, render_go_syntax_codec_fixture,
    render_toy_syntax_codec_fixture, syntax_backend_generated_artifact_envelope,
    syntax_backend_generated_artifact_replay, syntax_backend_input_diagnostics_contract,
    syntax_backend_input_diagnostics_negative_fixture, syntax_backend_validation_hook_contract,
    syntax_backend_validation_hook_sidecar_evidence, syntax_structural_model_boundary_manifest,
};
use tct::syntax_go_real_source_generated_codec_corpus::COMPLETED_PROMOTION_CLASS as GO_REAL_SOURCE_GENERATED_CODEC_CORPUS_COMPLETED_PROMOTION_CLASS;
use tct::syntax_go_source_instance_projection::{
    SyntaxGoSourceInstanceProjectionReport,
    parse_checked_report as parse_go_source_instance_report,
    parse_projection_artifact as parse_go_source_instance_projection_artifact,
};
use tct::syntax_go_unified_artifact::{GO_UNIFIED_GRAMMAR_ARTIFACT_ID, GO_UNIFIED_GRAMMAR_ID};
use tct::syntax_production_route::{
    CODEC_REGISTRY_SOURCE_OF_TRUTH, CSHARP_ARRAY_RANK_OBJECT_CREATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_ARROW_EXPRESSION_CLAUSE_COMPLETED_PROMOTION_CLASS,
    CSHARP_ATTRIBUTE_SYNTAX_COMPLETED_PROMOTION_CLASS, CSHARP_BASE_LIST_COMPLETED_PROMOTION_CLASS,
    CSHARP_BLOCK_SCOPED_NAMESPACE_COMPLETED_PROMOTION_CLASS,
    CSHARP_COLLECTION_INITIALIZER_COMPLETED_PROMOTION_CLASS,
    CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
    CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
    CSHARP_CONDITIONAL_ACCESS_COMPLETED_PROMOTION_CLASS,
    CSHARP_CONSTRUCTOR_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_ELEMENT_ACCESS_COMPLETED_PROMOTION_CLASS,
    CSHARP_ENUM_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_EVENT_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_FIELD_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_BINARY_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_ESCAPED_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_FLOAT_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_FOR_CONDITION_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_FOREACH_STRING_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_IF_ELSE_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_IMPLICIT_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_INTEGER_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_NULL_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_NULLABLE_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_NULLABLE_RETURN_TYPE_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_STRING_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_SWITCH_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_LOCAL_UNARY_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS,
    CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_COMPLETED_PROMOTION_CLASS,
    CSHARP_METHOD_INVOCATION_RETURN_COMPLETED_PROMOTION_CLASS,
    CSHARP_MINIMAL_CLASS_COMPLETED_PROMOTION_CLASS,
    CSHARP_NULLABLE_PARAMETER_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_NULLABLE_PROPERTY_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_PROPERTY_PLUS_METHOD_COMPLETED_PROMOTION_CLASS,
    CSHARP_RANGE_ELEMENT_ACCESS_COMPLETED_PROMOTION_CLASS,
    CSHARP_STRUCT_DECLARATION_COMPLETED_PROMOTION_CLASS,
    CSHARP_THIS_EXPRESSION_COMPLETED_PROMOTION_CLASS,
    CSHARP_TWO_PARAMETER_METHOD_COMPLETED_PROMOTION_CLASS,
    CSHARP_USING_DIRECTIVE_COMPLETED_PROMOTION_CLASS,
    GO_ARRAY_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS,
    GO_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS,
    GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_COMPLETED_PROMOTION_CLASS,
    GO_ESCAPED_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
    GO_FUNCTION_LITERAL_COMPLETED_PROMOTION_CLASS,
    GO_IMPORT_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS,
    GO_IMPORT_SPEC_NAME_COMPLETED_PROMOTION_CLASS, GO_INDEX_EXPRESSION_COMPLETED_PROMOTION_CLASS,
    GO_KEY_VALUE_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_BRANCH_STATEMENT_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_CALL_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_COMPOUND_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_FLOAT_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_FOR_CONDITION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_GENERIC_FUNCTION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_IF_ELSE_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_INCREMENT_DECREMENT_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_INTEGER_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_INTEGER_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_INTEGER_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_NIL_RETURN_IDENTIFIER_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_RANGE_STRING_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_SHORT_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_STRING_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_SWITCH_COMPLETED_PROMOTION_CLASS, GO_LOCAL_SWITCH_INIT_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_TYPE_SWITCH_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_UNTYPED_INTEGER_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_VARIADIC_ELLIPSIS_COMPLETED_PROMOTION_CLASS,
    GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_METHOD_DECLARATION_COMPLETED_PROMOTION_CLASS, GO_MINIMAL_FUNCTION_COMPLETED_PROMOTION_CLASS,
    GO_PACKAGE_CONST_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_PACKAGE_TYPE_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_PACKAGE_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS,
    GO_SLICE_EXPRESSION_COMPLETED_PROMOTION_CLASS,
    GO_SLICE_EXPRESSION_THREE_INDEX_COMPLETED_PROMOTION_CLASS,
    GO_STAR_EXPRESSION_COMPLETED_PROMOTION_CLASS, GO_STRUCT_TYPE_COMPLETED_PROMOTION_CLASS,
    GO_TYPE_SPEC_ASSIGN_COMPLETED_PROMOTION_CLASS, GO_TYPE_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS,
    GO_TYPE_SPEC_DOC_COMPLETED_PROMOTION_CLASS, GO_VALUE_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS,
    NEXT_MISSING_STABLE_PROMOTION_CLASS, ROUTE_ENTRYPOINT, SYNTAX_CODEC_REGISTRY,
    SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SYNTAX_COMPILER_CONSUMED_TOKEN_ACCOUNTING_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_RESPONSE_EVIDENCE_FIXTURE,
    SYNTAX_COMPILER_ROUTE_COMPLETED_PROMOTION_CLASS, SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
    SYNTAX_COMPILER_STATE_API_ROUTE_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SYNTAX_COMPILER_STRICT_RECOVERY_DECODE_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_TOKEN_VOCABULARY_BPE_COMPLETED_PROMOTION_CLASS, SyntaxCompilerRouteRequest,
    SyntaxProductionRouteRequest, SyntaxProductionRouteTarget,
    build_csharp_compilation_unit_source_file_compiler_route_artifact,
    build_csharp_local_string_return_literal_compiler_route_artifact,
    build_csharp_minimal_class_compiler_route_artifact,
    build_go_local_declaration_compiler_route_artifact,
    build_go_minimal_function_compiler_route_artifact, compile_checked_syntax_compiler_route,
    compile_checked_syntax_production_route, syntax_codec_registry_entry,
};
use tct::syntax_state_machine::CompiledSyntaxModel;
use tct::syntax_state_machine_replay::{
    GENERATED_CODEC_STATE_API_SUCCESSOR_NEXT_TASK,
    build_go_source_instance_generated_codec_state_api_report,
    build_go_source_instance_strict_recovery_decode_report,
};
use tct::syntax_validation_matrix::{
    COMPLETED_PROMOTION_CLASS as VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    REQUIRED_JSON_BACKEND_GRADE_CATEGORIES as VALIDATION_MATRIX_REQUIRED_CATEGORIES,
    REQUIRED_ROUTE_OWNED_EVIDENCE as VALIDATION_MATRIX_ROUTE_EVIDENCE,
};
use tct::toy_grammar_frontend::{
    BYTE_TOKEN_BASE, IDENTIFIER_END_TOKEN, INTEGER_TOKEN_BASE, INTEGER_TOKEN_END,
    INTEGER_VALUE_TOKEN, PAD_TOKEN, PROGRAM_END_TOKEN, PROGRAM_START_TOKEN, STATEMENT_TOKEN,
    STRING_END_TOKEN, STRING_VALUE_TOKEN, TOY_GRAMMAR_LANGUAGE_ID, TRUE_VALUE_TOKEN,
    ToyGrammarSpec, VOCAB_SIZE,
};
use tempfile::TempDir;

const GO_SOURCE_INSTANCE_CHECKED_REPORT_PATH: &str =
    "tests/fixtures/go_syntax_corpus/source-instance-projection-report-httptest-server-v1.json";

fn checked_go_source_instance_report() -> SyntaxGoSourceInstanceProjectionReport {
    let contents = fs::read_to_string(GO_SOURCE_INSTANCE_CHECKED_REPORT_PATH)
        .expect("checked Go source-instance projection report should exist");
    parse_go_source_instance_report(&contents)
        .expect("checked Go source-instance projection report should validate")
}

fn regenerated_go_source_instance_projection_artifact_for(
    report: &SyntaxGoSourceInstanceProjectionReport,
) -> String {
    let tempdir = TempDir::new().expect("Go source-instance projection tempdir should be created");
    let output_path = tempdir.path().join("projection.json");
    let output = Command::new(go_sidecar_go_bin())
        .arg("run")
        .arg("scripts/analysis/go_syntax_source_instance_project.go")
        .arg("--source")
        .arg(&report.source.path)
        .arg("--metadata")
        .arg(&report.metadata.path)
        .arg("--manifest-path")
        .arg(&report.manifest_path)
        .arg("--candidate-grammar-id")
        .arg(&report.candidate_grammar_id)
        .arg("--projection-output")
        .arg(&output_path)
        .output()
        .expect("Go source-instance projection helper should run");
    assert!(
        output.status.success(),
        "Go source-instance projection helper failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(output_path).expect("Go source-instance projection artifact should exist")
}

#[test]
#[ignore = "known failure: checked httptest-server projection report keeps per-file grammar id go_httptest_server_source_file_syntax_v0 without a per_file_grammar_blocker; see docs/frontends-development.md"]
fn generated_codec_state_api_exposes_admissible_raw_tokens_for_checked_go_source_instance() {
    if go_sidecar_toolchain(
        "generated_codec_state_api_exposes_admissible_raw_tokens_for_checked_go_source_instance",
    )
    .is_none()
    {
        return;
    }
    let report = checked_go_source_instance_report();
    let artifact_contents = regenerated_go_source_instance_projection_artifact_for(&report);
    let artifact = parse_go_source_instance_projection_artifact(&artifact_contents)
        .expect("Go source-instance projection artifact should parse");
    let raw_tokens = artifact
        .raw_token_ids_against_report(&report)
        .expect("projection artifact should emit raw token IDs");
    let generated_report = artifact
        .generated_codec_report_against_report(&report)
        .expect("projection artifact should emit generated-codec report");

    let state_api_report = build_go_source_instance_generated_codec_state_api_report(
        &report,
        &raw_tokens,
        &generated_report,
    )
    .expect("state API report should build for checked Go source-instance evidence");
    state_api_report
        .validate()
        .expect("state API report should validate");

    let root = state_api_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "empty_prefix")
        .expect("state API report should include empty-prefix query");
    assert_eq!(root.admissible_next_raw_token_ids, vec![0]);
    assert!(!root.eof_valid);

    let decls = state_api_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "after_file_decls_field_start")
        .expect("state API report should include File.Decls prefix query");
    assert_eq!(decls.active_rule.as_deref(), Some("File"));
    assert_eq!(decls.active_field.as_deref(), Some("Decls"));
    assert!(
        !decls.admissible_next_raw_token_ids.contains(&0),
        "multi-alternative child state should not admit generic node_start"
    );
    assert!(state_api_report.payload_range_state_api_ready);
    let prefix_state_metadata =
        generated_codec_prefix_state_api_metadata(&report.candidate_grammar_id)
            .expect("Go source-instance generated codec should expose live prefix metadata");
    assert_eq!(
        prefix_state_metadata.api_name,
        "target_neutral_go_source_instance_prefix_state_continuation_contract"
    );
    assert_eq!(
        prefix_state_metadata.strict_decode_function,
        "target_neutral_go_source_instance_strict_decode"
    );
    let payload = state_api_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "inside_first_terminal_payload_after_one_byte")
        .expect("state API report should include payload-range query");
    assert!(payload.payload_end_admissible);
    assert_eq!(payload.active_payload_byte_count, 1);
    assert_eq!(payload.admissible_next_raw_token_ranges.len(), 1);

    let accepted = state_api_report
        .transition_cases
        .iter()
        .find(|case| case.case_id == "file_decls_actual_alternative_node_start_is_accepted")
        .expect("state API report should prove actual next transition");
    assert!(accepted.accepted);

    let rejected = state_api_report
        .transition_cases
        .iter()
        .find(|case| case.case_id == "file_decls_generic_node_start_is_rejected")
        .expect("state API report should reject wrong generic node_start");
    assert_eq!(
        rejected.error_kind.as_deref(),
        Some("unexpected_structural_token")
    );
    assert!(state_api_report.strict_recovery_decode_wiring_ready);
    assert_eq!(
        state_api_report.next_missing_workstream_blocker,
        GENERATED_CODEC_STATE_API_SUCCESSOR_NEXT_TASK
    );

    let strict_recovery_report = build_go_source_instance_strict_recovery_decode_report(
        &report,
        &raw_tokens,
        &generated_report,
        &state_api_report,
    )
    .expect("strict/recovery decode report should build from state API evidence");
    strict_recovery_report
        .validate()
        .expect("strict/recovery decode report should validate");
    assert!(strict_recovery_report.state_api_consumed);
    assert!(strict_recovery_report.strict_recovery_decode_wiring_ready);
    assert!(
        strict_recovery_report
            .strict_cases
            .iter()
            .any(
                |case| case.case_id == "strict_complete_stream_accepts_state_api_eof"
                    && case.accepted
                    && case.eof_valid
            )
    );
    assert!(
        strict_recovery_report
            .recovery_cases
            .iter()
            .any(|case| case.case_id
                == "recovery_trailing_pad_normalizes_after_state_api_complete"
                && case.observed_outcome == "normalized_trailing_pad"
                && case.ignored_trailing_tokens == 1)
    );
}

/// Integration-level mixed-payload rejection for the generated-codec compile
/// path (`CompiledSyntaxModel::from_sidecar`).
///
/// Reconstructs a minimal mixed sidecar from public GrammarIR APIs rather than
/// lifting `mixed_payload_witness` out of `#[cfg(test)] pub(crate)`: that keeps
/// the production surface unchanged (the residual only needs a fixture the
/// integration test can name). Shape matches the lib-unit pin probe: a
/// Go-prefixed grammar with a pinned Nonterminal alternative (`EmptyStmt`) and
/// an unpinned IntegerPayload alternative (`number`) so layout assignment
/// succeeds and only `ensure_mixed_field_choice_tokens` hard-errors.
#[test]
fn go_generated_codec_route_rejects_unpinned_mixed_payload_sidecar() {
    let mut grammar = GrammarIr::new(
        "go_mixed_payload_generated_codec_rejection_probe_v0",
        GrammarIrProvenance {
            source_kind: "synthetic_mixed_payload_generated_codec_rejection_probe".to_string(),
            source_path: Some(
                "tests/syntax_generated_codec_gate_test.rs::go_generated_codec_route_rejects_unpinned_mixed_payload_sidecar"
                    .to_string(),
            ),
            source_sha256: None,
            metadata_path: None,
            metadata_sha256: None,
            parser: None,
        },
    );
    grammar.rules.push(GrammarRule {
        name: "wrapper".to_string(),
        node_kind: "wrapper".to_string(),
        fields: vec![GrammarField {
            name: "value".to_string(),
            required: true,
            multiple: false,
            alternatives: vec![
                GrammarFieldAlternative {
                    node_kind: "EmptyStmt".to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
                GrammarFieldAlternative {
                    node_kind: "number".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                },
            ],
        }],
    });
    grammar.rules.push(GrammarRule {
        name: "EmptyStmt".to_string(),
        node_kind: "EmptyStmt".to_string(),
        fields: vec![],
    });

    let layout = ToyGrammarSpec::assignment_language_v1().layout;
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&layout),
    )
    .expect("mixed-payload rejection probe should lower to a StructuralModel sidecar");

    let err = CompiledSyntaxModel::from_sidecar(&sidecar).expect_err(
        "Go-prefixed mixed sidecar must be rejected by the generated-codec compile path \
         (CompiledSyntaxModel::from_sidecar) instead of free-slot inventing choice tokens",
    );

    assert_eq!(
        err.field(),
        "syntax_state_machine.mixed_payload.choice_token",
        "typed error field must identify the mixed-payload choice_token phase: {err}"
    );
    let message = err.to_string();
    assert!(
        message.contains("owner_rule=wrapper"),
        "diagnostic must name the offending rule: {message}"
    );
    assert!(
        message.contains("field=value"),
        "diagnostic must name the offending field: {message}"
    );
    assert!(
        message.contains("node_kind=number"),
        "diagnostic must name the unpinned payload node kind: {message}"
    );
    assert!(
        message.contains("terminal_class=") || message.contains("IntegerPayload"),
        "diagnostic must name the terminal class: {message}"
    );
    assert!(
        message.contains("no pinned alternative"),
        "diagnostic must identify the pin guard: {message}"
    );
    assert!(
        message.contains(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        "diagnostic must point at the pin fixture path: {message}"
    );
    assert!(
        message.contains(GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE),
        "diagnostic must name the append rule: {message}"
    );
}

#[test]
fn toy_syntax_generated_codec_fixture_compiles_and_checks_decode_contracts() {
    let spec = ToyGrammarSpec::assignment_language_v1();
    let grammar = spec.to_grammar_ir();
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&spec.layout),
    )
    .unwrap();
    assert_eq!(sidecar.input.grammar_id, TOY_GRAMMAR_LANGUAGE_ID);
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

    let generated = render_toy_syntax_codec_fixture(&sidecar);
    assert_eq!(generated, render_toy_syntax_codec_fixture(&sidecar));
    assert!(!generated.is_empty());

    run_generated_syntax_codec_fixture_tests("toy-syntax-generated-codec", &generated);
}

#[test]
fn csharp_syntax_generated_codec_fixture_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_minimal_class_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
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

    let generated = render_csharp_syntax_codec_fixture(&sidecar);
    assert_eq!(generated, render_csharp_syntax_codec_fixture(&sidecar));
    assert!(!generated.is_empty());

    run_generated_syntax_codec_fixture_tests("csharp-syntax-generated-codec", &generated);
}

#[test]
fn csharp_minimal_class_syntax_generated_codec_compiles_through_compiler_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_minimal_class_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# minimal class sidecar should compile through target-agnostic compiler route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_MINIMAL_CLASS_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["entrypoint"],
        SYNTAX_COMPILER_ROUTE_ENTRYPOINT
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["generated_code_gate_completed_promotion_class"],
        SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-minimal-class-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_compilation_unit_source_file_syntax_generated_codec_compiles_through_compiler_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_compilation_unit_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("corpora/csharp/dotnet-samples-mini/src/pattern-matching/Shapes.cs")
    );
    assert_eq!(
        sidecar.input.provenance.source_sha256.as_deref(),
        Some("2714e85da9030b88559ce8548a3cce2e6f446681002380213c0bbf3e0d4a454e")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "NamespaceDeclarationSyntax"
                    && reference.field == "Members"
                    && reference.target_rule == "StructDeclarationSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AccessorDeclarationSyntax"
                && payload.field == "Keyword"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# compilation-unit source-file sidecar should compile through compiler route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["entrypoint"],
        SYNTAX_COMPILER_ROUTE_ENTRYPOINT
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["generated_code_gate_completed_promotion_class"],
        SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS
    );
    assert_target_agnostic_compiler_route_state_api_evidence(&artifact);
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact.source.contains(
            "generated_csharp_compilation_unit_source_file_writes_real_source_codec_report"
        )
    );
    assert!(
        artifact
            .source
            .contains("decode_compilation_unit_source_file_prefix_state")
    );
    assert!(artifact.source.contains(
        "generated_csharp_compilation_unit_source_file_prefix_state_accepts_selected_live_continuations"
    ));
    let prefix_state_metadata =
        generated_codec_prefix_state_api_metadata("csharp_compilation_unit_source_file_syntax_v0")
            .expect(
                "C# compilation-unit source-file generated codec should expose prefix metadata",
            );
    assert_eq!(
        prefix_state_metadata.api_name,
        "decode_compilation_unit_source_file_prefix_state"
    );
    assert_eq!(
        prefix_state_metadata.strict_decode_function,
        "decode_compilation_unit_source_file"
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/csharp-shapes-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "csharp-compilation-unit-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_CSHARP_COMPILATION_UNIT_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated C# Shapes codec report was not written"),
    )
    .expect("generated C# Shapes codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_csharp_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(report["target"], "csharp");
    assert_eq!(
        report["route"],
        "csharp_compilation_unit_source_file_syntax_v0"
    );
    assert_eq!(
        report["source_path"],
        "corpora/csharp/dotnet-samples-mini/src/pattern-matching/Shapes.cs"
    );
    assert_eq!(
        report["source_sha256"],
        "2714e85da9030b88559ce8548a3cce2e6f446681002380213c0bbf3e0d4a454e"
    );
    assert_eq!(
        report["generated_codec_marker"],
        "generated_csharp_compilation_unit_source_file_round_trips"
    );
    assert_eq!(report["parser_validation"]["roslyn_metadata"], "checked");
    assert_eq!(report["tct_token_count"], 77);
    assert_csharp_real_source_report_retargeted_structural_replay_tokens(&report, 282, 77);
    assert_eq!(report["baseline_byte_token_count"], 1003);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend C# real-source generated-codec corpus gate"
    );
}

#[test]
fn csharp_compilation_unit_logger_source_file_syntax_generated_codec_compiles_through_compiler_route()
 {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_compilation_unit_logger_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH,
        ))
        .unwrap();
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_logger_source_file_syntax_v0"
    );
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("corpora/csharp/dotnet-samples-mini/src/delegates-and-events/Logger.cs")
    );
    assert_eq!(
        sidecar.input.provenance.source_sha256.as_deref(),
        Some("fe3c59d6da3c21872acb23a4d7aae0476a9ef977bbf30b11eda9aa21e1c97a0f")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ClassDeclarationSyntax"
                && reference.field == "Members"
                && reference.target_rule == "MethodDeclarationSyntax")
    );

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# Logger compilation-unit sidecar should compile through compiler route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_COMPLETED_PROMOTION_CLASS
    );
    assert_target_agnostic_compiler_route_state_api_evidence(&artifact);
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH
    );
    assert!(artifact.source.contains(
        "generated_csharp_compilation_unit_logger_source_file_writes_real_source_codec_report"
    ));

    let direct_artifact =
        build_csharp_compilation_unit_logger_source_file_syntax_generated_codec_artifact();
    assert!(direct_artifact.source.contains(
        "generated_csharp_compilation_unit_logger_source_file_writes_real_source_codec_report"
    ));
    assert_eq!(
        direct_artifact.metadata["generation_options"]["corpus_record"],
        "delegates-and-events/Logger.cs"
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/csharp-logger-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "csharp-compilation-unit-logger-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated C# Logger codec report was not written"),
    )
    .expect("generated C# Logger codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_csharp_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(report["target"], "csharp");
    assert_eq!(
        report["route"],
        "csharp_compilation_unit_source_file_syntax_v0"
    );
    assert_eq!(
        report["source_path"],
        "corpora/csharp/dotnet-samples-mini/src/delegates-and-events/Logger.cs"
    );
    assert_eq!(
        report["source_sha256"],
        "fe3c59d6da3c21872acb23a4d7aae0476a9ef977bbf30b11eda9aa21e1c97a0f"
    );
    assert_eq!(
        report["generated_codec_marker"],
        "generated_csharp_compilation_unit_source_file_round_trips"
    );
    assert_eq!(report["parser_validation"]["roslyn_metadata"], "checked");
    assert_eq!(report["tct_token_count"], 29);
    assert_csharp_real_source_report_retargeted_structural_replay_tokens(&report, 340, 29);
    assert_eq!(report["baseline_byte_token_count"], 553);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend C# real-source generated-codec corpus gate"
    );
}

#[test]
fn csharp_compilation_unit_version_one_source_file_syntax_generated_codec_compiles_through_compiler_route()
 {
    let adapter = CSharpSyntaxMetadataAdapter::new(
        "csharp_compilation_unit_version_one_source_file_syntax_v0",
    );
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH,
        ))
        .unwrap();
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_version_one_source_file_syntax_v0"
    );
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("corpora/csharp/dotnet-samples-mini/src/events/VersionOne.cs")
    );
    assert_eq!(
        sidecar.input.provenance.source_sha256.as_deref(),
        Some("ee5faea8bc7d22031adba420d7662a001386a09af49858fcce81a8aced49ece7")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ClassDeclarationSyntax"
                && reference.field == "Members"
                && reference.target_rule == "EventFieldDeclarationSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "ConditionalAccessExpressionSyntax"
                    && reference.field == "WhenNotNull"
                    && reference.target_rule == "InvocationExpressionSyntax"
            )
    );

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# VersionOne compilation-unit sidecar should compile through compiler route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["construct_class"],
        "csharp_compilation_unit_version_one_source_file"
    );
    assert_target_agnostic_compiler_route_state_api_evidence(&artifact);
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH
    );
    assert!(artifact.source.contains(
        "generated_csharp_compilation_unit_version_one_source_file_writes_real_source_codec_report"
    ));

    let direct_artifact =
        build_csharp_compilation_unit_version_one_source_file_syntax_generated_codec_artifact();
    assert!(direct_artifact.source.contains(
        "generated_csharp_compilation_unit_version_one_source_file_writes_real_source_codec_report"
    ));
    assert_eq!(
        direct_artifact.metadata["generation_options"]["corpus_record"],
        "events/VersionOne.cs"
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/csharp-version-one-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "csharp-compilation-unit-version-one-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated C# VersionOne codec report was not written"),
    )
    .expect("generated C# VersionOne codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_csharp_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(report["target"], "csharp");
    assert_eq!(
        report["route"],
        "csharp_compilation_unit_source_file_syntax_v0"
    );
    assert_eq!(
        report["source_path"],
        "corpora/csharp/dotnet-samples-mini/src/events/VersionOne.cs"
    );
    assert_eq!(
        report["source_sha256"],
        "ee5faea8bc7d22031adba420d7662a001386a09af49858fcce81a8aced49ece7"
    );
    assert_eq!(
        report["generated_codec_marker"],
        "generated_csharp_compilation_unit_source_file_round_trips"
    );
    assert_eq!(report["parser_validation"]["roslyn_metadata"], "checked");
    assert_eq!(report["tct_token_count"], 54);
    assert_csharp_real_source_report_retargeted_structural_replay_tokens(&report, 337, 54);
    assert_eq!(report["baseline_byte_token_count"], 497);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend C# real-source generated-codec corpus gate"
    );
}

#[test]
fn csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_compiles_through_compiler_route()
 {
    let adapter = CSharpSyntaxMetadataAdapter::new(
        "csharp_compilation_unit_iterator_methods_source_file_syntax_v0",
    );
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH,
        ))
        .unwrap();
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_iterator_methods_source_file_syntax_v0"
    );
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("corpora/csharp/dotnet-samples-mini/src/iterators/IteratorMethods.cs")
    );
    assert_eq!(
        sidecar.input.provenance.source_sha256.as_deref(),
        Some("2f47b8449ee3d6056fa341da57ac691a8c3b13a2776d7fd0b03e847ea8f4e976")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "WhileStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ForEachStatementSyntax"
                && reference.field == "Statement"
                && reference.target_rule == "YieldStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "InvocationExpressionSyntax")
    );

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# IteratorMethods compilation-unit sidecar should compile through compiler route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["construct_class"],
        "csharp_compilation_unit_iterator_methods_source_file"
    );
    assert_target_agnostic_compiler_route_state_api_evidence(&artifact);
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH
    );
    assert!(artifact.source.contains(
        "generated_csharp_compilation_unit_iterator_methods_source_file_writes_real_source_codec_report"
    ));
    assert!(
        artifact
            .source
            .contains("decode_compilation_unit_source_file_prefix_state")
    );
    assert!(artifact.source.contains(
        "generated_csharp_compilation_unit_source_file_prefix_state_accepts_selected_live_continuations"
    ));

    let direct_artifact =
        build_csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_artifact(
        );
    assert!(direct_artifact.source.contains(
        "generated_csharp_compilation_unit_iterator_methods_source_file_writes_real_source_codec_report"
    ));
    assert_eq!(
        direct_artifact.metadata["generation_options"]["corpus_record"],
        "iterators/IteratorMethods.cs"
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root.join(
        "target/tct-syntax-real-source-demo/csharp-iterator-methods-generated-codec-demo.json",
    );
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "csharp-compilation-unit-iterator-methods-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated C# IteratorMethods codec report was not written"),
    )
    .expect("generated C# IteratorMethods codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_csharp_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(report["target"], "csharp");
    assert_eq!(
        report["route"],
        "csharp_compilation_unit_source_file_syntax_v0"
    );
    assert_eq!(
        report["source_path"],
        "corpora/csharp/dotnet-samples-mini/src/iterators/IteratorMethods.cs"
    );
    assert_eq!(
        report["source_sha256"],
        "2f47b8449ee3d6056fa341da57ac691a8c3b13a2776d7fd0b03e847ea8f4e976"
    );
    assert_eq!(
        report["generated_codec_marker"],
        "generated_csharp_compilation_unit_source_file_round_trips"
    );
    assert_eq!(report["parser_validation"]["roslyn_metadata"], "checked");
    assert_eq!(report["tct_token_count"], 121);
    assert_csharp_real_source_report_retargeted_structural_replay_tokens(&report, 482, 121);
    let tct_tokens = report["tct_tokens"]
        .as_array()
        .expect("C# IteratorMethods report should preserve raw token IDs");
    assert_eq!(tct_tokens.len(), 121);
    assert!(
        tct_tokens.iter().all(|token| token.as_u64().is_some()),
        "C# IteratorMethods raw token IDs must be unsigned integers"
    );
    let raw_token_ids_sha256 =
        json_sha256(&report["tct_tokens"]).expect("failed to hash C# IteratorMethods raw tokens");
    assert_eq!(
        report["raw_token_ids_sha256"].as_str(),
        Some(raw_token_ids_sha256.as_str())
    );
    assert_eq!(
        report["structural_replay"]["token_layout_kind"],
        "structural_model_replay_roles"
    );
    assert_eq!(
        report["structural_replay"]["source"],
        "src/syntax_state_machine_replay.rs::build_csharp_structural_replay_token_stream_from_sidecar"
    );
    assert_eq!(
        report["tct_tokens"],
        report["structural_replay"]["raw_token_ids"]
    );
    let structural_replay_tokens = report["structural_replay"]["raw_token_ids"]
        .as_array()
        .expect("C# IteratorMethods report should preserve structural replay raw token IDs");
    assert!(!structural_replay_tokens.is_empty());
    assert_eq!(
        report["structural_replay"]["raw_token_count"].as_u64(),
        Some(structural_replay_tokens.len() as u64)
    );
    let structural_replay_raw_token_ids_sha256 =
        json_sha256(&report["structural_replay"]["raw_token_ids"])
            .expect("failed to hash C# IteratorMethods structural replay raw tokens");
    assert_eq!(
        report["structural_replay"]["raw_token_ids_sha256"].as_str(),
        Some(structural_replay_raw_token_ids_sha256.as_str())
    );
    let structural_replay_segments = report["structural_replay"]["raw_token_segments"]
        .as_array()
        .expect("C# IteratorMethods report should preserve structural replay segments");
    assert!(!structural_replay_segments.is_empty());
    assert_eq!(
        report["structural_replay"]["raw_token_segment_count"].as_u64(),
        Some(structural_replay_segments.len() as u64)
    );
    let structural_replay_segments_sha256 =
        json_sha256(&report["structural_replay"]["raw_token_segments"])
            .expect("failed to hash C# IteratorMethods structural replay segments");
    assert_eq!(
        report["structural_replay"]["raw_token_segments_sha256"].as_str(),
        Some(structural_replay_segments_sha256.as_str())
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["payload_sequence_stream"],
        "payload_sequence_control_tokens"
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["structural_replay_stream"],
        "tct_tokens"
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["structural_replay_sidecar_stream"],
        "structural_replay.raw_token_ids"
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["payload_sequence_stream_kept_as_compatibility_control"],
        true
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["one_to_one_payload_sequence_offset_mapping"],
        false
    );
    assert_eq!(report["baseline_byte_token_count"], 1883);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend C# real-source generated-codec corpus gate"
    );
}

#[test]
fn csharp_compilation_unit_loan_source_file_syntax_generated_codec_compiles_through_compiler_route()
{
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_compilation_unit_loan_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH,
        ))
        .unwrap();
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_loan_source_file_syntax_v0"
    );
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("corpora/csharp/dotnet-samples-mini/src/serialization/Loan.cs")
    );
    assert_eq!(
        sidecar.input.provenance.source_sha256.as_deref(),
        Some("2435ba265f5db79ff296c9514d6c3480d053b348539a80d6bc63e8b2177ed609")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ClassDeclarationSyntax"
                && reference.field == "BaseList"
                && reference.target_rule == "BaseListSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "PropertyDeclarationSyntax"
                    && reference.field == "AttributeLists"
                    && reference.target_rule == "AttributeListSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "EventFieldDeclarationSyntax"
                    && reference.field == "Declaration"
                    && reference.target_rule == "VariableDeclarationSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "ConditionalAccessExpressionSyntax"
                    && reference.field == "WhenNotNull"
                    && reference.target_rule == "InvocationExpressionSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "ObjectCreationExpressionSyntax"
                    && reference.field == "ArgumentList"
                    && reference.target_rule == "ArgumentListSyntax"
            )
    );

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# Loan compilation-unit sidecar should compile through compiler route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["construct_class"],
        "csharp_compilation_unit_loan_source_file"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH
    );
    assert!(artifact.source.contains(
        "generated_csharp_compilation_unit_loan_source_file_writes_real_source_codec_report"
    ));

    let direct_artifact =
        build_csharp_compilation_unit_loan_source_file_syntax_generated_codec_artifact();
    assert!(direct_artifact.source.contains(
        "generated_csharp_compilation_unit_loan_source_file_writes_real_source_codec_report"
    ));
    assert_eq!(
        direct_artifact.metadata["generation_options"]["corpus_record"],
        "serialization/Loan.cs"
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path =
        repo_root.join("target/tct-syntax-real-source-demo/csharp-loan-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "csharp-compilation-unit-loan-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated C# Loan codec report was not written"),
    )
    .expect("generated C# Loan codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_csharp_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(report["target"], "csharp");
    assert_eq!(
        report["route"],
        "csharp_compilation_unit_source_file_syntax_v0"
    );
    assert_eq!(
        report["source_path"],
        "corpora/csharp/dotnet-samples-mini/src/serialization/Loan.cs"
    );
    assert_eq!(
        report["source_sha256"],
        "2435ba265f5db79ff296c9514d6c3480d053b348539a80d6bc63e8b2177ed609"
    );
    assert_eq!(
        report["generated_codec_marker"],
        "generated_csharp_compilation_unit_source_file_round_trips"
    );
    assert_eq!(report["parser_validation"]["roslyn_metadata"], "checked");
    assert_eq!(report["tct_token_count"], 100);
    assert_csharp_real_source_report_retargeted_structural_replay_tokens(&report, 555, 100);
    assert_eq!(report["baseline_byte_token_count"], 1139);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend C# real-source generated-codec corpus gate"
    );
}

#[test]
fn csharp_block_scoped_namespace_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_block_scoped_namespace_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_BLOCK_SCOPED_NAMESPACE_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/block_scoped_namespace/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CompilationUnitSyntax"
                && reference.field == "Members"
                && reference.target_rule == "NamespaceDeclarationSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "NamespaceDeclarationSyntax"
                    && reference.field == "Members"
                    && reference.target_rule == "ClassDeclarationSyntax"
            )
    );

    let artifact = build_csharp_block_scoped_namespace_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_BLOCK_SCOPED_NAMESPACE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_block_scoped_namespace_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-block-scoped-namespace-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_using_directive_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_using_directive_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_USING_DIRECTIVE_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/using_directive/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CompilationUnitSyntax"
            && reference.field == "Usings"
            && reference.target_rule == "UsingDirectiveSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CompilationUnitSyntax"
            && reference.field == "Members"
            && reference.target_rule == "NamespaceDeclarationSyntax"
    }));

    let artifact = build_csharp_using_directive_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_USING_DIRECTIVE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_using_directive_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-using-directive-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_qualified_name_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_qualified_name_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_QUALIFIED_NAME_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/qualified_name/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "UsingDirectiveSyntax"
            && reference.field == "Name"
            && reference.target_rule == "QualifiedNameSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "QualifiedNameSyntax"
            && reference.field == "Left"
            && reference.target_rule == "IdentifierNameSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "QualifiedNameSyntax"
            && reference.field == "Right"
            && reference.target_rule == "IdentifierNameSyntax"
    }));

    let artifact = build_csharp_qualified_name_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_QUALIFIED_NAME_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_qualified_name_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-qualified-name-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_constructor_declaration_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_constructor_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_CONSTRUCTOR_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/constructor_declaration/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ClassDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "ConstructorDeclarationSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ConstructorDeclarationSyntax"
            && reference.field == "Body"
            && reference.target_rule == "BlockSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ExpressionStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "AssignmentExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignmentExpressionSyntax"
            && reference.field == "Right"
            && reference.target_rule == "IdentifierNameSyntax"
    }));

    let artifact = build_csharp_constructor_declaration_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_CONSTRUCTOR_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_constructor_declaration_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-constructor-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_while_postfix_loop_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_while_postfix_loop_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_WHILE_POSTFIX_LOOP_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/while_postfix_loop/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockSyntax"
            && reference.field == "Statements"
            && reference.target_rule == "WhileStatementSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "WhileStatementSyntax"
            && reference.field == "Condition"
            && reference.target_rule == "BinaryExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ExpressionStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "PostfixUnaryExpressionSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "PostfixUnaryExpressionSyntax"
                    && payload.field == "OperatorToken"
                    && payload.payload_kind == "string"
            })
    );

    let artifact = build_csharp_while_postfix_loop_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_WHILE_POSTFIX_LOOP_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_while_postfix_loop_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-while-postfix-loop-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_interpolated_string_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_interpolated_string_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_INTERPOLATED_STRING_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/interpolated_string/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "InterpolatedStringExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "InterpolatedStringExpressionSyntax"
            && reference.field == "Contents"
            && reference.target_rule == "InterpolationSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "InterpolationSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "IdentifierNameSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "InterpolatedStringTextSyntax"
                    && payload.field == "TextToken"
                    && payload.payload_kind == "string"
            })
    );

    let artifact = build_csharp_interpolated_string_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_INTERPOLATED_STRING_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_interpolated_string_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-interpolated-string-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_member_access_expression_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_member_access_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "MemberAccessExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "MemberAccessExpressionSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "IdentifierNameSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "MemberAccessExpressionSyntax"
            && reference.field == "Name"
            && reference.target_rule == "IdentifierNameSyntax"
    }));

    let artifact = build_csharp_member_access_expression_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_member_access_expression_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-member-access-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_field_declaration_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_field_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_FIELD_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/field_declaration/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FieldDeclarationSyntax"
            && reference.field == "Declaration"
            && reference.target_rule == "VariableDeclarationSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "VariableDeclarationSyntax"
            && reference.field == "Type"
            && reference.target_rule == "PredefinedTypeSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "VariableDeclaratorSyntax"
                    && payload.field == "Identifier"
                    && payload.payload_kind == "identifier"
            })
    );

    let artifact = build_csharp_field_declaration_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_FIELD_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_field_declaration_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-field-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_base_list_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_base_list_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_BASE_LIST_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/base_list/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ClassDeclarationSyntax"
            && reference.field == "BaseList"
            && reference.target_rule == "BaseListSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BaseListSyntax"
            && reference.field == "Types"
            && reference.target_rule == "SimpleBaseTypeSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "IdentifierNameSyntax"
                    && payload.field == "Identifier"
                    && payload.payload_kind == "identifier"
            })
    );

    let artifact = build_csharp_base_list_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_BASE_LIST_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_base_list_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-base-list-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_conditional_access_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_conditional_access_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_CONDITIONAL_ACCESS_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/conditional_access/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ExpressionStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "ConditionalAccessExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ConditionalAccessExpressionSyntax"
            && reference.field == "WhenNotNull"
            && reference.target_rule == "InvocationExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "InvocationExpressionSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "MemberBindingExpressionSyntax"
    }));

    let artifact = build_csharp_conditional_access_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_CONDITIONAL_ACCESS_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_conditional_access_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-conditional-access-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_array_rank_object_creation_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_array_rank_object_creation_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_ARRAY_RANK_OBJECT_CREATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArrayTypeSyntax"
            && reference.field == "RankSpecifiers"
            && reference.target_rule == "ArrayRankSpecifierSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArrayRankSpecifierSyntax"
            && reference.field == "Sizes"
            && reference.target_rule == "OmittedArraySizeExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "ObjectCreationExpressionSyntax"
    }));

    let artifact = build_csharp_array_rank_object_creation_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_ARRAY_RANK_OBJECT_CREATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_array_rank_object_creation_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-array-rank-object-creation-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_this_expression_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_this_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_THIS_EXPRESSION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/this_expression/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArgumentSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "ThisExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ConditionalAccessExpressionSyntax"
            && reference.field == "WhenNotNull"
            && reference.target_rule == "InvocationExpressionSyntax"
    }));

    let artifact = build_csharp_this_expression_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_THIS_EXPRESSION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_this_expression_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-this-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_pattern_expression_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_pattern_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_PATTERN_EXPRESSION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "IsPatternExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "IsPatternExpressionSyntax"
            && reference.field == "Pattern"
            && reference.target_rule == "ConstantPatternSyntax"
    }));

    let artifact = build_csharp_pattern_expression_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_PATTERN_EXPRESSION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_pattern_expression_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-pattern-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_arrow_expression_clause_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_arrow_expression_clause_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_ARROW_EXPRESSION_CLAUSE_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "PropertyDeclarationSyntax"
            && reference.field == "ExpressionBody"
            && reference.target_rule == "ArrowExpressionClauseSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArrowExpressionClauseSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "LiteralExpressionSyntax"
    }));

    let artifact = build_csharp_arrow_expression_clause_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_ARROW_EXPRESSION_CLAUSE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_arrow_expression_clause_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-arrow-expression-clause-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_struct_declaration_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_struct_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_STRUCT_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/struct_declaration/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "NamespaceDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "StructDeclarationSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "StructDeclarationSyntax"
                    && payload.field == "Identifier"
                    && payload.payload_kind == "identifier"
            })
    );

    let artifact = build_csharp_struct_declaration_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_STRUCT_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_struct_declaration_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-struct-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_enum_declaration_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_enum_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_ENUM_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/enum_declaration/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "EnumDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "EnumMemberDeclarationSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "EnumMemberDeclarationSyntax"
                    && payload.field == "Identifier"
                    && payload.payload_kind == "identifier"
            })
    );

    let artifact = build_csharp_enum_declaration_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_ENUM_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_enum_declaration_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-enum-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_event_declaration_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_event_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_EVENT_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/event_declaration/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "EventFieldDeclarationSyntax"
            && reference.field == "Declaration"
            && reference.target_rule == "VariableDeclarationSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "VariableDeclaratorSyntax"
                    && payload.field == "Identifier"
                    && payload.payload_kind == "identifier"
            })
    );

    let artifact = build_csharp_event_declaration_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_EVENT_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_event_declaration_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-event-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_yield_statement_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_yield_statement_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_YIELD_STATEMENT_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/yield_statement/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockSyntax"
            && reference.field == "Statements"
            && reference.target_rule == "YieldStatementSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "YieldStatementSyntax"
                    && payload.field == "ReturnOrBreakKeyword"
                    && payload.payload_kind == "identifier"
            })
    );

    let artifact = build_csharp_yield_statement_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_YIELD_STATEMENT_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_yield_statement_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-yield-statement-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_attribute_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_attribute_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_ATTRIBUTE_SYNTAX_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/attribute_syntax/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ClassDeclarationSyntax"
            && reference.field == "AttributeLists"
            && reference.target_rule == "AttributeListSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "IdentifierNameSyntax"
                    && payload.field == "Identifier"
                    && payload.payload_kind == "identifier"
            })
    );

    let artifact = build_csharp_attribute_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_ATTRIBUTE_SYNTAX_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_attribute_syntax_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-attribute-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_generic_name_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_generic_name_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_GENERIC_NAME_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/generic_name/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "PropertyDeclarationSyntax"
            && reference.field == "Type"
            && reference.target_rule == "GenericNameSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "GenericNameSyntax"
                    && payload.field == "Identifier"
                    && payload.payload_kind == "identifier"
            })
    );

    let artifact = build_csharp_generic_name_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_GENERIC_NAME_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_generic_name_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-generic-name-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_string_return_literal_syntax_generated_codec_compiles_through_compiler_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_string_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ClassDeclarationSyntax"
                && reference.field == "Members"
                && reference.target_rule == "MethodDeclarationSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "C# local string return literal sidecar should compile through target-agnostic compiler route",
    );
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_STRING_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["entrypoint"],
        SYNTAX_COMPILER_ROUTE_ENTRYPOINT
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["generated_code_gate_completed_promotion_class"],
        SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["paired_route_evidence"]["csharp_non_minimal_fixture"],
        CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-string-return-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_integer_return_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_integer_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local integer return literal sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-integer-return-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_float_return_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_float_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_float_return_literal/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string"
                && payload.codec == "utf8_string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local float return literal sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_FLOAT_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_float_return_literal_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_float_return_literal_rejects_invalid_float_payloads")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-float-return-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_boolean_return_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_boolean_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_boolean_return_literal/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "identifier"
                && payload.codec == "utf8_identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local boolean return literal sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_boolean_return_literal_round_trips")
    );
    assert!(artifact.source.contains(
        "generated_csharp_local_boolean_return_literal_rejects_invalid_boolean_payloads"
    ));

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-boolean-return-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_null_return_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_null_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_NULL_RETURN_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_null_return_literal/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "identifier"
                && payload.codec == "utf8_identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local null return literal sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_NULL_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_NULL_RETURN_LITERAL_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_null_return_literal_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_null_return_literal_rejects_invalid_null_payloads")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-null-return-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_nullable_return_type_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_nullable_return_type_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_nullable_return_type/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "MethodDeclarationSyntax"
                    && reference.field == "ReturnType"
                    && reference.target_rule == "NullableTypeSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "NullableTypeSyntax"
                && payload.field == "QuestionToken"
                && payload.payload_kind == "string"
                && payload.codec == "utf8_string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local nullable return type sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_NULLABLE_RETURN_TYPE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_nullable_return_type_round_trips")
    );
    assert!(
        artifact.source.contains(
            "generated_csharp_local_nullable_return_type_rejects_invalid_nullable_marker"
        )
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_nullable_return_type_rejects_invalid_null_payloads")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-nullable-return-type-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_nullable_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_nullable_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_NULLABLE_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_nullable_declaration/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "VariableDeclarationSyntax"
                    && reference.field == "Type"
                    && reference.target_rule == "NullableTypeSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "VariableDeclaratorSyntax"
                    && reference.field == "Initializer"
                    && reference.target_rule == "EqualsValueClauseSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "NullableTypeSyntax"
                && payload.field == "QuestionToken"
                && payload.payload_kind == "string"
                && payload.codec == "utf8_string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "identifier"
                && payload.codec == "utf8_identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local nullable declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_NULLABLE_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_NULLABLE_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_nullable_declaration_round_trips")
    );
    assert!(
        artifact.source.contains(
            "generated_csharp_local_nullable_declaration_rejects_invalid_nullable_marker"
        )
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_nullable_declaration_rejects_invalid_null_payloads")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-nullable-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_nullable_property_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_nullable_property_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_NULLABLE_PROPERTY_DECLARATION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/nullable_property_declaration/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "PropertyDeclarationSyntax"
                    && reference.field == "Type"
                    && reference.target_rule == "NullableTypeSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "PropertyDeclarationSyntax"
                    && reference.field == "AccessorList"
                    && reference.target_rule == "AccessorListSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AccessorDeclarationSyntax"
                && payload.field == "Keyword"
                && payload.payload_kind == "string"
                && payload.codec == "utf8_string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "identifier"
                && payload.codec == "utf8_identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# nullable property declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_NULLABLE_PROPERTY_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_NULLABLE_PROPERTY_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_nullable_property_declaration_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_nullable_property_declaration_rejects_invalid_accessors")
    );
    assert!(
        artifact
            .source
            .contains("property_type_keyword: \"string?\"")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-nullable-property-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_nullable_parameter_declaration_syntax_generated_codec_compiles_through_production_route()
{
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_nullable_parameter_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_NULLABLE_PARAMETER_DECLARATION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/nullable_parameter_declaration/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ParameterSyntax"
                && reference.field == "Type"
                && reference.target_rule == "NullableTypeSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "MethodDeclarationSyntax"
                    && reference.field == "ParameterList"
                    && reference.target_rule == "ParameterListSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "NullableTypeSyntax"
                && payload.field == "QuestionToken"
                && payload.payload_kind == "string"
                && payload.codec == "utf8_string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string"
                && payload.codec == "utf8_string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# nullable parameter declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_NULLABLE_PARAMETER_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_NULLABLE_PARAMETER_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_nullable_parameter_declaration_round_trips")
    );
    assert!(artifact.source.contains(
        "generated_csharp_nullable_parameter_declaration_rejects_invalid_nullable_marker"
    ));
    assert!(
        artifact
            .source
            .contains("parameter_type_keyword: Some(\"string?\"")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-nullable-parameter-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_unary_integer_return_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_unary_integer_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_unary_integer_return/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "PrefixUnaryExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "PrefixUnaryExpressionSyntax"
                    && reference.field == "Operand"
                    && reference.target_rule == "LiteralExpressionSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(
                |payload| payload.owner_rule == "PrefixUnaryExpressionSyntax"
                    && payload.field == "OperatorToken"
                    && payload.payload_kind == "string"
                    && payload.codec == "utf8_string"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local unary integer return sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_unary_integer_return_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_unary_integer_return_rejects_invalid_operator")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-unary-integer-return-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_parenthesized_integer_return_syntax_generated_codec_compiles_through_production_route()
 {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_local_parenthesized_integer_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH,
        ))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_parenthesized_integer_return/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "ParenthesizedExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "ParenthesizedExpressionSyntax"
                    && reference.field == "Expression"
                    && reference.target_rule == "LiteralExpressionSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "C# local parenthesized integer return sidecar should compile through production route",
    );
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_parenthesized_integer_return_round_trips")
    );
    assert!(artifact.source.contains("PARENTHESIZED_EXPR_TOKEN"));

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-parenthesized-integer-return-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_binary_integer_return_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_binary_integer_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_binary_integer_return/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "BinaryExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BinaryExpressionSyntax"
                && reference.field == "Left"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BinaryExpressionSyntax"
                && reference.field == "Right"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BinaryExpressionSyntax"
                && payload.field == "OperatorToken"
                && payload.payload_kind == "string"
                && payload.codec == "utf8_string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local binary integer return sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_BINARY_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_binary_integer_return_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_binary_integer_return_rejects_invalid_operator")
    );
    assert!(artifact.source.contains("EXPECTED_BINARY_OPERATOR"));

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-binary-integer-return-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_typed_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_typed_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_TYPED_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "LocalDeclarationStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "VariableDeclarationSyntax"
                    && reference.field == "Variables"
                    && reference.target_rule == "VariableDeclaratorSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "IdentifierNameSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local typed declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_TYPED_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-typed-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_implicit_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_implicit_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_IMPLICIT_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_implicit_declaration/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "VariableDeclarationSyntax"
                    && reference.field == "Type"
                    && reference.target_rule == "IdentifierNameSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "IdentifierNameSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local implicit declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_IMPLICIT_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_IMPLICIT_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_implicit_declaration_round_trips")
    );
    assert!(artifact.source.contains(r#""var".to_string()"#));

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-implicit-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_integer_implicit_declaration_syntax_generated_codec_compiles_through_production_route()
 {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_local_integer_implicit_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_integer_implicit_declaration/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "VariableDeclarationSyntax"
                    && reference.field == "Type"
                    && reference.target_rule == "IdentifierNameSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "C# local integer implicit declaration sidecar should compile through production route",
    );
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_integer_implicit_declaration_round_trips")
    );
    assert!(artifact.source.contains(r#""int".to_string()"#));
    assert!(artifact.source.contains(r#""var".to_string()"#));
    assert!(
        artifact
            .source
            .contains("INTEGER_TOKEN_START + u32::from(value)")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-integer-implicit-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_assignment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_ASSIGNMENT_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "ExpressionStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "ExpressionStatementSyntax"
                    && reference.field == "Expression"
                    && reference.target_rule == "AssignmentExpressionSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "AssignmentExpressionSyntax"
                    && reference.field == "Right"
                    && reference.target_rule == "LiteralExpressionSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local assignment sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_ASSIGNMENT_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-assignment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_if_else_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_if_else_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_IF_ELSE_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_if_else/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "IfStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IfStatementSyntax"
                && reference.field == "Condition"
                && reference.target_rule == "BinaryExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IfStatementSyntax"
                && reference.field == "Else"
                && reference.target_rule == "ElseClauseSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local if/else sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_IF_ELSE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_IF_ELSE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_if_else_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_if_else_rejects_invalid_integer_payloads")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-if-else-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_for_condition_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_for_condition_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_for_condition/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "ForStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ForStatementSyntax"
                && reference.field == "Condition"
                && reference.target_rule == "BinaryExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local for-condition sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_FOR_CONDITION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_for_condition_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_for_condition_rejects_invalid_integer_payloads")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-for-condition-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_switch_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_switch_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_SWITCH_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_switch/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "SwitchStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SwitchStatementSyntax"
                && reference.field == "Sections"
                && reference.target_rule == "SwitchSectionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local switch sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_SWITCH_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_SWITCH_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_switch_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_switch_rejects_invalid_integer_payloads")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-switch-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_foreach_string_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_foreach_string_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_foreach_string/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "ForEachStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ForEachStatementSyntax"
                && reference.field == "Statement"
                && reference.target_rule == "BlockSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer"
                && payload.codec == "u8_integer")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local foreach string sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_FOREACH_STRING_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_foreach_string_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_foreach_string_rejects_invalid_integer_payloads")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-foreach-string-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_two_parameter_method_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_two_parameter_method_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_TWO_PARAMETER_METHOD_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/two_parameter_method/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "MethodDeclarationSyntax"
                    && reference.field == "ParameterList"
                    && reference.target_rule == "ParameterListSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ParameterListSyntax"
                && reference.field == "Parameters"
                && reference.target_rule == "ParameterSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "ParameterSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# two-parameter method sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_TWO_PARAMETER_METHOD_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_TWO_PARAMETER_METHOD_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_two_parameter_method_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_two_parameter_method_rejects_unknown_return_identifier")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-two-parameter-method-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_property_plus_method_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_property_plus_method_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/property_plus_method/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ClassDeclarationSyntax"
                && reference.field == "Members"
                && reference.target_rule == "PropertyDeclarationSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ClassDeclarationSyntax"
                && reference.field == "Members"
                && reference.target_rule == "MethodDeclarationSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "IdentifierNameSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# property-plus-method sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_PROPERTY_PLUS_METHOD_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_property_plus_method_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_property_plus_method_rejects_unknown_return_identifier")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-property-plus-method-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_method_invocation_return_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_method_invocation_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_METHOD_INVOCATION_RETURN_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/method_invocation_return/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "InvocationExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "InvocationExpressionSyntax"
                    && reference.field == "ArgumentList"
                    && reference.target_rule == "ArgumentListSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "IdentifierNameSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# method-invocation-return sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_METHOD_INVOCATION_RETURN_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_METHOD_INVOCATION_RETURN_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_method_invocation_return_round_trips")
    );
    assert!(
        artifact.source.contains(
            "generated_csharp_method_invocation_return_rejects_unknown_callee_identifier"
        )
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-method-invocation-return-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_method_invocation_argument_return_syntax_generated_codec_compiles_through_production_route()
 {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_method_invocation_argument_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH,
        ))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/method_invocation_argument_return/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ArgumentListSyntax"
                && reference.field == "Arguments"
                && reference.target_rule == "ArgumentSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ArgumentSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# method-invocation-argument-return sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_method_invocation_argument_return_round_trips")
    );
    assert!(artifact.source.contains(
        "generated_csharp_method_invocation_argument_return_rejects_argument_parameter_mismatch"
    ));

    run_generated_syntax_codec_fixture_tests(
        "csharp-method-invocation-argument-return-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_collection_initializer_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_collection_initializer_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_COLLECTION_INITIALIZER_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/collection_initializer/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "MethodDeclarationSyntax"
                    && reference.field == "ReturnType"
                    && reference.target_rule == "ArrayTypeSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "ArrayCreationExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "ArrayCreationExpressionSyntax"
                    && reference.field == "Initializer"
                    && reference.target_rule == "InitializerExpressionSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "InitializerExpressionSyntax"
                    && reference.field == "Expressions"
                    && reference.target_rule == "LiteralExpressionSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# collection-initializer sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_COLLECTION_INITIALIZER_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_COLLECTION_INITIALIZER_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_collection_initializer_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_collection_initializer_rejects_empty_values")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-collection-initializer-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_element_access_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_element_access_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_ELEMENT_ACCESS_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/element_access/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ParameterSyntax"
                && reference.field == "Type"
                && reference.target_rule == "ArrayTypeSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "ElementAccessExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "ElementAccessExpressionSyntax"
                    && reference.field == "ArgumentList"
                    && reference.target_rule == "BracketedArgumentListSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "BracketedArgumentListSyntax"
                    && reference.field == "Arguments"
                    && reference.target_rule == "ArgumentSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# element-access sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_ELEMENT_ACCESS_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_ELEMENT_ACCESS_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_element_access_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_element_access_rejects_unknown_target_identifier")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-element-access-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_range_element_access_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_range_element_access_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_RANGE_ELEMENT_ACCESS_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/range_element_access/source.cs")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "MethodDeclarationSyntax"
                    && reference.field == "ReturnType"
                    && reference.target_rule == "ArrayTypeSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStatementSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "ElementAccessExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ArgumentSyntax"
                && reference.field == "Expression"
                && reference.target_rule == "RangeExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "RangeExpressionSyntax"
                && reference.field == "LeftOperand"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "RangeExpressionSyntax"
                && reference.field == "RightOperand"
                && reference.target_rule == "LiteralExpressionSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# range element-access sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_RANGE_ELEMENT_ACCESS_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_RANGE_ELEMENT_ACCESS_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_range_element_access_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_range_element_access_rejects_unsupported_range_end")
    );

    run_generated_syntax_codec_fixture_tests(
        "csharp-range-element-access-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn csharp_local_escaped_assignment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_escaped_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CSHARP_LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert_eq!(
        sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata"
    );
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/csharp_syntax_sources/local_escaped_assignment/source.cs")
    );
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "ExpressionStatementSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "AssignmentExpressionSyntax"
                    && reference.field == "Right"
                    && reference.target_rule == "LiteralExpressionSyntax"
            )
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::CSharp,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("C# local escaped assignment sidecar should compile through production route");
    assert_eq!(artifact.target, "csharp");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        CSHARP_LOCAL_ESCAPED_ASSIGNMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        CSHARP_LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_csharp_local_escaped_assignment_round_trips")
    );
    assert!(artifact.source.contains(r#"hello \"updated\""#));

    run_generated_syntax_codec_fixture_tests(
        "csharp-local-escaped-assignment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_syntax_generated_codec_fixture_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_minimal_function_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
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
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncDecl"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );

    let generated = render_go_syntax_codec_fixture(&sidecar);
    assert_eq!(generated, render_go_syntax_codec_fixture(&sidecar));
    assert!(!generated.is_empty());

    run_generated_syntax_codec_fixture_tests("go-syntax-generated-codec", &generated);
}

#[test]
fn go_minimal_function_syntax_generated_codec_compiles_through_compiler_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_minimal_function_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go minimal function sidecar should compile through target-agnostic compiler route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_MINIMAL_FUNCTION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["entrypoint"],
        SYNTAX_COMPILER_ROUTE_ENTRYPOINT
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["generated_code_gate_completed_promotion_class"],
        SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS
    );
    assert!(
        artifact.metadata["target_agnostic_compiler_route"]["generated_code_gate_representatives"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == "go_minimal_function_syntax_v0")
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests("go-syntax-generated-codec", &artifact.source);
}

#[test]
fn go_minimal_function_real_source_strings_compare_demo_round_trips_through_generated_codec() {
    if go_sidecar_toolchain(
        "go_minimal_function_real_source_strings_compare_demo_round_trips_through_generated_codec",
    )
    .is_none()
    {
        return;
    }
    let corpus_report = assert_go_real_source_corpus_ready_for_strings_compare();
    assert_eq!(corpus_report["status"], "ready");
    assert_eq!(corpus_report["next_missing_construct"], Value::Null);

    let adapter = GoSyntaxMetadataAdapter::new("go_minimal_function_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go minimal function sidecar should compile through target-agnostic compiler route");
    assert!(
        artifact
            .source
            .contains("generated_go_minimal_function_real_source_strings_compare_demo")
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["entrypoint"],
        SYNTAX_COMPILER_ROUTE_ENTRYPOINT
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["generated_code_gate_completed_promotion_class"],
        SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["paired_route_evidence"]["go_real_source_marker"],
        "generated_go_minimal_function_real_source_strings_compare_demo"
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-strings-compare-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-real-source-strings-compare-syntax-generated-codec",
        &artifact.source,
        &[("TCT_GO_REAL_SOURCE_DEMO_REPORT", report_path.as_path())],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated real-source demo report was not written"),
    )
    .expect("generated real-source demo report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/strings/compare.go"
    );
    assert_eq!(
        report["source_sha256"],
        "af4e0fa7dfe63018d9808f89a4c511eeb8d773bd6f3b030f1c9592a1e0de7a85"
    );
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("demo report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("demo report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 89);
    assert_eq!(baseline_byte_token_count, 114);
    assert_eq!(report["compiler_boundary"], "sidecar-only");
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        GO_REAL_SOURCE_GENERATED_CODEC_CORPUS_COMPLETED_PROMOTION_CLASS
    );
    assert!(
        report["no_stable_promotion_claim"]
            .as_str()
            .unwrap()
            .contains("no TctCompiler route")
    );
}

#[test]
fn parser_backed_validation_matrix_evidence_is_compiler_route_owned() {
    let artifacts = vec![
        build_csharp_minimal_class_compiler_route_artifact().unwrap(),
        build_csharp_local_string_return_literal_compiler_route_artifact().unwrap(),
        build_csharp_compilation_unit_source_file_compiler_route_artifact().unwrap(),
        build_go_minimal_function_compiler_route_artifact().unwrap(),
        build_go_local_declaration_compiler_route_artifact().unwrap(),
    ];

    for artifact in artifacts {
        let route = &artifact.metadata["target_agnostic_compiler_route"];
        assert_eq!(route["entrypoint"], SYNTAX_COMPILER_ROUTE_ENTRYPOINT);
        assert_eq!(
            route["validation_matrix_completed_promotion_class"],
            VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS
        );
        assert_eq!(
            route["validation_matrix_next_missing_stable_promotion_class"],
            VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS
        );
        assert_eq!(route["rejects_arena_ir_boundaries"], true);
        assert_eq!(route["stable_backend_promoted"], true);
        assert_eq!(
            route["stable_backend_scope"],
            "feature_limited_parser_backed_syntax_backend"
        );
        assert_eq!(
            route["stable_metadata_promotion_completed_class"],
            "syntax backend stable metadata promotion implementation gate"
        );
        assert_eq!(route["generated_json_schema_output_changed"], false);
        for category in VALIDATION_MATRIX_REQUIRED_CATEGORIES {
            assert!(
                route["validation_matrix_categories"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|candidate| candidate == *category),
                "missing validation-matrix category {category}"
            );
        }
        for evidence in VALIDATION_MATRIX_ROUTE_EVIDENCE {
            assert!(
                route["validation_matrix_route_evidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|candidate| candidate == *evidence),
                "missing validation-matrix route evidence {evidence}"
            );
        }
    }
}

#[test]
fn go_local_declaration_syntax_generated_codec_compiles_through_compiler_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "DeclStmt")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local declaration sidecar should compile through target-agnostic compiler route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["entrypoint"],
        SYNTAX_COMPILER_ROUTE_ENTRYPOINT
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["generated_code_gate_completed_promotion_class"],
        SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["target_agnostic_compiler_route"]["paired_route_evidence"]["go_non_minimal_fixture"],
        GO_LOCAL_DECLARATION_METADATA_PATH
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_assignment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_ASSIGNMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "AssignStmt")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AssignStmt"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local assignment sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_ASSIGNMENT_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-assignment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_escaped_local_assignment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_escaped_local_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_ESCAPED_LOCAL_ASSIGNMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "AssignStmt")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AssignStmt"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go escaped local assignment sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_ESCAPED_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_ESCAPED_LOCAL_ASSIGNMENT_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-escaped-local-assignment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_short_var_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_short_var_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "AssignStmt")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AssignStmt"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local short variable declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_SHORT_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-short-var-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_integer_short_var_declaration_syntax_generated_codec_compiles_through_production_route()
{
    let adapter = GoSyntaxMetadataAdapter::new("go_local_integer_short_var_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "AssignStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "AssignStmt"
                && reference.field == "Rhs"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AssignStmt"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "Go local integer short variable declaration sidecar should compile through production route",
    );
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-integer-short-var-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_integer_return_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_integer_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "ReturnStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local integer return literal sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_INTEGER_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-integer-return-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_package_type_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_package_type_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_PACKAGE_TYPE_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Name"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Type"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go package type declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_PACKAGE_TYPE_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_PACKAGE_TYPE_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-package-type-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_type_spec_assign_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_type_spec_assign_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_TYPE_SPEC_ASSIGN_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Name"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "TypeSpec"
                && payload.field == "Assign"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Type"
                && reference.target_rule == "Ident")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go TypeSpec.Assign sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_TYPE_SPEC_ASSIGN_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_TYPE_SPEC_ASSIGN_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_type_spec_assign_preserves_type_spec_assign_tokens")
    );
    assert!(
        artifact.source.contains(
            "generated_go_type_spec_assign_preserves_inventory_type_spec_assign_presence"
        )
    );
    assert!(
        artifact
            .source
            .contains("go_syntax_presence_type_spec_alias.PRESENT")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-type-spec-assign-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_type_spec_comment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_type_spec_comment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_TYPE_SPEC_COMMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Comment"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CommentGroup"
                && reference.field == "List"
                && reference.target_rule == "Comment")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go TypeSpec.Comment sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_TYPE_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_TYPE_SPEC_COMMENT_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_type_spec_comment_preserves_type_spec_comment_tokens")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-type-spec-comment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_type_spec_doc_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_type_spec_doc_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_TYPE_SPEC_DOC_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Doc"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CommentGroup"
                && reference.field == "List"
                && reference.target_rule == "Comment")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go TypeSpec.Doc sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_TYPE_SPEC_DOC_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_TYPE_SPEC_DOC_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_type_spec_doc_preserves_type_spec_doc_tokens")
    );
    assert!(
        artifact
            .source
            .contains("generated_go_type_spec_doc_preserves_inventory_gen_decl_group_presence")
    );
    assert!(
        artifact
            .source
            .contains("go_syntax_presence_gen_decl_group.PRESENT")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-type-spec-doc-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_type_spec_type_params_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_type_spec_type_params_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_TYPE_SPEC_TYPE_PARAMS_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    for (owner, field, target) in [
        ("TypeSpec", "TypeParams", "FieldList"),
        ("TypeSpec", "Type", "StructType"),
        ("StructType", "Fields", "FieldList"),
        ("Field", "Names", "Ident"),
        ("Field", "Type", "Ident"),
    ] {
        assert!(
            sidecar.structural_model.references.iter().any(|reference| {
                reference.owner_rule == owner
                    && reference.field == field
                    && reference.target_rule == target
            }),
            "missing sidecar reference {owner}.{field} -> {target}"
        );
    }

    let artifact = build_go_type_spec_type_params_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_TYPE_SPEC_TYPE_PARAMS_METADATA_PATH
    );
    assert!(artifact.source.contains("TYPE_PARAMS_TOKEN"));
    assert!(artifact.source.contains("TYPE_PARAM_NAME_TOKEN"));
    assert!(artifact.source.contains("TYPE_PARAM_BOUND_TOKEN"));
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-type-spec-type-params-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_package_const_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_package_const_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_PACKAGE_CONST_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "GenDecl"
                && reference.field == "Specs"
                && reference.target_rule == "ValueSpec")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Values"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "GenDecl"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go package const declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_PACKAGE_CONST_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_PACKAGE_CONST_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-package-const-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_method_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_method_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_METHOD_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncDecl"
                && reference.field == "Recv"
                && reference.target_rule == "FieldList")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Names"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go method declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_METHOD_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_METHOD_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-method-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_struct_type_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_struct_type_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_STRUCT_TYPE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Type"
                && reference.target_rule == "StructType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "StructType"
                && reference.field == "Fields"
                && reference.target_rule == "FieldList")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Names"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go struct type sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_STRUCT_TYPE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_STRUCT_TYPE_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-struct-type-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_composite_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_composite_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_COMPOSITE_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "CompositeLit")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CompositeLit"
                && reference.field == "Type"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CompositeLit"
                && reference.field == "Elts"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go composite literal sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_COMPOSITE_LITERAL_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-composite-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_key_value_composite_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_key_value_composite_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_KEY_VALUE_COMPOSITE_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CompositeLit"
                && reference.field == "Elts"
                && reference.target_rule == "KeyValueExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "KeyValueExpr"
                && reference.field == "Key"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "KeyValueExpr"
                && reference.field == "Value"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go key/value composite literal sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_KEY_VALUE_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_KEY_VALUE_COMPOSITE_LITERAL_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-key-value-composite-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_array_composite_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_array_composite_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_ARRAY_COMPOSITE_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ArrayType"
                && reference.field == "Elt"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CompositeLit"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go array composite literal sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_ARRAY_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_ARRAY_COMPOSITE_LITERAL_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-array-composite-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_index_expression_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_index_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_INDEX_EXPRESSION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "IndexExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IndexExpr"
                && reference.field == "X"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IndexExpr"
                && reference.field == "Index"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go index expression sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_INDEX_EXPRESSION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_INDEX_EXPRESSION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-index-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_slice_expression_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_slice_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_SLICE_EXPRESSION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "SliceExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SliceExpr"
                && reference.field == "X"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SliceExpr"
                && reference.field == "Low"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go slice expression sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_SLICE_EXPRESSION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_SLICE_EXPRESSION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-slice-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_slice_expression_three_index_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_slice_expression_three_index_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_SLICE_EXPRESSION_THREE_INDEX_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    for field_name in ["Low", "High", "Max"] {
        assert!(
            sidecar
                .structural_model
                .references
                .iter()
                .any(|reference| reference.owner_rule == "SliceExpr"
                    && reference.field == field_name
                    && reference.target_rule == "BasicLit")
        );
    }
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "SliceExpr"
                && payload.field == "Slice3"
                && payload.payload_kind == "boolean")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go three-index slice expression sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_SLICE_EXPRESSION_THREE_INDEX_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_SLICE_EXPRESSION_THREE_INDEX_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-slice-expression-three-index-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_star_expression_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_star_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_STAR_EXPRESSION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "StarExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "StarExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "StarExpr"
                && reference.field == "X"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go star expression sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_STAR_EXPRESSION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_STAR_EXPRESSION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-star-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_package_var_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_package_var_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_PACKAGE_VAR_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "GenDecl"
                && reference.field == "Specs"
                && reference.target_rule == "ValueSpec")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Values"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go package var declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_PACKAGE_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_PACKAGE_VAR_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-package-var-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_value_spec_comment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_value_spec_comment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_VALUE_SPEC_COMMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Comment"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CommentGroup"
                && reference.field == "List"
                && reference.target_rule == "Comment")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go ValueSpec.Comment sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_VALUE_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_VALUE_SPEC_COMMENT_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-value-spec-comment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_import_spec_name_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_import_spec_name_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_IMPORT_SPEC_NAME_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ImportSpec"
                && reference.field == "Name"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ImportSpec"
                && reference.field == "Path"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go ImportSpec.Name sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_IMPORT_SPEC_NAME_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_IMPORT_SPEC_NAME_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-import-spec-name-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_import_spec_comment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_import_spec_comment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_IMPORT_SPEC_COMMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ImportSpec"
                && reference.field == "Comment"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CommentGroup"
                && reference.field == "List"
                && reference.target_rule == "Comment")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go ImportSpec.Comment sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_IMPORT_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_IMPORT_SPEC_COMMENT_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-import-spec-comment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_function_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_function_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_FUNCTION_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "FuncLit")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncLit"
                && reference.field == "Type"
                && reference.target_rule == "FuncType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncLit"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go function literal sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_FUNCTION_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_FUNCTION_LITERAL_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-function-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_generic_function_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_generic_function_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_GENERIC_FUNCTION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    for (owner, field, target) in [
        ("FuncType", "TypeParams", "FieldList"),
        ("FuncType", "Params", "FieldList"),
        ("FuncType", "Results", "FieldList"),
        ("Field", "Type", "Ident"),
        ("ReturnStmt", "Results", "Ident"),
    ] {
        assert!(
            sidecar.structural_model.references.iter().any(|reference| {
                reference.owner_rule == owner
                    && reference.field == field
                    && reference.target_rule == target
            }),
            "missing sidecar reference {owner}.{field} -> {target}"
        );
    }
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier"),
        "missing identifier payload leaf"
    );

    let artifact = build_go_local_generic_function_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_GENERIC_FUNCTION_METADATA_PATH
    );
    assert!(artifact.source.contains("TYPE_PARAMS_TOKEN"));
    assert!(artifact.source.contains("TYPE_PARAM_NAME_TOKEN"));
    assert!(artifact.source.contains("TYPE_PARAM_BOUND_TOKEN"));
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-generic-function-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_generic_function_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_generic_function_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_GENERIC_FUNCTION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar.structural_model.references.iter().any(|reference| {
            reference.owner_rule == "FuncType"
                && reference.field == "TypeParams"
                && reference.target_rule == "FieldList"
        }),
        "missing FuncType.TypeParams structural-model reference"
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go generic function sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_GENERIC_FUNCTION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_GENERIC_FUNCTION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-generic-function-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_concurrency_map_interface_type_assertion_syntax_generated_codec_compiles_at_sidecar_boundary()
{
    let adapter =
        GoSyntaxMetadataAdapter::new("go_concurrency_map_interface_type_assertion_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    for (owner, field, target) in [
        ("TypeSpec", "Type", "InterfaceType"),
        ("MapType", "Value", "ChanType"),
        ("SelectStmt", "Body", "BlockStmt"),
        ("CommClause", "Comm", "SendStmt"),
        ("AssignStmt", "Rhs", "TypeAssertExpr"),
    ] {
        assert!(
            sidecar.structural_model.references.iter().any(|reference| {
                reference.owner_rule == owner
                    && reference.field == field
                    && reference.target_rule == target
            }),
            "missing sidecar reference {owner}.{field} -> {target}"
        );
    }
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "ChanType"
                && payload.field == "Dir"
                && payload.payload_kind == "string"),
        "missing channel direction payload leaf"
    );

    let artifact =
        build_go_concurrency_map_interface_type_assertion_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH
    );
    assert!(artifact.source.contains("INTERFACE_TYPE_TOKEN"));
    assert!(artifact.source.contains("MAP_TYPE_TOKEN"));
    assert!(artifact.source.contains("TYPE_ASSERTION_TOKEN"));
    assert!(artifact.source.contains("TYPE_ASSERTION_TYPE_TOKEN"));
    assert!(artifact.source.contains("SELECT_STMT_TOKEN"));
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-concurrency-map-interface-type-assertion-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_concurrency_map_interface_type_assertion_syntax_generated_codec_compiles_through_production_route()
 {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_concurrency_map_interface_type_assertion_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    for (owner, field, target) in [
        ("TypeSpec", "Type", "InterfaceType"),
        ("MapType", "Value", "ChanType"),
        ("SelectStmt", "Body", "BlockStmt"),
        ("CommClause", "Comm", "SendStmt"),
        ("AssignStmt", "Rhs", "TypeAssertExpr"),
    ] {
        assert!(
            sidecar.structural_model.references.iter().any(|reference| {
                reference.owner_rule == owner
                    && reference.field == field
                    && reference.target_rule == target
            }),
            "missing sidecar reference {owner}.{field} -> {target}"
        );
    }
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "ChanType"
                && payload.field == "Dir"
                && payload.payload_kind == "string"),
        "missing channel direction payload leaf"
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go concurrency/map/interface/type-assertion sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-concurrency-map-interface-type-assertion-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_variadic_ellipsis_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_variadic_ellipsis_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    for (owner, field, target) in [
        ("Field", "Type", "Ellipsis"),
        ("Ellipsis", "Elt", "Ident"),
        ("ReturnStmt", "Results", "CallExpr"),
        ("CallExpr", "Args", "Ident"),
    ] {
        assert!(
            sidecar.structural_model.references.iter().any(|reference| {
                reference.owner_rule == owner
                    && reference.field == field
                    && reference.target_rule == target
            }),
            "missing sidecar reference {owner}.{field} -> {target}"
        );
    }
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "CallExpr"
                && payload.field == "Ellipsis"
                && payload.payload_kind == "string"),
        "missing ellipsis token payload leaf"
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier"),
        "missing identifier payload leaf"
    );

    let artifact = build_go_local_variadic_ellipsis_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH
    );
    assert!(artifact.source.contains("VARIADIC_PARAM_TOKEN"));
    assert!(artifact.source.contains("ELLIPSIS_TYPE_TOKEN"));
    assert!(artifact.source.contains("CALL_SPREAD_TOKEN"));
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-variadic-ellipsis-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_variadic_ellipsis_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_variadic_ellipsis_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    for (owner, field, target) in [
        ("Field", "Type", "Ellipsis"),
        ("Ellipsis", "Elt", "Ident"),
        ("ReturnStmt", "Results", "CallExpr"),
        ("CallExpr", "Args", "Ident"),
    ] {
        assert!(
            sidecar.structural_model.references.iter().any(|reference| {
                reference.owner_rule == owner
                    && reference.field == field
                    && reference.target_rule == target
            }),
            "missing sidecar reference {owner}.{field} -> {target}"
        );
    }
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "CallExpr"
                && payload.field == "Ellipsis"
                && payload.payload_kind == "string"),
        "missing ellipsis token payload leaf"
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go variadic ellipsis sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_VARIADIC_ELLIPSIS_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-variadic-ellipsis-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_os_signal_notify_context_source_span_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_os_signal_notify_context_source_span_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_OS_SIGNAL_NOTIFY_CONTEXT_SOURCE_SPAN_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/os/signal/signal.go")
    );
    assert_eq!(
        sidecar.input.provenance.source_sha256.as_deref(),
        Some("d99f44830137e1bc4da53ef7f0d5ce872d535219168e9f0408eb539c02f993fc")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    for (owner, field, target) in [
        ("File", "Decls", "GenDecl"),
        ("File", "Decls", "FuncDecl"),
        ("FuncDecl", "Doc", "CommentGroup"),
        ("Field", "Type", "Ellipsis"),
        ("BlockStmt", "List", "GoStmt"),
        ("BlockStmt", "List", "SelectStmt"),
        ("BlockStmt", "List", "CommClause"),
        ("CallExpr", "Args", "SelectorExpr"),
        ("ChanType", "Value", "SelectorExpr"),
    ] {
        assert!(
            sidecar
                .structural_model
                .references
                .iter()
                .any(|reference| reference.owner_rule == owner
                    && reference.field == field
                    && reference.target_rule == target),
            "missing sidecar reference {owner}.{field} -> {target}"
        );
    }
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );

    let artifact = build_go_os_signal_notify_context_source_span_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_os_signal_notify_context_source_span"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_OS_SIGNAL_NOTIFY_CONTEXT_SOURCE_SPAN_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_os_signal_notify_context_source_span_round_trips")
    );
    assert!(artifact.source.contains("SOURCE_SPAN_SELECTION_REPORT"));
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root.join(
        "target/tct-syntax-real-source-demo/go-os-signal-notify-context-source-span-generated-codec-demo.json",
    );
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-os-signal-notify-context-source-span-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_OS_SIGNAL_NOTIFY_CONTEXT_SOURCE_SPAN_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path)
            .expect("generated os/signal NotifyContext source-span codec report was not written"),
    )
    .expect("generated os/signal NotifyContext source-span codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go"
    );
    assert_eq!(
        report["source_sha256"],
        "d99f44830137e1bc4da53ef7f0d5ce872d535219168e9f0408eb539c02f993fc"
    );
    assert_eq!(
        report["route"],
        "go_os_signal_notify_context_source_span_syntax_v0"
    );
    assert_eq!(report["source_function"], "NotifyContext");
    assert_eq!(
        report["corpus_manifest"],
        "tests/fixtures/go_syntax_corpus/training-manifest-ten-record-v1.json"
    );
    assert_eq!(
        report["source_span"]["selection_report"],
        "tests/fixtures/go_syntax_corpus/os-signal-source-span-selection-v1.json"
    );
    assert_eq!(report["source_span"]["line_start"], 263);
    assert_eq!(report["source_span"]["line_end"], 297);
    assert_eq!(report["source_span"]["byte_start"], 6278);
    assert_eq!(report["source_span"]["byte_end"], 7645);
    assert_eq!(
        report["source_span"]["sha256"],
        "25d16848c9cd1076b958404c550a0b1c6eed806e6b1d7f9e31d7dba9bb26870c"
    );
    assert_eq!(report["canonical_render"]["bytes"], 1413);
    assert_eq!(
        report["canonical_render"]["sha256"],
        "21b8b2ec177e7cde274e30f43bbaaeb2bc931c2f31919a848c436c289d534642"
    );
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    assert_eq!(
        report["claim_boundary"],
        "source-span generated-codec evidence only; no whole-file os/signal corpus record, type-correctness, source-validity-by-construction, generated-hot-path, paper, arena-IR, or stable-backend claim"
    );
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("os/signal NotifyContext report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("os/signal NotifyContext report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert_eq!(tct_token_count, 1274);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 1413);
}

#[test]
fn go_strings_compare_source_file_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_strings_compare_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_STRINGS_COMPARE_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/strings/compare.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "CompareDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncDecl"
                && reference.field == "Doc"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Args"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "ImportSpec"
                && payload.field == "Path"
                && payload.payload_kind == "string")
    );

    let artifact = build_go_strings_compare_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_strings_compare_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_STRINGS_COMPARE_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_strings_compare_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("decode_strings_compare_source_file_prefix_state")
    );
    assert!(artifact.source.contains(
        "generated_go_strings_compare_source_file_prefix_state_accepts_selected_live_continuations"
    ));
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-strings-compare-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-strings-compare-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_STRINGS_COMPARE_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated strings-compare codec report was not written"),
    )
    .expect("generated strings-compare codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/strings/compare.go"
    );
    assert_eq!(
        report["source_sha256"],
        "af4e0fa7dfe63018d9808f89a4c511eeb8d773bd6f3b030f1c9592a1e0de7a85"
    );
    assert_eq!(report["route"], "go_strings_compare_source_file_syntax_v0");
    assert_eq!(report["source_function"], "Compare");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("strings-compare report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("strings-compare report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 587);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 630);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_crypto_sysrand_rand_windows_source_file_syntax_generated_codec_compiles_at_sidecar_boundary()
{
    let adapter =
        GoSyntaxMetadataAdapter::new("go_crypto_sysrand_rand_windows_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_FILE_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/crypto/internal/sysrand/rand_windows.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "ReadDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "CallExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Fun"
                && reference.target_rule == "SelectorExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "ImportSpec"
                && payload.field == "Path"
                && payload.payload_kind == "string")
    );

    let artifact =
        build_go_crypto_sysrand_rand_windows_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_crypto_sysrand_rand_windows_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_crypto_sysrand_rand_windows_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );
    assert!(artifact.source.contains("\"[]byte\""));

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root.join(
        "target/tct-syntax-real-source-demo/go-crypto-sysrand-rand-windows-generated-codec-demo.json",
    );
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-crypto-sysrand-rand-windows-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated crypto-sysrand codec report was not written"),
    )
    .expect("generated crypto-sysrand codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/crypto/internal/sysrand/rand_windows.go"
    );
    assert_eq!(
        report["source_sha256"],
        "8122c2f83776b2e76f45631835d8038853886e5db728e24fe97c60179197ed8b"
    );
    assert_eq!(
        report["route"],
        "go_crypto_sysrand_rand_windows_source_file_syntax_v0"
    );
    assert_eq!(report["source_function"], "read");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("crypto-sysrand report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("crypto-sysrand report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert_eq!(tct_token_count, 241);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 273);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_crypto_subtle_xor_source_file_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_crypto_subtle_xor_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_CRYPTO_SUBTLE_XOR_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/crypto/subtle/xor.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "XorDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Args"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact = build_go_crypto_subtle_xor_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_crypto_subtle_xor_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_CRYPTO_SUBTLE_XOR_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_crypto_subtle_xor_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );
    assert!(artifact.source.contains("\"[]byte\""));

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-crypto-subtle-xor-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-crypto-subtle-xor-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_CRYPTO_SUBTLE_XOR_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated crypto-subtle-xor codec report was not written"),
    )
    .expect("generated crypto-subtle-xor codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/crypto/subtle/xor.go"
    );
    assert_eq!(
        report["source_sha256"],
        "f438a661cf333ea488b3c4b227bdba8d2fd5914d916c7b327eb39e343db27296"
    );
    assert_eq!(
        report["route"],
        "go_crypto_subtle_xor_source_file_syntax_v0"
    );
    assert_eq!(report["source_function"], "XORBytes");
    assert_eq!(
        report["corpus_manifest"],
        "tests/fixtures/go_syntax_corpus/training-manifest-eight-record-v1.json"
    );
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("crypto-subtle-xor report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("crypto-subtle-xor report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 606);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_internal_coverage_uleb128_source_file_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_internal_coverage_uleb128_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_INTERNAL_COVERAGE_ULEB128_SOURCE_FILE_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/internal/coverage/uleb128/uleb128.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "FunctionDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ForStmt"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IfStmt"
                && reference.field == "Cond"
                && reference.target_rule == "BinaryExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "AssignStmt"
                && reference.field == "Rhs"
                && reference.target_rule == "CallExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BinaryExpr"
                && payload.field == "Op"
                && payload.payload_kind == "string")
    );

    let artifact = build_go_internal_coverage_uleb128_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_single_function_loop_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_INTERNAL_COVERAGE_ULEB128_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_single_function_loop_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );
    assert!(artifact.source.contains("FOR_STMT_TOKEN"));
    assert!(artifact.source.contains("SHORT_VAR_DECL_TOKEN"));
    assert!(artifact.source.contains("SHIFT_ASSIGN_TOKEN"));
    assert!(artifact.source.contains("BRANCH_KEYWORD_TOKEN"));

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root.join(
        "target/tct-syntax-real-source-demo/go-internal-coverage-uleb128-generated-codec-demo.json",
    );
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-internal-coverage-uleb128-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_INTERNAL_COVERAGE_ULEB128_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path)
            .expect("generated internal-coverage-uleb128 codec report was not written"),
    )
    .expect("generated internal-coverage-uleb128 codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/internal/coverage/uleb128/uleb128.go"
    );
    assert_eq!(
        report["source_sha256"],
        "608b9ea1f3cf69e6dd9886c91fafb412bac6aaf48fe211266a5505644934b422"
    );
    assert_eq!(
        report["route"],
        "go_internal_coverage_uleb128_source_file_syntax_v0"
    );
    assert_eq!(report["source_function"], "AppendUleb128");
    assert_eq!(
        report["corpus_manifest"],
        "tests/fixtures/go_syntax_corpus/training-manifest-nine-record-v1.json"
    );
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("internal-coverage-uleb128 report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("internal-coverage-uleb128 report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 360);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_sort_interface_source_file_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_sort_interface_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_SORT_INTERFACE_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/sort/example_interface_test.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "HeaderComments"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "SupportedDecls"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "SupportedDecls"
                && reference.target_rule == "GenDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Type"
                && reference.target_rule == "StructType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncDecl"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Args"
                && reference.target_rule == "FuncLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );

    let artifact = build_go_sort_interface_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_sort_interface_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_SORT_INTERFACE_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_sort_interface_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(artifact.source.contains("EXPECTED_COMPLETE_SOURCE_FILE"));

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-sort-interface-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-sort-interface-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_SORT_INTERFACE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated sort-interface codec report was not written"),
    )
    .expect("generated sort-interface codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/sort/example_interface_test.go"
    );
    assert_eq!(
        report["source_sha256"],
        "74de942ab67e4d7725f9e06c2389e8face1118e80865536ae709a0fc3367d5f1"
    );
    assert_eq!(report["source_function"], "Example");
    assert_eq!(report["route"], "go_sort_interface_source_file_syntax_v0");
    assert_eq!(
        report["generated_codec"],
        "go_sort_interface_source_file_syntax_v0"
    );
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("sort-interface report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("sort-interface report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 898);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 1501);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_strings_clone_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_strings_clone_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_STRINGS_CLONE_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/strings/clone.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "CloneDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncDecl"
                && reference.field == "Doc"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Args"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "ImportSpec"
                && payload.field == "Path"
                && payload.payload_kind == "string")
    );
}

#[test]
fn go_strings_clone_source_file_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_strings_clone_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_STRINGS_CLONE_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/strings/clone.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "CloneDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Args"
                && reference.target_rule == "Ident")
    );

    let artifact = build_go_strings_clone_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_strings_clone_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_STRINGS_CLONE_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_strings_clone_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-strings-clone-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-strings-clone-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_STRINGS_CLONE_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated strings-clone codec report was not written"),
    )
    .expect("generated strings-clone codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/strings/clone.go"
    );
    assert_eq!(
        report["source_sha256"],
        "5c18eb3b45b7d9ae63d0cc43f9e337c077de05dd47fd5a1ea49dafeb7e1b862b"
    );
    assert_eq!(report["route"], "go_strings_clone_source_file_syntax_v0");
    assert_eq!(report["source_function"], "Clone");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("strings-clone report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("strings-clone report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 752);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 809);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_sort_interface_source_file_declaration_prefix_syntax_generated_codec_compiles_at_sidecar_boundary()
 {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_sort_interface_source_file_declaration_prefix_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/sort/example_interface_test.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "SupportedDecls"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Type"
                && reference.target_rule == "StructType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSpec"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncDecl"
                && reference.field == "Recv"
                && reference.target_rule == "FieldList")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "AssignStmt"
                && reference.field == "Rhs"
                && reference.target_rule == "CompositeLit")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ExprStmt"
                && reference.field == "X"
                && reference.target_rule == "CallExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Args"
                && reference.target_rule == "FuncLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "ImportSpec"
                && payload.field == "Path"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BinaryExpr"
                && payload.field == "Op"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );

    let artifact =
        build_go_sort_interface_source_file_declaration_prefix_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_sort_interface_source_file_declaration_prefix"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_sort_interface_source_file_declaration_prefix_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-sort-interface-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-sort-interface-source-file-declaration-prefix-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_SORT_INTERFACE_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated sort-interface codec report was not written"),
    )
    .expect("generated sort-interface codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/sort/example_interface_test.go"
    );
    assert_eq!(
        report["source_sha256"],
        "74de942ab67e4d7725f9e06c2389e8face1118e80865536ae709a0fc3367d5f1"
    );
    assert_eq!(
        report["route"],
        "go_sort_interface_source_file_declaration_prefix_syntax_v0"
    );
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("sort-interface report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("sort-interface report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 898);
    assert_eq!(baseline_byte_token_count, 802);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_sort_search_source_file_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_sort_search_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_SORT_SEARCH_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "HeaderComments"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "SupportedDecls"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "OutputComments"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "RangeStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "CallExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "string")
    );

    let artifact = build_go_sort_search_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_sort_search_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_SORT_SEARCH_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_sort_search_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("decode_sort_search_example_prefix_state")
    );
    assert!(artifact.source.contains(
        "generated_go_sort_search_source_file_prefix_state_accepts_selected_live_continuations"
    ));
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(artifact.source.contains("EXPECTED_COMPLETE_SOURCE_FILE"));

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-sort-search-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-sort-search-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_SORT_SEARCH_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated sort-search codec report was not written"),
    )
    .expect("generated sort-search codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go"
    );
    assert_eq!(
        report["source_sha256"],
        "c65e8f2f0dab440d8f27a42c27199279f37256d4ef276863a7fd556529f4274d"
    );
    assert_eq!(report["route"], "go_sort_search_source_file_syntax_v0");
    assert_eq!(
        report["generated_codec"],
        "go_sort_search_source_file_syntax_v0"
    );
    assert_eq!(report["source_function"], "ExampleSearch");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("sort-search report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("sort-search report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 253);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 3294);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_sort_search_source_file_example_prefix_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_sort_search_source_file_example_prefix_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "ExampleDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "AssignStmt"
                && reference.field == "Rhs"
                && reference.target_rule == "CompositeLit")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Args"
                && reference.target_rule == "FuncLit")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IfStmt"
                && reference.field == "Cond"
                && reference.target_rule == "BinaryExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact =
        build_go_sort_search_source_file_example_prefix_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_sort_search_source_file_example_prefix"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_sort_search_source_file_example_prefix_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-sort-search-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-sort-search-source-file-example-prefix-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_SORT_SEARCH_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated sort-search codec report was not written"),
    )
    .expect("generated sort-search codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go"
    );
    assert_eq!(
        report["source_sha256"],
        "c65e8f2f0dab440d8f27a42c27199279f37256d4ef276863a7fd556529f4274d"
    );
    assert_eq!(
        report["route"],
        "go_sort_search_source_file_example_prefix_syntax_v0"
    );
    assert_eq!(report["source_function"], "ExampleSearch");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("sort-search report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("sort-search report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 253);
    assert_eq!(baseline_byte_token_count, 412);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_strings_search_source_file_finder_prefix_syntax_generated_codec_compiles_at_sidecar_boundary()
{
    let adapter =
        GoSyntaxMetadataAdapter::new("go_strings_search_source_file_finder_prefix_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/strings/search.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "FinderTypeDecl"
                && reference.target_rule == "GenDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "MakeFinderDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "ArrayType")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "UnaryExpr"
                && reference.field == "X"
                && reference.target_rule == "CompositeLit")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "RangeStmt"
                && reference.field == "X"
                && reference.target_rule == "SelectorExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IndexExpr"
                && reference.field == "Index"
                && reference.target_rule == "BinaryExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact =
        build_go_strings_search_source_file_finder_prefix_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_strings_search_source_file_finder_prefix"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_strings_search_source_file_finder_prefix_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-strings-search-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-strings-search-source-file-finder-prefix-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_STRINGS_SEARCH_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated strings-search codec report was not written"),
    )
    .expect("generated strings-search codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/strings/search.go"
    );
    assert_eq!(
        report["source_sha256"],
        "4fdef1f79f6e854789d9140b17ce2c2bbdbf0d4067a801e42607cf7877698cd1"
    );
    assert_eq!(
        report["route"],
        "go_strings_search_source_file_finder_prefix_syntax_v0"
    );
    assert_eq!(report["source_function"], "makeStringFinder");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("strings-search report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("strings-search report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 2753);
    assert_eq!(baseline_byte_token_count, 3426);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_strings_search_source_file_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_strings_search_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_STRINGS_SEARCH_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/strings/search.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "HeaderComments"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "SupportedDecls"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncDecl"
                && reference.field == "Recv"
                && reference.target_rule == "FieldList")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "Field"
                && reference.field == "Type"
                && reference.target_rule == "StarExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "BranchStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "UnaryExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact = build_go_strings_search_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_strings_search_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_STRINGS_SEARCH_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_strings_search_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(
        artifact
            .source
            .contains("func longestCommonSuffix(a, b string) (i int)")
    );
    assert!(
        artifact
            .source
            .contains("func (f *stringFinder) next(text string) int")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-strings-search-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-strings-search-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_STRINGS_SEARCH_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated strings-search codec report was not written"),
    )
    .expect("generated strings-search codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/strings/search.go"
    );
    assert_eq!(
        report["source_sha256"],
        "4fdef1f79f6e854789d9140b17ce2c2bbdbf0d4067a801e42607cf7877698cd1"
    );
    assert_eq!(report["route"], "go_strings_search_source_file_syntax_v0");
    assert_eq!(report["source_function"], "makeStringFinder");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("strings-search report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("strings-search report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 2753);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 4227);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_path_match_source_file_match_prefix_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_path_match_source_file_match_prefix_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/path/match.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "ErrBadPatternDecl"
                && reference.target_rule == "GenDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "MatchDecl"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Values"
                && reference.target_rule == "CallExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "LabeledStmt"
                && reference.field == "Stmt"
                && reference.target_rule == "ForStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BranchStmt"
                && reference.field == "Label"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SliceExpr"
                && reference.field == "Low"
                && reference.target_rule == "BinaryExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact = build_go_path_match_source_file_match_prefix_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_path_match_source_file_match_prefix"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_path_match_source_file_match_prefix_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-path-match-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-path-match-source-file-match-prefix-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_PATH_MATCH_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated path-match codec report was not written"),
    )
    .expect("generated path-match codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/path/match.go"
    );
    assert_eq!(
        report["source_sha256"],
        "bb88f659463a04e290492cff8d19c4d7d5c7d551db9b90b3049d3e51188d5ec8"
    );
    assert_eq!(
        report["route"],
        "go_path_match_source_file_match_prefix_syntax_v0"
    );
    assert_eq!(report["source_function"], "Match");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("path-match report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("path-match report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 1484);
    assert_eq!(baseline_byte_token_count, 2338);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_path_match_source_file_syntax_generated_codec_compiles_at_sidecar_boundary() {
    let adapter = GoSyntaxMetadataAdapter::new("go_path_match_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_PATH_MATCH_SOURCE_FILE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(
        sidecar.input.provenance.source_path.as_deref(),
        Some("tests/fixtures/go_syntax_corpus/src/path/match.go")
    );
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "HeaderComments"
                && reference.target_rule == "CommentGroup")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "SupportedDecls"
                && reference.target_rule == "FuncDecl")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SwitchStmt"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CaseClause"
                && reference.field == "Body"
                && reference.target_rule == "BranchStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "UnaryExpr"
                && reference.field == "X"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Comment"
                && payload.field == "Text"
                && payload.payload_kind == "string")
    );

    let artifact = build_go_path_match_source_file_syntax_generated_codec_artifact();
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["generation_options"]["construct_class"],
        "go_path_match_source_file"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_PATH_MATCH_SOURCE_FILE_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_path_match_source_file_round_trips")
    );
    assert!(
        artifact
            .source
            .contains("PARSER_BACKED_COMPLETE_SOURCE_FILE")
    );
    assert!(artifact.source.contains("func scanChunk(pattern string)"));
    assert!(artifact.source.contains("func matchChunk(chunk, s string)"));
    assert!(artifact.source.contains("func getEsc(chunk string)"));
    assert!(
        artifact
            .source
            .contains("SOURCE_PROGRAM_VALIDITY_CLAIM_BLOCKER")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root
        .join("target/tct-syntax-real-source-demo/go-path-match-generated-codec-demo.json");
    let _ = fs::remove_file(&report_path);
    run_generated_syntax_codec_fixture_tests_with_env(
        "go-path-match-source-file-syntax-generated-codec",
        &artifact.source,
        &[(
            "TCT_GO_PATH_MATCH_REAL_SOURCE_CODEC_REPORT",
            report_path.as_path(),
        )],
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("generated path-match codec report was not written"),
    )
    .expect("generated path-match codec report is not valid JSON");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_real_source_generated_codec_demo_v1"
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/path/match.go"
    );
    assert_eq!(
        report["source_sha256"],
        "bb88f659463a04e290492cff8d19c4d7d5c7d551db9b90b3049d3e51188d5ec8"
    );
    assert_eq!(report["route"], "go_path_match_source_file_syntax_v0");
    assert_eq!(report["source_function"], "Match");
    assert_eq!(report["parser_validation"]["gofmt"], "passed");
    assert_eq!(report["parser_validation"]["go_parser"], "passed");
    let tct_token_count = report["tct_token_count"]
        .as_u64()
        .expect("path-match report missing tct token count");
    let baseline_byte_token_count = report["baseline_byte_token_count"]
        .as_u64()
        .expect("path-match report missing baseline byte token count");
    assert!(tct_token_count > 0);
    assert!(baseline_byte_token_count > 0);
    assert_eq!(tct_token_count, 1484);
    assert_go_real_source_report_tokens(&report, tct_token_count);
    assert_eq!(baseline_byte_token_count, 5465);
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend Go real-source generated-codec corpus completion handoff gate"
    );
}

#[test]
fn go_local_parenthesized_integer_return_expression_syntax_generated_codec_compiles_through_production_route()
 {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_local_parenthesized_integer_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "ParenExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ParenExpr"
                && reference.field == "X"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "Go local parenthesized integer return expression sidecar should compile through production route",
    );
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-parenthesized-integer-return-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_unary_integer_return_expression_syntax_generated_codec_compiles_through_production_route()
 {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_local_unary_integer_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "UnaryExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "UnaryExpr"
                && reference.field == "X"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "UnaryExpr"
                && payload.field == "Op"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "Go local unary integer return expression sidecar should compile through production route",
    );
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-unary-integer-return-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_binary_integer_return_expression_syntax_generated_codec_compiles_through_production_route()
 {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_local_binary_integer_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "BinaryExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BinaryExpr"
                && reference.field == "X"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BinaryExpr"
                && reference.field == "Y"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BinaryExpr"
                && payload.field == "Op"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "Go local binary integer return expression sidecar should compile through production route",
    );
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-binary-integer-return-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_call_return_expression_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_call_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "CallExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Fun"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local call return expression sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_CALL_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-call-return-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_argument_call_return_expression_syntax_generated_codec_compiles_through_production_route()
 {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_local_argument_call_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "CallExpr")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Fun"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CallExpr"
                && reference.field == "Args"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "Go local argument call return expression sidecar should compile through production route",
    );
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-argument-call-return-expression-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_string_return_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_string_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "ReturnStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "string")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local string return literal sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_STRING_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-string-return-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_float_return_literal_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_float_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "ReturnStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "string")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local float return literal sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_FLOAT_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("generated_go_local_float_return_literal_round_trips")
    );
    assert!(
        artifact
            .source
            .contains(r#"RETURN_LITERAL_KIND: &str = "float""#)
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-float-return-literal-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_boolean_return_identifier_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_boolean_return_identifier_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "ReturnStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local boolean return identifier sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-boolean-return-identifier-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_nil_return_identifier_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_nil_return_identifier_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "ReturnStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local nil return identifier sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_NIL_RETURN_IDENTIFIER_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-nil-return-identifier-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_typed_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_typed_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_TYPED_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Type"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "string")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local typed declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_TYPED_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-typed-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_zero_value_typed_declaration_syntax_generated_codec_compiles_through_production_route()
{
    let adapter = GoSyntaxMetadataAdapter::new("go_local_zero_value_typed_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Type"
                && reference.target_rule == "Ident")
    );
    assert!(
        !sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Values"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        !sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect(
        "Go zero-value local typed declaration sidecar should compile through production route",
    );
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-zero-value-typed-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_integer_typed_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_integer_typed_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_INTEGER_TYPED_DECLARATION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Type"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Values"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go integer local typed declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_INTEGER_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_INTEGER_TYPED_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-integer-typed-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_untyped_integer_declaration_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_untyped_integer_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            GO_LOCAL_UNTYPED_INTEGER_DECLARATION_METADATA_PATH,
        ))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        !sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Type"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Values"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go untyped integer local declaration sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_UNTYPED_INTEGER_DECLARATION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_UNTYPED_INTEGER_DECLARATION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-untyped-integer-declaration-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_integer_assignment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_integer_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_INTEGER_ASSIGNMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "AssignStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ValueSpec"
                && reference.field == "Values"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "AssignStmt"
                && reference.field == "Rhs"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AssignStmt"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go integer local assignment sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_INTEGER_ASSIGNMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_INTEGER_ASSIGNMENT_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-integer-assignment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_compound_assignment_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_compound_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_COMPOUND_ASSIGNMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "AssignStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "AssignStmt"
                && reference.field == "Rhs"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AssignStmt"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local compound assignment sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_COMPOUND_ASSIGNMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_COMPOUND_ASSIGNMENT_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("ASSIGNMENT_OPERATOR: &str = \"+=\"")
    );
    assert!(
        artifact
            .source
            .contains("generated_go_local_compound_assignment_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-compound-assignment-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_increment_decrement_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_increment_decrement_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_INCREMENT_DECREMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "IncDecStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IncDecStmt"
                && reference.field == "X"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "IncDecStmt"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local increment/decrement sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_INCREMENT_DECREMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_INCREMENT_DECREMENT_METADATA_PATH
    );
    assert!(artifact.source.contains("UPDATE_OPERATOR: &str = \"++\""));
    assert!(
        artifact
            .source
            .contains("generated_go_local_increment_decrement_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-increment-decrement-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_branch_statement_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_branch_statement_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_BRANCH_STATEMENT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "BranchStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ForStmt"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ReturnStmt"
                && reference.field == "Results"
                && reference.target_rule == "BasicLit")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BranchStmt"
                && payload.field == "Tok"
                && payload.payload_kind == "string")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
    assert_eq!(
        sidecar
            .structural_model
            .token_ranges
            .integer_value_range
            .as_ref()
            .map(|range| (range.valid_min, range.valid_max)),
        Some((0, 255))
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go local branch statement sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_BRANCH_STATEMENT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_BRANCH_STATEMENT_METADATA_PATH
    );
    assert!(
        artifact
            .source
            .contains("INCLUDES_BRANCH_STATEMENT: bool = true")
    );
    assert!(
        artifact
            .source
            .contains("generated_go_local_branch_statement_round_trips")
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-branch-statement-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_if_else_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_if_else_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_IF_ELSE_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "IfStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "IfStmt"
                && reference.field == "Cond"
                && reference.target_rule == "BinaryExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go if/else sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_IF_ELSE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_IF_ELSE_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-if-else-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_for_condition_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_for_condition_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_FOR_CONDITION_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "ForStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ForStmt"
                && reference.field == "Cond"
                && reference.target_rule == "BinaryExpr")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go condition-only for sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_FOR_CONDITION_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_FOR_CONDITION_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-for-condition-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_switch_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_switch_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_SWITCH_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "SwitchStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "CaseClause"
                && reference.field == "Body"
                && reference.target_rule == "AssignStmt")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go single-case switch sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_SWITCH_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_SWITCH_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-switch-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_switch_init_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_switch_init_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_SWITCH_INIT_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SwitchStmt"
                && reference.field == "Init"
                && reference.target_rule == "AssignStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SwitchStmt"
                && reference.field == "Tag"
                && reference.target_rule == "Ident")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SwitchStmt"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go switch init sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_SWITCH_INIT_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_SWITCH_INIT_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-switch-init-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_type_switch_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_type_switch_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_TYPE_SWITCH_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "TypeSwitchStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSwitchStmt"
                && reference.field == "Assign"
                && reference.target_rule == "ExprStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "TypeSwitchStmt"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go type-switch sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_TYPE_SWITCH_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_TYPE_SWITCH_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-type-switch-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn go_local_range_string_syntax_generated_codec_compiles_through_production_route() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_range_string_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GO_LOCAL_RANGE_STRING_METADATA_PATH))
        .unwrap();
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert_eq!(sidecar.input.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "BlockStmt"
                && reference.field == "List"
                && reference.target_rule == "RangeStmt")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "RangeStmt"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );

    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target: SyntaxProductionRouteTarget::Go,
        grammar: &grammar,
        sidecar: &sidecar,
    })
    .expect("Go string range sidecar should compile through production route");
    assert_eq!(artifact.target, "go");
    assert_eq!(
        artifact.metadata["production_route"]["completed_construct_promotion_class"],
        GO_LOCAL_RANGE_STRING_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["production_route"]["next_missing_stable_promotion_class"],
        NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance"]["metadata_path"],
        GO_LOCAL_RANGE_STRING_METADATA_PATH
    );

    run_generated_syntax_codec_fixture_tests(
        "go-local-range-string-syntax-generated-codec",
        &artifact.source,
    );
}

#[test]
fn parser_backed_syntax_codec_registry_builds_every_registered_entry() {
    assert_eq!(
        CODEC_REGISTRY_SOURCE_OF_TRUTH,
        "src/syntax_production_route.rs::SYNTAX_CODEC_REGISTRY"
    );
    let route_manifest: Value = serde_json::from_str(
        &fs::read_to_string("tests/fixtures/syntax_backend/production-route-manifest-v1.json")
            .expect("checked syntax production route manifest should read"),
    )
    .expect("checked syntax production route manifest should parse");
    let manifest_grammar_ids = route_manifest["constructs"]
        .as_array()
        .expect("route manifest constructs should be an array")
        .iter()
        .map(|construct| {
            construct["grammar_id"]
                .as_str()
                .expect("route manifest grammar id should be a string")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(SYNTAX_CODEC_REGISTRY.len(), manifest_grammar_ids.len());
    assert!(
        manifest_grammar_ids.contains("go_strings_compare_source_file_syntax_v0"),
        "source-file Go strings/compare fixture should be registry-owned"
    );
    assert!(
        manifest_grammar_ids.contains("go_os_dir_unix_source_file_syntax_v0"),
        "source-file Go os/dir_unix fixture should be registry-owned"
    );
    assert!(
        manifest_grammar_ids.contains("go_concurrency_map_interface_type_assertion_syntax_v0"),
        "Go concurrency/map/interface/type-assertion fixture should be registry-owned"
    );
    assert!(
        manifest_grammar_ids.contains("csharp_compilation_unit_logger_source_file_syntax_v0"),
        "C# Logger whole-source compilation-unit fixture should be registry-owned"
    );
    for csharp_real_source_grammar_id in [
        "csharp_compilation_unit_version_one_source_file_syntax_v0",
        "csharp_compilation_unit_iterator_methods_source_file_syntax_v0",
        "csharp_compilation_unit_loan_source_file_syntax_v0",
    ] {
        assert!(
            manifest_grammar_ids.contains(csharp_real_source_grammar_id),
            "remaining C# whole-source compilation-unit fixture {csharp_real_source_grammar_id} should be registry-owned"
        );
    }
    assert!(
        manifest_grammar_ids.contains("go_local_generic_function_syntax_v0"),
        "Go generic function TypeParams fixture should be registry-owned"
    );
    for csharp_grammar_id in [
        "csharp_using_directive_syntax_v0",
        "csharp_qualified_name_syntax_v0",
        "csharp_generic_name_syntax_v0",
        "csharp_block_scoped_namespace_syntax_v0",
        "csharp_struct_declaration_syntax_v0",
        "csharp_enum_declaration_syntax_v0",
        "csharp_event_declaration_syntax_v0",
        "csharp_constructor_declaration_syntax_v0",
        "csharp_field_declaration_syntax_v0",
        "csharp_base_list_syntax_v0",
        "csharp_attribute_syntax_v0",
        "csharp_arrow_expression_clause_syntax_v0",
    ] {
        assert!(
            manifest_grammar_ids.contains(csharp_grammar_id),
            "C# declaration/member/type-shape fixture {csharp_grammar_id} should be registry-owned"
        );
    }
    for csharp_grammar_id in [
        "csharp_while_postfix_loop_syntax_v0",
        "csharp_member_access_expression_syntax_v0",
        "csharp_conditional_access_syntax_v0",
        "csharp_array_rank_object_creation_syntax_v0",
        "csharp_this_expression_syntax_v0",
    ] {
        assert!(
            manifest_grammar_ids.contains(csharp_grammar_id),
            "C# expression/control-flow fixture {csharp_grammar_id} should be registry-owned"
        );
    }

    let mut grammar_ids = BTreeSet::new();
    for entry in SYNTAX_CODEC_REGISTRY {
        assert!(
            manifest_grammar_ids.contains(entry.grammar_id),
            "registry entry {} should be listed in the checked production-route manifest",
            entry.grammar_id
        );
        assert!(
            grammar_ids.insert(entry.grammar_id),
            "duplicate syntax codec registry grammar id {}",
            entry.grammar_id
        );
        assert!(Path::new(entry.metadata_path).exists());
        assert!(!entry.completed_promotion_class.is_empty());
        assert!(!entry.generated_test_markers.is_empty());
        assert_eq!(
            syntax_codec_registry_entry(entry.grammar_id),
            Some(entry),
            "registry lookup must resolve {}",
            entry.grammar_id
        );

        let artifact = build_registered_parser_backed_syntax_generated_codec_artifact(
            entry.grammar_id,
        )
        .unwrap_or_else(|error| {
            panic!(
                "registry entry {} should build parser-backed syntax generated codec artifact: {error}",
                entry.grammar_id
            )
        });
        assert_eq!(artifact.target, entry.target_id);
        assert!(!artifact.source.is_empty());
        assert_eq!(
            artifact.metadata["grammar_input"]["grammar_id"],
            entry.grammar_id
        );
        assert_eq!(
            artifact.metadata["source_metadata_provenance"]["metadata_path"],
            entry.metadata_path
        );
        assert_eq!(
            artifact.metadata["generation_options"]["renderer"],
            entry.renderer().unwrap()
        );
        assert_eq!(
            artifact.metadata["generation_options"]["crate_name"],
            entry.crate_name().unwrap()
        );
        assert_eq!(
            artifact.metadata["syntax_codec_registry_entry"]["grammar_id"],
            entry.grammar_id
        );
        assert_eq!(
            artifact.metadata["syntax_codec_registry_entry"]["renderer_family"],
            entry.renderer_family().unwrap()
        );
        assert_eq!(
            artifact.metadata["syntax_codec_registry_entry"]["construct_class"],
            entry.construct_class()
        );
        if let Some(expected_completed_class) = match entry.grammar_id {
            "csharp_using_directive_syntax_v0" => {
                Some(CSHARP_USING_DIRECTIVE_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_block_scoped_namespace_syntax_v0" => {
                Some(CSHARP_BLOCK_SCOPED_NAMESPACE_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_struct_declaration_syntax_v0" => {
                Some(CSHARP_STRUCT_DECLARATION_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_enum_declaration_syntax_v0" => {
                Some(CSHARP_ENUM_DECLARATION_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_event_declaration_syntax_v0" => {
                Some(CSHARP_EVENT_DECLARATION_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_constructor_declaration_syntax_v0" => {
                Some(CSHARP_CONSTRUCTOR_DECLARATION_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_field_declaration_syntax_v0" => {
                Some(CSHARP_FIELD_DECLARATION_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_base_list_syntax_v0" => Some(CSHARP_BASE_LIST_COMPLETED_PROMOTION_CLASS),
            "csharp_conditional_access_syntax_v0" => {
                Some(CSHARP_CONDITIONAL_ACCESS_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_array_rank_object_creation_syntax_v0" => {
                Some(CSHARP_ARRAY_RANK_OBJECT_CREATION_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_this_expression_syntax_v0" => {
                Some(CSHARP_THIS_EXPRESSION_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_attribute_syntax_v0" => Some(CSHARP_ATTRIBUTE_SYNTAX_COMPLETED_PROMOTION_CLASS),
            "csharp_arrow_expression_clause_syntax_v0" => {
                Some(CSHARP_ARROW_EXPRESSION_CLAUSE_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_compilation_unit_logger_source_file_syntax_v0" => {
                Some(CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_compilation_unit_version_one_source_file_syntax_v0" => {
                Some(CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_compilation_unit_iterator_methods_source_file_syntax_v0" => {
                Some(CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_COMPLETED_PROMOTION_CLASS)
            }
            "csharp_compilation_unit_loan_source_file_syntax_v0" => {
                Some(CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_COMPLETED_PROMOTION_CLASS)
            }
            _ => None,
        } {
            assert_eq!(entry.completed_promotion_class, expected_completed_class);
        }
        assert_eq!(
            artifact.metadata["syntax_codec_registry_entry"]["generated_test_markers"]
                .as_array()
                .expect("registry metadata should carry generated markers")
                .len(),
            entry.generated_test_markers.len()
        );
    }
}

#[test]
fn syntax_codec_registry_freezes_per_file_go_grammar_entries_to_the_historical_pair() {
    // Registry freeze (M1 forward gate): per-file Go
    // go_*_source_file_syntax_v0 registrations are retired in favor of the
    // unified pinned inventory grammar artifact, so the registry must never
    // grow a new per-file Go entry. Exactly these two historical entries
    // predate unification and stay registered as scratch-route regression
    // evidence; adding a third per-file Go entry requires a recorded
    // per-file grammar blocker and a deliberate change to this freeze.
    let per_file_go_entries = SYNTAX_CODEC_REGISTRY
        .iter()
        .filter(|entry| {
            entry.target_id == "go" && entry.grammar_id.ends_with("_source_file_syntax_v0")
        })
        .map(|entry| entry.grammar_id)
        .collect::<Vec<_>>();
    assert_eq!(
        per_file_go_entries,
        [
            "go_strings_compare_source_file_syntax_v0",
            "go_os_dir_unix_source_file_syntax_v0",
        ],
        "new per-file Go grammar registrations are rejected by the forward gate; \
         route new Go source files through the unified grammar artifact \
         {GO_UNIFIED_GRAMMAR_ARTIFACT_ID} (grammar id {GO_UNIFIED_GRAMMAR_ID}) \
         instead of registering another go_*_source_file_syntax_v0 entry"
    );
}

#[test]
fn toy_syntax_generated_codec_artifact_metadata_is_reproducible() {
    let first = build_toy_syntax_generated_codec_artifact();
    let second = build_toy_syntax_generated_codec_artifact();

    assert_eq!(first.source, second.source);
    assert_eq!(first.metadata, second.metadata);
    assert_eq!(first.metadata_sha256, second.metadata_sha256);
    assert_eq!(
        first.metadata["artifact_kind"],
        "tct_syntax_generated_codec_reproducibility_metadata"
    );
    assert_eq!(first.metadata["artifact_version"], 1);
    assert_eq!(
        first.metadata["rendered_source"]["sha256"],
        sha256_hex(first.source.as_bytes())
    );
    assert_eq!(
        first.metadata["rendered_source"]["bytes"],
        json!(first.source.len())
    );
    assert_eq!(
        first.metadata["grammar_input"]["grammar_id"],
        TOY_GRAMMAR_LANGUAGE_ID
    );
    assert_eq!(
        first.metadata["structural_model"]["model_version"],
        "structural-model-v0"
    );
    assert_eq!(
        first.metadata["token_layout"]["pad_token"],
        json!(PAD_TOKEN)
    );
    assert_eq!(
        first.metadata["token_layout"]["vocab_size"],
        json!(VOCAB_SIZE)
    );
    assert_eq!(
        first.metadata["token_layout"]["raw_base_vocab_size"],
        json!(INTEGER_TOKEN_END)
    );
    assert_eq!(
        first.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    assert_eq!(
        first.metadata["generated_crate"]["crate_name"],
        "toy-syntax-generated-codec"
    );
    assert_eq!(
        first.metadata["source_metadata_provenance"]["source_kind"],
        "hand_written_toy_grammar"
    );
    assert_boundary_manifest_handoff(&first, "toy", "hand_written_toy_grammar", None);
    assert_eq!(
        first.metadata["sidecar_boundary"]["generated_json_schema_output_changed"],
        false
    );
    assert_eq!(
        first.metadata["sidecar_boundary"]["core_compiler_path_changed"],
        false
    );
    assert_eq!(
        first.metadata["sidecar_boundary"]["production_grammar_compiler_path_changed"],
        false
    );
    assert_generated_artifact_envelope_handoff(&first, "toy");
    assert_validation_hook_contract_handoff(&first, "toy");
    assert_validation_hook_sidecar_evidence_handoff(&first, "toy");
    assert_generated_artifact_replay_handoff(&first, "toy");
    assert_input_diagnostics_contract_handoff(&first, "toy");
    assert_input_diagnostics_negative_fixture_handoff(&first, "toy");

    write_toy_syntax_generated_codec_reproducibility_report(&first);
}

#[test]
fn parser_backed_syntax_generated_codec_artifact_metadata_is_reproducible() {
    let csharp_first = build_csharp_minimal_class_compiler_route_artifact()
        .expect("C# minimal class route artifact should be reproducible");
    let csharp_second = build_csharp_minimal_class_compiler_route_artifact()
        .expect("C# minimal class route artifact should be reproducible");
    let csharp_local_string_return_literal_first =
        build_csharp_local_string_return_literal_compiler_route_artifact()
            .expect("C# local string return literal route artifact should be reproducible");
    let csharp_local_string_return_literal_second =
        build_csharp_local_string_return_literal_compiler_route_artifact()
            .expect("C# local string return literal route artifact should be reproducible");
    let csharp_compilation_unit_source_file_first =
        build_csharp_compilation_unit_source_file_compiler_route_artifact()
            .expect("C# compilation-unit source-file route artifact should be reproducible");
    let csharp_compilation_unit_source_file_second =
        build_csharp_compilation_unit_source_file_compiler_route_artifact()
            .expect("C# compilation-unit source-file route artifact should be reproducible");
    let csharp_block_scoped_namespace_first =
        build_csharp_block_scoped_namespace_syntax_generated_codec_artifact();
    let csharp_block_scoped_namespace_second =
        build_csharp_block_scoped_namespace_syntax_generated_codec_artifact();
    let csharp_using_directive_first =
        build_csharp_using_directive_syntax_generated_codec_artifact();
    let csharp_using_directive_second =
        build_csharp_using_directive_syntax_generated_codec_artifact();
    let csharp_qualified_name_first = build_csharp_qualified_name_syntax_generated_codec_artifact();
    let csharp_qualified_name_second =
        build_csharp_qualified_name_syntax_generated_codec_artifact();
    let csharp_constructor_declaration_first =
        build_csharp_constructor_declaration_syntax_generated_codec_artifact();
    let csharp_constructor_declaration_second =
        build_csharp_constructor_declaration_syntax_generated_codec_artifact();
    let csharp_while_postfix_loop_first =
        build_csharp_while_postfix_loop_syntax_generated_codec_artifact();
    let csharp_while_postfix_loop_second =
        build_csharp_while_postfix_loop_syntax_generated_codec_artifact();
    let csharp_interpolated_string_first =
        build_csharp_interpolated_string_syntax_generated_codec_artifact();
    let csharp_interpolated_string_second =
        build_csharp_interpolated_string_syntax_generated_codec_artifact();
    let csharp_member_access_expression_first =
        build_csharp_member_access_expression_syntax_generated_codec_artifact();
    let csharp_member_access_expression_second =
        build_csharp_member_access_expression_syntax_generated_codec_artifact();
    let csharp_field_declaration_first =
        build_csharp_field_declaration_syntax_generated_codec_artifact();
    let csharp_field_declaration_second =
        build_csharp_field_declaration_syntax_generated_codec_artifact();
    let csharp_base_list_first = build_csharp_base_list_syntax_generated_codec_artifact();
    let csharp_base_list_second = build_csharp_base_list_syntax_generated_codec_artifact();
    let csharp_conditional_access_first =
        build_csharp_conditional_access_syntax_generated_codec_artifact();
    let csharp_conditional_access_second =
        build_csharp_conditional_access_syntax_generated_codec_artifact();
    let csharp_array_rank_object_creation_first =
        build_csharp_array_rank_object_creation_syntax_generated_codec_artifact();
    let csharp_array_rank_object_creation_second =
        build_csharp_array_rank_object_creation_syntax_generated_codec_artifact();
    let csharp_this_expression_first =
        build_csharp_this_expression_syntax_generated_codec_artifact();
    let csharp_this_expression_second =
        build_csharp_this_expression_syntax_generated_codec_artifact();
    let csharp_pattern_expression_first =
        build_csharp_pattern_expression_syntax_generated_codec_artifact();
    let csharp_pattern_expression_second =
        build_csharp_pattern_expression_syntax_generated_codec_artifact();
    let csharp_arrow_expression_clause_first =
        build_csharp_arrow_expression_clause_syntax_generated_codec_artifact();
    let csharp_arrow_expression_clause_second =
        build_csharp_arrow_expression_clause_syntax_generated_codec_artifact();
    let csharp_struct_declaration_first =
        build_csharp_struct_declaration_syntax_generated_codec_artifact();
    let csharp_struct_declaration_second =
        build_csharp_struct_declaration_syntax_generated_codec_artifact();
    let csharp_enum_declaration_first =
        build_csharp_enum_declaration_syntax_generated_codec_artifact();
    let csharp_enum_declaration_second =
        build_csharp_enum_declaration_syntax_generated_codec_artifact();
    let csharp_event_declaration_first =
        build_csharp_event_declaration_syntax_generated_codec_artifact();
    let csharp_event_declaration_second =
        build_csharp_event_declaration_syntax_generated_codec_artifact();
    let csharp_yield_statement_first =
        build_csharp_yield_statement_syntax_generated_codec_artifact();
    let csharp_yield_statement_second =
        build_csharp_yield_statement_syntax_generated_codec_artifact();
    let csharp_attribute_syntax_first = build_csharp_attribute_syntax_generated_codec_artifact();
    let csharp_attribute_syntax_second = build_csharp_attribute_syntax_generated_codec_artifact();
    let csharp_generic_name_first = build_csharp_generic_name_syntax_generated_codec_artifact();
    let csharp_generic_name_second = build_csharp_generic_name_syntax_generated_codec_artifact();
    let csharp_local_integer_return_literal_first =
        build_csharp_local_integer_return_literal_syntax_generated_codec_artifact();
    let csharp_local_integer_return_literal_second =
        build_csharp_local_integer_return_literal_syntax_generated_codec_artifact();
    let csharp_local_float_return_literal_first =
        build_csharp_local_float_return_literal_syntax_generated_codec_artifact();
    let csharp_local_float_return_literal_second =
        build_csharp_local_float_return_literal_syntax_generated_codec_artifact();
    let csharp_local_boolean_return_literal_first =
        build_csharp_local_boolean_return_literal_syntax_generated_codec_artifact();
    let csharp_local_boolean_return_literal_second =
        build_csharp_local_boolean_return_literal_syntax_generated_codec_artifact();
    let csharp_local_null_return_literal_first =
        build_csharp_local_null_return_literal_syntax_generated_codec_artifact();
    let csharp_local_null_return_literal_second =
        build_csharp_local_null_return_literal_syntax_generated_codec_artifact();
    let csharp_local_nullable_return_type_first =
        build_csharp_local_nullable_return_type_syntax_generated_codec_artifact();
    let csharp_local_nullable_return_type_second =
        build_csharp_local_nullable_return_type_syntax_generated_codec_artifact();
    let csharp_local_nullable_declaration_first =
        build_csharp_local_nullable_declaration_syntax_generated_codec_artifact();
    let csharp_local_nullable_declaration_second =
        build_csharp_local_nullable_declaration_syntax_generated_codec_artifact();
    let csharp_nullable_property_declaration_first =
        build_csharp_nullable_property_declaration_syntax_generated_codec_artifact();
    let csharp_nullable_property_declaration_second =
        build_csharp_nullable_property_declaration_syntax_generated_codec_artifact();
    let csharp_nullable_parameter_declaration_first =
        build_csharp_nullable_parameter_declaration_syntax_generated_codec_artifact();
    let csharp_nullable_parameter_declaration_second =
        build_csharp_nullable_parameter_declaration_syntax_generated_codec_artifact();
    let csharp_local_unary_integer_return_first =
        build_csharp_local_unary_integer_return_syntax_generated_codec_artifact();
    let csharp_local_unary_integer_return_second =
        build_csharp_local_unary_integer_return_syntax_generated_codec_artifact();
    let csharp_local_parenthesized_integer_return_first =
        build_csharp_local_parenthesized_integer_return_syntax_generated_codec_artifact();
    let csharp_local_parenthesized_integer_return_second =
        build_csharp_local_parenthesized_integer_return_syntax_generated_codec_artifact();
    let csharp_local_binary_integer_return_first =
        build_csharp_local_binary_integer_return_syntax_generated_codec_artifact();
    let csharp_local_binary_integer_return_second =
        build_csharp_local_binary_integer_return_syntax_generated_codec_artifact();
    let csharp_local_typed_declaration_first =
        build_csharp_local_typed_declaration_syntax_generated_codec_artifact();
    let csharp_local_typed_declaration_second =
        build_csharp_local_typed_declaration_syntax_generated_codec_artifact();
    let csharp_local_implicit_declaration_first =
        build_csharp_local_implicit_declaration_syntax_generated_codec_artifact();
    let csharp_local_implicit_declaration_second =
        build_csharp_local_implicit_declaration_syntax_generated_codec_artifact();
    let csharp_local_integer_implicit_declaration_first =
        build_csharp_local_integer_implicit_declaration_syntax_generated_codec_artifact();
    let csharp_local_integer_implicit_declaration_second =
        build_csharp_local_integer_implicit_declaration_syntax_generated_codec_artifact();
    let csharp_local_assignment_first =
        build_csharp_local_assignment_syntax_generated_codec_artifact();
    let csharp_local_assignment_second =
        build_csharp_local_assignment_syntax_generated_codec_artifact();
    let csharp_local_if_else_first = build_csharp_local_if_else_syntax_generated_codec_artifact();
    let csharp_local_if_else_second = build_csharp_local_if_else_syntax_generated_codec_artifact();
    let csharp_local_for_condition_first =
        build_csharp_local_for_condition_syntax_generated_codec_artifact();
    let csharp_local_for_condition_second =
        build_csharp_local_for_condition_syntax_generated_codec_artifact();
    let csharp_local_switch_first = build_csharp_local_switch_syntax_generated_codec_artifact();
    let csharp_local_switch_second = build_csharp_local_switch_syntax_generated_codec_artifact();
    let csharp_local_foreach_string_first =
        build_csharp_local_foreach_string_syntax_generated_codec_artifact();
    let csharp_local_foreach_string_second =
        build_csharp_local_foreach_string_syntax_generated_codec_artifact();
    let csharp_two_parameter_method_first =
        build_csharp_two_parameter_method_syntax_generated_codec_artifact();
    let csharp_two_parameter_method_second =
        build_csharp_two_parameter_method_syntax_generated_codec_artifact();
    let csharp_property_plus_method_first =
        build_csharp_property_plus_method_syntax_generated_codec_artifact();
    let csharp_property_plus_method_second =
        build_csharp_property_plus_method_syntax_generated_codec_artifact();
    let csharp_method_invocation_return_first =
        build_csharp_method_invocation_return_syntax_generated_codec_artifact();
    let csharp_method_invocation_return_second =
        build_csharp_method_invocation_return_syntax_generated_codec_artifact();
    let csharp_method_invocation_argument_return_first =
        build_csharp_method_invocation_argument_return_syntax_generated_codec_artifact();
    let csharp_method_invocation_argument_return_second =
        build_csharp_method_invocation_argument_return_syntax_generated_codec_artifact();
    let csharp_collection_initializer_first =
        build_csharp_collection_initializer_syntax_generated_codec_artifact();
    let csharp_collection_initializer_second =
        build_csharp_collection_initializer_syntax_generated_codec_artifact();
    let csharp_element_access_first = build_csharp_element_access_syntax_generated_codec_artifact();
    let csharp_element_access_second =
        build_csharp_element_access_syntax_generated_codec_artifact();
    let csharp_range_element_access_first =
        build_csharp_range_element_access_syntax_generated_codec_artifact();
    let csharp_range_element_access_second =
        build_csharp_range_element_access_syntax_generated_codec_artifact();
    let csharp_local_escaped_assignment_first =
        build_csharp_local_escaped_assignment_syntax_generated_codec_artifact();
    let csharp_local_escaped_assignment_second =
        build_csharp_local_escaped_assignment_syntax_generated_codec_artifact();
    let go_first = build_go_minimal_function_compiler_route_artifact()
        .expect("Go minimal function route artifact should be reproducible");
    let go_second = build_go_minimal_function_compiler_route_artifact()
        .expect("Go minimal function route artifact should be reproducible");
    let go_local_first = build_go_local_declaration_compiler_route_artifact()
        .expect("Go local declaration route artifact should be reproducible");
    let go_local_second = build_go_local_declaration_compiler_route_artifact()
        .expect("Go local declaration route artifact should be reproducible");
    let go_assignment_first = build_go_local_assignment_syntax_generated_codec_artifact();
    let go_assignment_second = build_go_local_assignment_syntax_generated_codec_artifact();
    let go_escaped_assignment_first =
        build_go_escaped_local_assignment_syntax_generated_codec_artifact();
    let go_escaped_assignment_second =
        build_go_escaped_local_assignment_syntax_generated_codec_artifact();
    let go_short_var_first = build_go_local_short_var_declaration_syntax_generated_codec_artifact();
    let go_short_var_second =
        build_go_local_short_var_declaration_syntax_generated_codec_artifact();
    let go_integer_short_var_first =
        build_go_local_integer_short_var_declaration_syntax_generated_codec_artifact();
    let go_integer_short_var_second =
        build_go_local_integer_short_var_declaration_syntax_generated_codec_artifact();
    let go_integer_return_literal_first =
        build_go_local_integer_return_literal_syntax_generated_codec_artifact();
    let go_integer_return_literal_second =
        build_go_local_integer_return_literal_syntax_generated_codec_artifact();
    let go_package_type_declaration_first =
        build_go_package_type_declaration_syntax_generated_codec_artifact();
    let go_package_type_declaration_second =
        build_go_package_type_declaration_syntax_generated_codec_artifact();
    let go_type_spec_assign_first = build_go_type_spec_assign_syntax_generated_codec_artifact();
    let go_type_spec_assign_second = build_go_type_spec_assign_syntax_generated_codec_artifact();
    let go_type_spec_comment_first = build_go_type_spec_comment_syntax_generated_codec_artifact();
    let go_type_spec_comment_second = build_go_type_spec_comment_syntax_generated_codec_artifact();
    let go_type_spec_doc_first = build_go_type_spec_doc_syntax_generated_codec_artifact();
    let go_type_spec_doc_second = build_go_type_spec_doc_syntax_generated_codec_artifact();
    let go_package_const_declaration_first =
        build_go_package_const_declaration_syntax_generated_codec_artifact();
    let go_package_const_declaration_second =
        build_go_package_const_declaration_syntax_generated_codec_artifact();
    let go_method_declaration_first = build_go_method_declaration_syntax_generated_codec_artifact();
    let go_method_declaration_second =
        build_go_method_declaration_syntax_generated_codec_artifact();
    let go_struct_type_first = build_go_struct_type_syntax_generated_codec_artifact();
    let go_struct_type_second = build_go_struct_type_syntax_generated_codec_artifact();
    let go_composite_literal_first = build_go_composite_literal_syntax_generated_codec_artifact();
    let go_composite_literal_second = build_go_composite_literal_syntax_generated_codec_artifact();
    let go_key_value_composite_literal_first =
        build_go_key_value_composite_literal_syntax_generated_codec_artifact();
    let go_key_value_composite_literal_second =
        build_go_key_value_composite_literal_syntax_generated_codec_artifact();
    let go_array_composite_literal_first =
        build_go_array_composite_literal_syntax_generated_codec_artifact();
    let go_array_composite_literal_second =
        build_go_array_composite_literal_syntax_generated_codec_artifact();
    let go_index_expression_first = build_go_index_expression_syntax_generated_codec_artifact();
    let go_index_expression_second = build_go_index_expression_syntax_generated_codec_artifact();
    let go_slice_expression_first = build_go_slice_expression_syntax_generated_codec_artifact();
    let go_slice_expression_second = build_go_slice_expression_syntax_generated_codec_artifact();
    let go_slice_expression_three_index_first =
        build_go_slice_expression_three_index_syntax_generated_codec_artifact();
    let go_slice_expression_three_index_second =
        build_go_slice_expression_three_index_syntax_generated_codec_artifact();
    let go_star_expression_first = build_go_star_expression_syntax_generated_codec_artifact();
    let go_star_expression_second = build_go_star_expression_syntax_generated_codec_artifact();
    let go_package_var_declaration_first =
        build_go_package_var_declaration_syntax_generated_codec_artifact();
    let go_package_var_declaration_second =
        build_go_package_var_declaration_syntax_generated_codec_artifact();
    let go_function_literal_first = build_go_function_literal_syntax_generated_codec_artifact();
    let go_function_literal_second = build_go_function_literal_syntax_generated_codec_artifact();
    let go_parenthesized_integer_return_expression_first =
        build_go_local_parenthesized_integer_return_expression_syntax_generated_codec_artifact();
    let go_parenthesized_integer_return_expression_second =
        build_go_local_parenthesized_integer_return_expression_syntax_generated_codec_artifact();
    let go_unary_integer_return_expression_first =
        build_go_local_unary_integer_return_expression_syntax_generated_codec_artifact();
    let go_unary_integer_return_expression_second =
        build_go_local_unary_integer_return_expression_syntax_generated_codec_artifact();
    let go_binary_integer_return_expression_first =
        build_go_local_binary_integer_return_expression_syntax_generated_codec_artifact();
    let go_binary_integer_return_expression_second =
        build_go_local_binary_integer_return_expression_syntax_generated_codec_artifact();
    let go_call_return_expression_first =
        build_go_local_call_return_expression_syntax_generated_codec_artifact();
    let go_call_return_expression_second =
        build_go_local_call_return_expression_syntax_generated_codec_artifact();
    let go_argument_call_return_expression_first =
        build_go_local_argument_call_return_expression_syntax_generated_codec_artifact();
    let go_argument_call_return_expression_second =
        build_go_local_argument_call_return_expression_syntax_generated_codec_artifact();
    let go_string_return_literal_first =
        build_go_local_string_return_literal_syntax_generated_codec_artifact();
    let go_string_return_literal_second =
        build_go_local_string_return_literal_syntax_generated_codec_artifact();
    let go_float_return_literal_first =
        build_go_local_float_return_literal_syntax_generated_codec_artifact();
    let go_float_return_literal_second =
        build_go_local_float_return_literal_syntax_generated_codec_artifact();
    let go_boolean_return_identifier_first =
        build_go_local_boolean_return_identifier_syntax_generated_codec_artifact();
    let go_boolean_return_identifier_second =
        build_go_local_boolean_return_identifier_syntax_generated_codec_artifact();
    let go_nil_return_identifier_first =
        build_go_local_nil_return_identifier_syntax_generated_codec_artifact();
    let go_nil_return_identifier_second =
        build_go_local_nil_return_identifier_syntax_generated_codec_artifact();
    let go_typed_local_first = build_go_local_typed_declaration_syntax_generated_codec_artifact();
    let go_typed_local_second = build_go_local_typed_declaration_syntax_generated_codec_artifact();
    let go_zero_value_typed_local_first =
        build_go_local_zero_value_typed_declaration_syntax_generated_codec_artifact();
    let go_zero_value_typed_local_second =
        build_go_local_zero_value_typed_declaration_syntax_generated_codec_artifact();
    let go_integer_typed_local_first =
        build_go_local_integer_typed_declaration_syntax_generated_codec_artifact();
    let go_integer_typed_local_second =
        build_go_local_integer_typed_declaration_syntax_generated_codec_artifact();
    let go_untyped_integer_local_first =
        build_go_local_untyped_integer_declaration_syntax_generated_codec_artifact();
    let go_untyped_integer_local_second =
        build_go_local_untyped_integer_declaration_syntax_generated_codec_artifact();
    let go_integer_assignment_first =
        build_go_local_integer_assignment_syntax_generated_codec_artifact();
    let go_integer_assignment_second =
        build_go_local_integer_assignment_syntax_generated_codec_artifact();
    let go_compound_assignment_first =
        build_go_local_compound_assignment_syntax_generated_codec_artifact();
    let go_compound_assignment_second =
        build_go_local_compound_assignment_syntax_generated_codec_artifact();
    let go_increment_decrement_first =
        build_go_local_increment_decrement_syntax_generated_codec_artifact();
    let go_increment_decrement_second =
        build_go_local_increment_decrement_syntax_generated_codec_artifact();
    let go_branch_statement_first =
        build_go_local_branch_statement_syntax_generated_codec_artifact();
    let go_branch_statement_second =
        build_go_local_branch_statement_syntax_generated_codec_artifact();
    let go_if_else_first = build_go_local_if_else_syntax_generated_codec_artifact();
    let go_if_else_second = build_go_local_if_else_syntax_generated_codec_artifact();
    let go_for_condition_first = build_go_local_for_condition_syntax_generated_codec_artifact();
    let go_for_condition_second = build_go_local_for_condition_syntax_generated_codec_artifact();
    let go_switch_first = build_go_local_switch_syntax_generated_codec_artifact();
    let go_switch_second = build_go_local_switch_syntax_generated_codec_artifact();
    let go_switch_init_first = build_go_local_switch_init_syntax_generated_codec_artifact();
    let go_switch_init_second = build_go_local_switch_init_syntax_generated_codec_artifact();
    let go_type_switch_first = build_go_local_type_switch_syntax_generated_codec_artifact();
    let go_type_switch_second = build_go_local_type_switch_syntax_generated_codec_artifact();
    let go_range_string_first = build_go_local_range_string_syntax_generated_codec_artifact();
    let go_range_string_second = build_go_local_range_string_syntax_generated_codec_artifact();

    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_first,
        &csharp_second,
        "csharp",
        "csharp_minimal_class_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_string_return_literal_first,
        &csharp_local_string_return_literal_second,
        "csharp",
        "csharp_local_string_return_literal_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-string-return-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_compilation_unit_source_file_first,
        &csharp_compilation_unit_source_file_second,
        "csharp",
        "csharp_compilation_unit_source_file_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-compilation-unit-source-file-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_block_scoped_namespace_first,
        &csharp_block_scoped_namespace_second,
        "csharp",
        "csharp_block_scoped_namespace_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_BLOCK_SCOPED_NAMESPACE_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-block-scoped-namespace-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_using_directive_first,
        &csharp_using_directive_second,
        "csharp",
        "csharp_using_directive_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_USING_DIRECTIVE_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-using-directive-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_qualified_name_first,
        &csharp_qualified_name_second,
        "csharp",
        "csharp_qualified_name_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_QUALIFIED_NAME_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-qualified-name-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_constructor_declaration_first,
        &csharp_constructor_declaration_second,
        "csharp",
        "csharp_constructor_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_CONSTRUCTOR_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-constructor-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_while_postfix_loop_first,
        &csharp_while_postfix_loop_second,
        "csharp",
        "csharp_while_postfix_loop_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_WHILE_POSTFIX_LOOP_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-while-postfix-loop-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_interpolated_string_first,
        &csharp_interpolated_string_second,
        "csharp",
        "csharp_interpolated_string_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_INTERPOLATED_STRING_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-interpolated-string-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_member_access_expression_first,
        &csharp_member_access_expression_second,
        "csharp",
        "csharp_member_access_expression_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-member-access-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_field_declaration_first,
        &csharp_field_declaration_second,
        "csharp",
        "csharp_field_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_FIELD_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-field-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_base_list_first,
        &csharp_base_list_second,
        "csharp",
        "csharp_base_list_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_BASE_LIST_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-base-list-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_conditional_access_first,
        &csharp_conditional_access_second,
        "csharp",
        "csharp_conditional_access_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_CONDITIONAL_ACCESS_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-conditional-access-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_array_rank_object_creation_first,
        &csharp_array_rank_object_creation_second,
        "csharp",
        "csharp_array_rank_object_creation_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_ARRAY_RANK_OBJECT_CREATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-array-rank-object-creation-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_this_expression_first,
        &csharp_this_expression_second,
        "csharp",
        "csharp_this_expression_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_THIS_EXPRESSION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-this-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_pattern_expression_first,
        &csharp_pattern_expression_second,
        "csharp",
        "csharp_pattern_expression_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_PATTERN_EXPRESSION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-pattern-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_arrow_expression_clause_first,
        &csharp_arrow_expression_clause_second,
        "csharp",
        "csharp_arrow_expression_clause_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_ARROW_EXPRESSION_CLAUSE_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-arrow-expression-clause-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_struct_declaration_first,
        &csharp_struct_declaration_second,
        "csharp",
        "csharp_struct_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_STRUCT_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-struct-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_enum_declaration_first,
        &csharp_enum_declaration_second,
        "csharp",
        "csharp_enum_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_ENUM_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-enum-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_event_declaration_first,
        &csharp_event_declaration_second,
        "csharp",
        "csharp_event_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_EVENT_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-event-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_yield_statement_first,
        &csharp_yield_statement_second,
        "csharp",
        "csharp_yield_statement_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_YIELD_STATEMENT_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-yield-statement-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_attribute_syntax_first,
        &csharp_attribute_syntax_second,
        "csharp",
        "csharp_attribute_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_ATTRIBUTE_SYNTAX_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-attribute-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_generic_name_first,
        &csharp_generic_name_second,
        "csharp",
        "csharp_generic_name_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_GENERIC_NAME_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-generic-name-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_integer_return_literal_first,
        &csharp_local_integer_return_literal_second,
        "csharp",
        "csharp_local_integer_return_literal_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-integer-return-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_float_return_literal_first,
        &csharp_local_float_return_literal_second,
        "csharp",
        "csharp_local_float_return_literal_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-float-return-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_boolean_return_literal_first,
        &csharp_local_boolean_return_literal_second,
        "csharp",
        "csharp_local_boolean_return_literal_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-boolean-return-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_null_return_literal_first,
        &csharp_local_null_return_literal_second,
        "csharp",
        "csharp_local_null_return_literal_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_NULL_RETURN_LITERAL_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-null-return-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_nullable_return_type_first,
        &csharp_local_nullable_return_type_second,
        "csharp",
        "csharp_local_nullable_return_type_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-nullable-return-type-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_nullable_declaration_first,
        &csharp_local_nullable_declaration_second,
        "csharp",
        "csharp_local_nullable_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_NULLABLE_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-nullable-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_nullable_property_declaration_first,
        &csharp_nullable_property_declaration_second,
        "csharp",
        "csharp_nullable_property_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_NULLABLE_PROPERTY_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-nullable-property-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_nullable_parameter_declaration_first,
        &csharp_nullable_parameter_declaration_second,
        "csharp",
        "csharp_nullable_parameter_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_NULLABLE_PARAMETER_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-nullable-parameter-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_unary_integer_return_first,
        &csharp_local_unary_integer_return_second,
        "csharp",
        "csharp_local_unary_integer_return_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-unary-integer-return-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_parenthesized_integer_return_first,
        &csharp_local_parenthesized_integer_return_second,
        "csharp",
        "csharp_local_parenthesized_integer_return_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-parenthesized-integer-return-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_binary_integer_return_first,
        &csharp_local_binary_integer_return_second,
        "csharp",
        "csharp_local_binary_integer_return_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-binary-integer-return-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_typed_declaration_first,
        &csharp_local_typed_declaration_second,
        "csharp",
        "csharp_local_typed_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_TYPED_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-typed-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_implicit_declaration_first,
        &csharp_local_implicit_declaration_second,
        "csharp",
        "csharp_local_implicit_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_IMPLICIT_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-implicit-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_integer_implicit_declaration_first,
        &csharp_local_integer_implicit_declaration_second,
        "csharp",
        "csharp_local_integer_implicit_declaration_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-integer-implicit-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_assignment_first,
        &csharp_local_assignment_second,
        "csharp",
        "csharp_local_assignment_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_ASSIGNMENT_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-assignment-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_if_else_first,
        &csharp_local_if_else_second,
        "csharp",
        "csharp_local_if_else_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_IF_ELSE_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-if-else-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_for_condition_first,
        &csharp_local_for_condition_second,
        "csharp",
        "csharp_local_for_condition_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-for-condition-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_switch_first,
        &csharp_local_switch_second,
        "csharp",
        "csharp_local_switch_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_SWITCH_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-switch-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_foreach_string_first,
        &csharp_local_foreach_string_second,
        "csharp",
        "csharp_local_foreach_string_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-foreach-string-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_two_parameter_method_first,
        &csharp_two_parameter_method_second,
        "csharp",
        "csharp_two_parameter_method_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_TWO_PARAMETER_METHOD_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-two-parameter-method-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_property_plus_method_first,
        &csharp_property_plus_method_second,
        "csharp",
        "csharp_property_plus_method_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-property-plus-method-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_method_invocation_return_first,
        &csharp_method_invocation_return_second,
        "csharp",
        "csharp_method_invocation_return_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_METHOD_INVOCATION_RETURN_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-method-invocation-return-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_method_invocation_argument_return_first,
        &csharp_method_invocation_argument_return_second,
        "csharp",
        "csharp_method_invocation_argument_return_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-method-invocation-argument-return-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_collection_initializer_first,
        &csharp_collection_initializer_second,
        "csharp",
        "csharp_collection_initializer_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_COLLECTION_INITIALIZER_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-collection-initializer-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_element_access_first,
        &csharp_element_access_second,
        "csharp",
        "csharp_element_access_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_ELEMENT_ACCESS_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-element-access-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_range_element_access_first,
        &csharp_range_element_access_second,
        "csharp",
        "csharp_range_element_access_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_RANGE_ELEMENT_ACCESS_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-range-element-access-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &csharp_local_escaped_assignment_first,
        &csharp_local_escaped_assignment_second,
        "csharp",
        "csharp_local_escaped_assignment_syntax_v0",
        "csharp_syntax_metadata",
        CSHARP_LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH,
        "dotnet",
        "render_csharp_syntax_codec_fixture",
        "csharp-local-escaped-assignment-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_first,
        &go_second,
        "go",
        "go_minimal_function_syntax_v0",
        "go_syntax_metadata",
        GO_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_local_first,
        &go_local_second,
        "go",
        "go_local_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-declaration-syntax-generated-codec",
    );
    assert_target_agnostic_compiler_route_reproducibility_artifact(
        &csharp_first,
        &csharp_second,
        "csharp",
        "csharp_minimal_class_syntax_v0",
        CSHARP_METADATA_PATH,
        "csharp_fixture",
    );
    assert_target_agnostic_compiler_route_reproducibility_artifact(
        &csharp_local_string_return_literal_first,
        &csharp_local_string_return_literal_second,
        "csharp",
        "csharp_local_string_return_literal_syntax_v0",
        CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH,
        "csharp_non_minimal_fixture",
    );
    assert_target_agnostic_compiler_route_reproducibility_artifact(
        &go_first,
        &go_second,
        "go",
        "go_minimal_function_syntax_v0",
        GO_METADATA_PATH,
        "go_fixture",
    );
    assert_target_agnostic_compiler_route_reproducibility_artifact(
        &go_local_first,
        &go_local_second,
        "go",
        "go_local_declaration_syntax_v0",
        GO_LOCAL_DECLARATION_METADATA_PATH,
        "go_non_minimal_fixture",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_assignment_first,
        &go_assignment_second,
        "go",
        "go_local_assignment_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_ASSIGNMENT_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-assignment-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_escaped_assignment_first,
        &go_escaped_assignment_second,
        "go",
        "go_escaped_local_assignment_syntax_v0",
        "go_syntax_metadata",
        GO_ESCAPED_LOCAL_ASSIGNMENT_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-escaped-local-assignment-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_short_var_first,
        &go_short_var_second,
        "go",
        "go_local_short_var_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-short-var-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_integer_short_var_first,
        &go_integer_short_var_second,
        "go",
        "go_local_integer_short_var_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-integer-short-var-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_integer_return_literal_first,
        &go_integer_return_literal_second,
        "go",
        "go_local_integer_return_literal_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-integer-return-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_package_type_declaration_first,
        &go_package_type_declaration_second,
        "go",
        "go_package_type_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_PACKAGE_TYPE_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-package-type-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_type_spec_assign_first,
        &go_type_spec_assign_second,
        "go",
        "go_type_spec_assign_syntax_v0",
        "go_syntax_metadata",
        GO_TYPE_SPEC_ASSIGN_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-type-spec-assign-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_type_spec_comment_first,
        &go_type_spec_comment_second,
        "go",
        "go_type_spec_comment_syntax_v0",
        "go_syntax_metadata",
        GO_TYPE_SPEC_COMMENT_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-type-spec-comment-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_type_spec_doc_first,
        &go_type_spec_doc_second,
        "go",
        "go_type_spec_doc_syntax_v0",
        "go_syntax_metadata",
        GO_TYPE_SPEC_DOC_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-type-spec-doc-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_package_const_declaration_first,
        &go_package_const_declaration_second,
        "go",
        "go_package_const_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_PACKAGE_CONST_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-package-const-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_method_declaration_first,
        &go_method_declaration_second,
        "go",
        "go_method_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_METHOD_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-method-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_struct_type_first,
        &go_struct_type_second,
        "go",
        "go_struct_type_syntax_v0",
        "go_syntax_metadata",
        GO_STRUCT_TYPE_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-struct-type-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_composite_literal_first,
        &go_composite_literal_second,
        "go",
        "go_composite_literal_syntax_v0",
        "go_syntax_metadata",
        GO_COMPOSITE_LITERAL_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-composite-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_key_value_composite_literal_first,
        &go_key_value_composite_literal_second,
        "go",
        "go_key_value_composite_literal_syntax_v0",
        "go_syntax_metadata",
        GO_KEY_VALUE_COMPOSITE_LITERAL_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-key-value-composite-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_array_composite_literal_first,
        &go_array_composite_literal_second,
        "go",
        "go_array_composite_literal_syntax_v0",
        "go_syntax_metadata",
        GO_ARRAY_COMPOSITE_LITERAL_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-array-composite-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_index_expression_first,
        &go_index_expression_second,
        "go",
        "go_index_expression_syntax_v0",
        "go_syntax_metadata",
        GO_INDEX_EXPRESSION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-index-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_slice_expression_first,
        &go_slice_expression_second,
        "go",
        "go_slice_expression_syntax_v0",
        "go_syntax_metadata",
        GO_SLICE_EXPRESSION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-slice-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_slice_expression_three_index_first,
        &go_slice_expression_three_index_second,
        "go",
        "go_slice_expression_three_index_syntax_v0",
        "go_syntax_metadata",
        GO_SLICE_EXPRESSION_THREE_INDEX_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-slice-expression-three-index-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_star_expression_first,
        &go_star_expression_second,
        "go",
        "go_star_expression_syntax_v0",
        "go_syntax_metadata",
        GO_STAR_EXPRESSION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-star-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_package_var_declaration_first,
        &go_package_var_declaration_second,
        "go",
        "go_package_var_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_PACKAGE_VAR_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-package-var-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_function_literal_first,
        &go_function_literal_second,
        "go",
        "go_function_literal_syntax_v0",
        "go_syntax_metadata",
        GO_FUNCTION_LITERAL_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-function-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_parenthesized_integer_return_expression_first,
        &go_parenthesized_integer_return_expression_second,
        "go",
        "go_local_parenthesized_integer_return_expression_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-parenthesized-integer-return-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_unary_integer_return_expression_first,
        &go_unary_integer_return_expression_second,
        "go",
        "go_local_unary_integer_return_expression_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-unary-integer-return-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_binary_integer_return_expression_first,
        &go_binary_integer_return_expression_second,
        "go",
        "go_local_binary_integer_return_expression_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-binary-integer-return-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_call_return_expression_first,
        &go_call_return_expression_second,
        "go",
        "go_local_call_return_expression_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-call-return-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_argument_call_return_expression_first,
        &go_argument_call_return_expression_second,
        "go",
        "go_local_argument_call_return_expression_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-argument-call-return-expression-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_string_return_literal_first,
        &go_string_return_literal_second,
        "go",
        "go_local_string_return_literal_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-string-return-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_float_return_literal_first,
        &go_float_return_literal_second,
        "go",
        "go_local_float_return_literal_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-float-return-literal-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_boolean_return_identifier_first,
        &go_boolean_return_identifier_second,
        "go",
        "go_local_boolean_return_identifier_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-boolean-return-identifier-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_nil_return_identifier_first,
        &go_nil_return_identifier_second,
        "go",
        "go_local_nil_return_identifier_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-nil-return-identifier-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_typed_local_first,
        &go_typed_local_second,
        "go",
        "go_local_typed_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_TYPED_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-typed-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_zero_value_typed_local_first,
        &go_zero_value_typed_local_second,
        "go",
        "go_local_zero_value_typed_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-zero-value-typed-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_integer_typed_local_first,
        &go_integer_typed_local_second,
        "go",
        "go_local_integer_typed_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_INTEGER_TYPED_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-integer-typed-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_untyped_integer_local_first,
        &go_untyped_integer_local_second,
        "go",
        "go_local_untyped_integer_declaration_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_UNTYPED_INTEGER_DECLARATION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-untyped-integer-declaration-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_integer_assignment_first,
        &go_integer_assignment_second,
        "go",
        "go_local_integer_assignment_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_INTEGER_ASSIGNMENT_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-integer-assignment-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_compound_assignment_first,
        &go_compound_assignment_second,
        "go",
        "go_local_compound_assignment_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_COMPOUND_ASSIGNMENT_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-compound-assignment-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_increment_decrement_first,
        &go_increment_decrement_second,
        "go",
        "go_local_increment_decrement_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_INCREMENT_DECREMENT_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-increment-decrement-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_branch_statement_first,
        &go_branch_statement_second,
        "go",
        "go_local_branch_statement_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_BRANCH_STATEMENT_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-branch-statement-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_if_else_first,
        &go_if_else_second,
        "go",
        "go_local_if_else_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_IF_ELSE_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-if-else-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_for_condition_first,
        &go_for_condition_second,
        "go",
        "go_local_for_condition_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_FOR_CONDITION_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-for-condition-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_switch_first,
        &go_switch_second,
        "go",
        "go_local_switch_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_SWITCH_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-switch-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_switch_init_first,
        &go_switch_init_second,
        "go",
        "go_local_switch_init_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_SWITCH_INIT_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-switch-init-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_type_switch_first,
        &go_type_switch_second,
        "go",
        "go_local_type_switch_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_TYPE_SWITCH_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-type-switch-syntax-generated-codec",
    );
    assert_parser_backed_syntax_generated_codec_artifact(
        &go_range_string_first,
        &go_range_string_second,
        "go",
        "go_local_range_string_syntax_v0",
        "go_syntax_metadata",
        GO_LOCAL_RANGE_STRING_METADATA_PATH,
        "go",
        "render_go_syntax_codec_fixture",
        "go-local-range-string-syntax-generated-codec",
    );

    write_parser_backed_syntax_generated_codec_reproducibility_report(&[
        csharp_first,
        csharp_local_string_return_literal_first,
        csharp_local_integer_return_literal_first,
        csharp_local_float_return_literal_first,
        csharp_local_boolean_return_literal_first,
        csharp_local_null_return_literal_first,
        csharp_local_nullable_return_type_first,
        csharp_local_nullable_declaration_first,
        csharp_nullable_property_declaration_first,
        csharp_nullable_parameter_declaration_first,
        csharp_local_unary_integer_return_first,
        csharp_local_parenthesized_integer_return_first,
        csharp_local_binary_integer_return_first,
        csharp_local_typed_declaration_first,
        csharp_local_implicit_declaration_first,
        csharp_local_integer_implicit_declaration_first,
        csharp_local_assignment_first,
        csharp_local_if_else_first,
        csharp_local_for_condition_first,
        csharp_local_switch_first,
        csharp_local_foreach_string_first,
        csharp_two_parameter_method_first,
        csharp_property_plus_method_first,
        csharp_method_invocation_return_first,
        csharp_method_invocation_argument_return_first,
        csharp_collection_initializer_first,
        csharp_element_access_first,
        csharp_range_element_access_first,
        csharp_local_escaped_assignment_first,
        go_first,
        go_local_first,
        go_assignment_first,
        go_escaped_assignment_first,
        go_short_var_first,
        go_integer_short_var_first,
        go_integer_return_literal_first,
        go_package_type_declaration_first,
        go_type_spec_assign_first,
        go_type_spec_comment_first,
        go_type_spec_doc_first,
        go_package_const_declaration_first,
        go_method_declaration_first,
        go_struct_type_first,
        go_composite_literal_first,
        go_key_value_composite_literal_first,
        go_array_composite_literal_first,
        go_index_expression_first,
        go_slice_expression_first,
        go_slice_expression_three_index_first,
        go_star_expression_first,
        go_package_var_declaration_first,
        go_function_literal_first,
        go_parenthesized_integer_return_expression_first,
        go_unary_integer_return_expression_first,
        go_binary_integer_return_expression_first,
        go_call_return_expression_first,
        go_string_return_literal_first,
        go_float_return_literal_first,
        go_boolean_return_identifier_first,
        go_nil_return_identifier_first,
        go_typed_local_first,
        go_zero_value_typed_local_first,
        go_integer_typed_local_first,
        go_untyped_integer_local_first,
        go_integer_assignment_first,
        go_compound_assignment_first,
        go_increment_decrement_first,
        go_branch_statement_first,
        go_if_else_first,
        go_for_condition_first,
        go_switch_first,
        go_switch_init_first,
        go_type_switch_first,
        go_range_string_first,
    ]);
}

#[test]
fn parser_backed_strict_recovery_and_consumed_token_evidence_is_compiler_route_owned() {
    let csharp_minimal = build_csharp_minimal_class_compiler_route_artifact()
        .expect("C# minimal class route artifact should carry decode evidence");
    let csharp_local = build_csharp_local_string_return_literal_compiler_route_artifact()
        .expect("C# local string return route artifact should carry decode evidence");
    let go_minimal = build_go_minimal_function_compiler_route_artifact()
        .expect("Go minimal function route artifact should carry decode evidence");
    let go_local = build_go_local_declaration_compiler_route_artifact()
        .expect("Go local declaration route artifact should carry decode evidence");

    assert_target_agnostic_compiler_route_decode_evidence(
        &csharp_minimal,
        "csharp_minimal_class_syntax_v0",
        &[
            "generated_csharp_strict_decode_rejects_eof_with_accounting",
            "generated_csharp_strict_decode_rejects_trailing_tokens",
            "generated_csharp_strict_decode_rejects_malformed_payload",
            "generated_csharp_recovery_normalizes_trailing_pad_only",
            "generated_csharp_recovery_keeps_partial_prefix_hard",
        ],
    );
    assert_target_agnostic_compiler_route_decode_evidence(
        &csharp_local,
        "csharp_local_string_return_literal_syntax_v0",
        &[
            "generated_csharp_local_string_return_literal_strict_decode_rejects_eof_with_accounting",
            "generated_csharp_local_string_return_literal_strict_decode_rejects_trailing_tokens",
            "generated_csharp_local_string_return_literal_strict_decode_rejects_malformed_payload",
            "generated_csharp_local_string_return_literal_recovery_normalizes_trailing_pad_only",
            "generated_csharp_local_string_return_literal_recovery_keeps_partial_prefix_hard",
        ],
    );
    assert_target_agnostic_compiler_route_decode_evidence(
        &go_minimal,
        "go_minimal_function_syntax_v0",
        &[
            "generated_go_strict_decode_rejects_eof_with_accounting",
            "generated_go_strict_decode_rejects_trailing_tokens",
            "generated_go_strict_decode_rejects_malformed_payload",
            "generated_go_decode_rejects_invalid_identifier_payloads_with_accounting",
            "generated_go_recovery_normalizes_trailing_pad_only",
            "generated_go_recovery_keeps_partial_prefix_hard",
            "generated_go_minimal_function_real_source_strings_compare_demo",
        ],
    );
    assert_target_agnostic_compiler_route_decode_evidence(
        &go_local,
        "go_local_declaration_syntax_v0",
        &[
            "generated_go_local_declaration_strict_decode_rejects_eof_with_accounting",
            "generated_go_local_declaration_strict_decode_rejects_trailing_tokens",
            "generated_go_local_declaration_strict_decode_rejects_malformed_payload",
            "generated_go_local_declaration_rejects_mismatched_return_identifier_with_accounting",
            "generated_go_local_declaration_recovery_normalizes_trailing_pad_only",
            "generated_go_local_declaration_recovery_keeps_partial_prefix_hard",
        ],
    );
}

#[test]
fn parser_backed_token_vocabulary_bpe_evidence_is_compiler_route_owned() {
    let csharp_minimal = build_csharp_minimal_class_compiler_route_artifact()
        .expect("C# minimal class route artifact should carry token evidence");
    let csharp_local = build_csharp_local_string_return_literal_compiler_route_artifact()
        .expect("C# local string return route artifact should carry token evidence");
    let go_minimal = build_go_minimal_function_compiler_route_artifact()
        .expect("Go minimal function route artifact should carry token evidence");
    let go_local = build_go_local_declaration_compiler_route_artifact()
        .expect("Go local declaration route artifact should carry token evidence");

    assert_target_agnostic_compiler_route_token_vocabulary_bpe_evidence(
        &csharp_minimal,
        "csharp_minimal_class_syntax_v0",
    );
    assert_target_agnostic_compiler_route_token_vocabulary_bpe_evidence(
        &csharp_local,
        "csharp_local_string_return_literal_syntax_v0",
    );
    assert_target_agnostic_compiler_route_token_vocabulary_bpe_evidence(
        &go_minimal,
        "go_minimal_function_syntax_v0",
    );
    assert_target_agnostic_compiler_route_token_vocabulary_bpe_evidence(
        &go_local,
        "go_local_declaration_syntax_v0",
    );
}

#[test]
fn toy_syntax_generated_codec_token_layout_reserves_bpe_boundary() {
    let spec = ToyGrammarSpec::assignment_language_v1();
    let grammar = spec.to_grammar_ir();
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&spec.layout),
    )
    .unwrap();

    let boundary = sidecar
        .structural_model
        .token_ranges
        .reserve_bpe_boundary(32)
        .unwrap();
    assert_eq!(boundary.raw_base_vocab_size, INTEGER_TOKEN_END);
    assert_eq!(boundary.bpe_start, INTEGER_TOKEN_END);
    assert_eq!(boundary.bpe_end_exclusive, INTEGER_TOKEN_END + 32);
    assert_eq!(boundary.pad_token, PAD_TOKEN);
    assert_eq!(boundary.vocab_size, VOCAB_SIZE);
    assert!(boundary.current_layout_hosts_bpe);
    assert!(
        sidecar
            .structural_model
            .token_ranges
            .ranges
            .iter()
            .all(|range| range.end_exclusive <= boundary.bpe_start)
    );
}

#[test]
fn toy_syntax_bpe_merge_artifact_round_trips_token_corpus() {
    let spec = ToyGrammarSpec::assignment_language_v1();
    let grammar = spec.to_grammar_ir();
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&spec.layout),
    )
    .unwrap();
    let boundary = sidecar
        .structural_model
        .token_ranges
        .reserve_bpe_boundary(2)
        .unwrap();
    let source_token_corpus = toy_syntax_token_corpus();
    let merge_table = toy_syntax_bpe_merge_table(&boundary, source_token_corpus.len());
    merge_table.validate().unwrap();

    assert_eq!(merge_table.base_vocab_size, boundary.raw_base_vocab_size);
    assert_eq!(merge_table.bpe_start(), boundary.bpe_start);
    assert_eq!(boundary.bpe_end_exclusive, boundary.bpe_start + 2);

    let encoder = BpeEncoder::new(&merge_table, boundary.bpe_end_exclusive);
    let decoder = BpeDecoder::new(&merge_table, boundary.bpe_end_exclusive);
    let encoded_token_corpus: Vec<Vec<u32>> = source_token_corpus
        .iter()
        .map(|tokens| encoder.encode(tokens))
        .collect();
    let decoded_token_corpus: Vec<Vec<u32>> = encoded_token_corpus
        .iter()
        .map(|tokens| decoder.decode_strict(tokens).unwrap())
        .collect();

    assert_eq!(decoded_token_corpus, source_token_corpus);
    assert!(
        encoded_token_corpus
            .iter()
            .flatten()
            .any(|token| { (boundary.bpe_start..boundary.bpe_end_exclusive).contains(token) })
    );
    assert!(
        encoded_token_corpus
            .iter()
            .flatten()
            .all(|token| *token < boundary.bpe_end_exclusive)
    );
    assert!(
        encoded_token_corpus
            .iter()
            .flatten()
            .all(|token| *token != boundary.pad_token && *token < boundary.vocab_size)
    );

    let report = toy_syntax_bpe_merge_artifact_report(
        &sidecar,
        &boundary,
        &merge_table,
        &source_token_corpus,
        &encoded_token_corpus,
    );
    assert_eq!(
        report["artifact_kind"],
        "tct_syntax_bpe_merge_artifact_evidence"
    );
    assert_eq!(
        report["merge_table"]["sha256"],
        json_sha256(&merge_table).unwrap()
    );
    assert_eq!(
        report["source_token_corpus"]["sha256"],
        json_sha256(&source_token_corpus).unwrap()
    );
    assert_eq!(report["proof"]["decode_round_trip_ok"], true);
    assert_eq!(report["proof"]["bpe_ids_fit_reserved_boundary"], true);
    assert_eq!(report["proof"]["tokens_below_vocab_size"], true);
    assert_eq!(
        report["sidecar_boundary"]["generated_json_schema_output_changed"],
        false
    );

    write_toy_syntax_bpe_merge_artifact_report(&report);
}

#[allow(clippy::too_many_arguments)]
fn assert_parser_backed_syntax_generated_codec_artifact(
    first: &SyntaxGeneratedCodecArtifact,
    second: &SyntaxGeneratedCodecArtifact,
    target: &str,
    grammar_id: &str,
    source_kind: &str,
    metadata_path: &str,
    parser_tool: &str,
    renderer: &str,
    crate_name: &str,
) {
    assert_eq!(first.target, target);
    assert_eq!(first.source, second.source);
    assert_eq!(first.metadata, second.metadata);
    assert_eq!(first.metadata_sha256, second.metadata_sha256);
    assert_eq!(
        first.metadata["artifact_kind"],
        "tct_syntax_generated_codec_reproducibility_metadata"
    );
    assert_eq!(first.metadata["artifact_version"], 1);
    assert_eq!(
        first.metadata["rendered_source"]["sha256"],
        sha256_hex(first.source.as_bytes())
    );
    assert_eq!(
        first.metadata["rendered_source"]["bytes"],
        json!(first.source.len())
    );
    assert_eq!(first.metadata["grammar_input"]["grammar_id"], grammar_id);
    assert_eq!(
        first.metadata["grammar_input"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        first.metadata["structural_model"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        first.metadata["source_metadata_provenance"]["source_kind"],
        source_kind
    );
    assert_eq!(
        first.metadata["source_metadata_provenance"]["metadata_path"],
        metadata_path
    );
    assert_eq!(
        first.metadata["source_metadata_provenance"]["parser"]["tool"],
        parser_tool
    );
    assert_eq!(
        first.metadata["source_metadata_provenance"]["source_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        first.metadata["source_metadata_provenance"]["metadata_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        first.metadata["source_metadata_provenance"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_boundary_manifest_handoff(first, target, source_kind, Some(parser_tool));
    assert_eq!(first.metadata["generated_crate"]["crate_name"], crate_name);
    assert_eq!(first.metadata["generated_crate"]["renderer"], renderer);
    assert_eq!(
        first.metadata["generation_options"]["compiler_path"],
        "sidecar-only"
    );
    let expected_raw_base_vocab_size = match target {
        "go" => 640,
        "csharp" => 576,
        _ => 320,
    };
    assert_eq!(
        first.metadata["token_layout"]["raw_base_vocab_size"],
        json!(expected_raw_base_vocab_size)
    );
    assert_eq!(
        first.metadata["sidecar_boundary"]["generated_json_schema_output_changed"],
        false
    );
    assert_eq!(
        first.metadata["sidecar_boundary"]["core_compiler_path_changed"],
        false
    );
    assert_eq!(
        first.metadata["sidecar_boundary"]["production_grammar_compiler_path_changed"],
        false
    );
    assert_generated_artifact_envelope_handoff(first, target);
    assert_validation_hook_contract_handoff(first, target);
    assert_validation_hook_sidecar_evidence_handoff(first, target);
    assert_generated_artifact_replay_handoff(first, target);
    assert_input_diagnostics_contract_handoff(first, target);
    assert_input_diagnostics_negative_fixture_handoff(first, target);
}

fn assert_target_agnostic_compiler_route_reproducibility_artifact(
    first: &SyntaxGeneratedCodecArtifact,
    second: &SyntaxGeneratedCodecArtifact,
    target: &str,
    grammar_id: &str,
    metadata_path: &str,
    paired_route_evidence_key: &str,
) {
    let route = &first.metadata["target_agnostic_compiler_route"];
    assert_eq!(route, &second.metadata["target_agnostic_compiler_route"]);
    assert_eq!(route["entrypoint"], SYNTAX_COMPILER_ROUTE_ENTRYPOINT);
    assert_eq!(route["delegates_to"], ROUTE_ENTRYPOINT);
    assert_eq!(route["target"], target);
    assert_eq!(route["grammar_id"], grammar_id);
    assert_eq!(
        route["accepted_input"],
        "checked GrammarIR -> StructuralModel"
    );
    assert_eq!(route["output_artifact"], "SyntaxGeneratedCodecArtifact");
    assert_eq!(route["rejects_arena_ir_boundaries"], true);
    assert_eq!(route["generated_json_schema_output_changed"], false);
    assert_eq!(route["core_compiler_path_changed"], false);
    assert_eq!(
        route["completed_promotion_class"],
        SYNTAX_COMPILER_ROUTE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        route["generated_code_gate_completed_promotion_class"],
        SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        route["paired_route_evidence"][paired_route_evidence_key],
        metadata_path
    );
    assert!(
        route["generated_code_gate_representatives"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == grammar_id),
        "missing generated-code route representative {grammar_id}"
    );
    assert!(
        route["parser_backed_reproducibility_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == grammar_id),
        "missing parser-backed reproducibility route evidence {grammar_id}"
    );
}

fn assert_target_agnostic_compiler_route_decode_evidence(
    artifact: &SyntaxGeneratedCodecArtifact,
    grammar_id: &str,
    required_source_markers: &[&str],
) {
    let route = &artifact.metadata["target_agnostic_compiler_route"];
    assert_eq!(route["entrypoint"], SYNTAX_COMPILER_ROUTE_ENTRYPOINT);
    assert_eq!(
        route["strict_recovery_decode_policy"],
        "syntax-parser-backed-recovery-v1"
    );
    assert_eq!(
        route["strict_recovery_decode_completed_promotion_class"],
        SYNTAX_COMPILER_STRICT_RECOVERY_DECODE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        route["consumed_token_accounting_completed_promotion_class"],
        SYNTAX_COMPILER_CONSUMED_TOKEN_ACCOUNTING_COMPLETED_PROMOTION_CLASS
    );
    assert!(
        route["strict_recovery_decode_route_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == grammar_id),
        "missing strict/recovery route evidence {grammar_id}"
    );
    assert!(
        route["consumed_token_accounting_route_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == grammar_id),
        "missing consumed-token route evidence {grammar_id}"
    );
    for accounting_case in [
        "complete_stream",
        "trailing_pad_recovery",
        "eof",
        "non_pad_surplus",
        "malformed_structural_tokens",
        "malformed_payload_tokens",
        "invalid_parser_backed_payloads",
        "partial_prefix_failures",
    ] {
        assert!(
            route["consumed_token_accounting_cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate == accounting_case),
            "missing consumed-token route accounting case {accounting_case}"
        );
    }
    for marker in required_source_markers {
        assert!(
            artifact.source.contains(marker),
            "route-owned artifact for {grammar_id} is missing generated source marker {marker}"
        );
    }
}

fn assert_target_agnostic_compiler_route_token_vocabulary_bpe_evidence(
    artifact: &SyntaxGeneratedCodecArtifact,
    grammar_id: &str,
) {
    let route = &artifact.metadata["target_agnostic_compiler_route"];
    assert_eq!(route["entrypoint"], SYNTAX_COMPILER_ROUTE_ENTRYPOINT);
    assert_eq!(
        route["token_vocabulary_bpe_completed_promotion_class"],
        SYNTAX_COMPILER_TOKEN_VOCABULARY_BPE_COMPLETED_PROMOTION_CLASS
    );
    assert!(
        route["token_vocabulary_bpe_route_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == grammar_id),
        "missing token/vocabulary/BPE route evidence {grammar_id}"
    );
    for invariant_case in [
        "pad_token_below_vocab_size",
        "pad_token_outside_raw_base_ranges",
        "structural_ranges_do_not_overlap_payload_ranges",
        "bpe_range_starts_at_raw_base_vocab_size",
        "bpe_range_below_vocab_size_or_relocates_pad",
        "bpe_ids_do_not_collide_with_pad_token",
        "emitted_tokens_below_vocab_size",
        "emitted_tokens_exclude_pad_token",
    ] {
        assert!(
            route["token_vocabulary_bpe_invariant_cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate == invariant_case),
            "missing token/vocabulary/BPE route invariant case {invariant_case}"
        );
    }

    let layout = &artifact.metadata["token_layout"];
    let vocab_size = layout["vocab_size"].as_u64().unwrap();
    let pad_token = layout["pad_token"].as_u64().unwrap();
    let raw_base_vocab_size = layout["raw_base_vocab_size"].as_u64().unwrap();
    let expected_raw_base_vocab_size = if grammar_id.starts_with("go_") {
        640
    } else {
        576
    };
    assert_eq!(raw_base_vocab_size, expected_raw_base_vocab_size);
    assert!(pad_token < vocab_size);
    assert!(pad_token >= raw_base_vocab_size);
    assert_eq!(
        layout["raw_base_vocab_size"],
        json!(expected_raw_base_vocab_size)
    );
    assert!(
        layout["structural_ranges"]
            .as_array()
            .expect("structural_ranges should be an array")
            .iter()
            .all(|range| range["end_exclusive"].as_u64().unwrap() <= raw_base_vocab_size),
        "structural ranges must stay below PAD_TOKEN"
    );
    assert!(
        layout["payload_ranges"]
            .as_array()
            .expect("payload_ranges should be an array")
            .iter()
            .all(|range| range["end_exclusive"].as_u64().unwrap() <= raw_base_vocab_size),
        "payload ranges must stay below PAD_TOKEN"
    );
    assert_eq!(
        layout["bpe_boundary"]["raw_base_vocab_size"],
        layout["raw_base_vocab_size"]
    );
    assert_eq!(
        layout["bpe_boundary"]["bpe_start"],
        json!(expected_raw_base_vocab_size)
    );
    assert_eq!(
        layout["bpe_boundary"]["bpe_end_exclusive"],
        json!(expected_raw_base_vocab_size)
    );
    assert_eq!(layout["bpe_boundary"]["bpe_merges"], json!(0));
    assert_eq!(layout["bpe_boundary"]["pad_token"], layout["pad_token"]);
    assert_eq!(layout["bpe_boundary"]["vocab_size"], layout["vocab_size"]);
    assert!(
        layout["bpe_boundary"]["bpe_end_exclusive"]
            .as_u64()
            .unwrap()
            <= vocab_size
    );
    assert_eq!(
        layout["bpe_boundary"]["current_layout_hosts_bpe"],
        json!(true)
    );
    let vocab_marker = format!("pub const VOCAB_SIZE: u32 = {vocab_size};");
    let pad_marker = format!("pub const PAD_TOKEN: u32 = {pad_token};");
    for source_marker in [
        vocab_marker.as_str(),
        pad_marker.as_str(),
        "fn validate_tokens(tokens: &[u32]) -> Result<(), SyntaxCodecError>",
        "collides with PAD or exceeds VOCAB_SIZE",
    ] {
        assert!(
            artifact.source.contains(source_marker),
            "route-owned token evidence for {grammar_id} is missing source marker {source_marker}"
        );
    }
}

fn assert_target_agnostic_compiler_route_state_api_evidence(
    artifact: &SyntaxGeneratedCodecArtifact,
) {
    let route = &artifact.metadata["target_agnostic_compiler_route"];
    assert_eq!(route["entrypoint"], SYNTAX_COMPILER_ROUTE_ENTRYPOINT);
    let state_api = &route["generated_codec_state_api_route"];
    assert_eq!(
        state_api["completed_promotion_class"],
        SYNTAX_COMPILER_STATE_API_ROUTE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        state_api["next_missing_stable_promotion_class"],
        SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(
        state_api["source_integration_fixture"],
        "tests/fixtures/syntax_backend/generated-codec-state-api-integration-v1.json"
    );
    assert_eq!(state_api["record_query_count"], json!(10));
    assert!(
        state_api["route_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == "csharp_compilation_unit_source_file_syntax_v0")
    );
    assert_eq!(
        state_api["multi_token_bpe_raw_expansion_runtime_replay_ready"],
        json!(true)
    );
    assert_eq!(
        state_api["multi_token_bpe_raw_expansion_replay_case_count"],
        json!(12)
    );
    assert_eq!(
        state_api["next_generated_codec_state_api_work"],
        SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    let mask_contract = &state_api["admissible_token_mask_contract"];
    assert_eq!(
        mask_contract["completed_promotion_class"],
        SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        mask_contract["next_missing_stable_promotion_class"],
        SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(mask_contract["record_query_count"], json!(10));
    assert_eq!(mask_contract["finite_mask_record_count"], json!(10));
    assert_eq!(mask_contract["payload_range_mask_record_count"], json!(10));
    assert_eq!(mask_contract["eof_validity_record_count"], json!(10));
    assert_eq!(
        mask_contract["bpe_raw_expansion_replay_case_count"],
        json!(12)
    );
    assert_eq!(
        mask_contract["bpe_raw_expansion_accepted_case_count"],
        json!(6)
    );
    assert_eq!(
        mask_contract["bpe_raw_expansion_first_failure_case_count"],
        json!(6)
    );
    assert_eq!(
        mask_contract["bpe_ids_used_as_syntax_validity_authority"],
        json!(false)
    );
    let continuation_contract = &state_api["prefix_state_continuation_contract"];
    assert_eq!(
        continuation_contract["completed_promotion_class"],
        SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_COMPLETED_PROMOTION_CLASS
    );
    assert_eq!(
        continuation_contract["next_missing_stable_promotion_class"],
        SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS
    );
    assert_eq!(continuation_contract["record_query_count"], json!(10));
    assert_eq!(continuation_contract["raw_prefix_request_count"], json!(40));
    assert_eq!(
        continuation_contract["finite_admissible_response_record_count"],
        json!(10)
    );
    assert_eq!(
        continuation_contract["payload_range_response_record_count"],
        json!(10)
    );
    assert_eq!(
        continuation_contract["eof_validity_response_record_count"],
        json!(10)
    );
    assert_eq!(
        continuation_contract["bpe_raw_expansion_replay_case_count"],
        json!(12)
    );
    assert_eq!(
        continuation_contract["bpe_raw_expansion_first_failure_case_count"],
        json!(6)
    );
    assert_eq!(
        continuation_contract["response_evidence_fixture"],
        SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_RESPONSE_EVIDENCE_FIXTURE
    );
    assert_eq!(
        continuation_contract["response_evidence_record_count"],
        json!(10)
    );
    assert_eq!(
        continuation_contract["response_evidence_raw_prefix_response_count"],
        json!(40)
    );
    assert_eq!(
        continuation_contract["response_evidence_bpe_replay_response_count"],
        json!(12)
    );
    assert_eq!(
        continuation_contract["response_evidence_checked_against_target_neutral_api"],
        json!(true)
    );
    assert_eq!(
        continuation_contract["consumes_route_owned_admissible_mask_contract"],
        json!(true)
    );
    assert_eq!(
        continuation_contract["requires_consumed_raw_token_count"],
        json!(true)
    );
    assert_eq!(
        continuation_contract["requires_first_failure_diagnostics"],
        json!(true)
    );
    assert_eq!(
        continuation_contract["bpe_ids_used_as_syntax_validity_authority"],
        json!(false)
    );
    assert_eq!(state_api["arena_ir_proof_used"], json!(false));
    assert_eq!(
        state_api["state_api_stable_backend_promotion_allowed"],
        json!(false)
    );
}

fn assert_boundary_manifest_handoff(
    artifact: &SyntaxGeneratedCodecArtifact,
    target: &str,
    source_kind: &str,
    parser_tool: Option<&str>,
) {
    let manifest = syntax_structural_model_boundary_manifest();
    let handoff = &artifact.metadata["boundary_manifest"];

    assert_eq!(handoff["path"], STRUCTURAL_MODEL_BOUNDARY_MANIFEST_PATH);
    assert_eq!(
        handoff["artifact_kind"],
        "tct_syntax_structural_model_boundary_manifest"
    );
    assert_eq!(handoff["artifact_version"], 1);
    assert_eq!(
        handoff["sha256"],
        json_sha256(&manifest).expect("failed to hash syntax StructuralModel manifest")
    );
    assert_eq!(handoff["sidecar_boundary"], "GrammarIR -> StructuralModel");
    assert_eq!(handoff["target_ids"], json!(["toy", "csharp", "go"]));
    assert_eq!(handoff["compiler_boundary_claim"]["sidecar_only"], true);
    assert_eq!(
        handoff["compiler_boundary_claim"]["generated_rust_source_substrings_used"],
        false
    );
    assert_eq!(
        handoff["compiler_boundary_claim"]["tct_compiler_changed"],
        false
    );
    assert_eq!(
        handoff["compiler_boundary_claim"]["generated_json_schema_output_changed"],
        false
    );
    assert_eq!(handoff["target"]["target"], target);
    assert_eq!(handoff["target"]["source_kind"], source_kind);
    assert_eq!(handoff["target"]["model_version"], "structural-model-v0");
    assert_eq!(
        handoff["target"]["boundary"]["role"],
        "sidecar_evidence_only"
    );
    assert_eq!(
        handoff["target"]["boundary"]["core_compiler_path_changed"],
        false
    );
    assert_eq!(
        handoff["target"]["boundary"]["production_grammar_compiler_path_changed"],
        false
    );
    assert_eq!(
        handoff["target"]["target_manifest_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    match parser_tool {
        Some(tool) => assert_eq!(handoff["target"]["parser"]["tool"], tool),
        None => assert!(handoff["target"]["parser"].is_null()),
    }
}

fn assert_generated_artifact_envelope_handoff(
    artifact: &SyntaxGeneratedCodecArtifact,
    target: &str,
) {
    let envelope = syntax_backend_generated_artifact_envelope();
    let handoff = &artifact.metadata["generated_artifact_envelope"];

    assert_eq!(handoff["path"], GENERATED_ARTIFACT_ENVELOPE_PATH);
    assert_eq!(handoff["artifact_kind"], envelope.artifact_kind);
    assert_eq!(handoff["artifact_version"], envelope.version);
    assert_eq!(handoff["api_version"], envelope.api_version);
    assert_eq!(
        handoff["fixture_sha256"],
        sha256_hex(GENERATED_ARTIFACT_ENVELOPE_FIXTURE.as_bytes())
    );
    assert_eq!(
        handoff["canonical_sha256"],
        json_sha256(&envelope).expect("failed to hash syntax generated artifact envelope")
    );
    assert_eq!(handoff["target"], target);
    assert_eq!(handoff["target_ids"], json!(["toy", "csharp", "go"]));
    assert_eq!(
        handoff["source_api_boundary"]["module_path"],
        "src/syntax_backend_boundary.rs"
    );
    assert_eq!(
        handoff["source_api_boundary"]["contract_fixture"],
        "tests/fixtures/syntax_backend/production-boundary-contract-v1.json"
    );
    assert_eq!(
        handoff["metadata_surface"],
        "generated_codec_artifact_metadata"
    );
    for required in [
        "artifact_kind",
        "version",
        "api_version",
        "source_contract",
        "compiler_boundary",
        "inputs",
        "outputs",
        "generated_codec_artifact_metadata",
        "rendered_source_sha256",
        "generated_crate_identity",
        "target_manifest_sha256",
        "generation_options_sha256",
        "source_metadata_provenance",
        "structural_model",
        "token_layout",
        "sidecar_boundary",
        "validation_hook_contract",
        "validation_hook_contract_sha256",
        "validation_hook_sidecar_evidence",
        "validation_hook_sidecar_evidence_sha256",
        "generated_artifact_replay",
        "generated_artifact_replay_sha256",
        "input_diagnostics_contract",
        "input_diagnostics_contract_sha256",
        "input_diagnostics_negative_fixture",
        "input_diagnostics_negative_fixture_sha256",
    ] {
        assert!(
            handoff["metadata_required_fields"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field.as_str() == Some(required)),
            "missing envelope metadata field {required}"
        );
    }
    for required in [
        "rendered_source_sha256",
        "boundary_manifest_sha256",
        "target_manifest_sha256",
        "generation_options_sha256",
        "source_metadata_provenance_sha256",
        "structural_model_sha256",
        "token_layout_sha256",
    ] {
        assert!(
            handoff["provenance_hashes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field.as_str() == Some(required)),
            "missing envelope provenance hash {required}"
        );
    }
    assert_eq!(handoff["persistence_policy"]["checked_in"], false);
    assert_eq!(
        handoff["persistence_policy"]["volatile_report_dir"],
        "target/tct-syntax-generated-codec-reproducibility"
    );
    assert_eq!(
        handoff["persistence_policy"]["promotion_required_for_checked_in_artifacts"],
        true
    );
    assert_eq!(
        handoff["decode_policy"]["strict_decode_authoritative"],
        true
    );
    assert_eq!(
        handoff["decode_policy"]["recovery_normalizes_trailing_pad_only"],
        true
    );
    assert_eq!(
        handoff["decode_policy"]["partial_prefix_failures_are_hard"],
        true
    );
    assert_eq!(
        handoff["decode_policy"]["consumed_token_accounting_required"],
        true
    );
    assert_eq!(
        handoff["token_layout_metadata"]["boundary_manifest_path"],
        STRUCTURAL_MODEL_BOUNDARY_MANIFEST_PATH
    );
    assert_eq!(
        handoff["token_layout_metadata"]["requires_bpe_collision_evidence"],
        true
    );
    assert_eq!(handoff["compiler_boundary"]["sidecar_only"], true);
    assert_eq!(
        handoff["compiler_boundary"]["tct_compiler_route_allowed"],
        false
    );
    assert_eq!(
        handoff["compiler_boundary"]["generated_json_schema_output_changed"],
        false
    );
    assert_eq!(
        handoff["compiler_boundary"]["core_compiler_path_changed"],
        false
    );
    assert_eq!(
        handoff["compiler_boundary"]["production_grammar_compiler_path_changed"],
        false
    );
    assert_eq!(artifact.metadata["api_version"], "syntax-generator-api-v0");
    assert_eq!(
        artifact.metadata["source_contract"]["readiness_report"],
        "reports/m6-cross-target-contract-closure-2026-07-21.md"
    );
    assert_eq!(artifact.metadata["compiler_boundary"]["sidecar_only"], true);
    assert_eq!(
        artifact.metadata["inputs"]
            .as_array()
            .expect("metadata inputs must be an array")
            .len(),
        3
    );
    assert_eq!(
        artifact.metadata["outputs"]
            .as_array()
            .expect("metadata outputs must be an array")
            .len(),
        4
    );
    assert_eq!(
        artifact.metadata["rendered_source_sha256"],
        artifact.metadata["rendered_source"]["sha256"]
    );
    assert_eq!(
        artifact.metadata["boundary_manifest_sha256"],
        artifact.metadata["boundary_manifest"]["sha256"]
    );
    assert_eq!(
        artifact.metadata["target_manifest_sha256"],
        artifact.metadata["boundary_manifest"]["target"]["target_manifest_sha256"]
    );
    assert_eq!(
        artifact.metadata["source_metadata_provenance_sha256"],
        artifact.metadata["source_metadata_provenance"]["sha256"]
    );
    assert_eq!(
        artifact.metadata["structural_model_sha256"],
        artifact.metadata["structural_model"]["sha256"]
    );
    assert_eq!(
        artifact.metadata["token_layout_sha256"],
        artifact.metadata["token_layout"]["sha256"]
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["envelope_path"],
        GENERATED_ARTIFACT_ENVELOPE_PATH
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["envelope_fixture_sha256"],
        handoff["fixture_sha256"]
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["target"],
        target
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["persistence_policy"]["checked_in"],
        false
    );
    assert_eq!(
        artifact.metadata["token_layout"]["pad_token"],
        artifact.metadata["token_layout"]["bpe_boundary"]["pad_token"]
    );
    assert_eq!(
        artifact.metadata["token_layout"]["vocab_size"],
        artifact.metadata["token_layout"]["bpe_boundary"]["vocab_size"]
    );
    assert!(
        artifact.metadata["token_layout"]["structural_ranges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|range| range["role"] == "structural_decisions")
    );
    assert!(
        artifact.metadata["token_layout"]["payload_ranges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|range| range["role"] != "structural_decisions")
    );
}

fn assert_validation_hook_contract_handoff(artifact: &SyntaxGeneratedCodecArtifact, target: &str) {
    let envelope = syntax_backend_generated_artifact_envelope();
    let contract = syntax_backend_validation_hook_contract(&envelope);
    let handoff = &artifact.metadata["validation_hook_contract"];

    assert_eq!(handoff["path"], VALIDATION_HOOK_CONTRACT_PATH);
    assert_eq!(handoff["artifact_kind"], contract.artifact_kind);
    assert_eq!(handoff["artifact_version"], contract.version);
    assert_eq!(handoff["api_version"], contract.api_version);
    assert_eq!(
        handoff["fixture_sha256"],
        sha256_hex(VALIDATION_HOOK_CONTRACT_FIXTURE.as_bytes())
    );
    assert_eq!(
        handoff["canonical_sha256"],
        json_sha256(&contract).expect("failed to hash syntax validation hook contract")
    );
    assert_eq!(
        artifact.metadata["validation_hook_contract_sha256"],
        handoff["canonical_sha256"]
    );
    assert_eq!(
        handoff["source_artifact_envelope"]["fixture_path"],
        GENERATED_ARTIFACT_ENVELOPE_PATH
    );
    assert_eq!(handoff["target"], target);
    assert_eq!(handoff["target_ids"], json!(["toy", "csharp", "go"]));
    for surface in [
        "source_reparse_or_equivalent_check",
        "renderer_or_formatter_sidecar_status",
        "external_check_non_promotion_policy",
        "validation_hook_provenance",
    ] {
        assert!(
            handoff["validation_surfaces"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate.as_str() == Some(surface)),
            "missing validation hook handoff surface {surface}"
        );
    }
    assert_eq!(
        handoff["source_reparse_or_equivalent_check"]["required"],
        true
    );
    assert_eq!(
        handoff["source_reparse_or_equivalent_check"]["target"],
        target
    );
    assert!(
        !handoff["source_reparse_or_equivalent_check"]["check_kind"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert!(
        !handoff["source_reparse_or_equivalent_check"]["sidecar_evidence"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        handoff["renderer_formatter_status"]["contract"]["renderer_metadata_required"],
        true
    );
    assert_eq!(
        handoff["renderer_formatter_status"]["contract"]["formatter_status_sidecar_only"],
        true
    );
    assert_eq!(
        handoff["renderer_formatter_status"]["contract"]["external_formatter_stable_gate_promoted"],
        false
    );
    assert_eq!(
        handoff["renderer_formatter_status"]["contract"]["generated_hot_path_external_checks_allowed"],
        false
    );
    assert!(
        !handoff["renderer_formatter_status"]["renderer_metadata"]["id"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        handoff["external_check_policy"]["policy"]["stable_gate_promoted"],
        false
    );
    assert_eq!(
        handoff["external_check_policy"]["policy"]["sidecar_only_until_explicit_promotion"],
        true
    );
    assert_eq!(
        handoff["external_check_policy"]["policy"]["disallowed_in_generated_hot_path"],
        true
    );
    for check in [
        "parser",
        "renderer",
        "formatter",
        "compiler",
        "functional_validator",
    ] {
        assert!(
            handoff["external_check_policy"]["policy"]["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate.as_str() == Some(check)),
            "missing external check policy entry {check}"
        );
    }
    let provenance = &handoff["validation_hook_provenance"];
    assert_eq!(provenance["target_id"], target);
    assert!(
        !provenance["source_or_metadata_sha256"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    if target == "toy" {
        assert_eq!(provenance["parser_or_grammar_identity"]["kind"], "grammar");
    } else {
        assert_eq!(provenance["parser_or_grammar_identity"]["kind"], "parser");
    }
    assert!(
        !provenance["renderer_metadata_id"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    for required in [
        "target_id",
        "source_or_metadata_sha256",
        "parser_or_grammar_identity",
        "renderer_metadata_id",
        "validation_hook_ids",
        "external_check_policy",
        "sidecar_boundary_sha256",
    ] {
        assert!(
            provenance["required_fields"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field.as_str() == Some(required)),
            "missing validation hook provenance field {required}"
        );
    }
    for hook_id in [
        "grammar_ir.required_fields",
        "toy_grammar_frontend.strict_decode",
        "grammar_structural_token_ranges",
    ] {
        assert!(
            provenance["validation_hook_ids"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate.as_str() == Some(hook_id)),
            "missing validation hook id {hook_id}"
        );
    }
    assert!(
        provenance["validation_hooks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|hook| hook["runs_in_generated_hot_path"] == false)
    );
    assert_eq!(handoff["compiler_boundary"]["sidecar_only"], true);
    assert_eq!(
        handoff["compiler_boundary"]["tct_compiler_route_allowed"],
        false
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["validation_hook_contract_path"],
        VALIDATION_HOOK_CONTRACT_PATH
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["validation_hook_contract_fixture_sha256"],
        handoff["fixture_sha256"]
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["validation_hook_contract"]["target"],
        target
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["validation_hook_contract"]["external_check_policy"]
            ["policy"]["stable_gate_promoted"],
        false
    );
}

fn assert_validation_hook_sidecar_evidence_handoff(
    artifact: &SyntaxGeneratedCodecArtifact,
    target: &str,
) {
    let envelope = syntax_backend_generated_artifact_envelope();
    let contract = syntax_backend_validation_hook_contract(&envelope);
    let evidence = syntax_backend_validation_hook_sidecar_evidence(&contract);
    let handoff = &artifact.metadata["validation_hook_sidecar_evidence"];

    assert_eq!(handoff["path"], VALIDATION_HOOK_SIDECAR_EVIDENCE_PATH);
    assert_eq!(handoff["artifact_kind"], evidence.artifact_kind);
    assert_eq!(handoff["artifact_version"], evidence.version);
    assert_eq!(handoff["api_version"], evidence.api_version);
    assert_eq!(
        handoff["fixture_sha256"],
        sha256_hex(VALIDATION_HOOK_SIDECAR_EVIDENCE_FIXTURE.as_bytes())
    );
    assert_eq!(
        handoff["canonical_sha256"],
        json_sha256(&evidence).expect("failed to hash validation hook sidecar evidence")
    );
    assert_eq!(
        artifact.metadata["validation_hook_sidecar_evidence_sha256"],
        handoff["canonical_sha256"]
    );
    assert_eq!(
        handoff["source_validation_hook_contract"]["fixture_path"],
        VALIDATION_HOOK_CONTRACT_PATH
    );
    assert_eq!(
        handoff["validation_hook_contract_sha256"],
        artifact.metadata["validation_hook_contract_sha256"]
    );
    assert_eq!(handoff["target"], target);
    assert_eq!(handoff["target_ids"], json!(["toy", "csharp", "go"]));

    let record = &handoff["record"];
    assert_eq!(record["target_id"], target);
    assert_eq!(
        record["source_or_metadata_sha256"],
        artifact.metadata["validation_hook_contract"]["validation_hook_provenance"]["source_or_metadata_sha256"]
    );
    if target == "toy" {
        assert_eq!(record["parser_or_grammar_identity"]["kind"], "grammar");
    } else {
        assert_eq!(record["parser_or_grammar_identity"]["kind"], "parser");
        assert!(
            record["parser_or_grammar_identity"]["package"]
                .as_str()
                .is_some()
        );
        assert!(
            record["parser_or_grammar_identity"]["package_version"]
                .as_str()
                .is_some()
        );
    }
    assert_eq!(
        record["renderer_metadata_id"],
        artifact.metadata["validation_hook_contract"]["validation_hook_provenance"]["renderer_metadata_id"]
    );
    assert_eq!(
        record["renderer_formatter_status"]["external_formatter_stable_gate_promoted"],
        false
    );
    assert_eq!(
        record["renderer_formatter_status"]["generated_hot_path_external_checks_allowed"],
        false
    );
    for hook_id in [
        "grammar_ir.required_fields",
        "toy_grammar_frontend.strict_decode",
        "grammar_structural_token_ranges",
    ] {
        assert!(
            record["validation_hook_ids"]
                .as_array()
                .expect("validation hook ids must be an array")
                .iter()
                .any(|candidate| candidate.as_str() == Some(hook_id)),
            "missing validation hook sidecar id {hook_id}"
        );
    }
    for hash_field in [
        "validation_hook_ids_sha256",
        "external_check_policy_sha256",
        "validation_hook_contract_sha256",
        "sidecar_boundary_sha256",
    ] {
        assert_eq!(
            record[hash_field].as_str().unwrap().len(),
            64,
            "invalid validation hook sidecar hash field {hash_field}"
        );
    }
    assert_eq!(
        record["external_check_policy_sha256"],
        artifact.metadata["validation_hook_contract"]["external_check_policy"]["sha256"]
    );
    assert_eq!(
        record["validation_hook_contract_sha256"],
        artifact.metadata["validation_hook_contract_sha256"]
    );
    assert_eq!(
        record["sidecar_boundary"]["role"],
        artifact.metadata["boundary_manifest"]["target"]["boundary"]["role"]
    );
    assert_eq!(
        record["sidecar_boundary"]["generated_json_schema_output_changed"],
        false
    );
    assert_eq!(handoff["sidecar_status"]["sidecar_only"], true);
    assert_eq!(handoff["sidecar_status"]["stable_gate_promoted"], false);
    assert_eq!(handoff["sidecar_status"]["live_toolchain_required"], false);
    assert_eq!(
        handoff["sidecar_status"]["generated_hot_path_external_checks_allowed"],
        false
    );
    assert_eq!(handoff["compiler_boundary"]["sidecar_only"], true);
    assert_eq!(
        handoff["compiler_boundary"]["tct_compiler_route_allowed"],
        false
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["validation_hook_sidecar_evidence_path"],
        VALIDATION_HOOK_SIDECAR_EVIDENCE_PATH
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["validation_hook_sidecar_evidence_fixture_sha256"],
        handoff["fixture_sha256"]
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["validation_hook_sidecar_evidence"]
            ["target"],
        target
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["validation_hook_sidecar_evidence"]
            ["record"],
        handoff["record"]
    );
}

fn assert_generated_artifact_replay_handoff(artifact: &SyntaxGeneratedCodecArtifact, target: &str) {
    let envelope = syntax_backend_generated_artifact_envelope();
    let validation_hook_contract = syntax_backend_validation_hook_contract(&envelope);
    let sidecar_evidence =
        syntax_backend_validation_hook_sidecar_evidence(&validation_hook_contract);
    let input_diagnostics_contract =
        syntax_backend_input_diagnostics_contract(&validation_hook_contract);
    let input_diagnostics_negative_fixture =
        syntax_backend_input_diagnostics_negative_fixture(&input_diagnostics_contract);
    let replay = syntax_backend_generated_artifact_replay(
        &envelope,
        &sidecar_evidence,
        &input_diagnostics_contract,
        &input_diagnostics_negative_fixture,
    );
    let handoff = &artifact.metadata["generated_artifact_replay"];

    assert_eq!(handoff["path"], GENERATED_ARTIFACT_REPLAY_PATH);
    assert_eq!(handoff["artifact_kind"], replay.artifact_kind);
    assert_eq!(handoff["artifact_version"], replay.version);
    assert_eq!(
        handoff["fixture_sha256"],
        sha256_hex(GENERATED_ARTIFACT_REPLAY_FIXTURE.as_bytes())
    );
    assert_eq!(
        handoff["canonical_sha256"],
        json_sha256(&replay).expect("failed to hash generated artifact replay fixture")
    );
    assert_eq!(
        artifact.metadata["generated_artifact_replay_sha256"],
        handoff["canonical_sha256"]
    );
    assert_eq!(handoff["target"], target);
    assert_eq!(handoff["target_ids"], json!(["toy", "csharp", "go"]));
    assert_eq!(
        handoff["source_generated_artifact_envelope"]["fixture_path"],
        GENERATED_ARTIFACT_ENVELOPE_PATH
    );
    assert_eq!(
        handoff["source_validation_hook_sidecar_evidence"]["fixture_path"],
        VALIDATION_HOOK_SIDECAR_EVIDENCE_PATH
    );
    assert_eq!(
        handoff["source_input_diagnostics_negative_fixture"]["fixture_path"],
        INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE_PATH
    );
    let record = &handoff["record"];
    assert_eq!(record["target_id"], target);
    assert_eq!(
        record["rendered_source_sha256"],
        artifact.metadata["rendered_source_sha256"]
    );
    assert_eq!(
        record["source_or_metadata_sha256"],
        artifact.metadata["validation_hook_contract"]["validation_hook_provenance"]["source_or_metadata_sha256"]
    );
    assert_eq!(
        record["parser_or_grammar_identity"],
        artifact.metadata["validation_hook_contract"]["validation_hook_provenance"]["parser_or_grammar_identity"]
    );
    assert_eq!(
        record["grammar_input_sha256"],
        artifact.metadata["grammar_input"]["sha256"]
    );
    assert_eq!(
        record["source_metadata_provenance_sha256"],
        artifact.metadata["source_metadata_provenance_sha256"]
    );
    assert_eq!(
        record["structural_model_sha256"],
        artifact.metadata["structural_model_sha256"]
    );
    assert_eq!(
        record["token_layout_sha256"],
        artifact.metadata["token_layout_sha256"]
    );
    assert_eq!(
        record["generated_crate_identity_sha256"],
        artifact.metadata["generated_crate_identity"]["sha256"]
    );
    assert_eq!(
        record["generation_options_sha256"],
        artifact.metadata["generation_options_sha256"]
    );
    assert_eq!(
        record["generated_artifact_envelope_sha256"],
        artifact.metadata["generated_artifact_envelope"]["canonical_sha256"]
    );
    assert_eq!(
        record["validation_hook_sidecar_evidence_sha256"],
        artifact.metadata["validation_hook_sidecar_evidence_sha256"]
    );
    assert_eq!(
        record["input_diagnostics_contract_sha256"],
        artifact.metadata["input_diagnostics_contract_sha256"]
    );
    assert_eq!(
        record["input_diagnostics_negative_fixture_sha256"],
        artifact.metadata["input_diagnostics_negative_fixture_sha256"]
    );
    assert_eq!(
        record["boundary_manifest_sha256"],
        artifact.metadata["boundary_manifest_sha256"]
    );
    assert_eq!(
        record["target_manifest_sha256"],
        artifact.metadata["target_manifest_sha256"]
    );
    assert!(
        record["compiler_boundary_status"]["syntax_frontend_only"]
            .as_bool()
            .unwrap()
    );
    assert!(
        !record["compiler_boundary_status"]["stable_tct_backend_route"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        handoff["volatile_report_policy"]["checked_in_reports"],
        false
    );
    assert!(
        handoff["deterministic_hash_fields"]
            .as_array()
            .expect("replay deterministic hash fields must be an array")
            .iter()
            .any(|field| field == "validation_hook_sidecar_evidence_sha256")
    );
    assert_eq!(handoff["compiler_boundary"]["sidecar_only"], true);
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["generated_artifact_replay_path"],
        GENERATED_ARTIFACT_REPLAY_PATH
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["generated_artifact_replay_fixture_sha256"],
        handoff["fixture_sha256"]
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["generated_artifact_replay"]["record"],
        handoff["record"]
    );
}

fn assert_input_diagnostics_contract_handoff(
    artifact: &SyntaxGeneratedCodecArtifact,
    target: &str,
) {
    let envelope = syntax_backend_generated_artifact_envelope();
    let validation_hook_contract = syntax_backend_validation_hook_contract(&envelope);
    let contract = syntax_backend_input_diagnostics_contract(&validation_hook_contract);
    let handoff = &artifact.metadata["input_diagnostics_contract"];

    assert_eq!(handoff["path"], INPUT_DIAGNOSTICS_CONTRACT_PATH);
    assert_eq!(handoff["artifact_kind"], contract.artifact_kind);
    assert_eq!(handoff["artifact_version"], contract.version);
    assert_eq!(handoff["api_version"], contract.api_version);
    assert_eq!(
        handoff["fixture_sha256"],
        sha256_hex(INPUT_DIAGNOSTICS_CONTRACT_FIXTURE.as_bytes())
    );
    assert_eq!(
        handoff["canonical_sha256"],
        json_sha256(&contract).expect("failed to hash input diagnostics contract")
    );
    assert_eq!(
        artifact.metadata["input_diagnostics_contract_sha256"],
        handoff["canonical_sha256"]
    );
    assert_eq!(
        handoff["source_validation_hook_contract"]["fixture_path"],
        VALIDATION_HOOK_CONTRACT_PATH
    );
    assert_eq!(handoff["target"], target);
    assert_eq!(handoff["target_ids"], json!(["toy", "csharp", "go"]));
    for field in [
        "target_id",
        "source_or_metadata_sha256",
        "parser_or_grammar_identity",
        "diagnostics",
        "unsupported_syntax_errors",
        "sidecar_boundary",
    ] {
        assert!(
            handoff["input_surface"]["contract"]["required_fields"]
                .as_array()
                .expect("input diagnostics required fields must be an array")
                .iter()
                .any(|required| required == field),
            "missing input diagnostics required field {field}"
        );
    }
    assert_eq!(handoff["input_surface"]["target_id"], target);
    assert!(
        !handoff["input_surface"]["source_or_metadata_sha256"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert!(
        ["parser", "grammar"].contains(
            &handoff["input_surface"]["parser_or_grammar_identity"]["kind"]
                .as_str()
                .unwrap()
        )
    );
    for field in [
        "severity",
        "code",
        "message",
        "rule_path",
        "source_path",
        "span",
    ] {
        assert!(
            handoff["diagnostic_shape"]["required_fields"]
                .as_array()
                .expect("input diagnostics fields must be an array")
                .iter()
                .any(|required| required == field),
            "missing diagnostic field {field}"
        );
    }
    assert!(
        handoff["diagnostic_shape"]["required_severities"]
            .as_array()
            .expect("required severities must be an array")
            .iter()
            .any(|severity| severity == "error")
    );
    assert!(
        handoff["diagnostic_shape"]["required_severities"]
            .as_array()
            .expect("required severities must be an array")
            .iter()
            .any(|severity| severity == "warning")
    );
    assert_eq!(
        handoff["diagnostic_shape"]["rule_path_format"],
        "grammar_rule_or_parser_node_path"
    );
    assert_eq!(
        handoff["diagnostic_shape"]["span_policy"],
        "optional_when_source_offsets_unavailable"
    );
    assert_eq!(handoff["unsupported_syntax_error_shape"]["required"], true);
    assert_eq!(
        handoff["unsupported_syntax_error_shape"]["requires_rule_path"],
        true
    );
    assert_eq!(
        handoff["unsupported_syntax_error_shape"]["stable_gate_promoted"],
        false
    );
    for field in [
        "target_id",
        "unsupported_kind",
        "rule_path",
        "message",
        "source_or_metadata_sha256",
        "parser_or_grammar_identity",
    ] {
        assert!(
            handoff["unsupported_syntax_error_shape"]["fields"]
                .as_array()
                .expect("unsupported syntax fields must be an array")
                .iter()
                .any(|required| required == field),
            "missing unsupported syntax field {field}"
        );
    }
    let provenance = &handoff["diagnostics_provenance"];
    for field in [
        "target_id",
        "source_or_metadata_sha256",
        "parser_or_grammar_identity",
        "sidecar_boundary_sha256",
        "diagnostic_contract_sha256",
    ] {
        assert!(
            provenance["required_fields"]
                .as_array()
                .expect("diagnostics provenance fields must be an array")
                .iter()
                .any(|required| required == field),
            "missing diagnostics provenance field {field}"
        );
    }
    assert_eq!(provenance["target_id"], target);
    assert_eq!(
        provenance["diagnostic_contract_sha256"],
        handoff["canonical_sha256"]
    );
    assert_eq!(handoff["sidecar_status"]["sidecar_only"], true);
    assert_eq!(handoff["sidecar_status"]["stable_gate_promoted"], false);
    assert_eq!(handoff["sidecar_status"]["live_toolchain_required"], false);
    assert_eq!(
        handoff["sidecar_status"]["generated_hot_path_external_checks_allowed"],
        false
    );
    assert_eq!(handoff["compiler_boundary"]["sidecar_only"], true);
    assert_eq!(
        handoff["compiler_boundary"]["tct_compiler_route_allowed"],
        false
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_contract_path"],
        INPUT_DIAGNOSTICS_CONTRACT_PATH
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_contract_fixture_sha256"],
        handoff["fixture_sha256"]
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_contract"]["target"],
        target
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_contract"]["diagnostics_provenance"],
        handoff["diagnostics_provenance"]
    );
}

fn assert_input_diagnostics_negative_fixture_handoff(
    artifact: &SyntaxGeneratedCodecArtifact,
    target: &str,
) {
    let envelope = syntax_backend_generated_artifact_envelope();
    let validation_hook_contract = syntax_backend_validation_hook_contract(&envelope);
    let contract = syntax_backend_input_diagnostics_contract(&validation_hook_contract);
    let fixture = syntax_backend_input_diagnostics_negative_fixture(&contract);
    let handoff = &artifact.metadata["input_diagnostics_negative_fixture"];

    assert_eq!(handoff["path"], INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE_PATH);
    assert_eq!(handoff["artifact_kind"], fixture.artifact_kind);
    assert_eq!(handoff["artifact_version"], fixture.version);
    assert_eq!(handoff["api_version"], fixture.api_version);
    assert_eq!(
        handoff["fixture_sha256"],
        sha256_hex(INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE.as_bytes())
    );
    assert_eq!(
        handoff["canonical_sha256"],
        json_sha256(&fixture).expect("failed to hash input diagnostics negative fixture")
    );
    assert_eq!(
        artifact.metadata["input_diagnostics_negative_fixture_sha256"],
        handoff["canonical_sha256"]
    );
    assert_eq!(
        handoff["source_input_diagnostics_contract"]["fixture_path"],
        INPUT_DIAGNOSTICS_CONTRACT_PATH
    );
    assert_eq!(
        handoff["source_input_diagnostics_contract_sha256"],
        artifact.metadata["input_diagnostics_contract_sha256"]
    );
    assert_eq!(handoff["target"], target);
    assert_eq!(
        handoff["fixture_target_ids"],
        json!(["toy", "csharp", "go"])
    );
    assert_eq!(
        handoff["applies_to_contract_target_ids"],
        json!(["toy", "csharp", "go"])
    );
    assert_eq!(handoff["coverage_scope"], "direct_fixture_target");
    for fixture_id in [
        "toy-missing-expression",
        "toy-unsupported-call-expression",
        "csharp-empty-field-alternatives",
        "csharp-unsupported-pointer-type",
        "go-empty-field-alternatives",
        "go-unsupported-gostmt",
    ] {
        assert!(
            handoff["fixture_ids"]
                .as_array()
                .expect("negative fixture ids must be an array")
                .iter()
                .any(|candidate| candidate.as_str() == Some(fixture_id)),
            "missing negative fixture id {fixture_id}"
        );
    }
    for fixture_kind in ["malformed_syntax", "unsupported_syntax"] {
        assert!(
            handoff["fixture_kinds"]
                .as_array()
                .expect("negative fixture kinds must be an array")
                .iter()
                .any(|candidate| candidate.as_str() == Some(fixture_kind)),
            "missing negative fixture kind {fixture_kind}"
        );
    }
    for severity in ["error", "warning"] {
        assert!(
            handoff["diagnostic_severities"]
                .as_array()
                .expect("negative fixture severities must be an array")
                .iter()
                .any(|candidate| candidate.as_str() == Some(severity)),
            "missing negative fixture severity {severity}"
        );
    }
    assert!(
        handoff["rule_path_evidence"]
            .as_array()
            .expect("negative fixture rule path evidence must be an array")
            .iter()
            .any(|record| record["diagnostic_rule_paths"]
                .as_array()
                .expect("diagnostic rule paths must be an array")
                .contains(&json!(["assignment", "expression"])))
    );
    assert!(
        handoff["unsupported_syntax_errors"]
            .as_array()
            .expect("negative fixture unsupported syntax errors must be an array")
            .iter()
            .any(|error| {
                error["target_id"] == target
                    || (target == "toy"
                        && error["unsupported_kind"] == "CallExpr"
                        && error["rule_path"] == json!(["assignment", "expression", "call"]))
            })
    );
    for hash_field in [
        "source_or_metadata_sha256",
        "sidecar_boundary_sha256",
        "diagnostic_contract_sha256",
    ] {
        assert!(
            handoff[hash_field]
                .as_array()
                .expect("negative fixture hash field must be an array")
                .iter()
                .all(|value| value.as_str().unwrap().len() == 64),
            "invalid negative fixture hash field {hash_field}"
        );
    }
    assert!(
        handoff["parser_or_grammar_identity"]
            .as_array()
            .expect("negative fixture provenance identities must be an array")
            .iter()
            .any(|identity| identity["kind"] == "grammar")
    );
    assert!(
        handoff["parser_or_grammar_identity"]
            .as_array()
            .expect("negative fixture provenance identities must be an array")
            .iter()
            .any(|identity| identity["kind"] == "parser")
    );
    assert_eq!(handoff["sidecar_status"]["sidecar_only"], true);
    assert_eq!(handoff["sidecar_status"]["stable_gate_promoted"], false);
    assert_eq!(handoff["sidecar_status"]["live_toolchain_required"], false);
    assert_eq!(
        handoff["sidecar_status"]["generated_hot_path_external_checks_allowed"],
        false
    );
    assert_eq!(handoff["compiler_boundary"]["sidecar_only"], true);
    assert_eq!(
        handoff["compiler_boundary"]["tct_compiler_route_allowed"],
        false
    );
    assert!(
        handoff["non_goals"]
            .as_array()
            .expect("negative fixture non-goals must be an array")
            .iter()
            .any(|goal| goal == "no TctCompiler route")
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_negative_fixture_path"],
        INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE_PATH
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_negative_fixture_fixture_sha256"],
        handoff["fixture_sha256"]
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_negative_fixture"]
            ["target"],
        target
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_negative_fixture"]
            ["fixture_ids"],
        handoff["fixture_ids"]
    );
    assert_eq!(
        artifact.metadata["generated_codec_artifact_metadata"]["input_diagnostics_negative_fixture"]
            ["unsupported_syntax_errors"],
        handoff["unsupported_syntax_errors"]
    );
}

fn write_toy_syntax_generated_codec_reproducibility_report(
    artifact: &SyntaxGeneratedCodecArtifact,
) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_dir = repo_root.join("target/tct-syntax-generated-codec-reproducibility");
    fs::create_dir_all(&report_dir)
        .expect("failed to create generated syntax codec reproducibility report dir");
    let report_path = report_dir.join("toy-syntax-generated-codec.json");
    let report = json!({
        "artifact_kind": "tct_syntax_generated_codec_reproducibility_report",
        "artifact_version": 1,
        "boundary_manifest": &artifact.metadata["boundary_manifest"],
        "generated_artifact_envelope": &artifact.metadata["generated_artifact_envelope"],
        "validation_hook_contract": &artifact.metadata["validation_hook_contract"],
        "validation_hook_sidecar_evidence": &artifact.metadata["validation_hook_sidecar_evidence"],
        "generated_artifact_replay": &artifact.metadata["generated_artifact_replay"],
        "input_diagnostics_contract": &artifact.metadata["input_diagnostics_contract"],
        "input_diagnostics_negative_fixture": &artifact.metadata["input_diagnostics_negative_fixture"],
        "metadata_sha256": &artifact.metadata_sha256,
        "metadata": &artifact.metadata,
    });
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report)
            .expect("failed to render generated syntax codec reproducibility report"),
    )
    .expect("failed to write generated syntax codec reproducibility report");
    let written: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("failed to read generated syntax codec report"),
    )
    .expect("failed to parse generated syntax codec report");
    assert_eq!(written, report);
}

fn write_parser_backed_syntax_generated_codec_reproducibility_report(
    artifacts: &[SyntaxGeneratedCodecArtifact],
) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_dir = repo_root.join("target/tct-syntax-generated-codec-reproducibility");
    fs::create_dir_all(&report_dir)
        .expect("failed to create parser-backed syntax codec reproducibility report dir");
    let report_path = report_dir.join("parser-backed-syntax-generated-codecs.json");
    let report = json!({
        "artifact_kind": "tct_parser_backed_syntax_generated_codec_reproducibility_report",
        "artifact_version": 1,
        "artifacts": artifacts
            .iter()
            .map(|artifact| {
                json!({
                    "target": artifact.target,
                    "boundary_manifest": &artifact.metadata["boundary_manifest"],
                    "generated_artifact_envelope": &artifact.metadata["generated_artifact_envelope"],
                    "validation_hook_contract": &artifact.metadata["validation_hook_contract"],
                    "validation_hook_sidecar_evidence": &artifact.metadata["validation_hook_sidecar_evidence"],
                    "generated_artifact_replay": &artifact.metadata["generated_artifact_replay"],
                    "input_diagnostics_contract": &artifact.metadata["input_diagnostics_contract"],
                    "input_diagnostics_negative_fixture": &artifact.metadata["input_diagnostics_negative_fixture"],
                    "target_agnostic_compiler_route": artifact
                        .metadata
                        .get("target_agnostic_compiler_route")
                        .cloned()
                        .unwrap_or(Value::Null),
                    "metadata_sha256": &artifact.metadata_sha256,
                    "metadata": &artifact.metadata,
                })
            })
            .collect::<Vec<_>>(),
    });
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report)
            .expect("failed to render parser-backed syntax codec report"),
    )
    .expect("failed to write parser-backed syntax codec report");
    let written: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("failed to read parser-backed syntax codec report"),
    )
    .expect("failed to parse parser-backed syntax codec report");
    assert_eq!(written, report);
}

fn toy_syntax_bpe_merge_table(
    boundary: &GrammarStructuralBpeBoundary,
    sequence_count: usize,
) -> BpeMergeTable {
    BpeMergeTable {
        schema: "toy-syntax-sidecar".to_string(),
        base_vocab_size: boundary.raw_base_vocab_size,
        merges: vec![
            BpeMerge {
                pair: [PROGRAM_START_TOKEN, STATEMENT_TOKEN],
                new_token: boundary.bpe_start,
                frequency: sequence_count as u64,
            },
            BpeMerge {
                pair: [INTEGER_VALUE_TOKEN, INTEGER_TOKEN_BASE + 7],
                new_token: boundary.bpe_start + 1,
                frequency: 1,
            },
        ],
        metadata: BpeMetadata {
            trained_on_instances: sequence_count,
            training_date: "2026-05-07".to_string(),
            max_merges: 2,
            dict_size: None,
        },
    }
}

fn toy_syntax_token_corpus() -> Vec<Vec<u32>> {
    vec![
        toy_syntax_assignment_tokens(
            "alpha",
            &[INTEGER_VALUE_TOKEN, INTEGER_TOKEN_BASE + 7],
            None,
        ),
        toy_syntax_assignment_tokens("beta", &[TRUE_VALUE_TOKEN], None),
        toy_syntax_assignment_tokens("gamma", &[STRING_VALUE_TOKEN], Some("ok")),
    ]
}

fn toy_syntax_assignment_tokens(
    identifier: &str,
    value_prefix_tokens: &[u32],
    string_value: Option<&str>,
) -> Vec<u32> {
    let mut tokens = vec![PROGRAM_START_TOKEN, STATEMENT_TOKEN];
    push_toy_utf8_payload(identifier, IDENTIFIER_END_TOKEN, &mut tokens);
    tokens.extend_from_slice(value_prefix_tokens);
    if let Some(string_value) = string_value {
        push_toy_utf8_payload(string_value, STRING_END_TOKEN, &mut tokens);
    }
    tokens.push(PROGRAM_END_TOKEN);
    tokens
}

fn push_toy_utf8_payload(value: &str, terminator: u32, tokens: &mut Vec<u32>) {
    for byte in value.as_bytes() {
        tokens.push(BYTE_TOKEN_BASE + u32::from(*byte));
    }
    tokens.push(terminator);
}

fn toy_syntax_bpe_merge_artifact_report(
    sidecar: &GrammarStructuralModelSidecar,
    boundary: &GrammarStructuralBpeBoundary,
    merge_table: &BpeMergeTable,
    source_token_corpus: &[Vec<u32>],
    encoded_token_corpus: &[Vec<u32>],
) -> Value {
    let merge_ids_fit_reserved_boundary = merge_table
        .merges
        .iter()
        .all(|merge| (boundary.bpe_start..boundary.bpe_end_exclusive).contains(&merge.new_token));
    let tokens_below_vocab_size = encoded_token_corpus
        .iter()
        .flatten()
        .all(|token| *token < boundary.vocab_size && *token != boundary.pad_token);
    let bpe_token_count = encoded_token_corpus
        .iter()
        .flatten()
        .filter(|token| (boundary.bpe_start..boundary.bpe_end_exclusive).contains(token))
        .count();

    json!({
        "artifact_kind": "tct_syntax_bpe_merge_artifact_evidence",
        "artifact_version": 1,
        "tool": "syntax_generated_codec_gate_test",
        "version": 1,
        "syntax_token_layout": {
            "raw_base_vocab_size": boundary.raw_base_vocab_size,
            "bpe_start": boundary.bpe_start,
            "bpe_end_exclusive": boundary.bpe_end_exclusive,
            "bpe_vocab_size": boundary.bpe_end_exclusive,
            "pad_token": boundary.pad_token,
            "vocab_size": boundary.vocab_size,
            "current_layout_hosts_bpe": boundary.current_layout_hosts_bpe,
        },
        "merge_table": {
            "schema": &merge_table.schema,
            "base_vocab_size": merge_table.base_vocab_size,
            "bpe_start": merge_table.bpe_start(),
            "merge_count": merge_table.merges.len(),
            "sha256": json_sha256(merge_table)
                .expect("failed to hash syntax BPE merge table"),
            "merges": &merge_table.merges,
        },
        "source_token_corpus": {
            "sequence_count": source_token_corpus.len(),
            "token_count": source_token_corpus.iter().map(Vec::len).sum::<usize>(),
            "sha256": json_sha256(&source_token_corpus)
                .expect("failed to hash syntax BPE source token corpus"),
        },
        "encoded_token_corpus": {
            "sequence_count": encoded_token_corpus.len(),
            "token_count": encoded_token_corpus.iter().map(Vec::len).sum::<usize>(),
            "bpe_token_count": bpe_token_count,
            "sha256": json_sha256(&encoded_token_corpus)
                .expect("failed to hash syntax BPE encoded token corpus"),
        },
        "sidecar_boundary": {
            "generated_json_schema_output_changed": sidecar
                .structural_model
                .boundary
                .generated_json_schema_output_changed,
            "core_compiler_path_changed": sidecar.structural_model.boundary.core_compiler_path_changed,
            "production_grammar_compiler_path_changed": sidecar
                .structural_model
                .boundary
                .production_grammar_compiler_path_changed,
        },
        "proof": {
            "decode_round_trip_ok": true,
            "bpe_ids_fit_reserved_boundary": merge_ids_fit_reserved_boundary,
            "tokens_below_vocab_size": tokens_below_vocab_size,
            "generated_json_schema_output_changed": sidecar
                .structural_model
                .boundary
                .generated_json_schema_output_changed,
        },
    })
}

fn write_toy_syntax_bpe_merge_artifact_report(report: &Value) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_dir = repo_root.join("target/tct-syntax-generated-codec-bpe");
    fs::create_dir_all(&report_dir)
        .expect("failed to create generated syntax BPE artifact report dir");
    let report_path = report_dir.join("toy-syntax-bpe-merge-artifact.json");
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(report)
            .expect("failed to render generated syntax BPE artifact report"),
    )
    .expect("failed to write generated syntax BPE artifact report");
    let written: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("failed to read generated syntax BPE artifact report"),
    )
    .expect("failed to parse generated syntax BPE artifact report");
    assert_eq!(written, *report);
}

const PINNED_GO_SIDECAR_VERSION: &str = "go1.24.3";

#[derive(Debug)]
struct GoSidecarToolchain {
    go_bin: OsString,
    gofmt_bin: OsString,
    version: String,
    goroot: String,
}

#[test]
fn go_sidecar_toolchain_matches_pinned_version_when_available_or_strict() {
    let Some(toolchain) = go_sidecar_toolchain(
        "go_sidecar_toolchain_matches_pinned_version_when_available_or_strict",
    ) else {
        return;
    };
    assert_eq!(toolchain.version, PINNED_GO_SIDECAR_VERSION);
    assert!(!toolchain.goroot.is_empty());
    assert!(!toolchain.go_bin.as_os_str().is_empty());
    assert!(!toolchain.gofmt_bin.as_os_str().is_empty());
}

/// Resolve the sidecar `go` through the unified pin resolver (`tct::go_toolchain`:
/// `TCT_GO_BIN`, then `PATH` when it reports the pin, then `$HOME/sdk/<pin>/bin/go`).
/// Falls back to a bare `go` so a missing or mismatched pin still reaches the
/// `TCT_GO_SIDECAR_SKIP` / strict-failure logic below.
fn go_sidecar_go_bin() -> OsString {
    tct::go_toolchain::go_bin_os_string()
        .unwrap_or_else(|_| std::env::var_os("TCT_GO_BIN").unwrap_or_else(|| OsString::from("go")))
}

fn go_sidecar_gofmt_bin() -> OsString {
    tct::go_toolchain::gofmt_bin_os_string().unwrap_or_else(|_| {
        std::env::var_os("TCT_GOFMT_BIN").unwrap_or_else(|| OsString::from("gofmt"))
    })
}

fn go_sidecar_toolchain(test_name: &str) -> Option<GoSidecarToolchain> {
    let go_bin = go_sidecar_go_bin();
    let gofmt_bin = go_sidecar_gofmt_bin();

    let go_version_output = match Command::new(&go_bin).arg("version").output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            go_sidecar_skip_or_panic(
                test_name,
                &format!(
                    "go version failed with status {}; stderr={}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            );
            return None;
        }
        Err(error) => {
            go_sidecar_skip_or_panic(
                test_name,
                &format!("go binary {:?} is unavailable: {error}", go_bin),
            );
            return None;
        }
    };
    let go_version_stdout = String::from_utf8_lossy(&go_version_output.stdout);
    let Some(version) = parse_go_version(&go_version_stdout) else {
        go_sidecar_skip_or_panic(
            test_name,
            &format!("could not parse go version output {go_version_stdout:?}"),
        );
        return None;
    };
    if version != PINNED_GO_SIDECAR_VERSION {
        go_sidecar_skip_or_panic(
            test_name,
            &format!("go version {version} does not match pinned {PINNED_GO_SIDECAR_VERSION}"),
        );
        return None;
    }

    let goroot_output = match Command::new(&go_bin).args(["env", "GOROOT"]).output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            go_sidecar_skip_or_panic(
                test_name,
                &format!(
                    "go env GOROOT failed with status {}; stderr={}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            );
            return None;
        }
        Err(error) => {
            go_sidecar_skip_or_panic(test_name, &format!("go env GOROOT failed: {error}"));
            return None;
        }
    };
    let goroot = String::from_utf8_lossy(&goroot_output.stdout)
        .trim()
        .to_string();
    if goroot.is_empty() {
        go_sidecar_skip_or_panic(test_name, "go env GOROOT returned an empty path");
        return None;
    }

    match Command::new(&gofmt_bin).arg("-h").output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            go_sidecar_skip_or_panic(
                test_name,
                &format!(
                    "gofmt -h failed with status {}; stderr={}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            );
            return None;
        }
        Err(error) => {
            go_sidecar_skip_or_panic(
                test_name,
                &format!("gofmt binary {:?} is unavailable: {error}", gofmt_bin),
            );
            return None;
        }
    }

    Some(GoSidecarToolchain {
        go_bin,
        gofmt_bin,
        version,
        goroot,
    })
}

fn parse_go_version(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .find(|part| {
            part.strip_prefix("go")
                .is_some_and(|suffix| suffix.as_bytes().first().is_some_and(u8::is_ascii_digit))
        })
        .map(str::to_string)
}

fn go_sidecar_skip_or_panic(test_name: &str, reason: &str) {
    let message = format!(
        "TCT_GO_SIDECAR_SKIP test={test_name} required_go_version={PINNED_GO_SIDECAR_VERSION} reason={reason}; set TCT_REQUIRE_GO_SIDECARS=1 to make missing or mismatched Go sidecars a hard failure"
    );
    if strict_go_sidecars_required() {
        panic!("{message}");
    }
    eprintln!("{message}");
}

fn strict_go_sidecars_required() -> bool {
    std::env::var("TCT_REQUIRE_GO_SIDECARS")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn assert_go_real_source_report_tokens(report: &Value, expected_count: u64) {
    let tokens = report["tct_tokens"]
        .as_array()
        .expect("Go real-source report missing tct_tokens array");
    assert_eq!(tokens.len() as u64, expected_count);
    assert!(
        tokens.iter().all(|token| token.as_u64().is_some()),
        "Go real-source report tokens must be unsigned integers"
    );
}

fn assert_csharp_real_source_report_retargeted_structural_replay_tokens(
    report: &Value,
    expected_payload_sequence_count: u64,
    expected_structural_replay_count: u64,
) {
    let payload_sequence_tokens = report["payload_sequence_control_tokens"]
        .as_array()
        .expect("C# retargeted real-source report should preserve payload-sequence control tokens");
    assert_eq!(
        payload_sequence_tokens.len() as u64,
        expected_payload_sequence_count
    );
    assert!(
        payload_sequence_tokens
            .iter()
            .all(|token| token.as_u64().is_some()),
        "C# payload-sequence control tokens must be unsigned integers"
    );
    let payload_sequence_tokens_sha256 = json_sha256(&report["payload_sequence_control_tokens"])
        .expect("failed to hash C# payload-sequence control tokens");
    assert_eq!(
        report["payload_sequence_control_tokens_sha256"].as_str(),
        Some(payload_sequence_tokens_sha256.as_str())
    );
    assert_eq!(
        report["payload_sequence_control_token_count"].as_u64(),
        Some(expected_payload_sequence_count)
    );

    let tct_tokens = report["tct_tokens"]
        .as_array()
        .expect("C# retargeted real-source report should preserve authoritative raw token IDs");
    assert_eq!(tct_tokens.len() as u64, expected_structural_replay_count);
    assert_eq!(
        report["tct_tokens"],
        report["structural_replay"]["raw_token_ids"]
    );
    let raw_token_ids_sha256 =
        json_sha256(&report["tct_tokens"]).expect("failed to hash C# retargeted raw tokens");
    assert_eq!(
        report["raw_token_ids_sha256"].as_str(),
        Some(raw_token_ids_sha256.as_str())
    );
    assert_eq!(
        report["structural_replay"]["raw_token_count"].as_u64(),
        Some(expected_structural_replay_count)
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["mapping_kind"],
        "retargeted_structural_replay_stream_offsets"
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["authoritative_raw_token_stream"],
        "tct_tokens"
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["payload_sequence_stream"],
        "payload_sequence_control_tokens"
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["payload_sequence_token_count"].as_u64(),
        Some(expected_payload_sequence_count)
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["structural_replay_stream"],
        "tct_tokens"
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["structural_replay_sidecar_stream"],
        "structural_replay.raw_token_ids"
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["structural_replay_token_count"].as_u64(),
        Some(expected_structural_replay_count)
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["payload_sequence_stream_kept_as_compatibility_control"],
        true
    );
    assert_eq!(
        report["raw_token_offset_mapping"]["one_to_one_payload_sequence_offset_mapping"],
        false
    );
}

fn assert_go_real_source_corpus_ready_for_strings_compare() -> Value {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_path = repo_root.join("target/tct-analysis/go-syntax-corpus-check/report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).expect("failed to create Go syntax corpus check report dir");
    }
    let output = Command::new(go_sidecar_go_bin())
        .args([
            "run",
            "scripts/analysis/go_syntax_corpus_check.go",
            "--manifest",
            "tests/fixtures/go_syntax_corpus/manifest-v1.json",
            "--json-output",
        ])
        .arg(&report_path)
        .current_dir(repo_root)
        .output()
        .expect("failed to run Go syntax corpus check");
    assert!(
        output.status.success(),
        "Go syntax corpus check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report_path).expect("failed to read Go syntax corpus check report"),
    )
    .expect("failed to parse Go syntax corpus check report");
    assert_eq!(report["artifact_kind"], "tct_go_syntax_corpus_check_v1");
    assert_eq!(report["status"], "ready");
    let files = report["files"]
        .as_array()
        .expect("Go syntax corpus report missing files");
    let strings_compare = files
        .iter()
        .find(|file| file["path"] == "tests/fixtures/go_syntax_corpus/src/strings/compare.go")
        .expect("Go syntax corpus report missing strings/compare.go");
    assert_eq!(
        strings_compare["sha256"],
        "af4e0fa7dfe63018d9808f89a4c511eeb8d773bd6f3b030f1c9592a1e0de7a85"
    );
    assert!(
        strings_compare["unsupported_constructs"]
            .as_array()
            .expect("strings/compare.go missing unsupported construct list")
            .is_empty()
    );
    report
}

fn run_generated_syntax_codec_fixture_tests(crate_name: &str, generated_source: &str) {
    run_generated_syntax_codec_fixture_tests_with_env(crate_name, generated_source, &[]);
}

fn run_generated_syntax_codec_fixture_tests_with_env(
    crate_name: &str,
    generated_source: &str,
    envs: &[(&str, &Path)],
) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crate_dir = TempDir::new().expect("failed to create generated syntax codec crate");
    fs::create_dir_all(crate_dir.path().join("src"))
        .expect("failed to create generated syntax codec src dir");
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_bin(crate_name.replace('-', "_"), "src/main.rs")
        .with_sha2()
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated syntax codec Cargo.toml");
    copy_workspace_cargo_lock(repo_root, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock for generated syntax codec");
    fs::write(crate_dir.path().join("src/main.rs"), generated_source)
        .expect("failed to write generated syntax codec source");

    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, repo_root);
    command
        .args(["test", "--quiet"])
        .current_dir(crate_dir.path())
        .env(GENERATED_EXTERNAL_BUILD_ENV, "1");
    // The generated fixture tests pick their Go sidecar from these variables, so
    // hand them the pinned toolchain instead of whatever `go` is first on PATH.
    if let Ok(toolchain) = tct::go_toolchain::resolve() {
        command.env("TCT_GO_BIN", &toolchain.go_bin);
        command.env("TCT_GOFMT_BIN", &toolchain.gofmt_bin);
    }
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command
        .output()
        .expect("failed to run generated syntax codec cargo test");
    if !output.status.success() {
        panic!(
            "generated syntax codec cargo test failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
