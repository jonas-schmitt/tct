use std::path::Path;

use tct::go_syntax_metadata::GoSyntaxMetadataAdapter;
use tct::grammar_ir::GrammarTerminalKind;
use tct::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralTokenLayout, GrammarStructuralTokenRange,
    lower_grammar_ir_to_structural_model,
};

const METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/minimal_function/syntax-metadata-v0.json";
const SOURCE_PATH: &str = "tests/fixtures/go_syntax_sources/minimal/main.go";
const SOURCE_SHA256: &str = "f0a87ff5993e610218e870cf01576c90d2779efd4278c56db68678ef30a63b78";
const LOCAL_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_declaration/syntax-metadata-v0.json";
const LOCAL_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_declaration/main.go";
const LOCAL_DECLARATION_SOURCE_SHA256: &str =
    "2008d64112b4e97d6071f46331bc71fe6f5fb29c1ed731a43623482fd892a17c";
const LOCAL_ASSIGNMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_assignment/syntax-metadata-v0.json";
const LOCAL_ASSIGNMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_assignment/main.go";
const LOCAL_ASSIGNMENT_SOURCE_SHA256: &str =
    "2926c97d312f1156ce6134ec6be5d5394a2f17aa1fdf9e2ca2d33bf4b675bfef";
const LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_escaped_assignment/syntax-metadata-v0.json";
const LOCAL_ESCAPED_ASSIGNMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_escaped_assignment/main.go";
const LOCAL_ESCAPED_ASSIGNMENT_SOURCE_SHA256: &str =
    "50f1bc4d3dadfb7a4a0333d0709f067dcbd24b7de0364840b7f711f6664186f3";
const LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_short_var_declaration/syntax-metadata-v0.json";
const LOCAL_SHORT_VAR_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_short_var_declaration/main.go";
const LOCAL_SHORT_VAR_DECLARATION_SOURCE_SHA256: &str =
    "465a154fd1f9b7bdf6082d16487f047fe794ec915c58d0fed1450d03dcbfb304";
const LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_integer_short_var_declaration/syntax-metadata-v0.json";
const LOCAL_INTEGER_SHORT_VAR_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_integer_short_var_declaration/main.go";
const LOCAL_INTEGER_SHORT_VAR_DECLARATION_SOURCE_SHA256: &str =
    "3b10fe242fd8a032a26eff5b284435a5e34d77bb3905cb7df8e869a47e58ce05";
const LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_integer_return_literal/syntax-metadata-v0.json";
const LOCAL_INTEGER_RETURN_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_integer_return_literal/main.go";
const LOCAL_INTEGER_RETURN_LITERAL_SOURCE_SHA256: &str =
    "1c6ac48e802baa130feea63fa378ddbd9c4ff253533048d3d99af9aee64dd0bf";
const LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/local_parenthesized_integer_return_expression/syntax-metadata-v0.json";
const LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_parenthesized_integer_return_expression/main.go";
const LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_SOURCE_SHA256: &str =
    "f0290b7b559a30810a00a0c3e542cd9f03bc7757315624db98435b49d844f4d2";
const LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/local_unary_integer_return_expression/syntax-metadata-v0.json";
const LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_unary_integer_return_expression/main.go";
const LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_SOURCE_SHA256: &str =
    "e5e80da0a6c39c781446240e28ff66217085da735d181535300e32a3a7be931b";
const LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/local_binary_integer_return_expression/syntax-metadata-v0.json";
const LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_binary_integer_return_expression/main.go";
const LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_SOURCE_SHA256: &str =
    "7b3a816f27dfe38abfc618e8af780a6c6132be3ba4ecf36cf857430d6b40de8a";
const LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_call_return_expression/syntax-metadata-v0.json";
const LOCAL_CALL_RETURN_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_call_return_expression/main.go";
const LOCAL_CALL_RETURN_EXPRESSION_SOURCE_SHA256: &str =
    "87d452894d3a7d9cf874cff8d93d37edd9192e244a362189440e69835c6b6e13";
const LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/local_argument_call_return_expression/syntax-metadata-v0.json";
const LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_argument_call_return_expression/main.go";
const LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_SOURCE_SHA256: &str =
    "7e5f1055c7323ecbcb23310c4f4cb37bc8657a371ddaa918d922a7f56e4c4339";
const LOCAL_STRING_RETURN_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_string_return_literal/syntax-metadata-v0.json";
const LOCAL_STRING_RETURN_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_string_return_literal/main.go";
const LOCAL_STRING_RETURN_LITERAL_SOURCE_SHA256: &str =
    "ae1c27ac17e48fa169b0f261343277aa9b74c147f30c13741e2e642cccebad58";
const LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_float_return_literal/syntax-metadata-v0.json";
const LOCAL_FLOAT_RETURN_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_float_return_literal/main.go";
const LOCAL_FLOAT_RETURN_LITERAL_SOURCE_SHA256: &str =
    "9591e2fed44a03e6c3416a922898f3973de3995bb6af07148cd9e4702040d495";
const LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_boolean_return_identifier/syntax-metadata-v0.json";
const LOCAL_BOOLEAN_RETURN_IDENTIFIER_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_boolean_return_identifier/main.go";
const LOCAL_BOOLEAN_RETURN_IDENTIFIER_SOURCE_SHA256: &str =
    "22380512194c482ea4473ce0165f57056cae7dacfe452cbd3e7a1ce3389be38b";
const LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_nil_return_identifier/syntax-metadata-v0.json";
const LOCAL_NIL_RETURN_IDENTIFIER_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_nil_return_identifier/main.go";
const LOCAL_NIL_RETURN_IDENTIFIER_SOURCE_SHA256: &str =
    "4701e02bf8af242d8a444fdc5e0f3f99931080ec104150554a5768803926a3dd";
const LOCAL_TYPED_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_typed_declaration/syntax-metadata-v0.json";
const LOCAL_TYPED_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_typed_declaration/main.go";
const LOCAL_TYPED_DECLARATION_SOURCE_SHA256: &str =
    "76721175937a794b6f050af17a74d8273b80ce30d59520c1dfc0a220754f6658";
const LOCAL_ZERO_VALUE_TYPED_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_zero_value_typed_declaration/syntax-metadata-v0.json";
const LOCAL_ZERO_VALUE_TYPED_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_zero_value_typed_declaration/main.go";
const LOCAL_ZERO_VALUE_TYPED_DECLARATION_SOURCE_SHA256: &str =
    "b54a38609b5a83baf6c00dede6d43fea9f3aa1d9c9ed5497a8c2972b85195137";
const LOCAL_INTEGER_TYPED_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_integer_typed_declaration/syntax-metadata-v0.json";
const LOCAL_INTEGER_TYPED_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_integer_typed_declaration/main.go";
const LOCAL_INTEGER_TYPED_DECLARATION_SOURCE_SHA256: &str =
    "d7f93031d13cc4d889bb74f3fa3b5c0201285cc91407ae901432fc73537f787a";
const LOCAL_UNTYPED_INTEGER_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_untyped_integer_declaration/syntax-metadata-v0.json";
const LOCAL_UNTYPED_INTEGER_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_untyped_integer_declaration/main.go";
const LOCAL_UNTYPED_INTEGER_DECLARATION_SOURCE_SHA256: &str =
    "ee01861e7cbf16de8289f0462f2eafb3462e1d5852bc7e462ebb905537a5b829";
const PACKAGE_TYPE_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/package_type_declaration/syntax-metadata-v0.json";
const PACKAGE_TYPE_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/package_type_declaration/main.go";
const PACKAGE_TYPE_DECLARATION_SOURCE_SHA256: &str =
    "46e8fdbb32fd6b7bd8a6570197b7d4e2cef38a8b674a84c91da77d6f17b52820";
const TYPE_SPEC_ASSIGN_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/type_spec_assign/syntax-metadata-v0.json";
const TYPE_SPEC_ASSIGN_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/type_spec_assign/main.go";
const TYPE_SPEC_ASSIGN_SOURCE_SHA256: &str =
    "588bbe7d0e720368a41adf8e0c29a4e6dfe63dc528aa10a1561da5431f35210b";
const TYPE_SPEC_COMMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/type_spec_comment/syntax-metadata-v0.json";
const TYPE_SPEC_COMMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/type_spec_comment/main.go";
const TYPE_SPEC_COMMENT_SOURCE_SHA256: &str =
    "d70064b7313ff1689c646a2475135f52d5385a0bfb1e9d0364b0d16de8da41c5";
const TYPE_SPEC_DOC_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/type_spec_doc/syntax-metadata-v0.json";
const TYPE_SPEC_DOC_SOURCE_PATH: &str = "tests/fixtures/go_syntax_corpus/src/type_spec_doc/main.go";
const TYPE_SPEC_DOC_SOURCE_SHA256: &str =
    "97cef23ea5a24ece5c7df0ebddff0080a7925edd915ed2beff94ac112e6bfac8";
const TYPE_SPEC_TYPE_PARAMS_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/type_spec_type_params/syntax-metadata-v0.json";
const TYPE_SPEC_TYPE_PARAMS_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/type_spec_type_params/main.go";
const TYPE_SPEC_TYPE_PARAMS_SOURCE_SHA256: &str =
    "038287eb30f80a7f28f3c4f7b19a23246858ac8f00c9f453ff44b99fe33c5d7f";
const PACKAGE_CONST_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/package_const_declaration/syntax-metadata-v0.json";
const PACKAGE_CONST_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/package_const_declaration/main.go";
const PACKAGE_CONST_DECLARATION_SOURCE_SHA256: &str =
    "d0c92426b53245e038b6ca44da5975f507a0df0544d652ba52f7b2dfabf8dffd";
const METHOD_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/method_declaration/syntax-metadata-v0.json";
const METHOD_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/method_declaration/main.go";
const METHOD_DECLARATION_SOURCE_SHA256: &str =
    "1d249f33e428ec0f77f4da9d459b23b9bedfa037736a6f1e9660bd7e83251e89";
const STRUCT_TYPE_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/struct_type/syntax-metadata-v0.json";
const STRUCT_TYPE_SOURCE_PATH: &str = "tests/fixtures/go_syntax_metadata/struct_type/main.go";
const STRUCT_TYPE_SOURCE_SHA256: &str =
    "8b7ef069d0bf791dd9f2e95c0f6aabd82b8a714626bb9d5831fbd51d47db3c96";
const COMPOSITE_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/composite_literal/syntax-metadata-v0.json";
const COMPOSITE_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/composite_literal/main.go";
const COMPOSITE_LITERAL_SOURCE_SHA256: &str =
    "11d49cd8c039c6a3e674b15b2df60c199a0c66494c2f52699699bf8acabca394";
const KEY_VALUE_COMPOSITE_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/key_value_composite_literal/syntax-metadata-v0.json";
const KEY_VALUE_COMPOSITE_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/key_value_composite_literal/main.go";
const KEY_VALUE_COMPOSITE_LITERAL_SOURCE_SHA256: &str =
    "b4541a875227334a28e4ece9211575761abce1a40900c4c98079eea3f69ed76c";
const ARRAY_COMPOSITE_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/array_composite_literal/syntax-metadata-v0.json";
const ARRAY_COMPOSITE_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/array_composite_literal/main.go";
const ARRAY_COMPOSITE_LITERAL_SOURCE_SHA256: &str =
    "c9c457cbeb451abe7d06a123226ffbb753cee05caa61503070533834300402cd";
const INDEX_EXPRESSION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/index_expression/syntax-metadata-v0.json";
const INDEX_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/index_expression/main.go";
const INDEX_EXPRESSION_SOURCE_SHA256: &str =
    "b7236e7f8a60394bcef3053ce1a8b6b762f6fb4ed94827e2598e94752e54d1e1";
const SLICE_EXPRESSION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/slice_expression/syntax-metadata-v0.json";
const SLICE_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/slice_expression/main.go";
const SLICE_EXPRESSION_SOURCE_SHA256: &str =
    "72ea742c75dab3dd52516bdb0a4c8b27ee8e964476f671ed018fbb0d00f4e90e";
const SLICE_EXPRESSION_THREE_INDEX_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/slice_expression_three_index/syntax-metadata-v0.json";
const SLICE_EXPRESSION_THREE_INDEX_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/slice_expression_three_index/main.go";
const SLICE_EXPRESSION_THREE_INDEX_SOURCE_SHA256: &str =
    "7cd934057a213f2aa754088315e0b7fe52a96cb2e5341accc69410d897e05ccf";
const STAR_EXPRESSION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/star_expression/syntax-metadata-v0.json";
const STAR_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/star_expression/main.go";
const STAR_EXPRESSION_SOURCE_SHA256: &str =
    "7a726fe77893be185b64e713750c75702d8fc003c06291edf6aecc65106d4b07";
const PACKAGE_VAR_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/package_var_declaration/syntax-metadata-v0.json";
const PACKAGE_VAR_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/package_var_declaration/main.go";
const PACKAGE_VAR_DECLARATION_SOURCE_SHA256: &str =
    "b6a164729eac95b7f354f1df9257cd397775a8c412ac2787068ea2a2b580529b";
const VALUE_SPEC_COMMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/value_spec_comment/syntax-metadata-v0.json";
const VALUE_SPEC_COMMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/value_spec_comment/main.go";
const VALUE_SPEC_COMMENT_SOURCE_SHA256: &str =
    "129111a4fcdc7dfa487e33464b184c08934e7f018eff14c789e590be3e289d46";
const IMPORT_SPEC_NAME_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/import_spec_name/syntax-metadata-v0.json";
const IMPORT_SPEC_NAME_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/import_spec_name/main.go";
const IMPORT_SPEC_NAME_SOURCE_SHA256: &str =
    "6cbe0825172e2269bbf21f42572eac2aac80edfff194825207dd371bc6250e4b";
const IMPORT_SPEC_COMMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/import_spec_comment/syntax-metadata-v0.json";
const IMPORT_SPEC_COMMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/import_spec_comment/main.go";
const IMPORT_SPEC_COMMENT_SOURCE_SHA256: &str =
    "049a406aa94a01b39645f81411888eab3b158ff5b8c98778edcfba26351a2b01";
const FUNCTION_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/function_literal/syntax-metadata-v0.json";
const FUNCTION_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/function_literal/main.go";
const FUNCTION_LITERAL_SOURCE_SHA256: &str =
    "3ad4eb2f23b212b9fcd852628a737bfdcebc1f4b6dbd0fe12d30f9f780b3cb3a";
const LOCAL_GENERIC_FUNCTION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_generic_function/syntax-metadata-v0.json";
const LOCAL_GENERIC_FUNCTION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/local_generic_function/main.go";
const LOCAL_GENERIC_FUNCTION_SOURCE_SHA256: &str =
    "3e65bcd305120feb7c1fccae68e66bb900132e8f96e96ec48aa2ab5db9b2c544";
const CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/concurrency_map_interface_type_assertion/syntax-metadata-v0.json";
const CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/concurrency_map_interface_type_assertion/main.go";
const CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_SOURCE_SHA256: &str =
    "5981c6c63514c052e03f831c79b4a671ec6739529e178feab31b09898272779a";
const LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_variadic_ellipsis/syntax-metadata-v0.json";
const LOCAL_VARIADIC_ELLIPSIS_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/local_variadic_ellipsis/main.go";
const LOCAL_VARIADIC_ELLIPSIS_SOURCE_SHA256: &str =
    "39d1a7fb1503b5e7c69ef08f33743a474a76aa53422e9749427d8ea31fab6031";
const SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/sort_interface_source_file_declaration_prefix/syntax-metadata-v0.json";
const SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/sort/example_interface_test.go";
const SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_SOURCE_SHA256: &str =
    "74de942ab67e4d7725f9e06c2389e8face1118e80865536ae709a0fc3367d5f1";
const SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/sort_search_source_file_example_prefix/syntax-metadata-v0.json";
const SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go";
const SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_SOURCE_SHA256: &str =
    "c65e8f2f0dab440d8f27a42c27199279f37256d4ef276863a7fd556529f4274d";
const SORT_SEARCH_SOURCE_FILE_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/sort_search_source_file/syntax-metadata-v0.json";
const SORT_SEARCH_SOURCE_FILE_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go";
const SORT_SEARCH_SOURCE_FILE_SOURCE_SHA256: &str =
    "c65e8f2f0dab440d8f27a42c27199279f37256d4ef276863a7fd556529f4274d";
const STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/strings_search_source_file_finder_prefix/syntax-metadata-v0.json";
const STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/strings/search.go";
const STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_SOURCE_SHA256: &str =
    "4fdef1f79f6e854789d9140b17ce2c2bbdbf0d4067a801e42607cf7877698cd1";
const STRINGS_SEARCH_SOURCE_FILE_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/strings_search_source_file/syntax-metadata-v0.json";
const STRINGS_SEARCH_SOURCE_FILE_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/strings/search.go";
const STRINGS_SEARCH_SOURCE_FILE_SOURCE_SHA256: &str =
    "4fdef1f79f6e854789d9140b17ce2c2bbdbf0d4067a801e42607cf7877698cd1";
const PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/path_match_source_file_match_prefix/syntax-metadata-v0.json";
const PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/path/match.go";
const PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_SOURCE_SHA256: &str =
    "bb88f659463a04e290492cff8d19c4d7d5c7d551db9b90b3049d3e51188d5ec8";
const PATH_MATCH_SOURCE_FILE_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/path_match_source_file/syntax-metadata-v0.json";
const PATH_MATCH_SOURCE_FILE_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/path/match.go";
const PATH_MATCH_SOURCE_FILE_SOURCE_SHA256: &str =
    "bb88f659463a04e290492cff8d19c4d7d5c7d551db9b90b3049d3e51188d5ec8";
const OS_SIGNAL_SOURCE_FILE_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/os_signal_source_file/syntax-metadata-v0.json";
const OS_SIGNAL_NOTIFY_CONTEXT_SOURCE_SPAN_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/os_signal_notify_context_source_span/syntax-metadata-v0.json";
const OS_SIGNAL_SOURCE_FILE_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go";
const OS_SIGNAL_SOURCE_FILE_SOURCE_SHA256: &str =
    "d99f44830137e1bc4da53ef7f0d5ce872d535219168e9f0408eb539c02f993fc";
const LOCAL_INTEGER_ASSIGNMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_integer_assignment/syntax-metadata-v0.json";
const LOCAL_INTEGER_ASSIGNMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_integer_assignment/main.go";
const LOCAL_INTEGER_ASSIGNMENT_SOURCE_SHA256: &str =
    "a71a4c05df99d30be34bc027186c5da4cdb73e1e3e2dac6603c86185e305bd80";
const LOCAL_COMPOUND_ASSIGNMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_compound_assignment/syntax-metadata-v0.json";
const LOCAL_COMPOUND_ASSIGNMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_compound_assignment/main.go";
const LOCAL_COMPOUND_ASSIGNMENT_SOURCE_SHA256: &str =
    "59e63934ab5f06ce71a5352332f1e2324e3749263e7d652029de78d3168a8136";
const LOCAL_INCREMENT_DECREMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_increment_decrement/syntax-metadata-v0.json";
const LOCAL_INCREMENT_DECREMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_increment_decrement/main.go";
const LOCAL_INCREMENT_DECREMENT_SOURCE_SHA256: &str =
    "40ab7937e68a9fe48e209e6d21c4ae4755c4dfac0e8781e5b85e883ba8712701";
const LOCAL_BRANCH_STATEMENT_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_branch_statement/syntax-metadata-v0.json";
const LOCAL_BRANCH_STATEMENT_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_branch_statement/main.go";
const LOCAL_BRANCH_STATEMENT_SOURCE_SHA256: &str =
    "3622b9cffebea30d8536fa9de7459d3d0014b8b8a026655c17bbe01a69cd87ea";
const LOCAL_IF_ELSE_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_if_else/syntax-metadata-v0.json";
const LOCAL_IF_ELSE_SOURCE_PATH: &str = "tests/fixtures/go_syntax_metadata/local_if_else/main.go";
const LOCAL_IF_ELSE_SOURCE_SHA256: &str =
    "1022d7c988f752ea2845ed62d7bf19af1671f3b7b026c0d774311f241c34eddb";
const LOCAL_FOR_CONDITION_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_for_condition/syntax-metadata-v0.json";
const LOCAL_FOR_CONDITION_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_for_condition/main.go";
const LOCAL_FOR_CONDITION_SOURCE_SHA256: &str =
    "c93ef4286175ae49eead3bcfd7d555cefe5bc6288f5b7a3a7bbe3ec3f7787bdc";
const LOCAL_SWITCH_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_switch/syntax-metadata-v0.json";
const LOCAL_SWITCH_SOURCE_PATH: &str = "tests/fixtures/go_syntax_metadata/local_switch/main.go";
const LOCAL_SWITCH_SOURCE_SHA256: &str =
    "9c848704da7fdf165948312ee209a3286e04a8b4d98b8821fb977d4c81a30f03";
const LOCAL_TYPE_SWITCH_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_type_switch/syntax-metadata-v0.json";
const LOCAL_TYPE_SWITCH_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/local_type_switch/main.go";
const LOCAL_TYPE_SWITCH_SOURCE_SHA256: &str =
    "ead97b6aab4e363cd320c6724443f28ae4a04cd2fea439a043b37b3363910c9c";
const LOCAL_RANGE_STRING_METADATA_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_range_string/syntax-metadata-v0.json";
const LOCAL_RANGE_STRING_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_metadata/local_range_string/main.go";
const LOCAL_RANGE_STRING_SOURCE_SHA256: &str =
    "51886c7f3f98d8d4de1b108226647e056ef357b915801ba6a5c220d1e7cb3302";

#[test]
fn go_syntax_metadata_lowers_to_grammar_ir_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_minimal_function_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_minimal_function_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(grammar.provenance.source_path.as_deref(), Some(SOURCE_PATH));
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(METADATA_PATH)
    );
    assert!(
        grammar
            .provenance
            .metadata_sha256
            .as_ref()
            .is_some_and(|sha| sha.len() == 64)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.version, "go1.24.3");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));
    assert_eq!(parser.package_version.as_deref(), Some("go1.24.3"));

    let func_decl = grammar.rule("FuncDecl").unwrap();
    let name = func_decl
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(
        name.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let body = func_decl
        .fields
        .iter()
        .find(|field| field.name == "Body")
        .unwrap();
    assert_eq!(
        body.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    assert_eq!(body.alternatives[0].node_kind, "BlockStmt");

    let field = grammar.rule("Field").unwrap();
    let names = field
        .fields
        .iter()
        .find(|field| field.name == "Names")
        .unwrap();
    assert!(!names.required);
    assert!(names.multiple);
    assert_eq!(names.alternatives[0].node_kind, "Ident");

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
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
    assert_eq!(sidecar.input.provenance.parser.as_ref().unwrap().tool, "go");
    assert_eq!(sidecar.proof.grammar_shape.rules, 12);
    assert_eq!(sidecar.proof.grammar_shape.fields, 20);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 16);
    assert_eq!(sidecar.structural_model.references.len(), 16);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncDecl"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
    );

    let decision_graph = &sidecar.structural_model.decision_graph;
    let func_name_decision = decision_graph
        .nodes
        .iter()
        .find(|node| node.id == "decision.rule.FuncDecl.field.Name")
        .unwrap();
    assert_eq!(func_name_decision.decision_kind, "payload");
    assert_eq!(
        func_name_decision.token_range.as_deref(),
        Some("go.structural_tokens")
    );
    let func_name_payload = decision_graph
        .nodes
        .iter()
        .find(|node| node.id == "decision.rule.FuncDecl.field.Name.alternative.identifier")
        .unwrap();
    assert_eq!(func_name_payload.node_kind, "alternative_payload");
    assert_eq!(
        func_name_payload.terminal,
        Some(GrammarTerminalKind::IdentifierPayload)
    );
    assert_eq!(
        func_name_payload.payload_kind.as_deref(),
        Some("identifier")
    );
    assert_eq!(
        func_name_payload.token_range.as_deref(),
        Some("go.utf8_byte_payload")
    );
    assert!(decision_graph.edges.iter().any(|edge| {
        edge.source == "decision.rule.FuncDecl.field.Name"
            && edge.target == "decision.rule.FuncDecl.field.Name.alternative.identifier"
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
fn go_local_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "DeclStmt")
    );
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ReturnStmt")
    );
    let value_spec = grammar.rule("ValueSpec").unwrap();
    let values = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Values")
        .unwrap();
    assert_eq!(values.alternatives[0].node_kind, "BasicLit");
    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 12);
    assert_eq!(sidecar.proof.grammar_shape.fields, 18);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 14);
    assert_eq!(sidecar.structural_model.references.len(), 14);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "DeclStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Values"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_local_assignment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_ASSIGNMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_assignment_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_ASSIGNMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_ASSIGNMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_ASSIGNMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    let assign = grammar.rule("AssignStmt").unwrap();
    let lhs = assign
        .fields
        .iter()
        .find(|field| field.name == "Lhs")
        .unwrap();
    assert_eq!(lhs.alternatives[0].node_kind, "Ident");
    let operator = assign
        .fields
        .iter()
        .find(|field| field.name == "Tok")
        .unwrap();
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let rhs = assign
        .fields
        .iter()
        .find(|field| field.name == "Rhs")
        .unwrap();
    assert_eq!(rhs.alternatives[0].node_kind, "BasicLit");

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 21);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 17);
    assert_eq!(sidecar.structural_model.references.len(), 17);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "AssignStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "AssignStmt"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_local_escaped_assignment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_escaped_local_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_escaped_local_assignment_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_ESCAPED_ASSIGNMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_ESCAPED_ASSIGNMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    let assign = grammar.rule("AssignStmt").unwrap();
    let operator = assign
        .fields
        .iter()
        .find(|field| field.name == "Tok")
        .unwrap();
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let rhs = assign
        .fields
        .iter()
        .find(|field| field.name == "Rhs")
        .unwrap();
    assert_eq!(rhs.alternatives[0].node_kind, "BasicLit");

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 21);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 17);
    assert_eq!(sidecar.structural_model.references.len(), 17);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "AssignStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "AssignStmt"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_local_short_var_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_short_var_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_short_var_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_SHORT_VAR_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_SHORT_VAR_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "DeclStmt")
    );
    let assign = grammar.rule("AssignStmt").unwrap();
    let lhs = assign
        .fields
        .iter()
        .find(|field| field.name == "Lhs")
        .unwrap();
    assert_eq!(lhs.alternatives[0].node_kind, "Ident");
    let operator = assign
        .fields
        .iter()
        .find(|field| field.name == "Tok")
        .unwrap();
    assert_eq!(operator.alternatives[0].node_kind, "define_operator");
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let rhs = assign
        .fields
        .iter()
        .find(|field| field.name == "Rhs")
        .unwrap();
    assert_eq!(rhs.alternatives[0].node_kind, "BasicLit");

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
    assert_eq!(sidecar.proof.grammar_shape.fields, 16);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 6);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 12);
    assert_eq!(sidecar.structural_model.references.len(), 12);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "AssignStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "AssignStmt"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_local_integer_short_var_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_integer_short_var_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_integer_short_var_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_INTEGER_SHORT_VAR_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_INTEGER_SHORT_VAR_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "DeclStmt")
    );
    let assign = grammar.rule("AssignStmt").unwrap();
    let operator = assign
        .fields
        .iter()
        .find(|field| field.name == "Tok")
        .unwrap();
    assert_eq!(operator.alternatives[0].node_kind, "define_operator");
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let rhs = assign
        .fields
        .iter()
        .find(|field| field.name == "Rhs")
        .unwrap();
    assert_eq!(rhs.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
    assert_eq!(sidecar.proof.grammar_shape.fields, 16);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 6);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 12);
    assert_eq!(sidecar.structural_model.references.len(), 12);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "AssignStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "AssignStmt"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_integer_return_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_integer_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_integer_return_literal_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_INTEGER_RETURN_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_INTEGER_RETURN_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ReturnStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "DeclStmt")
    );
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
    assert_eq!(sidecar.proof.grammar_shape.fields, 13);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 9);
    assert_eq!(sidecar.structural_model.references.len(), 9);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_parenthesized_integer_return_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_local_parenthesized_integer_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_parenthesized_integer_return_expression_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "ParenExpr");

    let paren_expr = grammar.rule("ParenExpr").unwrap();
    let expr = paren_expr
        .fields
        .iter()
        .find(|field| field.name == "X")
        .unwrap();
    assert_eq!(expr.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
    assert_eq!(sidecar.proof.grammar_shape.fields, 14);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 10);
    assert_eq!(sidecar.structural_model.references.len(), 10);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "ParenExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ParenExpr"
            && reference.field == "X"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_unary_integer_return_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_local_unary_integer_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_unary_integer_return_expression_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "UnaryExpr");

    let unary_expr = grammar.rule("UnaryExpr").unwrap();
    let op = unary_expr
        .fields
        .iter()
        .find(|field| field.name == "Op")
        .unwrap();
    assert_eq!(
        op.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let expr = unary_expr
        .fields
        .iter()
        .find(|field| field.name == "X")
        .unwrap();
    assert_eq!(expr.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
    assert_eq!(sidecar.proof.grammar_shape.fields, 15);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 10);
    assert_eq!(sidecar.structural_model.references.len(), 10);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "UnaryExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "UnaryExpr"
            && reference.field == "X"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "UnaryExpr"
                    && payload.field == "Op"
                    && payload.payload_kind == "string"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_binary_integer_return_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_local_binary_integer_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_binary_integer_return_expression_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "BinaryExpr");

    let binary_expr = grammar.rule("BinaryExpr").unwrap();
    let op = binary_expr
        .fields
        .iter()
        .find(|field| field.name == "Op")
        .unwrap();
    assert_eq!(
        op.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let left = binary_expr
        .fields
        .iter()
        .find(|field| field.name == "X")
        .unwrap();
    assert_eq!(left.alternatives[0].node_kind, "BasicLit");
    let right = binary_expr
        .fields
        .iter()
        .find(|field| field.name == "Y")
        .unwrap();
    assert_eq!(right.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
    assert_eq!(sidecar.proof.grammar_shape.fields, 16);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 11);
    assert_eq!(sidecar.structural_model.references.len(), 11);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "BinaryExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BinaryExpr"
            && reference.field == "X"
            && reference.target_rule == "BasicLit"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BinaryExpr"
            && reference.field == "Y"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BinaryExpr"
                    && payload.field == "Op"
                    && payload.payload_kind == "string"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_call_return_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_call_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_call_return_expression_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_CALL_RETURN_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_CALL_RETURN_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "CallExpr");
    let call = grammar.rule("CallExpr").unwrap();
    let fun = call
        .fields
        .iter()
        .find(|field| field.name == "Fun")
        .unwrap();
    assert_eq!(fun.alternatives[0].node_kind, "Ident");
    assert!(!call.fields.iter().any(|field| field.name == "Args"));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
    assert_eq!(sidecar.proof.grammar_shape.fields, 13);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 3);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 10);
    assert_eq!(sidecar.structural_model.references.len(), 10);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "CallExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CallExpr"
            && reference.field == "Fun"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );
}

#[test]
fn go_local_argument_call_return_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_local_argument_call_return_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_argument_call_return_expression_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "CallExpr");
    let call = grammar.rule("CallExpr").unwrap();
    let fun = call
        .fields
        .iter()
        .find(|field| field.name == "Fun")
        .unwrap();
    assert_eq!(fun.alternatives[0].node_kind, "Ident");
    let args = call
        .fields
        .iter()
        .find(|field| field.name == "Args")
        .unwrap();
    assert!(args.required);
    assert!(args.multiple);
    assert_eq!(args.alternatives[0].node_kind, "BasicLit");
    let literal = grammar.rule("BasicLit").unwrap();
    let literal_value = literal
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(literal_value.alternatives[0].node_kind, "string_literal");
    assert_eq!(
        literal_value.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
    assert_eq!(sidecar.proof.grammar_shape.fields, 15);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 11);
    assert_eq!(sidecar.structural_model.references.len(), 11);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "CallExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CallExpr"
            && reference.field == "Fun"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CallExpr"
            && reference.field == "Args"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "string"
            })
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
}

#[test]
fn go_local_string_return_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_string_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_STRING_RETURN_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_string_return_literal_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_STRING_RETURN_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_STRING_RETURN_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_STRING_RETURN_LITERAL_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ReturnStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "DeclStmt")
    );
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
    assert_eq!(sidecar.proof.grammar_shape.fields, 13);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 9);
    assert_eq!(sidecar.structural_model.references.len(), 9);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "string"
            })
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
}

#[test]
fn go_local_float_return_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_float_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_float_return_literal_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_FLOAT_RETURN_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_FLOAT_RETURN_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ReturnStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "DeclStmt")
    );
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
    assert_eq!(sidecar.proof.grammar_shape.fields, 13);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 9);
    assert_eq!(sidecar.structural_model.references.len(), 9);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "string"
            })
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
}

#[test]
fn go_local_boolean_return_identifier_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_boolean_return_identifier_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_boolean_return_identifier_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_BOOLEAN_RETURN_IDENTIFIER_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_BOOLEAN_RETURN_IDENTIFIER_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ReturnStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "DeclStmt")
    );
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "Ident");

    let ident = grammar.rule("Ident").unwrap();
    let name = ident
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(
        name.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
    assert_eq!(sidecar.proof.grammar_shape.fields, 12);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 3);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 9);
    assert_eq!(sidecar.structural_model.references.len(), 9);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Ident"
                    && payload.field == "Name"
                    && payload.payload_kind == "identifier"
            })
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
}

#[test]
fn go_local_nil_return_identifier_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_nil_return_identifier_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_nil_return_identifier_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_NIL_RETURN_IDENTIFIER_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_NIL_RETURN_IDENTIFIER_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ReturnStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    assert!(
        !statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "DeclStmt")
    );
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "Ident");

    let ident = grammar.rule("Ident").unwrap();
    let name = ident
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(
        name.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
    assert_eq!(sidecar.proof.grammar_shape.fields, 12);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 3);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 9);
    assert_eq!(sidecar.structural_model.references.len(), 9);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Ident"
                    && payload.field == "Name"
                    && payload.payload_kind == "identifier"
            })
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
}

#[test]
fn go_local_typed_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_typed_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_TYPED_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_typed_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_TYPED_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_TYPED_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_TYPED_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let value_spec = grammar.rule("ValueSpec").unwrap();
    let names = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Names")
        .unwrap();
    assert_eq!(names.alternatives[0].node_kind, "Ident");
    let local_type = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(local_type.required);
    assert!(!local_type.multiple);
    assert_eq!(local_type.alternatives[0].node_kind, "Ident");
    let values = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Values")
        .unwrap();
    assert_eq!(values.alternatives[0].node_kind, "BasicLit");

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 12);
    assert_eq!(sidecar.proof.grammar_shape.fields, 19);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 15);
    assert_eq!(sidecar.structural_model.references.len(), 15);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Type"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Values"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_local_zero_value_typed_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_zero_value_typed_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_ZERO_VALUE_TYPED_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_zero_value_typed_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_ZERO_VALUE_TYPED_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_ZERO_VALUE_TYPED_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_ZERO_VALUE_TYPED_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let value_spec = grammar.rule("ValueSpec").unwrap();
    let names = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Names")
        .unwrap();
    assert_eq!(names.alternatives[0].node_kind, "Ident");
    let local_type = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(local_type.required);
    assert!(!local_type.multiple);
    assert_eq!(local_type.alternatives[0].node_kind, "Ident");
    assert!(!value_spec.fields.iter().any(|field| field.name == "Values"));
    assert!(grammar.rule("BasicLit").is_none());

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert_eq!(sidecar.proof.grammar_shape.fields, 17);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 6);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 14);
    assert_eq!(sidecar.structural_model.references.len(), 14);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Type"
            && reference.target_rule == "Ident"
    }));
    assert!(
        !sidecar.structural_model.references.iter().any(|reference| {
            reference.owner_rule == "ValueSpec"
                && reference.field == "Values"
                && reference.target_rule == "BasicLit"
        })
    );
    assert!(
        !sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit")
    );
}

#[test]
fn go_local_integer_typed_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_integer_typed_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_INTEGER_TYPED_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_integer_typed_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_INTEGER_TYPED_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_INTEGER_TYPED_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_INTEGER_TYPED_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let value_spec = grammar.rule("ValueSpec").unwrap();
    let local_type = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(local_type.required);
    assert_eq!(local_type.alternatives[0].node_kind, "Ident");
    let values = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Values")
        .unwrap();
    assert_eq!(values.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 12);
    assert_eq!(sidecar.proof.grammar_shape.fields, 19);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 15);
    assert_eq!(sidecar.structural_model.references.len(), 15);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Type"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Values"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
    );
}

#[test]
fn go_local_untyped_integer_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_untyped_integer_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_UNTYPED_INTEGER_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_local_untyped_integer_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_UNTYPED_INTEGER_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_UNTYPED_INTEGER_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_UNTYPED_INTEGER_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let value_spec = grammar.rule("ValueSpec").unwrap();
    assert!(!value_spec.fields.iter().any(|field| field.name == "Type"));
    let values = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Values")
        .unwrap();
    assert_eq!(values.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 12);
    assert_eq!(sidecar.proof.grammar_shape.fields, 18);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 14);
    assert_eq!(sidecar.structural_model.references.len(), 14);
    assert!(
        !sidecar.structural_model.references.iter().any(|reference| {
            reference.owner_rule == "ValueSpec"
                && reference.field == "Type"
                && reference.target_rule == "Ident"
        })
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Values"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
    );
}

#[test]
fn go_package_type_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_package_type_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(PACKAGE_TYPE_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_package_type_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(PACKAGE_TYPE_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(PACKAGE_TYPE_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(PACKAGE_TYPE_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let decls = file
        .fields
        .iter()
        .find(|field| field.name == "Decls")
        .unwrap();
    assert!(
        decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "GenDecl")
    );
    assert!(
        decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );

    let type_spec = grammar.rule("TypeSpec").unwrap();
    assert!(type_spec.fields.iter().any(|field| field.name == "Name"));
    assert!(type_spec.fields.iter().any(|field| field.name == "Type"));
    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert_eq!(sidecar.proof.grammar_shape.fields, 17);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 13);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Name"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Type"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
    );
}

#[test]
fn go_type_spec_assign_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_type_spec_assign_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(TYPE_SPEC_ASSIGN_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_type_spec_assign_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(TYPE_SPEC_ASSIGN_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(TYPE_SPEC_ASSIGN_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(TYPE_SPEC_ASSIGN_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let type_spec = grammar.rule("TypeSpec").unwrap();
    let assign = type_spec
        .fields
        .iter()
        .find(|field| field.name == "Assign")
        .unwrap();
    assert!(assign.required);
    assert_eq!(assign.alternatives[0].node_kind, "assign_token");
    assert_eq!(
        assign.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    assert!(type_spec.fields.iter().any(|field| field.name == "Name"));
    assert!(type_spec.fields.iter().any(|field| field.name == "Type"));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 22);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 17);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Name"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Type"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "TypeSpec"
                    && payload.field == "Assign"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_type_spec_comment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_type_spec_comment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(TYPE_SPEC_COMMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_type_spec_comment_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(TYPE_SPEC_COMMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(TYPE_SPEC_COMMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(TYPE_SPEC_COMMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    assert!(file.fields.iter().any(|field| {
        field.name == "Comments" && field.alternatives[0].node_kind == "CommentGroup"
    }));
    let type_spec = grammar.rule("TypeSpec").unwrap();
    assert!(type_spec.fields.iter().any(|field| {
        field.name == "Comment" && field.alternatives[0].node_kind == "CommentGroup"
    }));
    assert!(type_spec.fields.iter().any(|field| field.name == "Name"));
    assert!(type_spec.fields.iter().any(|field| field.name == "Type"));
    let comment_group = grammar.rule("CommentGroup").unwrap();
    assert!(
        comment_group
            .fields
            .iter()
            .any(|field| { field.name == "List" && field.alternatives[0].node_kind == "Comment" })
    );
    let comment = grammar.rule("Comment").unwrap();
    assert!(comment.fields.iter().any(|field| {
        field.name == "Text"
            && field.alternatives[0].node_kind == "comment_text"
            && field.alternatives[0].terminal == GrammarTerminalKind::StringPayload
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 22);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 18);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Comment"
            && reference.target_rule == "CommentGroup"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CommentGroup"
            && reference.field == "List"
            && reference.target_rule == "Comment"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_type_spec_doc_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_type_spec_doc_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(TYPE_SPEC_DOC_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_type_spec_doc_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(TYPE_SPEC_DOC_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(TYPE_SPEC_DOC_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(TYPE_SPEC_DOC_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    assert!(file.fields.iter().any(|field| {
        field.name == "Comments" && field.alternatives[0].node_kind == "CommentGroup"
    }));
    let type_spec = grammar.rule("TypeSpec").unwrap();
    assert!(
        type_spec.fields.iter().any(|field| {
            field.name == "Doc" && field.alternatives[0].node_kind == "CommentGroup"
        })
    );
    assert!(type_spec.fields.iter().any(|field| field.name == "Name"));
    assert!(type_spec.fields.iter().any(|field| field.name == "Type"));
    let comment_group = grammar.rule("CommentGroup").unwrap();
    assert!(
        comment_group
            .fields
            .iter()
            .any(|field| { field.name == "List" && field.alternatives[0].node_kind == "Comment" })
    );
    let comment = grammar.rule("Comment").unwrap();
    assert!(comment.fields.iter().any(|field| {
        field.name == "Text"
            && field.alternatives[0].node_kind == "comment_text"
            && field.alternatives[0].terminal == GrammarTerminalKind::StringPayload
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 22);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 18);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Doc"
            && reference.target_rule == "CommentGroup"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CommentGroup"
            && reference.field == "List"
            && reference.target_rule == "Comment"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_type_spec_type_params_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_type_spec_type_params_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(TYPE_SPEC_TYPE_PARAMS_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_type_spec_type_params_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(TYPE_SPEC_TYPE_PARAMS_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(TYPE_SPEC_TYPE_PARAMS_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(TYPE_SPEC_TYPE_PARAMS_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let type_spec = grammar.rule("TypeSpec").unwrap();
    let type_params = type_spec
        .fields
        .iter()
        .find(|field| field.name == "TypeParams")
        .unwrap();
    assert!(type_params.required);
    assert!(!type_params.multiple);
    assert!(
        type_params
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FieldList")
    );
    assert!(type_spec.fields.iter().any(|field| field.name == "Name"));
    assert!(type_spec.fields.iter().any(|field| field.name == "Type"));
    let struct_type = grammar.rule("StructType").unwrap();
    assert!(struct_type.fields.iter().any(|field| {
        field.name == "Fields"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "FieldList")
    }));
    let field = grammar.rule("Field").unwrap();
    assert!(
        field
            .fields
            .iter()
            .any(|field| field.name == "Names" && field.multiple)
    );
    assert!(field.fields.iter().any(|field| {
        field.name == "Type"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ident")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    for (owner, field, target) in [
        ("TypeSpec", "TypeParams", "FieldList"),
        ("TypeSpec", "Name", "Ident"),
        ("TypeSpec", "Type", "StructType"),
        ("StructType", "Fields", "FieldList"),
        ("FieldList", "List", "Field"),
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
}

#[test]
fn go_package_const_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_package_const_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(PACKAGE_CONST_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_package_const_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(PACKAGE_CONST_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(PACKAGE_CONST_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(PACKAGE_CONST_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let decls = file
        .fields
        .iter()
        .find(|field| field.name == "Decls")
        .unwrap();
    assert!(decls.multiple);
    assert!(
        decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "GenDecl")
    );
    assert!(
        decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );

    let gen_decl = grammar.rule("GenDecl").unwrap();
    assert!(gen_decl.fields.iter().any(|field| {
        field.name == "Tok"
            && field.alternatives[0].node_kind == "token"
            && field.alternatives[0].terminal == GrammarTerminalKind::StringPayload
    }));
    assert!(
        gen_decl.fields.iter().any(|field| {
            field.name == "Specs" && field.alternatives[0].node_kind == "ValueSpec"
        })
    );
    let value_spec = grammar.rule("ValueSpec").unwrap();
    assert!(
        value_spec
            .fields
            .iter()
            .any(|field| { field.name == "Names" && field.alternatives[0].node_kind == "Ident" })
    );
    assert!(
        value_spec
            .fields
            .iter()
            .any(|field| { field.name == "Values" && field.alternatives[0].node_kind == "Ident" })
    );
    assert!(!value_spec.fields.iter().any(|field| field.name == "Type"));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
    assert_eq!(sidecar.proof.grammar_shape.fields, 16);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 2);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 15);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "GenDecl"
            && reference.field == "Specs"
            && reference.target_rule == "ValueSpec"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Values"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "GenDecl"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_method_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_method_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(METHOD_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_method_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(METHOD_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(METHOD_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(METHOD_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let func_decl = grammar.rule("FuncDecl").unwrap();
    assert!(func_decl.fields.iter().any(|field| field.name == "Recv"));
    let field = grammar.rule("Field").unwrap();
    let names = field
        .fields
        .iter()
        .find(|field| field.name == "Names")
        .unwrap();
    assert!(!names.required);
    assert!(names.multiple);
    assert!(
        field
            .fields
            .iter()
            .any(|field| field.name == "Type" && field.required)
    );
    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert_eq!(sidecar.proof.grammar_shape.fields, 19);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 6);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 15);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FuncDecl"
            && reference.field == "Recv"
            && reference.target_rule == "FieldList"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Names"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "Ident"
    }));
}

#[test]
fn go_struct_type_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_struct_type_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(STRUCT_TYPE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_struct_type_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(STRUCT_TYPE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(STRUCT_TYPE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(STRUCT_TYPE_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let type_spec = grammar.rule("TypeSpec").unwrap();
    let type_field = type_spec
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(
        type_field
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "StructType")
    );
    let struct_type = grammar.rule("StructType").unwrap();
    assert!(
        struct_type
            .fields
            .iter()
            .any(|field| field.name == "Fields")
    );
    let field = grammar.rule("Field").unwrap();
    assert!(field.fields.iter().any(|field| field.name == "Names"));
    assert!(
        field
            .fields
            .iter()
            .any(|field| field.name == "Type" && field.required)
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 12);
    assert_eq!(sidecar.proof.grammar_shape.fields, 19);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 6);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 15);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Type"
            && reference.target_rule == "StructType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "StructType"
            && reference.field == "Fields"
            && reference.target_rule == "FieldList"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Names"
            && reference.target_rule == "Ident"
    }));
}

#[test]
fn go_composite_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_composite_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(COMPOSITE_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_composite_literal_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(COMPOSITE_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(COMPOSITE_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(COMPOSITE_LITERAL_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert!(results.multiple);
    assert!(
        results
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "CompositeLit")
    );
    let composite_lit = grammar.rule("CompositeLit").unwrap();
    assert!(
        composite_lit
            .fields
            .iter()
            .any(|field| field.name == "Type" && field.required)
    );
    assert!(
        composite_lit
            .fields
            .iter()
            .any(|field| field.name == "Elts" && field.multiple)
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 21);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 17);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Type"
            && reference.target_rule == "StructType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "CompositeLit"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CompositeLit"
            && reference.field == "Type"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CompositeLit"
            && reference.field == "Elts"
            && reference.target_rule == "BasicLit"
    }));
}

#[test]
fn go_key_value_composite_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_key_value_composite_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(KEY_VALUE_COMPOSITE_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_key_value_composite_literal_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(KEY_VALUE_COMPOSITE_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(KEY_VALUE_COMPOSITE_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(KEY_VALUE_COMPOSITE_LITERAL_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let composite_lit = grammar.rule("CompositeLit").unwrap();
    let elements = composite_lit
        .fields
        .iter()
        .find(|field| field.name == "Elts")
        .unwrap();
    assert!(elements.multiple);
    assert!(
        elements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "KeyValueExpr")
    );
    let key_value_expr = grammar.rule("KeyValueExpr").unwrap();
    assert!(
        key_value_expr
            .fields
            .iter()
            .any(|field| field.name == "Key" && field.required)
    );
    assert!(
        key_value_expr
            .fields
            .iter()
            .any(|field| field.name == "Value" && field.required)
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 14);
    assert_eq!(sidecar.proof.grammar_shape.fields, 23);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 19);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CompositeLit"
            && reference.field == "Elts"
            && reference.target_rule == "KeyValueExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "KeyValueExpr"
            && reference.field == "Key"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "KeyValueExpr"
            && reference.field == "Value"
            && reference.target_rule == "BasicLit"
    }));
}

#[test]
fn go_array_composite_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_array_composite_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(ARRAY_COMPOSITE_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_array_composite_literal_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(ARRAY_COMPOSITE_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(ARRAY_COMPOSITE_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(ARRAY_COMPOSITE_LITERAL_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let field = grammar.rule("Field").unwrap();
    assert!(field.fields.iter().any(|field| {
        field.name == "Type"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "ArrayType")
    }));
    let array_type = grammar.rule("ArrayType").unwrap();
    assert!(
        array_type
            .fields
            .iter()
            .any(|field| field.name == "Elt" && field.required)
    );
    let composite_lit = grammar.rule("CompositeLit").unwrap();
    assert!(composite_lit.fields.iter().any(|field| {
        field.name == "Type"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "ArrayType")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert_eq!(sidecar.proof.grammar_shape.fields, 16);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 12);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "ArrayType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArrayType"
            && reference.field == "Elt"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CompositeLit"
            && reference.field == "Type"
            && reference.target_rule == "ArrayType"
    }));
}

#[test]
fn go_index_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_index_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(INDEX_EXPRESSION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_index_expression_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(INDEX_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(INDEX_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(INDEX_EXPRESSION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let field = grammar.rule("Field").unwrap();
    assert!(field.fields.iter().any(|field| {
        field.name == "Names"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ident")
    }));
    assert!(field.fields.iter().any(|field| {
        field.name == "Type"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "ArrayType")
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ident")
    }));
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    assert!(return_stmt.fields.iter().any(|field| {
        field.name == "Results"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "IndexExpr")
    }));
    let index_expr = grammar.rule("IndexExpr").unwrap();
    assert!(index_expr.fields.iter().any(|field| {
        field.name == "X"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ident")
    }));
    assert!(index_expr.fields.iter().any(|field| {
        field.name == "Index"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "BasicLit")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert_eq!(sidecar.proof.grammar_shape.fields, 17);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 14);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "ArrayType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "IndexExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "IndexExpr"
            && reference.field == "X"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "IndexExpr"
            && reference.field == "Index"
            && reference.target_rule == "BasicLit"
    }));
}

#[test]
fn go_slice_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_slice_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(SLICE_EXPRESSION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_slice_expression_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(SLICE_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(SLICE_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(SLICE_EXPRESSION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let field = grammar.rule("Field").unwrap();
    assert!(field.fields.iter().any(|field| {
        field.name == "Type"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "ArrayType")
    }));
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    assert!(return_stmt.fields.iter().any(|field| {
        field.name == "Results"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "SliceExpr")
    }));
    let slice_expr = grammar.rule("SliceExpr").unwrap();
    assert!(slice_expr.fields.iter().any(|field| {
        field.name == "X"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ident")
    }));
    assert!(slice_expr.fields.iter().any(|field| {
        field.name == "Low"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "BasicLit")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert_eq!(sidecar.proof.grammar_shape.fields, 17);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 13);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "ArrayType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "SliceExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SliceExpr"
            && reference.field == "X"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SliceExpr"
            && reference.field == "Low"
            && reference.target_rule == "BasicLit"
    }));
}

#[test]
fn go_slice_expression_three_index_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_slice_expression_three_index_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(SLICE_EXPRESSION_THREE_INDEX_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_slice_expression_three_index_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(SLICE_EXPRESSION_THREE_INDEX_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(SLICE_EXPRESSION_THREE_INDEX_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(SLICE_EXPRESSION_THREE_INDEX_METADATA_PATH)
    );

    let slice_expr = grammar.rule("SliceExpr").unwrap();
    for field_name in ["X", "Low", "High", "Max"] {
        assert!(slice_expr.fields.iter().any(|field| {
            field.name == field_name
                && field
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal)
        }));
    }
    assert!(slice_expr.fields.iter().any(|field| {
        field.name == "Slice3"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.terminal == GrammarTerminalKind::BooleanPayload)
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 23);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 17);
    for field_name in ["Low", "High", "Max"] {
        assert!(sidecar.structural_model.references.iter().any(|reference| {
            reference.owner_rule == "SliceExpr"
                && reference.field == field_name
                && reference.target_rule == "BasicLit"
        }));
    }
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "SliceExpr"
                    && payload.field == "Slice3"
                    && payload.payload_kind == "boolean"
            })
    );
}

#[test]
fn go_star_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_star_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(STAR_EXPRESSION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_star_expression_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(STAR_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(STAR_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(STAR_EXPRESSION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let field = grammar.rule("Field").unwrap();
    assert!(field.fields.iter().any(|field| {
        field.name == "Type"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "StarExpr")
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ident")
    }));
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    assert!(return_stmt.fields.iter().any(|field| {
        field.name == "Results"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "StarExpr")
    }));
    let star_expr = grammar.rule("StarExpr").unwrap();
    assert!(star_expr.fields.iter().any(|field| {
        field.name == "X"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ident")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
    assert_eq!(sidecar.proof.grammar_shape.fields, 14);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 3);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 12);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "StarExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "StarExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "StarExpr"
            && reference.field == "X"
            && reference.target_rule == "Ident"
    }));
}

#[test]
fn go_package_var_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_package_var_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(PACKAGE_VAR_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_package_var_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(PACKAGE_VAR_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(PACKAGE_VAR_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(PACKAGE_VAR_DECLARATION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let decls = file
        .fields
        .iter()
        .find(|field| field.name == "Decls")
        .unwrap();
    assert!(decls.multiple);
    assert!(
        decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "GenDecl")
    );
    assert!(
        decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );

    let gen_decl = grammar.rule("GenDecl").unwrap();
    assert!(
        gen_decl.fields.iter().any(|field| {
            field.name == "Tok" && field.alternatives[0].node_kind == "var_keyword"
        })
    );
    assert!(
        gen_decl.fields.iter().any(|field| {
            field.name == "Specs" && field.alternatives[0].node_kind == "ValueSpec"
        })
    );
    let value_spec = grammar.rule("ValueSpec").unwrap();
    assert!(
        value_spec
            .fields
            .iter()
            .any(|field| { field.name == "Names" && field.alternatives[0].node_kind == "Ident" })
    );
    assert!(
        value_spec
            .fields
            .iter()
            .any(|field| { field.name == "Type" && field.alternatives[0].node_kind == "Ident" })
    );
    assert!(
        value_spec.fields.iter().any(|field| {
            field.name == "Values" && field.alternatives[0].node_kind == "BasicLit"
        })
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert_eq!(sidecar.proof.grammar_shape.fields, 18);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 14);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "GenDecl"
            && reference.field == "Specs"
            && reference.target_rule == "ValueSpec"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Values"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
    );
}

#[test]
fn go_value_spec_comment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_value_spec_comment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(VALUE_SPEC_COMMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_value_spec_comment_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(VALUE_SPEC_COMMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(VALUE_SPEC_COMMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(VALUE_SPEC_COMMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    assert!(file.fields.iter().any(|field| {
        field.name == "Comments" && field.alternatives[0].node_kind == "CommentGroup"
    }));

    let value_spec = grammar.rule("ValueSpec").unwrap();
    assert!(value_spec.fields.iter().any(|field| {
        field.name == "Comment" && field.alternatives[0].node_kind == "CommentGroup"
    }));
    assert!(
        value_spec.fields.iter().any(|field| {
            field.name == "Values" && field.alternatives[0].node_kind == "BasicLit"
        })
    );
    let comment_group = grammar.rule("CommentGroup").unwrap();
    assert!(
        comment_group
            .fields
            .iter()
            .any(|field| { field.name == "List" && field.alternatives[0].node_kind == "Comment" })
    );
    let comment = grammar.rule("Comment").unwrap();
    assert!(comment.fields.iter().any(|field| {
        field.name == "Text"
            && field.alternatives[0].node_kind == "comment_text"
            && field.alternatives[0].terminal == GrammarTerminalKind::StringPayload
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 22);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 18);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Comment"
            && reference.target_rule == "CommentGroup"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CommentGroup"
            && reference.field == "List"
            && reference.target_rule == "Comment"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_import_spec_name_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_import_spec_name_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(IMPORT_SPEC_NAME_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_import_spec_name_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(IMPORT_SPEC_NAME_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(IMPORT_SPEC_NAME_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(IMPORT_SPEC_NAME_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let import_spec = grammar.rule("ImportSpec").unwrap();
    assert!(
        import_spec
            .fields
            .iter()
            .any(|field| { field.name == "Name" && field.alternatives[0].node_kind == "Ident" })
    );
    assert!(
        import_spec
            .fields
            .iter()
            .any(|field| { field.name == "Path" && field.alternatives[0].node_kind == "BasicLit" })
    );
    let ident = grammar.rule("Ident").unwrap();
    assert!(ident.fields.iter().any(|field| {
        field.name == "Name"
            && field.alternatives[0].node_kind == "identifier"
            && field.alternatives[0].terminal == GrammarTerminalKind::IdentifierPayload
    }));
    let basic_lit = grammar.rule("BasicLit").unwrap();
    assert!(basic_lit.fields.iter().any(|field| {
        field.name == "Value"
            && field.alternatives[0].node_kind == "string_literal"
            && field.alternatives[0].terminal == GrammarTerminalKind::StringPayload
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ImportSpec"
            && reference.field == "Name"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ImportSpec"
            && reference.field == "Path"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Ident"
                    && payload.field == "Name"
                    && payload.payload_kind == "identifier"
            })
    );
}

#[test]
fn go_import_spec_comment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_import_spec_comment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(IMPORT_SPEC_COMMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_import_spec_comment_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(IMPORT_SPEC_COMMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(IMPORT_SPEC_COMMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(IMPORT_SPEC_COMMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    assert!(file.fields.iter().any(|field| {
        field.name == "Comments" && field.alternatives[0].node_kind == "CommentGroup"
    }));
    let import_spec = grammar.rule("ImportSpec").unwrap();
    assert!(
        import_spec
            .fields
            .iter()
            .any(|field| { field.name == "Path" && field.alternatives[0].node_kind == "BasicLit" })
    );
    assert!(import_spec.fields.iter().any(|field| {
        field.name == "Comment" && field.alternatives[0].node_kind == "CommentGroup"
    }));
    let comment_group = grammar.rule("CommentGroup").unwrap();
    assert!(
        comment_group
            .fields
            .iter()
            .any(|field| { field.name == "List" && field.alternatives[0].node_kind == "Comment" })
    );
    let comment = grammar.rule("Comment").unwrap();
    assert!(comment.fields.iter().any(|field| {
        field.name == "Text"
            && field.alternatives[0].node_kind == "comment_text"
            && field.alternatives[0].terminal == GrammarTerminalKind::StringPayload
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ImportSpec"
            && reference.field == "Comment"
            && reference.target_rule == "CommentGroup"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CommentGroup"
            && reference.field == "List"
            && reference.target_rule == "Comment"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_function_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_function_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(FUNCTION_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_function_literal_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(FUNCTION_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(FUNCTION_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(FUNCTION_LITERAL_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert!(results.multiple);
    assert!(
        results
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncLit")
    );
    let func_lit = grammar.rule("FuncLit").unwrap();
    assert!(
        func_lit
            .fields
            .iter()
            .any(|field| field.name == "Type" && field.required)
    );
    assert!(
        func_lit
            .fields
            .iter()
            .any(|field| field.name == "Body" && field.required)
    );
    let field = grammar.rule("Field").unwrap();
    let field_type = field
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(
        field_type
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncType")
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
    assert_eq!(sidecar.proof.grammar_shape.fields, 15);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 13);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "FuncLit"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FuncLit"
            && reference.field == "Type"
            && reference.target_rule == "FuncType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FuncLit"
            && reference.field == "Body"
            && reference.target_rule == "BlockStmt"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "BasicLit"
                && payload.field == "Value"
                && payload.payload_kind == "integer")
    );
}

#[test]
fn go_local_generic_function_metadata_lowers_type_params_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_generic_function_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_GENERIC_FUNCTION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_generic_function_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_GENERIC_FUNCTION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_GENERIC_FUNCTION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_GENERIC_FUNCTION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let func_type = grammar.rule("FuncType").unwrap();
    let type_params = func_type
        .fields
        .iter()
        .find(|field| field.name == "TypeParams")
        .unwrap();
    assert!(type_params.required);
    assert!(!type_params.multiple);
    assert!(
        type_params
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FieldList")
    );
    let params = func_type
        .fields
        .iter()
        .find(|field| field.name == "Params")
        .unwrap();
    assert!(params.required);
    let results = func_type
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert!(results.required);
    let field = grammar.rule("Field").unwrap();
    assert!(
        field
            .fields
            .iter()
            .any(|field| field.name == "Names" && field.multiple)
    );
    assert!(field.fields.iter().any(|field| {
        field.name == "Type"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ident")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FuncType"
            && reference.field == "TypeParams"
            && reference.target_rule == "FieldList"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FuncType"
            && reference.field == "Params"
            && reference.target_rule == "FieldList"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FuncType"
            && reference.field == "Results"
            && reference.target_rule == "FieldList"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "Ident"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );
}

#[test]
fn go_concurrency_map_interface_type_assertion_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_concurrency_map_interface_type_assertion_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_concurrency_map_interface_type_assertion_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let type_spec = grammar.rule("TypeSpec").unwrap();
    let type_field = type_spec
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(
        type_field
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "InterfaceType")
    );
    assert!(
        type_field
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "StructType")
    );

    let map_type = grammar.rule("MapType").unwrap();
    let map_value = map_type
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert!(
        map_value
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ChanType")
    );
    let chan_type = grammar.rule("ChanType").unwrap();
    let channel_direction = chan_type
        .fields
        .iter()
        .find(|field| field.name == "Dir")
        .unwrap();
    assert_eq!(
        channel_direction.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    for node_kind in ["DeferStmt", "GoStmt", "SelectStmt", "SendStmt"] {
        assert!(
            statements
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == node_kind)
        );
    }
    let assignment = grammar.rule("AssignStmt").unwrap();
    let rhs = assignment
        .fields
        .iter()
        .find(|field| field.name == "Rhs")
        .unwrap();
    assert!(
        rhs.alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "TypeAssertExpr")
    );
    assert!(
        rhs.alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "UnaryExpr")
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSpec"
            && reference.field == "Type"
            && reference.target_rule == "InterfaceType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "ChanType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "MapType"
            && reference.field == "Value"
            && reference.target_rule == "ChanType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SelectStmt"
            && reference.field == "Body"
            && reference.target_rule == "BlockStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CommClause"
            && reference.field == "Comm"
            && reference.target_rule == "SendStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SendStmt"
            && reference.field == "Value"
            && reference.target_rule == "UnaryExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "TypeAssertExpr"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "ChanType"
                && payload.field == "Dir"
                && payload.payload_kind == "string")
    );
}

#[test]
fn go_local_variadic_ellipsis_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_variadic_ellipsis_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_variadic_ellipsis_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_VARIADIC_ELLIPSIS_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_VARIADIC_ELLIPSIS_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let field = grammar.rule("Field").unwrap();
    let field_type = field
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(
        field_type
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "Ellipsis")
    );
    let ellipsis = grammar.rule("Ellipsis").unwrap();
    let elt = ellipsis
        .fields
        .iter()
        .find(|field| field.name == "Elt")
        .unwrap();
    assert_eq!(elt.alternatives[0].node_kind, "Ident");
    let call_expr = grammar.rule("CallExpr").unwrap();
    let call_ellipsis = call_expr
        .fields
        .iter()
        .find(|field| field.name == "Ellipsis")
        .unwrap();
    assert!(!call_ellipsis.required);
    assert_eq!(call_ellipsis.alternatives[0].node_kind, "ellipsis_token");
    assert_eq!(
        call_ellipsis.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "Ellipsis"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Ellipsis"
            && reference.field == "Elt"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "CallExpr"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "CallExpr"
                && payload.field == "Ellipsis"
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
}

#[test]
fn go_sort_interface_source_file_declaration_prefix_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_sort_interface_source_file_declaration_prefix_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_sort_interface_source_file_declaration_prefix_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(SORT_INTERFACE_SOURCE_FILE_DECLARATION_PREFIX_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let imports = file
        .fields
        .iter()
        .find(|field| field.name == "Imports")
        .unwrap();
    assert!(imports.required);
    assert!(imports.multiple);
    let supported_decls = file
        .fields
        .iter()
        .find(|field| field.name == "SupportedDecls")
        .unwrap();
    assert!(supported_decls.required);
    assert!(supported_decls.multiple);
    assert!(
        supported_decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "GenDecl")
    );
    assert!(
        supported_decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );
    let output_comments = file
        .fields
        .iter()
        .find(|field| field.name == "OutputComments")
        .unwrap();
    assert!(output_comments.required);
    assert!(
        output_comments
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "CommentGroup")
    );

    let gen_decl = grammar.rule("GenDecl").unwrap();
    let specs = gen_decl
        .fields
        .iter()
        .find(|field| field.name == "Specs")
        .unwrap();
    assert!(
        specs
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ImportSpec")
    );
    assert!(
        specs
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "TypeSpec")
    );

    let type_spec = grammar.rule("TypeSpec").unwrap();
    let type_field = type_spec
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(
        type_field
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "StructType")
    );
    assert!(
        type_field
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ArrayType")
    );

    let func_decl = grammar.rule("FuncDecl").unwrap();
    let receiver = func_decl
        .fields
        .iter()
        .find(|field| field.name == "Recv")
        .unwrap();
    assert!(!receiver.required);
    assert!(
        func_decl
            .fields
            .iter()
            .any(|field| field.name == "Body" && field.required)
    );
    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ReturnStmt")
    );
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ExprStmt")
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.proof.grammar_shape.rules >= 23);
    assert!(sidecar.proof.grammar_shape.fields >= 41);
    assert!(sidecar.proof.grammar_shape.repeated_fields >= 10);
    assert!(sidecar.proof.grammar_shape.payload_terminals >= 8);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "File"
                && reference.field == "Imports"
                && reference.target_rule == "GenDecl")
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
                && reference.field == "Lhs"
                && reference.target_rule == "IndexExpr")
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
            .references
            .iter()
            .any(|reference| reference.owner_rule == "FuncLit"
                && reference.field == "Body"
                && reference.target_rule == "BlockStmt")
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
}

#[test]
fn go_sort_search_source_file_example_prefix_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_sort_search_source_file_example_prefix_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_sort_search_source_file_example_prefix_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(SORT_SEARCH_SOURCE_FILE_EXAMPLE_PREFIX_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let example_decl = file
        .fields
        .iter()
        .find(|field| field.name == "ExampleDecl")
        .unwrap();
    assert!(example_decl.required);
    assert!(
        example_decl
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );
    let output_comments = file
        .fields
        .iter()
        .find(|field| field.name == "OutputComments")
        .unwrap();
    assert!(output_comments.required);

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "ExampleDecl"
            && reference.target_rule == "FuncDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "CompositeLit"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "CallExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "IfStmt"
            && reference.field == "Cond"
            && reference.target_rule == "BinaryExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CallExpr"
            && reference.field == "Args"
            && reference.target_rule == "FuncLit"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BinaryExpr"
            && reference.field == "X"
            && reference.target_rule == "IndexExpr"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_sort_search_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_sort_search_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(SORT_SEARCH_SOURCE_FILE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_sort_search_source_file_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(SORT_SEARCH_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(SORT_SEARCH_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(SORT_SEARCH_SOURCE_FILE_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let header_comments = file
        .fields
        .iter()
        .find(|field| field.name == "HeaderComments")
        .unwrap();
    assert!(header_comments.required);
    assert!(header_comments.multiple);
    let supported_decls = file
        .fields
        .iter()
        .find(|field| field.name == "SupportedDecls")
        .unwrap();
    assert!(supported_decls.required);
    assert!(supported_decls.multiple);
    assert!(
        supported_decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );
    let output_comments = file
        .fields
        .iter()
        .find(|field| field.name == "OutputComments")
        .unwrap();
    assert!(output_comments.required);
    assert!(output_comments.multiple);

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "HeaderComments"
            && reference.target_rule == "CommentGroup"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "SupportedDecls"
            && reference.target_rule == "FuncDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "RangeStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "CallExpr"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_strings_search_source_file_finder_prefix_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        GoSyntaxMetadataAdapter::new("go_strings_search_source_file_finder_prefix_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_strings_search_source_file_finder_prefix_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(STRINGS_SEARCH_SOURCE_FILE_FINDER_PREFIX_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let finder_type = file
        .fields
        .iter()
        .find(|field| field.name == "FinderTypeDecl")
        .unwrap();
    assert!(finder_type.required);
    assert!(
        finder_type
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "GenDecl")
    );
    let make_finder = file
        .fields
        .iter()
        .find(|field| field.name == "MakeFinderDecl")
        .unwrap();
    assert!(make_finder.required);
    assert!(
        make_finder
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "FinderTypeDecl"
            && reference.target_rule == "GenDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "MakeFinderDecl"
            && reference.target_rule == "FuncDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "ArrayType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "UnaryExpr"
            && reference.field == "X"
            && reference.target_rule == "CompositeLit"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "RangeStmt"
            && reference.field == "X"
            && reference.target_rule == "SelectorExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "IndexExpr"
            && reference.field == "Index"
            && reference.target_rule == "BinaryExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SliceExpr"
            && reference.field == "High"
            && reference.target_rule == "BinaryExpr"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_strings_search_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_strings_search_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(STRINGS_SEARCH_SOURCE_FILE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_strings_search_source_file_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(STRINGS_SEARCH_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(STRINGS_SEARCH_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(STRINGS_SEARCH_SOURCE_FILE_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let header_comments = file
        .fields
        .iter()
        .find(|field| field.name == "HeaderComments")
        .unwrap();
    assert!(header_comments.required);
    assert!(header_comments.multiple);
    let supported_decls = file
        .fields
        .iter()
        .find(|field| field.name == "SupportedDecls")
        .unwrap();
    assert!(supported_decls.required);
    assert!(supported_decls.multiple);
    assert!(
        supported_decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "GenDecl")
    );
    assert!(
        supported_decls
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "HeaderComments"
            && reference.target_rule == "CommentGroup"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "SupportedDecls"
            && reference.target_rule == "FuncDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FuncDecl"
            && reference.field == "Recv"
            && reference.target_rule == "FieldList"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "StarExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "BranchStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "UnaryExpr"
            && reference.field == "X"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_path_match_source_file_match_prefix_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_path_match_source_file_match_prefix_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_path_match_source_file_match_prefix_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(PATH_MATCH_SOURCE_FILE_MATCH_PREFIX_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    assert!(file.fields.iter().any(|field| {
        field.name == "Imports"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "GenDecl")
    }));
    assert!(file.fields.iter().any(|field| {
        field.name == "ErrBadPatternDecl"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "GenDecl")
    }));
    assert!(file.fields.iter().any(|field| {
        field.name == "MatchDecl"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "FuncDecl")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "ErrBadPatternDecl"
            && reference.target_rule == "GenDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "MatchDecl"
            && reference.target_rule == "FuncDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Values"
            && reference.target_rule == "CallExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "LabeledStmt"
            && reference.field == "Stmt"
            && reference.target_rule == "ForStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ForStmt"
            && reference.field == "Cond"
            && reference.target_rule == "BinaryExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BranchStmt"
            && reference.field == "Label"
            && reference.target_rule == "Ident"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "CallExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SliceExpr"
            && reference.field == "Low"
            && reference.target_rule == "BinaryExpr"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_path_match_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_path_match_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(PATH_MATCH_SOURCE_FILE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_path_match_source_file_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(PATH_MATCH_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(PATH_MATCH_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(PATH_MATCH_SOURCE_FILE_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    assert!(file.fields.iter().any(|field| {
        field.name == "HeaderComments"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "CommentGroup")
    }));
    assert!(file.fields.iter().any(|field| {
        field.name == "SupportedDecls"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "FuncDecl")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "SupportedDecls"
            && reference.target_rule == "FuncDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SwitchStmt"
            && reference.field == "Body"
            && reference.target_rule == "BlockStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CaseClause"
            && reference.field == "Body"
            && reference.target_rule == "BranchStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "UnaryExpr"
            && reference.field == "X"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_os_signal_notify_context_source_span_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_os_signal_notify_context_source_span_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(
            OS_SIGNAL_NOTIFY_CONTEXT_SOURCE_SPAN_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "go_os_signal_notify_context_source_span_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(OS_SIGNAL_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(OS_SIGNAL_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(OS_SIGNAL_NOTIFY_CONTEXT_SOURCE_SPAN_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let function = grammar.rule("FuncDecl").unwrap();
    assert!(function.fields.iter().any(|field| {
        field.name == "Doc"
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "CommentGroup")
    }));
    let block = grammar.rule("BlockStmt").unwrap();
    for target in ["GoStmt", "SelectStmt", "CommClause"] {
        assert!(
            block.fields.iter().any(|field| field.name == "List"
                && field
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.node_kind == target)),
            "missing BlockStmt.List -> {target}"
        );
    }
    let field = grammar.rule("Field").unwrap();
    assert!(field.fields.iter().any(|grammar_field| {
        grammar_field.name == "Type"
            && grammar_field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "Ellipsis")
    }));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ChanType"
                && reference.field == "Value"
                && reference.target_rule == "SelectorExpr")
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_os_signal_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_os_signal_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(OS_SIGNAL_SOURCE_FILE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_os_signal_source_file_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(OS_SIGNAL_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(OS_SIGNAL_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(OS_SIGNAL_SOURCE_FILE_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let file = grammar.rule("File").unwrap();
    let declarations = file
        .fields
        .iter()
        .find(|field| field.name == "Decls")
        .unwrap();
    assert!(declarations.required);
    assert!(declarations.multiple);
    assert!(
        declarations
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "FuncDecl")
    );
    assert!(
        declarations
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "GenDecl")
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "File"
            && reference.field == "Decls"
            && reference.target_rule == "FuncDecl"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "ChanType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "Field"
            && reference.field == "Type"
            && reference.target_rule == "MapType"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "SelectStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "GoStmt"
            && reference.field == "Call"
            && reference.target_rule == "CallExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "InterfaceType"
            && reference.field == "Methods"
            && reference.target_rule == "FieldList"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeAssertExpr"
            && reference.field == "X"
            && reference.target_rule == "SelectorExpr"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Ident"
                    && payload.field == "Name"
                    && payload.payload_kind == "identifier"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "Comment"
                    && payload.field == "Text"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_local_integer_assignment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_integer_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_INTEGER_ASSIGNMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_integer_assignment_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_INTEGER_ASSIGNMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_INTEGER_ASSIGNMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_INTEGER_ASSIGNMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(statements.multiple);
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "AssignStmt")
    );

    let value_spec = grammar.rule("ValueSpec").unwrap();
    assert!(!value_spec.fields.iter().any(|field| field.name == "Type"));
    let values = value_spec
        .fields
        .iter()
        .find(|field| field.name == "Values")
        .unwrap();
    assert_eq!(values.alternatives[0].node_kind, "BasicLit");

    let assign = grammar.rule("AssignStmt").unwrap();
    let operator = assign
        .fields
        .iter()
        .find(|field| field.name == "Tok")
        .unwrap();
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let rhs = assign
        .fields
        .iter()
        .find(|field| field.name == "Rhs")
        .unwrap();
    assert_eq!(rhs.alternatives[0].node_kind, "BasicLit");

    let basic_lit = grammar.rule("BasicLit").unwrap();
    let value = basic_lit
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .unwrap();
    assert_eq!(
        value.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 21);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 17);
    assert_eq!(sidecar.structural_model.references.len(), 17);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ValueSpec"
            && reference.field == "Values"
            && reference.target_rule == "BasicLit"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "AssignStmt"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_compound_assignment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_compound_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_COMPOUND_ASSIGNMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_compound_assignment_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_COMPOUND_ASSIGNMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_COMPOUND_ASSIGNMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_COMPOUND_ASSIGNMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let assign = grammar.rule("AssignStmt").unwrap();
    let operator = assign
        .fields
        .iter()
        .find(|field| field.name == "Tok")
        .unwrap();
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let rhs = assign
        .fields
        .iter()
        .find(|field| field.name == "Rhs")
        .unwrap();
    assert_eq!(rhs.alternatives[0].node_kind, "BasicLit");

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 21);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 17);
    assert_eq!(sidecar.structural_model.references.len(), 17);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AssignStmt"
            && reference.field == "Rhs"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "AssignStmt"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_increment_decrement_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_increment_decrement_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_INCREMENT_DECREMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_increment_decrement_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_INCREMENT_DECREMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_INCREMENT_DECREMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_INCREMENT_DECREMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let inc_dec = grammar.rule("IncDecStmt").unwrap();
    let target = inc_dec
        .fields
        .iter()
        .find(|field| field.name == "X")
        .unwrap();
    assert_eq!(target.alternatives[0].node_kind, "Ident");
    let operator = inc_dec
        .fields
        .iter()
        .find(|field| field.name == "Tok")
        .unwrap();
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.fields, 20);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 16);
    assert_eq!(sidecar.structural_model.references.len(), 16);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "IncDecStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "IncDecStmt"
            && reference.field == "X"
            && reference.target_rule == "Ident"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "IncDecStmt"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_branch_statement_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_branch_statement_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_BRANCH_STATEMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_branch_statement_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_BRANCH_STATEMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_BRANCH_STATEMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_BRANCH_STATEMENT_METADATA_PATH)
    );
    let parser = grammar.provenance.parser.as_ref().unwrap();
    assert_eq!(parser.tool, "go");
    assert_eq!(parser.package.as_deref(), Some("go/parser"));

    let for_stmt = grammar.rule("ForStmt").unwrap();
    let body = for_stmt
        .fields
        .iter()
        .find(|field| field.name == "Body")
        .unwrap();
    assert_eq!(body.alternatives[0].node_kind, "BlockStmt");
    let branch_stmt = grammar.rule("BranchStmt").unwrap();
    let branch_keyword = branch_stmt
        .fields
        .iter()
        .find(|field| field.name == "Tok")
        .unwrap();
    assert_eq!(
        branch_keyword.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let return_stmt = grammar.rule("ReturnStmt").unwrap();
    let results = return_stmt
        .fields
        .iter()
        .find(|field| field.name == "Results")
        .unwrap();
    assert_eq!(results.alternatives[0].node_kind, "BasicLit");

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert_eq!(sidecar.proof.grammar_shape.fields, 15);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 12);
    assert_eq!(sidecar.structural_model.references.len(), 12);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "ForStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "BranchStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ForStmt"
            && reference.field == "Body"
            && reference.target_rule == "BlockStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStmt"
            && reference.field == "Results"
            && reference.target_rule == "BasicLit"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BranchStmt"
                    && payload.field == "Tok"
                    && payload.payload_kind == "string"
            })
    );
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
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
}

#[test]
fn go_local_if_else_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_if_else_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_IF_ELSE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_if_else_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_IF_ELSE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_IF_ELSE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_IF_ELSE_METADATA_PATH)
    );

    let block = grammar.rule("BlockStmt").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "List")
        .unwrap();
    assert!(
        statements
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "IfStmt")
    );

    let if_stmt = grammar.rule("IfStmt").unwrap();
    assert!(if_stmt.fields.iter().any(|field| field.name == "Cond"));
    assert!(if_stmt.fields.iter().any(|field| field.name == "Body"));
    assert!(if_stmt.fields.iter().any(|field| field.name == "Else"));

    let binary = grammar.rule("BinaryExpr").unwrap();
    assert_eq!(
        binary
            .fields
            .iter()
            .find(|field| field.name == "Op")
            .unwrap()
            .alternatives[0]
            .terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 15);
    assert_eq!(sidecar.proof.grammar_shape.fields, 27);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 7);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 23);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "IfStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "IfStmt"
            && reference.field == "Cond"
            && reference.target_rule == "BinaryExpr"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "integer"
            })
    );
}

#[test]
fn go_local_for_condition_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_for_condition_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_FOR_CONDITION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_for_condition_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_FOR_CONDITION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_FOR_CONDITION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_FOR_CONDITION_METADATA_PATH)
    );

    let for_stmt = grammar.rule("ForStmt").unwrap();
    assert!(for_stmt.fields.iter().any(|field| field.name == "Cond"));
    assert!(for_stmt.fields.iter().any(|field| field.name == "Body"));
    let block = grammar.rule("BlockStmt").unwrap();
    assert!(
        block
            .fields
            .iter()
            .find(|field| field.name == "List")
            .unwrap()
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ForStmt")
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 15);
    assert_eq!(sidecar.proof.grammar_shape.fields, 26);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 7);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 22);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ForStmt"
            && reference.field == "Cond"
            && reference.target_rule == "BinaryExpr"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ForStmt"
            && reference.field == "Body"
            && reference.target_rule == "BlockStmt"
    }));
}

#[test]
fn go_local_switch_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_switch_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_SWITCH_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_switch_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_SWITCH_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_SWITCH_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_SWITCH_METADATA_PATH)
    );

    let switch_stmt = grammar.rule("SwitchStmt").unwrap();
    assert!(switch_stmt.fields.iter().any(|field| field.name == "Tag"));
    assert!(switch_stmt.fields.iter().any(|field| field.name == "Body"));
    let case_clause = grammar.rule("CaseClause").unwrap();
    assert!(case_clause.fields.iter().any(|field| field.name == "List"));
    assert!(case_clause.fields.iter().any(|field| field.name == "Body"));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 15);
    assert_eq!(sidecar.proof.grammar_shape.fields, 25);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 11);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 23);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "SwitchStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SwitchStmt"
            && reference.field == "Body"
            && reference.target_rule == "BlockStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CaseClause"
            && reference.field == "Body"
            && reference.target_rule == "AssignStmt"
    }));
}

#[test]
fn go_local_type_switch_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_type_switch_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_TYPE_SWITCH_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_type_switch_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_TYPE_SWITCH_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_TYPE_SWITCH_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_TYPE_SWITCH_METADATA_PATH)
    );

    let type_switch_stmt = grammar.rule("TypeSwitchStmt").unwrap();
    assert!(
        type_switch_stmt
            .fields
            .iter()
            .any(|field| field.name == "Assign")
    );
    assert!(
        type_switch_stmt
            .fields
            .iter()
            .any(|field| field.name == "Body")
    );
    let type_assert_expr = grammar.rule("TypeAssertExpr").unwrap();
    assert!(
        type_assert_expr
            .fields
            .iter()
            .any(|field| field.name == "X")
    );
    let case_clause = grammar.rule("CaseClause").unwrap();
    assert!(case_clause.fields.iter().any(|field| field.name == "List"));
    assert!(case_clause.fields.iter().any(|field| field.name == "Body"));

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 19);
    assert_eq!(sidecar.proof.grammar_shape.fields, 31);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 14);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 7);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 28);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockStmt"
            && reference.field == "List"
            && reference.target_rule == "TypeSwitchStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSwitchStmt"
            && reference.field == "Assign"
            && reference.target_rule == "ExprStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeSwitchStmt"
            && reference.field == "Body"
            && reference.target_rule == "BlockStmt"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "TypeAssertExpr"
            && reference.field == "X"
            && reference.target_rule == "Ident"
    }));
}

#[test]
fn go_local_range_string_metadata_lowers_to_structural_model_sidecar() {
    let adapter = GoSyntaxMetadataAdapter::new("go_local_range_string_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_RANGE_STRING_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "go_local_range_string_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "go_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_RANGE_STRING_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_RANGE_STRING_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_RANGE_STRING_METADATA_PATH)
    );

    let range_stmt = grammar.rule("RangeStmt").unwrap();
    assert!(range_stmt.fields.iter().any(|field| field.name == "X"));
    assert!(range_stmt.fields.iter().any(|field| field.name == "Body"));
    let block = grammar.rule("BlockStmt").unwrap();
    assert!(
        block
            .fields
            .iter()
            .find(|field| field.name == "List")
            .unwrap()
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "RangeStmt")
    );

    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 14);
    assert_eq!(sidecar.proof.grammar_shape.fields, 23);
    assert_eq!(sidecar.proof.grammar_shape.repeated_fields, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 7);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 20);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "RangeStmt"
            && reference.field == "X"
            && reference.target_rule == "BasicLit"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "RangeStmt"
            && reference.field == "Body"
            && reference.target_rule == "BlockStmt"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "BasicLit"
                    && payload.field == "Value"
                    && payload.payload_kind == "string"
            })
    );
}

#[test]
fn go_syntax_token_layout_reserves_bpe_boundary_without_collisions() {
    let layout = go_syntax_token_layout();
    let boundary = layout.reserve_bpe_boundary(16).unwrap();

    assert_eq!(boundary.raw_base_vocab_size, 640);
    assert_eq!(boundary.bpe_start, 640);
    assert_eq!(boundary.bpe_end_exclusive, 656);
    assert_eq!(boundary.pad_token, 656);
    assert_eq!(boundary.vocab_size, 657);
    assert!(!boundary.current_layout_hosts_bpe);
    assert!(
        layout
            .ranges
            .iter()
            .all(|range| range.end_exclusive <= boundary.bpe_start)
    );
    assert_eq!(layout.pad_token, boundary.bpe_start);

    let raw_tokens = go_minimal_function_raw_syntax_tokens(&layout);
    assert!(!raw_tokens.is_empty());
    assert!(
        raw_tokens
            .iter()
            .all(|token| *token < layout.vocab_size && *token != layout.pad_token)
    );
    assert!(raw_tokens.iter().all(|token| *token < boundary.bpe_start));
    assert!(raw_tokens.iter().any(|token| {
        let payload = token_range_by_role(&layout, "utf8_byte_payload");
        (payload.start..payload.end_exclusive).contains(token)
    }));
}

#[test]
fn go_syntax_metadata_rejects_empty_field_alternatives_with_context() {
    let adapter = GoSyntaxMetadataAdapter::new("bad_go");
    let error = adapter
        .lower_metadata_str(
            r#"{
              "artifact_kind": "tct_go_syntax_metadata_v0",
              "version": 0,
              "source": {
                "path": "tests/fixtures/go_syntax_sources/minimal/main.go",
                "sha256": "f0a87ff5993e610218e870cf01576c90d2779efd4278c56db68678ef30a63b78"
              },
              "parser": {
                "tool": "go",
                "version": "go1.24.3"
              },
              "rules": [{
                "name": "FuncDecl",
                "node_kind": "FuncDecl",
                "fields": [{
                  "name": "Name",
                  "required": true,
                  "alternatives": []
                }]
              }]
            }"#,
            None,
            None,
        )
        .unwrap_err();

    assert_eq!(error.phase, "lower");
    assert_eq!(error.rule.as_deref(), Some("FuncDecl"));
    assert_eq!(error.field.as_deref(), Some("Name"));
    assert_eq!(
        error.path.as_deref(),
        Some("rules[0].fields[0].alternatives")
    );
    assert!(
        error
            .to_string()
            .contains("field has no syntax alternatives")
    );
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

fn go_minimal_function_raw_syntax_tokens(layout: &GrammarStructuralTokenLayout) -> Vec<u32> {
    let structural = token_range_by_role(layout, "structural_decisions");
    let utf8_payload = token_range_by_role(layout, "utf8_byte_payload");
    let payload_end = structural.start + 12;
    let mut tokens = vec![structural.start, structural.start + 1];
    push_go_utf8_payload("main", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 2);
    push_go_utf8_payload("fmt", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 3);
    tokens.push(structural.start + 4);
    push_go_utf8_payload("hello", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 5);
    push_go_utf8_payload("name", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 6);
    push_go_utf8_payload("string", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 7);
    push_go_utf8_payload("string", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 8);
    tokens.push(structural.start + 9);
    push_go_utf8_payload("fmt", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 10);
    push_go_utf8_payload("Sprintf", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 11);
    push_go_utf8_payload("hello %s", utf8_payload.start, payload_end, &mut tokens);
    tokens.push(structural.start + 13);
    tokens
}

fn push_go_utf8_payload(value: &str, utf8_start: u32, payload_end: u32, tokens: &mut Vec<u32>) {
    tokens.extend(
        value
            .as_bytes()
            .iter()
            .map(|byte| utf8_start + u32::from(*byte)),
    );
    tokens.push(payload_end);
}

fn token_range_by_role<'a>(
    layout: &'a GrammarStructuralTokenLayout,
    role: &str,
) -> &'a GrammarStructuralTokenRange {
    layout
        .ranges
        .iter()
        .find(|range| range.role == role)
        .unwrap_or_else(|| panic!("missing token range role {role}"))
}
