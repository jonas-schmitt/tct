use std::{fs, path::Path};

use tct::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use tct::grammar_ir::GrammarTerminalKind;
use tct::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
    GrammarStructuralTokenRange, lower_grammar_ir_to_structural_model,
};

const METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/minimal_class/syntax-metadata-v0.json";
const COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/compilation_unit_source_file/syntax-metadata-v0.json";
const COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH: &str = "tests/fixtures/csharp_syntax_metadata/compilation_unit_logger_source_file/syntax-metadata-v0.json";
const COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH: &str = "tests/fixtures/csharp_syntax_metadata/compilation_unit_version_one_source_file/syntax-metadata-v0.json";
const COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH: &str = "tests/fixtures/csharp_syntax_metadata/compilation_unit_iterator_methods_source_file/syntax-metadata-v0.json";
const COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH: &str = "tests/fixtures/csharp_syntax_metadata/compilation_unit_loan_source_file/syntax-metadata-v0.json";
const BLOCK_SCOPED_NAMESPACE_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/block_scoped_namespace/syntax-metadata-v0.json";
const USING_DIRECTIVE_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/using_directive/syntax-metadata-v0.json";
const QUALIFIED_NAME_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/qualified_name/syntax-metadata-v0.json";
const CONSTRUCTOR_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/constructor_declaration/syntax-metadata-v0.json";
const WHILE_POSTFIX_LOOP_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/while_postfix_loop/syntax-metadata-v0.json";
const INTERPOLATED_STRING_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/interpolated_string/syntax-metadata-v0.json";
const MEMBER_ACCESS_EXPRESSION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/member_access_expression/syntax-metadata-v0.json";
const FIELD_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/field_declaration/syntax-metadata-v0.json";
const BASE_LIST_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/base_list/syntax-metadata-v0.json";
const CONDITIONAL_ACCESS_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/conditional_access/syntax-metadata-v0.json";
const ARRAY_RANK_OBJECT_CREATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/array_rank_object_creation/syntax-metadata-v0.json";
const THIS_EXPRESSION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/this_expression/syntax-metadata-v0.json";
const PATTERN_EXPRESSION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/pattern_expression/syntax-metadata-v0.json";
const ARROW_EXPRESSION_CLAUSE_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/arrow_expression_clause/syntax-metadata-v0.json";
const STRUCT_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/struct_declaration/syntax-metadata-v0.json";
const ENUM_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/enum_declaration/syntax-metadata-v0.json";
const EVENT_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/event_declaration/syntax-metadata-v0.json";
const YIELD_STATEMENT_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/yield_statement/syntax-metadata-v0.json";
const ATTRIBUTE_SYNTAX_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/attribute_syntax/syntax-metadata-v0.json";
const GENERIC_NAME_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/generic_name/syntax-metadata-v0.json";
const LOCAL_TYPED_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_typed_declaration/syntax-metadata-v0.json";
const LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_integer_return_literal/syntax-metadata-v0.json";
const LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_float_return_literal/syntax-metadata-v0.json";
const LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_boolean_return_literal/syntax-metadata-v0.json";
const LOCAL_NULL_RETURN_LITERAL_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_null_return_literal/syntax-metadata-v0.json";
const LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_nullable_return_type/syntax-metadata-v0.json";
const LOCAL_NULLABLE_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_nullable_declaration/syntax-metadata-v0.json";
const NULLABLE_PROPERTY_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/nullable_property_declaration/syntax-metadata-v0.json";
const NULLABLE_PARAMETER_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/nullable_parameter_declaration/syntax-metadata-v0.json";
const LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_unary_integer_return/syntax-metadata-v0.json";
const LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH: &str = "tests/fixtures/csharp_syntax_metadata/local_parenthesized_integer_return/syntax-metadata-v0.json";
const LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_binary_integer_return/syntax-metadata-v0.json";
const LOCAL_IMPLICIT_DECLARATION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_implicit_declaration/syntax-metadata-v0.json";
const LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH: &str = "tests/fixtures/csharp_syntax_metadata/local_integer_implicit_declaration/syntax-metadata-v0.json";
const LOCAL_ASSIGNMENT_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_assignment/syntax-metadata-v0.json";
const LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_escaped_assignment/syntax-metadata-v0.json";
const LOCAL_IF_ELSE_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_if_else/syntax-metadata-v0.json";
const LOCAL_FOR_CONDITION_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_for_condition/syntax-metadata-v0.json";
const LOCAL_SWITCH_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_switch/syntax-metadata-v0.json";
const LOCAL_FOREACH_STRING_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/local_foreach_string/syntax-metadata-v0.json";
const TWO_PARAMETER_METHOD_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/two_parameter_method/syntax-metadata-v0.json";
const PROPERTY_PLUS_METHOD_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/property_plus_method/syntax-metadata-v0.json";
const METHOD_INVOCATION_RETURN_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/method_invocation_return/syntax-metadata-v0.json";
const METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH: &str = "tests/fixtures/csharp_syntax_metadata/method_invocation_argument_return/syntax-metadata-v0.json";
const COLLECTION_INITIALIZER_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/collection_initializer/syntax-metadata-v0.json";
const ELEMENT_ACCESS_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/element_access/syntax-metadata-v0.json";
const RANGE_ELEMENT_ACCESS_METADATA_PATH: &str =
    "tests/fixtures/csharp_syntax_metadata/range_element_access/syntax-metadata-v0.json";
const SOURCE_PATH: &str = "tests/fixtures/csharp_syntax_sources/minimal_class/source.cs";
const COMPILATION_UNIT_SOURCE_FILE_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/pattern-matching/Shapes.cs";
const COMPILATION_UNIT_LOGGER_SOURCE_FILE_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/delegates-and-events/Logger.cs";
const COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/events/VersionOne.cs";
const COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/iterators/IteratorMethods.cs";
const COMPILATION_UNIT_LOAN_SOURCE_FILE_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/serialization/Loan.cs";
const BLOCK_SCOPED_NAMESPACE_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/block_scoped_namespace/source.cs";
const USING_DIRECTIVE_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/using_directive/source.cs";
const QUALIFIED_NAME_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/qualified_name/source.cs";
const CONSTRUCTOR_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/constructor_declaration/source.cs";
const WHILE_POSTFIX_LOOP_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/while_postfix_loop/source.cs";
const INTERPOLATED_STRING_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/interpolated_string/source.cs";
const MEMBER_ACCESS_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs";
const FIELD_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/field_declaration/source.cs";
const BASE_LIST_SOURCE_PATH: &str = "tests/fixtures/csharp_syntax_sources/base_list/source.cs";
const CONDITIONAL_ACCESS_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/conditional_access/source.cs";
const ARRAY_RANK_OBJECT_CREATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs";
const THIS_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/this_expression/source.cs";
const PATTERN_EXPRESSION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs";
const ARROW_EXPRESSION_CLAUSE_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs";
const STRUCT_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/struct_declaration/source.cs";
const ENUM_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/enum_declaration/source.cs";
const EVENT_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/event_declaration/source.cs";
const YIELD_STATEMENT_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/yield_statement/source.cs";
const ATTRIBUTE_SYNTAX_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/attribute_syntax/source.cs";
const GENERIC_NAME_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/generic_name/source.cs";
const LOCAL_TYPED_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_typed_declaration/source.cs";
const LOCAL_INTEGER_RETURN_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_integer_return_literal/source.cs";
const LOCAL_FLOAT_RETURN_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_float_return_literal/source.cs";
const LOCAL_BOOLEAN_RETURN_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_boolean_return_literal/source.cs";
const LOCAL_NULL_RETURN_LITERAL_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_null_return_literal/source.cs";
const LOCAL_NULLABLE_RETURN_TYPE_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_nullable_return_type/source.cs";
const LOCAL_NULLABLE_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_nullable_declaration/source.cs";
const NULLABLE_PROPERTY_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/nullable_property_declaration/source.cs";
const NULLABLE_PARAMETER_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/nullable_parameter_declaration/source.cs";
const LOCAL_UNARY_INTEGER_RETURN_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_unary_integer_return/source.cs";
const LOCAL_PARENTHESIZED_INTEGER_RETURN_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_parenthesized_integer_return/source.cs";
const LOCAL_BINARY_INTEGER_RETURN_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_binary_integer_return/source.cs";
const LOCAL_IMPLICIT_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_implicit_declaration/source.cs";
const LOCAL_INTEGER_IMPLICIT_DECLARATION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_integer_implicit_declaration/source.cs";
const LOCAL_ASSIGNMENT_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_assignment/source.cs";
const LOCAL_ESCAPED_ASSIGNMENT_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_escaped_assignment/source.cs";
const LOCAL_IF_ELSE_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_if_else/source.cs";
const LOCAL_FOR_CONDITION_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_for_condition/source.cs";
const LOCAL_SWITCH_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_switch/source.cs";
const LOCAL_FOREACH_STRING_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/local_foreach_string/source.cs";
const TWO_PARAMETER_METHOD_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/two_parameter_method/source.cs";
const PROPERTY_PLUS_METHOD_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/property_plus_method/source.cs";
const METHOD_INVOCATION_RETURN_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/method_invocation_return/source.cs";
const METHOD_INVOCATION_ARGUMENT_RETURN_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/method_invocation_argument_return/source.cs";
const COLLECTION_INITIALIZER_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/collection_initializer/source.cs";
const ELEMENT_ACCESS_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/element_access/source.cs";
const RANGE_ELEMENT_ACCESS_SOURCE_PATH: &str =
    "tests/fixtures/csharp_syntax_sources/range_element_access/source.cs";
const RENDERED_PATH: &str = "tests/fixtures/csharp_syntax_sources/minimal_class/rendered.cs";
const SOURCE_SHA256: &str = "baab4d0526ab5d0d9f33472fba4a90f6e3e11b2a719a47a60d94291ff37094f1";
const COMPILATION_UNIT_SOURCE_FILE_SOURCE_SHA256: &str =
    "2714e85da9030b88559ce8548a3cce2e6f446681002380213c0bbf3e0d4a454e";
const COMPILATION_UNIT_LOGGER_SOURCE_FILE_SOURCE_SHA256: &str =
    "fe3c59d6da3c21872acb23a4d7aae0476a9ef977bbf30b11eda9aa21e1c97a0f";
const COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SOURCE_SHA256: &str =
    "ee5faea8bc7d22031adba420d7662a001386a09af49858fcce81a8aced49ece7";
const COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_SOURCE_SHA256: &str =
    "2f47b8449ee3d6056fa341da57ac691a8c3b13a2776d7fd0b03e847ea8f4e976";
const COMPILATION_UNIT_LOAN_SOURCE_FILE_SOURCE_SHA256: &str =
    "2435ba265f5db79ff296c9514d6c3480d053b348539a80d6bc63e8b2177ed609";
const BLOCK_SCOPED_NAMESPACE_SOURCE_SHA256: &str =
    "7ee5489ba712f2b930642773d25a55b474a2ec821c2c5810945625f9796ebdee";
const USING_DIRECTIVE_SOURCE_SHA256: &str =
    "d087b653047f7c160c8c7ec4d67c8c2a581371947e1d118cc31d1e22d7f71f64";
const QUALIFIED_NAME_SOURCE_SHA256: &str =
    "da5721a7cf98add2d163e276fec3667c7ffffc76fb0cb230274329809d947b2c";
const CONSTRUCTOR_DECLARATION_SOURCE_SHA256: &str =
    "0f9e218bfac7b8da71e3221ac9c23791d9e23509587dae76434900b7fe2de61d";
const WHILE_POSTFIX_LOOP_SOURCE_SHA256: &str =
    "c3c8160a5173c32ba476aef5aae49512ab52be292a4424b17039428cd955f2bb";
const INTERPOLATED_STRING_SOURCE_SHA256: &str =
    "340132cceb796cfea74ebfc6f90315ed669e82a4353d13a049079a789675d3d0";
const MEMBER_ACCESS_EXPRESSION_SOURCE_SHA256: &str =
    "20d9f9ad99fa1c87c0e3d098885327332f888e9563179ab54d70c131fdfbe0d1";
const FIELD_DECLARATION_SOURCE_SHA256: &str =
    "35d96de96ce3f4cc8a0de1ba9e865cfdb0754e8385b862f3b9ffdf87cde30653";
const BASE_LIST_SOURCE_SHA256: &str =
    "d7ffaca8287a1e0afa4d64759a74aa482e8ce5af030253ccafbbce45c08552cd";
const CONDITIONAL_ACCESS_SOURCE_SHA256: &str =
    "e35b1451222635791c5c9797b0744e1dbe137918da53639a9f3f59be113a5079";
const ARRAY_RANK_OBJECT_CREATION_SOURCE_SHA256: &str =
    "9160262a1efbf2dade73efe9dbc564c676b5f904deb59eda61ee0226b88a2f3b";
const THIS_EXPRESSION_SOURCE_SHA256: &str =
    "b56ef51c5a1e99a793ec717aa99acaa475ad6faf621b86bcd85408801242edbd";
const PATTERN_EXPRESSION_SOURCE_SHA256: &str =
    "97ef9cfbf9d15fcf18d249cd16f6001332e28b487edf742bef6a6ca68f383bf9";
const ARROW_EXPRESSION_CLAUSE_SOURCE_SHA256: &str =
    "028e6712c32fb46038ca5b3bbfbb221d9399b9a67e59597e2b9b53f6b58f834f";
const STRUCT_DECLARATION_SOURCE_SHA256: &str =
    "65e74491d4d6e43bb8d30b9aa8f978c2a468fcc9232379b9b7cdabb46299746a";
const ENUM_DECLARATION_SOURCE_SHA256: &str =
    "ba2ac2905a80543d4573de3df15583baefe4777da4e2dfc43b3e4bf2e50851b5";
const EVENT_DECLARATION_SOURCE_SHA256: &str =
    "4a2cba4a881185f9a793bff8c1b4feb5f70dfa8ceaa9e544a268436076a8bee5";
const YIELD_STATEMENT_SOURCE_SHA256: &str =
    "52264c6bb9bcd91d549c3352db44241cc05dc7e1f53f10f3191d8ea2049dd480";
const ATTRIBUTE_SYNTAX_SOURCE_SHA256: &str =
    "0311a0c6549fb706134fd40092728b263a667e517ec67840ed5f24bb37e92bbc";
const GENERIC_NAME_SOURCE_SHA256: &str =
    "efc57a051ca4dbe7984473df1726f073374a8037517c0dc8b047b5723cd7cc31";
const LOCAL_TYPED_DECLARATION_SOURCE_SHA256: &str =
    "5ec8a201e346b7e924039b8ffd9e74c899e428ce545706364eacfcee5f2ae1dc";
const LOCAL_INTEGER_RETURN_LITERAL_SOURCE_SHA256: &str =
    "45b0f29cf7343a89a9a20cc9a7600aab4146b958a9fff26a43c13d769bd579d3";
const LOCAL_FLOAT_RETURN_LITERAL_SOURCE_SHA256: &str =
    "93f820e09afdadc8d8b98e93d0542a7f539865777301d2cd1d2a6a1f8ace1676";
const LOCAL_BOOLEAN_RETURN_LITERAL_SOURCE_SHA256: &str =
    "1349771597f784599b52a8db1e945c4530554c310ec4a8f7cb219a8747084fd6";
const LOCAL_NULL_RETURN_LITERAL_SOURCE_SHA256: &str =
    "2b6fb67267050c089951d4e765247aa7b9ba9101f64c91bcaaf72b61ce60f512";
const LOCAL_NULLABLE_RETURN_TYPE_SOURCE_SHA256: &str =
    "93d9fdf727fa31c1066da2882e683692214b3c2cf884e67d2a0e2feef0c3c709";
const LOCAL_NULLABLE_DECLARATION_SOURCE_SHA256: &str =
    "e1b01192dea1f1365370b5f83ace5b8c186069185b289569df742644c8fce908";
const NULLABLE_PROPERTY_DECLARATION_SOURCE_SHA256: &str =
    "d9079d23234933565e24a217bbfbe71da6aa1857249cbb4b74c9cbf21d9075a3";
const NULLABLE_PARAMETER_DECLARATION_SOURCE_SHA256: &str =
    "f8698bfcbedaad8fda81af1e2e9096750235d7d1cb37d403347fe02cf259208a";
const LOCAL_UNARY_INTEGER_RETURN_SOURCE_SHA256: &str =
    "8fc7592020567fe155aa5a476e3913e2e5c852055f61f8898f37bdbe90c3dbca";
const LOCAL_PARENTHESIZED_INTEGER_RETURN_SOURCE_SHA256: &str =
    "361ebbb92f9c01b190e4fc5376c8b399c857aa186241047d43b779a1bedb7cc1";
const LOCAL_BINARY_INTEGER_RETURN_SOURCE_SHA256: &str =
    "b33512be966caa9487276e25b2da37e89af7991fbbbb287d773d38033cede3ff";
const LOCAL_IMPLICIT_DECLARATION_SOURCE_SHA256: &str =
    "c4bd3dd04ee79d91c491987cee10f52cfead4aa68fc128437c5267225f9e3216";
const LOCAL_INTEGER_IMPLICIT_DECLARATION_SOURCE_SHA256: &str =
    "4914696d612bf1a74e1ba09b34e79ecb350dc82cfdf3861b39215c0894e7f5b6";
const LOCAL_ASSIGNMENT_SOURCE_SHA256: &str =
    "a1f87afe6c9d1c3f1fa40f531b231758a4ff6c82ec93049d061cbb1590a0e0e6";
const LOCAL_ESCAPED_ASSIGNMENT_SOURCE_SHA256: &str =
    "195740a6fb113bc35353ab7c91c87f1e9d920d5788e9c9712370dd2e01ac3705";
const LOCAL_IF_ELSE_SOURCE_SHA256: &str =
    "d8079742a005317be0ac0a0dbcd192c00ecdf39d1e3370648d8ff7d133815517";
const LOCAL_FOR_CONDITION_SOURCE_SHA256: &str =
    "e2c90b4e9ae03b6226599a1b0360201bff9c830da42b6b62379fc520941de3fb";
const LOCAL_SWITCH_SOURCE_SHA256: &str =
    "a9a4978dbce2849c7578887938e3eab22f719eb25cd9908e12fb780b39ad40ee";
const LOCAL_FOREACH_STRING_SOURCE_SHA256: &str =
    "7964206acd68fd653523679789084e206c342fde1d30e950e1d63ddea7c95825";
const TWO_PARAMETER_METHOD_SOURCE_SHA256: &str =
    "508df827929af9c5279e48fbfbcaa2037387793bf417153dcb7f79e170406b6e";
const PROPERTY_PLUS_METHOD_SOURCE_SHA256: &str =
    "d28e9cde461ff48ee8a21e91b127bd93805d2f8f64479e803a8e2e12771a4ac3";
const METHOD_INVOCATION_RETURN_SOURCE_SHA256: &str =
    "50143c760860bb32577068caef4e2151bfc051b63441d0f888f3cb92b14738f6";
const METHOD_INVOCATION_ARGUMENT_RETURN_SOURCE_SHA256: &str =
    "479ba3da7ff626244909505dedff21c2fff0716d4744ea51989ff0c92513bfa9";
const COLLECTION_INITIALIZER_SOURCE_SHA256: &str =
    "4a3f9277a9c500009015e2a3b5cf3b265adb90cfef53cff3bee40870a8dfbe7e";
const ELEMENT_ACCESS_SOURCE_SHA256: &str =
    "aaff86ec34a2b21db133926f9ced98b0d3acf5c79889b15932cea001e451c8d0";
const RANGE_ELEMENT_ACCESS_SOURCE_SHA256: &str =
    "9b5fc31469d2faa6765d11b40b976ffb71566f721687083976fdd9aeb9972928";

#[test]
fn csharp_syntax_metadata_lowers_to_grammar_ir_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_minimal_class_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_minimal_class_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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
    assert_eq!(parser.tool, "dotnet");
    assert_eq!(parser.version, "10.0.203");
    assert_eq!(
        parser.package.as_deref(),
        Some("Microsoft.CodeAnalysis.CSharp")
    );
    assert_eq!(parser.package_version.as_deref(), Some("5.3.0"));

    let property = grammar.rule("PropertyDeclarationSyntax").unwrap();
    let type_field = property
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(
        type_field.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    assert_eq!(type_field.alternatives[0].node_kind, "PredefinedTypeSyntax");
    let initializer = property
        .fields
        .iter()
        .find(|field| field.name == "Initializer")
        .unwrap();
    assert_eq!(
        initializer.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
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
    assert_eq!(
        sidecar.input.provenance.parser.as_ref().unwrap().tool,
        "dotnet"
    );
    assert_eq!(sidecar.proof.grammar_shape.rules, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 4);
    assert_eq!(sidecar.structural_model.references.len(), 4);
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(
                |reference| reference.owner_rule == "PropertyDeclarationSyntax"
                    && reference.field == "Type"
                    && reference.target_rule == "PredefinedTypeSyntax"
            )
    );

    let decision_graph = &sidecar.structural_model.decision_graph;
    let initializer_decision = decision_graph
        .nodes
        .iter()
        .find(|node| node.id == "decision.rule.PropertyDeclarationSyntax.field.Initializer")
        .unwrap();
    assert_eq!(initializer_decision.decision_kind, "payload");
    assert_eq!(
        initializer_decision.token_range.as_deref(),
        Some("csharp.structural_tokens")
    );
    let string_payload = decision_graph
        .nodes
        .iter()
        .find(|node| {
            node.id
                == "decision.rule.PropertyDeclarationSyntax.field.Initializer.alternative.StringLiteralExpressionSyntax"
        })
        .unwrap();
    assert_eq!(string_payload.node_kind, "alternative_payload");
    assert_eq!(
        string_payload.terminal,
        Some(GrammarTerminalKind::StringPayload)
    );
    assert_eq!(string_payload.payload_kind.as_deref(), Some("string"));
    assert_eq!(
        string_payload.token_range.as_deref(),
        Some("csharp.utf8_byte_payload")
    );
    assert!(decision_graph.edges.iter().any(|edge| {
        edge.source == "decision.rule.PropertyDeclarationSyntax.field.Initializer"
            && edge.target
                == "decision.rule.PropertyDeclarationSyntax.field.Initializer.alternative.StringLiteralExpressionSyntax"
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
fn csharp_block_scoped_namespace_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_block_scoped_namespace_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(BLOCK_SCOPED_NAMESPACE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_block_scoped_namespace_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(BLOCK_SCOPED_NAMESPACE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(BLOCK_SCOPED_NAMESPACE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(BLOCK_SCOPED_NAMESPACE_METADATA_PATH)
    );

    let compilation_unit = grammar.rule("CompilationUnitSyntax").unwrap();
    let members = compilation_unit
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        members.alternatives[0].node_kind,
        "NamespaceDeclarationSyntax"
    );
    assert_eq!(
        members.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let namespace = grammar.rule("NamespaceDeclarationSyntax").unwrap();
    let namespace_name = namespace
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(
        namespace_name.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 4);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "NamespaceDeclarationSyntax"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "NamespaceDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "ClassDeclarationSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_using_directive_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_using_directive_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(USING_DIRECTIVE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_using_directive_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(USING_DIRECTIVE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(USING_DIRECTIVE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(USING_DIRECTIVE_METADATA_PATH)
    );

    let compilation_unit = grammar.rule("CompilationUnitSyntax").unwrap();
    let usings = compilation_unit
        .fields
        .iter()
        .find(|field| field.name == "Usings")
        .unwrap();
    assert_eq!(usings.alternatives[0].node_kind, "UsingDirectiveSyntax");
    assert_eq!(
        usings.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let using = grammar.rule("UsingDirectiveSyntax").unwrap();
    let using_name = using
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(
        using_name.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 6);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 5);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "UsingDirectiveSyntax"
                && payload.field == "Name"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "CompilationUnitSyntax"
            && reference.field == "Usings"
            && reference.target_rule == "UsingDirectiveSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_qualified_name_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_qualified_name_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(QUALIFIED_NAME_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_qualified_name_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(QUALIFIED_NAME_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(QUALIFIED_NAME_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(QUALIFIED_NAME_METADATA_PATH)
    );

    let using = grammar.rule("UsingDirectiveSyntax").unwrap();
    let using_name = using
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(using_name.alternatives[0].node_kind, "QualifiedNameSyntax");
    assert_eq!(
        using_name.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let qualified_name = grammar.rule("QualifiedNameSyntax").unwrap();
    let left = qualified_name
        .fields
        .iter()
        .find(|field| field.name == "Left")
        .unwrap();
    assert_eq!(left.alternatives[0].node_kind, "IdentifierNameSyntax");
    let right = qualified_name
        .fields
        .iter()
        .find(|field| field.name == "Right")
        .unwrap();
    assert_eq!(right.alternatives[0].node_kind, "IdentifierNameSyntax");

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 8);
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "IdentifierNameSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_compilation_unit_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_compilation_unit_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_source_file_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(COMPILATION_UNIT_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(COMPILATION_UNIT_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH)
    );

    let compilation_unit = grammar.rule("CompilationUnitSyntax").unwrap();
    let usings = compilation_unit
        .fields
        .iter()
        .find(|field| field.name == "Usings")
        .unwrap();
    assert_eq!(usings.alternatives[0].node_kind, "UsingDirectiveSyntax");
    let members = compilation_unit
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        members.alternatives[0].node_kind,
        "NamespaceDeclarationSyntax"
    );

    let using = grammar.rule("UsingDirectiveSyntax").unwrap();
    let using_name = using
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(using_name.alternatives[0].node_kind, "QualifiedNameSyntax");
    assert_eq!(using_name.alternatives[1].node_kind, "IdentifierNameSyntax");
    let qualified_name = grammar.rule("QualifiedNameSyntax").unwrap();
    let qualified_left = qualified_name
        .fields
        .iter()
        .find(|field| field.name == "Left")
        .unwrap();
    assert_eq!(
        qualified_left.alternatives[0].node_kind,
        "QualifiedNameSyntax"
    );
    assert_eq!(
        qualified_left.alternatives[1].node_kind,
        "IdentifierNameSyntax"
    );

    let namespace = grammar.rule("NamespaceDeclarationSyntax").unwrap();
    let namespace_members = namespace
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        namespace_members.alternatives[0].node_kind,
        "ClassDeclarationSyntax"
    );
    assert_eq!(
        namespace_members.alternatives[1].node_kind,
        "StructDeclarationSyntax"
    );

    let class_decl = grammar.rule("ClassDeclarationSyntax").unwrap();
    let class_members = class_decl
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        class_members.alternatives[0].node_kind,
        "PropertyDeclarationSyntax"
    );
    assert_eq!(
        class_members.alternatives[1].node_kind,
        "ConstructorDeclarationSyntax"
    );
    let struct_decl = grammar.rule("StructDeclarationSyntax").unwrap();
    let struct_members = struct_decl
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        struct_members.alternatives[0].node_kind,
        "PropertyDeclarationSyntax"
    );
    assert_eq!(
        struct_members.alternatives[1].node_kind,
        "ConstructorDeclarationSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 17);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 9);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 24);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "QualifiedNameSyntax"
            && reference.field == "Left"
            && reference.target_rule == "QualifiedNameSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "NamespaceDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "StructDeclarationSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ConstructorDeclarationSyntax"
            && reference.field == "Body"
            && reference.target_rule == "BlockSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "AccessorDeclarationSyntax"
                && payload.field == "Keyword"
                && payload.payload_kind == "string")
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

#[test]
fn csharp_compilation_unit_logger_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_compilation_unit_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_source_file_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(COMPILATION_UNIT_LOGGER_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(COMPILATION_UNIT_LOGGER_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH)
    );

    let file_scoped_namespace = grammar
        .rule("FileScopedNamespaceDeclarationSyntax")
        .unwrap();
    let namespace_members = file_scoped_namespace
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        namespace_members.alternatives[0].node_kind,
        "EnumDeclarationSyntax"
    );
    assert_eq!(
        namespace_members.alternatives[1].node_kind,
        "ClassDeclarationSyntax"
    );
    let class_decl = grammar.rule("ClassDeclarationSyntax").unwrap();
    let class_members = class_decl
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        class_members.alternatives[0].node_kind,
        "FieldDeclarationSyntax"
    );
    assert_eq!(
        class_members.alternatives[1].node_kind,
        "PropertyDeclarationSyntax"
    );
    assert_eq!(
        class_members.alternatives[2].node_kind,
        "MethodDeclarationSyntax"
    );
    let if_statement = grammar.rule("IfStatementSyntax").unwrap();
    let if_condition = if_statement
        .fields
        .iter()
        .find(|field| field.name == "Condition")
        .unwrap();
    assert_eq!(
        if_condition.alternatives[0].node_kind,
        "ParenthesizedExpressionSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.proof.grammar_shape.rules >= 30);
    assert!(sidecar.proof.grammar_shape.payload_terminals >= 10);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "FileScopedNamespaceDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "EnumDeclarationSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ClassDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "MethodDeclarationSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "InterpolatedStringExpressionSyntax"
            && reference.field == "Contents"
            && reference.target_rule == "InterpolationSyntax"
    }));
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

#[test]
fn csharp_compilation_unit_version_one_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new(
        "csharp_compilation_unit_version_one_source_file_syntax_v0",
    );
    let grammar = adapter
        .lower_metadata_file(Path::new(
            COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_version_one_source_file_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH)
    );

    let class_decl = grammar.rule("ClassDeclarationSyntax").unwrap();
    let class_members = class_decl
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        class_members.alternatives[0].node_kind,
        "PropertyDeclarationSyntax"
    );
    assert_eq!(
        class_members.alternatives[1].node_kind,
        "ConstructorDeclarationSyntax"
    );
    assert_eq!(
        class_members.alternatives[2].node_kind,
        "EventFieldDeclarationSyntax"
    );
    assert_eq!(
        class_members.alternatives[3].node_kind,
        "MethodDeclarationSyntax"
    );
    let foreach_statement = grammar.rule("ForEachStatementSyntax").unwrap();
    assert!(
        foreach_statement
            .fields
            .iter()
            .any(|field| field.name == "Statement"
                && field.alternatives[0].node_kind == "BlockSyntax")
    );
    let conditional_access = grammar.rule("ConditionalAccessExpressionSyntax").unwrap();
    assert!(
        conditional_access
            .fields
            .iter()
            .any(|field| field.name == "WhenNotNull"
                && field.alternatives[0].node_kind == "InvocationExpressionSyntax")
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.proof.grammar_shape.rules >= 30);
    assert!(sidecar.proof.grammar_shape.payload_terminals >= 10);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ClassDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "EventFieldDeclarationSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ForEachStatementSyntax"
            && reference.field == "Statement"
            && reference.target_rule == "BlockSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ConditionalAccessExpressionSyntax"
            && reference.field == "WhenNotNull"
            && reference.target_rule == "InvocationExpressionSyntax"
    }));
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

#[test]
fn csharp_compilation_unit_iterator_methods_source_file_metadata_lowers_to_structural_model_sidecar()
 {
    let adapter = CSharpSyntaxMetadataAdapter::new(
        "csharp_compilation_unit_iterator_methods_source_file_syntax_v0",
    );
    let grammar = adapter
        .lower_metadata_file(Path::new(
            COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH,
        ))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_iterator_methods_source_file_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH)
    );

    let class_decl = grammar.rule("ClassDeclarationSyntax").unwrap();
    let class_members = class_decl
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        class_members.alternatives[0].node_kind,
        "MethodDeclarationSyntax"
    );
    let block = grammar.rule("BlockSyntax").unwrap();
    let statement_kinds = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap()
        .alternatives
        .iter()
        .map(|alternative| alternative.node_kind.as_str())
        .collect::<Vec<_>>();
    assert!(statement_kinds.contains(&"YieldStatementSyntax"));
    assert!(statement_kinds.contains(&"WhileStatementSyntax"));
    assert!(statement_kinds.contains(&"ForEachStatementSyntax"));
    assert!(statement_kinds.contains(&"IfStatementSyntax"));
    assert!(statement_kinds.contains(&"ReturnStatementSyntax"));

    let while_statement = grammar.rule("WhileStatementSyntax").unwrap();
    let while_body = while_statement
        .fields
        .iter()
        .find(|field| field.name == "Statement")
        .unwrap();
    assert_eq!(
        while_body
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["YieldStatementSyntax", "IfStatementSyntax"]
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let return_expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert!(
        return_expression
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "ArrayCreationExpressionSyntax")
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.proof.grammar_shape.rules >= 30);
    assert!(sidecar.proof.grammar_shape.payload_terminals >= 10);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "WhileStatementSyntax"
            && reference.field == "Statement"
            && reference.target_rule == "IfStatementSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ForEachStatementSyntax"
            && reference.field == "Statement"
            && reference.target_rule == "YieldStatementSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "ArrayCreationExpressionSyntax"
    }));
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

#[test]
fn csharp_compilation_unit_loan_source_file_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_compilation_unit_loan_source_file_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_compilation_unit_loan_source_file_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(COMPILATION_UNIT_LOAN_SOURCE_FILE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(COMPILATION_UNIT_LOAN_SOURCE_FILE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH)
    );

    let class_decl = grammar.rule("ClassDeclarationSyntax").unwrap();
    let class_members = class_decl
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        class_members
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "PropertyDeclarationSyntax",
            "FieldDeclarationSyntax",
            "EventFieldDeclarationSyntax",
            "ConstructorDeclarationSyntax",
        ]
    );
    assert!(class_decl.fields.iter().any(
        |field| field.name == "BaseList" && field.alternatives[0].node_kind == "BaseListSyntax"
    ));

    let property = grammar.rule("PropertyDeclarationSyntax").unwrap();
    let attribute_lists = property
        .fields
        .iter()
        .find(|field| field.name == "AttributeLists")
        .unwrap();
    assert!(!attribute_lists.required);
    assert!(attribute_lists.multiple);
    assert_eq!(
        attribute_lists.alternatives[0].node_kind,
        "AttributeListSyntax"
    );
    let property_type = property
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(
        property_type
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["PredefinedTypeSyntax", "IdentifierNameSyntax"]
    );

    let variable_declaration = grammar.rule("VariableDeclarationSyntax").unwrap();
    let variable_type = variable_declaration
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert!(
        variable_type
            .alternatives
            .iter()
            .any(|alternative| alternative.node_kind == "NullableTypeSyntax")
    );

    let accessor = grammar.rule("AccessorDeclarationSyntax").unwrap();
    let accessor_body = accessor
        .fields
        .iter()
        .find(|field| field.name == "Body")
        .unwrap();
    assert!(!accessor_body.required);
    assert_eq!(accessor_body.alternatives[0].node_kind, "BlockSyntax");
    let block = grammar.rule("BlockSyntax").unwrap();
    let statement_kinds = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap()
        .alternatives
        .iter()
        .map(|alternative| alternative.node_kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        statement_kinds,
        vec!["ReturnStatementSyntax", "ExpressionStatementSyntax"]
    );

    let conditional_access = grammar.rule("ConditionalAccessExpressionSyntax").unwrap();
    assert!(
        conditional_access
            .fields
            .iter()
            .any(|field| field.name == "WhenNotNull"
                && field.alternatives[0].node_kind == "InvocationExpressionSyntax")
    );
    let argument = grammar.rule("ArgumentSyntax").unwrap();
    let argument_expressions = argument
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap()
        .alternatives
        .iter()
        .map(|alternative| alternative.node_kind.as_str())
        .collect::<Vec<_>>();
    assert!(argument_expressions.contains(&"ThisExpressionSyntax"));
    assert!(argument_expressions.contains(&"ObjectCreationExpressionSyntax"));
    assert!(argument_expressions.contains(&"InvocationExpressionSyntax"));

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert!(sidecar.proof.grammar_shape.rules >= 30);
    assert!(sidecar.proof.grammar_shape.payload_terminals >= 10);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ClassDeclarationSyntax"
            && reference.field == "BaseList"
            && reference.target_rule == "BaseListSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "VariableDeclarationSyntax"
            && reference.field == "Type"
            && reference.target_rule == "NullableTypeSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ConditionalAccessExpressionSyntax"
            && reference.field == "WhenNotNull"
            && reference.target_rule == "InvocationExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ObjectCreationExpressionSyntax"
            && reference.field == "ArgumentList"
            && reference.target_rule == "ArgumentListSyntax"
    }));
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

#[test]
fn csharp_constructor_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_constructor_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CONSTRUCTOR_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_constructor_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(CONSTRUCTOR_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(CONSTRUCTOR_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(CONSTRUCTOR_DECLARATION_METADATA_PATH)
    );

    let class = grammar.rule("ClassDeclarationSyntax").unwrap();
    let members = class
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        members
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["PropertyDeclarationSyntax", "ConstructorDeclarationSyntax"]
    );
    let constructor = grammar.rule("ConstructorDeclarationSyntax").unwrap();
    let parameter_list = constructor
        .fields
        .iter()
        .find(|field| field.name == "ParameterList")
        .unwrap();
    assert_eq!(
        parameter_list.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    assert_eq!(
        parameter_list.alternatives[0].node_kind,
        "ParameterListSyntax"
    );
    let body = constructor
        .fields
        .iter()
        .find(|field| field.name == "Body")
        .unwrap();
    assert_eq!(body.alternatives[0].node_kind, "BlockSyntax");
    let assignment = grammar.rule("AssignmentExpressionSyntax").unwrap();
    let left = assignment
        .fields
        .iter()
        .find(|field| field.name == "Left")
        .unwrap();
    assert_eq!(left.alternatives[0].node_kind, "IdentifierNameSyntax");
    let right = assignment
        .fields
        .iter()
        .find(|field| field.name == "Right")
        .unwrap();
    assert_eq!(right.alternatives[0].node_kind, "IdentifierNameSyntax");

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 14);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 8);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 15);
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
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_while_postfix_loop_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_while_postfix_loop_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(WHILE_POSTFIX_LOOP_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_while_postfix_loop_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(WHILE_POSTFIX_LOOP_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(WHILE_POSTFIX_LOOP_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(WHILE_POSTFIX_LOOP_METADATA_PATH)
    );

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(
        statements
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "LocalDeclarationStatementSyntax",
            "WhileStatementSyntax",
            "ExpressionStatementSyntax",
            "ReturnStatementSyntax"
        ]
    );
    let while_statement = grammar.rule("WhileStatementSyntax").unwrap();
    let condition = while_statement
        .fields
        .iter()
        .find(|field| field.name == "Condition")
        .unwrap();
    assert_eq!(
        condition.alternatives[0].node_kind,
        "BinaryExpressionSyntax"
    );
    let statement = while_statement
        .fields
        .iter()
        .find(|field| field.name == "Statement")
        .unwrap();
    assert_eq!(statement.alternatives[0].node_kind, "BlockSyntax");
    let postfix = grammar.rule("PostfixUnaryExpressionSyntax").unwrap();
    let operand = postfix
        .fields
        .iter()
        .find(|field| field.name == "Operand")
        .unwrap();
    assert_eq!(operand.alternatives[0].node_kind, "IdentifierNameSyntax");
    let postfix_operator = postfix
        .fields
        .iter()
        .find(|field| field.name == "OperatorToken")
        .unwrap();
    assert_eq!(
        postfix_operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 17);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 9);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 21);
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
            .any(
                |payload| payload.owner_rule == "PostfixUnaryExpressionSyntax"
                    && payload.field == "OperatorToken"
                    && payload.payload_kind == "string"
            )
    );
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_interpolated_string_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_interpolated_string_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(INTERPOLATED_STRING_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_interpolated_string_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(INTERPOLATED_STRING_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(INTERPOLATED_STRING_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(INTERPOLATED_STRING_METADATA_PATH)
    );

    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let return_expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        return_expression.alternatives[0].node_kind,
        "InterpolatedStringExpressionSyntax"
    );
    let interpolated = grammar.rule("InterpolatedStringExpressionSyntax").unwrap();
    let contents = interpolated
        .fields
        .iter()
        .find(|field| field.name == "Contents")
        .unwrap();
    assert_eq!(
        contents
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["InterpolatedStringTextSyntax", "InterpolationSyntax"]
    );
    let text = grammar.rule("InterpolatedStringTextSyntax").unwrap();
    let text_token = text
        .fields
        .iter()
        .find(|field| field.name == "TextToken")
        .unwrap();
    assert_eq!(
        text_token.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let interpolation = grammar.rule("InterpolationSyntax").unwrap();
    let interpolation_expression = interpolation
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        interpolation_expression.alternatives[0].node_kind,
        "IdentifierNameSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 7);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 13);
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
            .any(
                |payload| payload.owner_rule == "InterpolatedStringTextSyntax"
                    && payload.field == "TextToken"
                    && payload.payload_kind == "string"
            )
    );
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_member_access_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_member_access_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(MEMBER_ACCESS_EXPRESSION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_member_access_expression_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(MEMBER_ACCESS_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(MEMBER_ACCESS_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(MEMBER_ACCESS_EXPRESSION_METADATA_PATH)
    );

    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let return_expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        return_expression.alternatives[0].node_kind,
        "MemberAccessExpressionSyntax"
    );
    let member_access = grammar.rule("MemberAccessExpressionSyntax").unwrap();
    assert!(
        member_access
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "IdentifierNameSyntax")
    );
    assert!(
        member_access.fields.iter().any(|field| field.name == "Name"
            && field.alternatives[0].node_kind == "IdentifierNameSyntax")
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 9);
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
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_field_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_field_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(FIELD_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_field_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(FIELD_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(FIELD_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(FIELD_DECLARATION_METADATA_PATH)
    );

    let class_rule = grammar.rule("ClassDeclarationSyntax").unwrap();
    let class_members = class_rule
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        class_members.alternatives[0].node_kind,
        "FieldDeclarationSyntax"
    );
    assert_eq!(
        class_members.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let field_rule = grammar.rule("FieldDeclarationSyntax").unwrap();
    let field_declaration = field_rule
        .fields
        .iter()
        .find(|field| field.name == "Declaration")
        .unwrap();
    assert_eq!(
        field_declaration.alternatives[0].node_kind,
        "VariableDeclarationSyntax"
    );
    let declaration_rule = grammar.rule("VariableDeclarationSyntax").unwrap();
    let declaration_type = declaration_rule
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(
        declaration_type.alternatives[0].node_kind,
        "PredefinedTypeSyntax"
    );
    let declaration_variables = declaration_rule
        .fields
        .iter()
        .find(|field| field.name == "Variables")
        .unwrap();
    assert_eq!(
        declaration_variables.alternatives[0].node_kind,
        "VariableDeclaratorSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 6);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "VariableDeclaratorSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );
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
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_base_list_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_base_list_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(BASE_LIST_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_base_list_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(BASE_LIST_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(BASE_LIST_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(BASE_LIST_METADATA_PATH)
    );

    let class_rule = grammar.rule("ClassDeclarationSyntax").unwrap();
    let class_base_list = class_rule
        .fields
        .iter()
        .find(|field| field.name == "BaseList")
        .unwrap();
    assert_eq!(class_base_list.alternatives[0].node_kind, "BaseListSyntax");
    assert_eq!(
        class_base_list.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let base_list_rule = grammar.rule("BaseListSyntax").unwrap();
    let base_list_types = base_list_rule
        .fields
        .iter()
        .find(|field| field.name == "Types")
        .unwrap();
    assert_eq!(
        base_list_types.alternatives[0].node_kind,
        "SimpleBaseTypeSyntax"
    );
    let simple_base_type_rule = grammar.rule("SimpleBaseTypeSyntax").unwrap();
    let simple_base_type = simple_base_type_rule
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(
        simple_base_type.alternatives[0].node_kind,
        "IdentifierNameSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 6);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 3);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 5);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "IdentifierNameSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );
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
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "SimpleBaseTypeSyntax"
            && reference.field == "Type"
            && reference.target_rule == "IdentifierNameSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_conditional_access_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_conditional_access_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(CONDITIONAL_ACCESS_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_conditional_access_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(CONDITIONAL_ACCESS_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(CONDITIONAL_ACCESS_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(CONDITIONAL_ACCESS_METADATA_PATH)
    );

    let expression_statement = grammar.rule("ExpressionStatementSyntax").unwrap();
    let expression = expression_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression.alternatives[0].node_kind,
        "ConditionalAccessExpressionSyntax"
    );
    let conditional_access = grammar.rule("ConditionalAccessExpressionSyntax").unwrap();
    let when_not_null = conditional_access
        .fields
        .iter()
        .find(|field| field.name == "WhenNotNull")
        .unwrap();
    assert_eq!(
        when_not_null.alternatives[0].node_kind,
        "InvocationExpressionSyntax"
    );
    let invocation = grammar.rule("InvocationExpressionSyntax").unwrap();
    let invocation_expression = invocation
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        invocation_expression.alternatives[0].node_kind,
        "MemberBindingExpressionSyntax"
    );
    let member_binding = grammar.rule("MemberBindingExpressionSyntax").unwrap();
    let member_name = member_binding
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(
        member_name.alternatives[0].node_kind,
        "IdentifierNameSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 15);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 16);
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_array_rank_object_creation_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_array_rank_object_creation_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(ARRAY_RANK_OBJECT_CREATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_array_rank_object_creation_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(ARRAY_RANK_OBJECT_CREATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(ARRAY_RANK_OBJECT_CREATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(ARRAY_RANK_OBJECT_CREATION_METADATA_PATH)
    );

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert!(statements.alternatives.iter().any(|alternative| {
        alternative.node_kind == "LocalDeclarationStatementSyntax"
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }));
    assert!(statements.alternatives.iter().any(|alternative| {
        alternative.node_kind == "ReturnStatementSyntax"
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }));
    let array_type = grammar.rule("ArrayTypeSyntax").unwrap();
    let rank_specifiers = array_type
        .fields
        .iter()
        .find(|field| field.name == "RankSpecifiers")
        .unwrap();
    assert_eq!(
        rank_specifiers.alternatives[0].node_kind,
        "ArrayRankSpecifierSyntax"
    );
    let array_rank = grammar.rule("ArrayRankSpecifierSyntax").unwrap();
    let sizes = array_rank
        .fields
        .iter()
        .find(|field| field.name == "Sizes")
        .unwrap();
    assert!(sizes.alternatives.iter().any(|alternative| {
        alternative.node_kind == "OmittedArraySizeExpressionSyntax"
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }));
    assert!(sizes.alternatives.iter().any(|alternative| {
        alternative.node_kind == "LiteralExpressionSyntax"
            && alternative.terminal == GrammarTerminalKind::Nonterminal
    }));
    let array_creation = grammar.rule("ArrayCreationExpressionSyntax").unwrap();
    let array_creation_initializer = array_creation
        .fields
        .iter()
        .find(|field| field.name == "Initializer")
        .unwrap();
    assert!(!array_creation_initializer.required);
    assert_eq!(
        array_creation_initializer.alternatives[0].node_kind,
        "InitializerExpressionSyntax"
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let return_expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        return_expression.alternatives[0].node_kind,
        "ObjectCreationExpressionSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 20);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 8);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 22);
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
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_this_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_this_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(THIS_EXPRESSION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_this_expression_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(THIS_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(THIS_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(THIS_EXPRESSION_METADATA_PATH)
    );

    let argument = grammar.rule("ArgumentSyntax").unwrap();
    let expression = argument
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(expression.alternatives[0].node_kind, "ThisExpressionSyntax");
    assert!(
        grammar
            .rule("ThisExpressionSyntax")
            .unwrap()
            .fields
            .is_empty()
    );
    let conditional_access = grammar.rule("ConditionalAccessExpressionSyntax").unwrap();
    let when_not_null = conditional_access
        .fields
        .iter()
        .find(|field| field.name == "WhenNotNull")
        .unwrap();
    assert_eq!(
        when_not_null.alternatives[0].node_kind,
        "InvocationExpressionSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 16);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 16);
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
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_pattern_expression_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_pattern_expression_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(PATTERN_EXPRESSION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_pattern_expression_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(PATTERN_EXPRESSION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(PATTERN_EXPRESSION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(PATTERN_EXPRESSION_METADATA_PATH)
    );

    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression.alternatives[0].node_kind,
        "IsPatternExpressionSyntax"
    );
    let is_pattern = grammar.rule("IsPatternExpressionSyntax").unwrap();
    let pattern = is_pattern
        .fields
        .iter()
        .find(|field| field.name == "Pattern")
        .unwrap();
    assert_eq!(pattern.alternatives[0].node_kind, "ConstantPatternSyntax");
    let constant = grammar.rule("ConstantPatternSyntax").unwrap();
    let constant_expression = constant
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        constant_expression.alternatives[0].node_kind,
        "LiteralExpressionSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 14);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 9);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 14);
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
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_arrow_expression_clause_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_arrow_expression_clause_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(ARROW_EXPRESSION_CLAUSE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_arrow_expression_clause_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(ARROW_EXPRESSION_CLAUSE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(ARROW_EXPRESSION_CLAUSE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(ARROW_EXPRESSION_CLAUSE_METADATA_PATH)
    );

    let property = grammar.rule("PropertyDeclarationSyntax").unwrap();
    let expression_body = property
        .fields
        .iter()
        .find(|field| field.name == "ExpressionBody")
        .unwrap();
    assert_eq!(
        expression_body.alternatives[0].node_kind,
        "ArrowExpressionClauseSyntax"
    );
    let arrow = grammar.rule("ArrowExpressionClauseSyntax").unwrap();
    let expression = arrow
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression.alternatives[0].node_kind,
        "LiteralExpressionSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 6);
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
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_struct_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_struct_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(STRUCT_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_struct_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(STRUCT_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(STRUCT_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(STRUCT_DECLARATION_METADATA_PATH)
    );

    let namespace = grammar.rule("NamespaceDeclarationSyntax").unwrap();
    let namespace_members = namespace
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        namespace_members.alternatives[0].node_kind,
        "StructDeclarationSyntax"
    );
    assert_eq!(
        namespace_members.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let struct_rule = grammar.rule("StructDeclarationSyntax").unwrap();
    let struct_identifier = struct_rule
        .fields
        .iter()
        .find(|field| field.name == "Identifier")
        .unwrap();
    assert_eq!(
        struct_identifier.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 5);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 5);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 4);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "StructDeclarationSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "NamespaceDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "StructDeclarationSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_enum_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_enum_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(ENUM_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_enum_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(ENUM_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(ENUM_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(ENUM_DECLARATION_METADATA_PATH)
    );

    let namespace = grammar.rule("NamespaceDeclarationSyntax").unwrap();
    let namespace_members = namespace
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        namespace_members.alternatives[0].node_kind,
        "EnumDeclarationSyntax"
    );
    assert_eq!(
        namespace_members.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let enum_rule = grammar.rule("EnumDeclarationSyntax").unwrap();
    let enum_identifier = enum_rule
        .fields
        .iter()
        .find(|field| field.name == "Identifier")
        .unwrap();
    assert_eq!(
        enum_identifier.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let enum_members = enum_rule
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        enum_members.alternatives[0].node_kind,
        "EnumMemberDeclarationSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 4);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 3);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 3);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(
                |payload| payload.owner_rule == "EnumMemberDeclarationSyntax"
                    && payload.field == "Identifier"
                    && payload.payload_kind == "identifier"
            )
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "EnumDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "EnumMemberDeclarationSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_event_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_event_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(EVENT_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_event_declaration_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(EVENT_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(EVENT_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(EVENT_DECLARATION_METADATA_PATH)
    );

    let class_rule = grammar.rule("ClassDeclarationSyntax").unwrap();
    let class_members = class_rule
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert_eq!(
        class_members.alternatives[0].node_kind,
        "EventFieldDeclarationSyntax"
    );
    assert_eq!(
        class_members.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let event_rule = grammar.rule("EventFieldDeclarationSyntax").unwrap();
    let event_declaration = event_rule
        .fields
        .iter()
        .find(|field| field.name == "Declaration")
        .unwrap();
    assert_eq!(
        event_declaration.alternatives[0].node_kind,
        "VariableDeclarationSyntax"
    );
    let declaration_rule = grammar.rule("VariableDeclarationSyntax").unwrap();
    let declaration_type = declaration_rule
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(
        declaration_type.alternatives[0].node_kind,
        "IdentifierNameSyntax"
    );
    let declaration_variables = declaration_rule
        .fields
        .iter()
        .find(|field| field.name == "Variables")
        .unwrap();
    assert_eq!(
        declaration_variables.alternatives[0].node_kind,
        "VariableDeclaratorSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 7);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 4);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 6);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "VariableDeclaratorSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "EventFieldDeclarationSyntax"
            && reference.field == "Declaration"
            && reference.target_rule == "VariableDeclarationSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "VariableDeclarationSyntax"
            && reference.field == "Type"
            && reference.target_rule == "IdentifierNameSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_yield_statement_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_yield_statement_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(YIELD_STATEMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_yield_statement_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(YIELD_STATEMENT_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(YIELD_STATEMENT_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(YIELD_STATEMENT_METADATA_PATH)
    );

    let block_rule = grammar.rule("BlockSyntax").unwrap();
    let statements = block_rule
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(statements.alternatives[0].node_kind, "YieldStatementSyntax");
    assert_eq!(
        statements.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let yield_rule = grammar.rule("YieldStatementSyntax").unwrap();
    let yield_keyword = yield_rule
        .fields
        .iter()
        .find(|field| field.name == "ReturnOrBreakKeyword")
        .unwrap();
    assert_eq!(
        yield_keyword.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let yield_expression = yield_rule
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        yield_expression.alternatives[0].node_kind,
        "LiteralExpressionSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 7);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "YieldStatementSyntax"
                && payload.field == "ReturnOrBreakKeyword"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BlockSyntax"
            && reference.field == "Statements"
            && reference.target_rule == "YieldStatementSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "YieldStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "LiteralExpressionSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_attribute_syntax_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_attribute_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(ATTRIBUTE_SYNTAX_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_attribute_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(ATTRIBUTE_SYNTAX_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(ATTRIBUTE_SYNTAX_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(ATTRIBUTE_SYNTAX_METADATA_PATH)
    );

    let class_rule = grammar.rule("ClassDeclarationSyntax").unwrap();
    let attribute_lists = class_rule
        .fields
        .iter()
        .find(|field| field.name == "AttributeLists")
        .unwrap();
    assert_eq!(
        attribute_lists.alternatives[0].node_kind,
        "AttributeListSyntax"
    );
    assert_eq!(
        attribute_lists.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let attribute_list_rule = grammar.rule("AttributeListSyntax").unwrap();
    let attributes = attribute_list_rule
        .fields
        .iter()
        .find(|field| field.name == "Attributes")
        .unwrap();
    assert_eq!(attributes.alternatives[0].node_kind, "AttributeSyntax");
    let attribute_rule = grammar.rule("AttributeSyntax").unwrap();
    let attribute_name = attribute_rule
        .fields
        .iter()
        .find(|field| field.name == "Name")
        .unwrap();
    assert_eq!(
        attribute_name.alternatives[0].node_kind,
        "IdentifierNameSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 7);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "IdentifierNameSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ClassDeclarationSyntax"
            && reference.field == "AttributeLists"
            && reference.target_rule == "AttributeListSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "AttributeSyntax"
            && reference.field == "Name"
            && reference.target_rule == "IdentifierNameSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_generic_name_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_generic_name_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(GENERIC_NAME_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_generic_name_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(GENERIC_NAME_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(GENERIC_NAME_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(GENERIC_NAME_METADATA_PATH)
    );

    let property_rule = grammar.rule("PropertyDeclarationSyntax").unwrap();
    let property_type = property_rule
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(property_type.alternatives[0].node_kind, "GenericNameSyntax");
    assert_eq!(
        property_type.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let generic_rule = grammar.rule("GenericNameSyntax").unwrap();
    let type_arguments = generic_rule
        .fields
        .iter()
        .find(|field| field.name == "TypeArgumentList")
        .unwrap();
    assert_eq!(
        type_arguments.alternatives[0].node_kind,
        "TypeArgumentListSyntax"
    );
    let argument_rule = grammar.rule("TypeArgumentListSyntax").unwrap();
    let arguments = argument_rule
        .fields
        .iter()
        .find(|field| field.name == "Arguments")
        .unwrap();
    assert_eq!(arguments.alternatives[0].node_kind, "PredefinedTypeSyntax");

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
    assert_eq!(sidecar.proof.grammar_shape.payload_terminals, 6);
    assert_eq!(sidecar.proof.grammar_shape.nonterminal_references, 8);
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "GenericNameSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "PropertyDeclarationSyntax"
            && reference.field == "Type"
            && reference.target_rule == "GenericNameSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "GenericNameSyntax"
            && reference.field == "TypeArgumentList"
            && reference.target_rule == "TypeArgumentListSyntax"
    }));
    assert!(
        !sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed
    );
    assert!(!sidecar.structural_model.boundary.core_compiler_path_changed);
}

#[test]
fn csharp_local_integer_return_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_integer_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_integer_return_literal_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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

    let return_type = grammar.rule("PredefinedTypeSyntax").unwrap();
    let keyword = return_type
        .fields
        .iter()
        .find(|field| field.name == "Keyword")
        .unwrap();
    assert_eq!(keyword.alternatives[0].node_kind, "int_keyword");
    assert_eq!(
        keyword.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "integer_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
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
            .any(|payload| payload.owner_rule == "PredefinedTypeSyntax"
                && payload.field == "Keyword"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.proof.checks["integer_domains_match_integer_payloads"]);
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
}

#[test]
fn csharp_local_float_return_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_float_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_float_return_literal_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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

    let return_type = grammar.rule("PredefinedTypeSyntax").unwrap();
    let keyword = return_type
        .fields
        .iter()
        .find(|field| field.name == "Keyword")
        .unwrap();
    assert_eq!(keyword.alternatives[0].node_kind, "double_keyword");
    assert_eq!(
        keyword.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "float_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "PredefinedTypeSyntax"
                && payload.field == "Keyword"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_local_boolean_return_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_boolean_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_boolean_return_literal_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_BOOLEAN_RETURN_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_BOOLEAN_RETURN_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH)
    );

    let return_type = grammar.rule("PredefinedTypeSyntax").unwrap();
    let keyword = return_type
        .fields
        .iter()
        .find(|field| field.name == "Keyword")
        .unwrap();
    assert_eq!(keyword.alternatives[0].node_kind, "bool_keyword");
    assert_eq!(
        keyword.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "boolean_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "PredefinedTypeSyntax"
                && payload.field == "Keyword"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_local_null_return_literal_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_null_return_literal_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_NULL_RETURN_LITERAL_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_null_return_literal_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_NULL_RETURN_LITERAL_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_NULL_RETURN_LITERAL_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_NULL_RETURN_LITERAL_METADATA_PATH)
    );

    let return_type = grammar.rule("PredefinedTypeSyntax").unwrap();
    let keyword = return_type
        .fields
        .iter()
        .find(|field| field.name == "Keyword")
        .unwrap();
    assert_eq!(keyword.alternatives[0].node_kind, "object_keyword");
    assert_eq!(
        keyword.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "null_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 8);
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "PredefinedTypeSyntax"
                && payload.field == "Keyword"
                && payload.payload_kind == "identifier")
    );
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_local_nullable_return_type_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_nullable_return_type_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_nullable_return_type_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_NULLABLE_RETURN_TYPE_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_NULLABLE_RETURN_TYPE_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH)
    );

    let nullable = grammar.rule("NullableTypeSyntax").unwrap();
    let element_type = nullable
        .fields
        .iter()
        .find(|field| field.name == "ElementType")
        .unwrap();
    assert_eq!(
        element_type.alternatives[0].node_kind,
        "PredefinedTypeSyntax"
    );
    assert_eq!(
        element_type.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let question = nullable
        .fields
        .iter()
        .find(|field| field.name == "QuestionToken")
        .unwrap();
    assert_eq!(question.alternatives[0].node_kind, "nullable_marker");
    assert_eq!(
        question.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let return_type = grammar.rule("PredefinedTypeSyntax").unwrap();
    let keyword = return_type
        .fields
        .iter()
        .find(|field| field.name == "Keyword")
        .unwrap();
    assert_eq!(keyword.alternatives[0].node_kind, "string_keyword");
    assert_eq!(
        keyword.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "null_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
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
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_local_nullable_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_nullable_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_NULLABLE_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_nullable_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_NULLABLE_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_NULLABLE_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_NULLABLE_DECLARATION_METADATA_PATH)
    );

    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    let return_type = method
        .fields
        .iter()
        .find(|field| field.name == "ReturnType")
        .unwrap();
    assert_eq!(
        return_type.alternatives[0].node_kind,
        "PredefinedTypeSyntax"
    );
    let declaration = grammar.rule("VariableDeclarationSyntax").unwrap();
    let local_type = declaration
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(local_type.alternatives[0].node_kind, "NullableTypeSyntax");
    assert_eq!(
        local_type.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let nullable = grammar.rule("NullableTypeSyntax").unwrap();
    let question = nullable
        .fields
        .iter()
        .find(|field| field.name == "QuestionToken")
        .unwrap();
    assert_eq!(question.alternatives[0].node_kind, "nullable_marker");
    assert_eq!(
        question.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "null_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 14);
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
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_nullable_property_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_nullable_property_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(NULLABLE_PROPERTY_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_nullable_property_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(NULLABLE_PROPERTY_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(NULLABLE_PROPERTY_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(NULLABLE_PROPERTY_DECLARATION_METADATA_PATH)
    );

    let property = grammar.rule("PropertyDeclarationSyntax").unwrap();
    let property_type = property
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(
        property_type.alternatives[0].node_kind,
        "NullableTypeSyntax"
    );
    assert_eq!(
        property_type.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let accessor_list = property
        .fields
        .iter()
        .find(|field| field.name == "AccessorList")
        .unwrap();
    assert_eq!(
        accessor_list.alternatives[0].node_kind,
        "AccessorListSyntax"
    );
    let nullable = grammar.rule("NullableTypeSyntax").unwrap();
    let question = nullable
        .fields
        .iter()
        .find(|field| field.name == "QuestionToken")
        .unwrap();
    assert_eq!(question.alternatives[0].node_kind, "nullable_marker");
    assert_eq!(
        question.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let accessors = grammar.rule("AccessorDeclarationSyntax").unwrap();
    let keyword = accessors
        .fields
        .iter()
        .find(|field| field.name == "Keyword")
        .unwrap();
    assert_eq!(
        keyword
            .alternatives
            .iter()
            .map(|alternative| (alternative.node_kind.as_str(), alternative.terminal))
            .collect::<Vec<_>>(),
        vec![
            ("get_accessor_keyword", GrammarTerminalKind::StringPayload),
            ("init_accessor_keyword", GrammarTerminalKind::StringPayload),
        ]
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "null_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
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
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_nullable_parameter_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_nullable_parameter_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(NULLABLE_PARAMETER_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_nullable_parameter_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(NULLABLE_PARAMETER_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(NULLABLE_PARAMETER_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(NULLABLE_PARAMETER_DECLARATION_METADATA_PATH)
    );

    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    let parameter_list = method
        .fields
        .iter()
        .find(|field| field.name == "ParameterList")
        .unwrap();
    assert_eq!(
        parameter_list.alternatives[0].node_kind,
        "ParameterListSyntax"
    );
    let parameters = grammar.rule("ParameterListSyntax").unwrap();
    let parameter_field = parameters
        .fields
        .iter()
        .find(|field| field.name == "Parameters")
        .unwrap();
    assert_eq!(parameter_field.alternatives[0].node_kind, "ParameterSyntax");
    let parameter = grammar.rule("ParameterSyntax").unwrap();
    let parameter_type = parameter
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(
        parameter_type.alternatives[0].node_kind,
        "NullableTypeSyntax"
    );
    assert_eq!(
        parameter_type.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let nullable = grammar.rule("NullableTypeSyntax").unwrap();
    let question = nullable
        .fields
        .iter()
        .find(|field| field.name == "QuestionToken")
        .unwrap();
    assert_eq!(question.alternatives[0].node_kind, "nullable_marker");
    assert_eq!(
        question.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "string_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
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
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_local_unary_integer_return_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_unary_integer_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_unary_integer_return_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_UNARY_INTEGER_RETURN_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_UNARY_INTEGER_RETURN_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH)
    );

    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression.alternatives[0].node_kind,
        "PrefixUnaryExpressionSyntax"
    );
    assert_eq!(
        expression.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let prefix_unary = grammar.rule("PrefixUnaryExpressionSyntax").unwrap();
    let operator = prefix_unary
        .fields
        .iter()
        .find(|field| field.name == "OperatorToken")
        .unwrap();
    assert_eq!(operator.alternatives[0].node_kind, "unary_operator");
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let operand = prefix_unary
        .fields
        .iter()
        .find(|field| field.name == "Operand")
        .unwrap();
    assert_eq!(operand.alternatives[0].node_kind, "LiteralExpressionSyntax");
    assert_eq!(
        operand.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "integer_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
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
    assert!(sidecar.proof.checks["integer_domains_match_integer_payloads"]);
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_local_parenthesized_integer_return_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_local_parenthesized_integer_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_parenthesized_integer_return_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_PARENTHESIZED_INTEGER_RETURN_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_PARENTHESIZED_INTEGER_RETURN_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH)
    );

    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression.alternatives[0].node_kind,
        "ParenthesizedExpressionSyntax"
    );
    assert_eq!(
        expression.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let parenthesized = grammar.rule("ParenthesizedExpressionSyntax").unwrap();
    let inner_expression = parenthesized
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        inner_expression.alternatives[0].node_kind,
        "LiteralExpressionSyntax"
    );
    assert_eq!(
        inner_expression.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "integer_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
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
    assert!(sidecar.proof.checks["integer_domains_match_integer_payloads"]);
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_local_binary_integer_return_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_binary_integer_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_binary_integer_return_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_BINARY_INTEGER_RETURN_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_BINARY_INTEGER_RETURN_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH)
    );

    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression.alternatives[0].node_kind,
        "BinaryExpressionSyntax"
    );
    assert_eq!(
        expression.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let binary_expression = grammar.rule("BinaryExpressionSyntax").unwrap();
    let left = binary_expression
        .fields
        .iter()
        .find(|field| field.name == "Left")
        .unwrap();
    assert_eq!(left.alternatives[0].node_kind, "LiteralExpressionSyntax");
    assert_eq!(
        left.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let operator = binary_expression
        .fields
        .iter()
        .find(|field| field.name == "OperatorToken")
        .unwrap();
    assert_eq!(operator.alternatives[0].node_kind, "binary_operator");
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::StringPayload
    );
    let right = binary_expression
        .fields
        .iter()
        .find(|field| field.name == "Right")
        .unwrap();
    assert_eq!(right.alternatives[0].node_kind, "LiteralExpressionSyntax");
    assert_eq!(
        right.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "integer_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
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
    assert!(sidecar.proof.checks["integer_domains_match_integer_payloads"]);
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
    assert!(sidecar.proof.checks["decision_graph_payloads_match_payload_terminals"]);
}

#[test]
fn csharp_local_typed_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_typed_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_TYPED_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_typed_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(statements.alternatives.len(), 2);
    assert_eq!(
        statements
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["LocalDeclarationStatementSyntax", "ReturnStatementSyntax"]
    );
    let variable = grammar.rule("VariableDeclaratorSyntax").unwrap();
    let initializer = variable
        .fields
        .iter()
        .find(|field| field.name == "Initializer")
        .unwrap();
    assert_eq!(
        initializer.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    assert_eq!(
        initializer.alternatives[0].node_kind,
        "EqualsValueClauseSyntax"
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let return_expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        return_expression.alternatives[0].node_kind,
        "IdentifierNameSyntax"
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "IdentifierNameSyntax"
                && payload.field == "Identifier"
                && payload.payload_kind == "identifier")
    );
}

#[test]
fn csharp_local_implicit_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_implicit_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_IMPLICIT_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_implicit_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_IMPLICIT_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_IMPLICIT_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_IMPLICIT_DECLARATION_METADATA_PATH)
    );

    let declaration = grammar.rule("VariableDeclarationSyntax").unwrap();
    let local_type = declaration
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(local_type.alternatives[0].node_kind, "IdentifierNameSyntax");
    assert_eq!(
        local_type.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );
}

#[test]
fn csharp_local_integer_implicit_declaration_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_local_integer_implicit_declaration_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_integer_implicit_declaration_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_INTEGER_IMPLICIT_DECLARATION_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_INTEGER_IMPLICIT_DECLARATION_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH)
    );

    let return_type = grammar.rule("PredefinedTypeSyntax").unwrap();
    let keyword = return_type
        .fields
        .iter()
        .find(|field| field.name == "Keyword")
        .unwrap();
    assert_eq!(keyword.alternatives[0].node_kind, "int_keyword");
    assert_eq!(
        keyword.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );

    let declaration = grammar.rule("VariableDeclarationSyntax").unwrap();
    let local_type = declaration
        .fields
        .iter()
        .find(|field| field.name == "Type")
        .unwrap();
    assert_eq!(local_type.alternatives[0].node_kind, "IdentifierNameSyntax");
    assert_eq!(
        local_type.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );

    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "integer_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 13);
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
    assert!(sidecar.proof.checks["integer_domains_match_integer_payloads"]);
    assert!(sidecar.proof.checks["decision_graph_token_ranges_known"]);
}

#[test]
fn csharp_local_assignment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_ASSIGNMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_local_assignment_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(statements.alternatives.len(), 3);
    assert_eq!(
        statements
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "LocalDeclarationStatementSyntax",
            "ExpressionStatementSyntax",
            "ReturnStatementSyntax"
        ]
    );
    let expression = grammar.rule("ExpressionStatementSyntax").unwrap();
    let expression_field = expression
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression_field.alternatives[0].terminal,
        GrammarTerminalKind::Nonterminal
    );
    assert_eq!(
        expression_field.alternatives[0].node_kind,
        "AssignmentExpressionSyntax"
    );
    let assignment = grammar.rule("AssignmentExpressionSyntax").unwrap();
    let left = assignment
        .fields
        .iter()
        .find(|field| field.name == "Left")
        .unwrap();
    assert_eq!(left.alternatives[0].node_kind, "IdentifierNameSyntax");
    let right = assignment
        .fields
        .iter()
        .find(|field| field.name == "Right")
        .unwrap();
    assert_eq!(right.alternatives[0].node_kind, "LiteralExpressionSyntax");

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 15);
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
                    && reference.field == "Left"
                    && reference.target_rule == "IdentifierNameSyntax"
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
}

#[test]
fn csharp_local_escaped_assignment_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_escaped_assignment_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_local_escaped_assignment_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(
        statements
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "LocalDeclarationStatementSyntax",
            "ExpressionStatementSyntax",
            "ReturnStatementSyntax"
        ]
    );
    let assignment = grammar.rule("AssignmentExpressionSyntax").unwrap();
    let right = assignment
        .fields
        .iter()
        .find(|field| field.name == "Right")
        .unwrap();
    assert_eq!(right.alternatives[0].node_kind, "LiteralExpressionSyntax");

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 15);
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
}

#[test]
fn csharp_local_if_else_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_if_else_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_IF_ELSE_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_local_if_else_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(
        statements
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "LocalDeclarationStatementSyntax",
            "IfStatementSyntax",
            "ExpressionStatementSyntax",
            "ReturnStatementSyntax"
        ]
    );
    let if_statement = grammar.rule("IfStatementSyntax").unwrap();
    assert!(
        if_statement
            .fields
            .iter()
            .any(|field| field.name == "Condition"
                && field.alternatives[0].node_kind == "BinaryExpressionSyntax")
    );
    assert!(
        if_statement
            .fields
            .iter()
            .any(|field| field.name == "Else"
                && field.alternatives[0].node_kind == "ElseClauseSyntax")
    );
    let binary_expression = grammar.rule("BinaryExpressionSyntax").unwrap();
    let operator = binary_expression
        .fields
        .iter()
        .find(|field| field.name == "OperatorToken")
        .unwrap();
    assert_eq!(operator.alternatives[0].node_kind, "greater_than_token");
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "integer_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 18);
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
}

#[test]
fn csharp_local_for_condition_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_for_condition_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_FOR_CONDITION_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_local_for_condition_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(
        statements
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "LocalDeclarationStatementSyntax",
            "ForStatementSyntax",
            "ExpressionStatementSyntax",
            "ReturnStatementSyntax"
        ]
    );
    let for_statement = grammar.rule("ForStatementSyntax").unwrap();
    assert!(
        for_statement
            .fields
            .iter()
            .any(|field| field.name == "Condition"
                && field.alternatives[0].node_kind == "BinaryExpressionSyntax")
    );
    assert!(
        for_statement
            .fields
            .iter()
            .any(|field| field.name == "Statement"
                && field.alternatives[0].node_kind == "BlockSyntax")
    );
    let binary_expression = grammar.rule("BinaryExpressionSyntax").unwrap();
    let operator = binary_expression
        .fields
        .iter()
        .find(|field| field.name == "OperatorToken")
        .unwrap();
    assert_eq!(operator.alternatives[0].node_kind, "less_than_token");
    assert_eq!(
        operator.alternatives[0].terminal,
        GrammarTerminalKind::IdentifierPayload
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "integer_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 17);
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
}

#[test]
fn csharp_local_switch_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_switch_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_SWITCH_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_local_switch_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
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

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(
        statements
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "LocalDeclarationStatementSyntax",
            "SwitchStatementSyntax",
            "ExpressionStatementSyntax",
            "BreakStatementSyntax",
            "ReturnStatementSyntax"
        ]
    );
    let switch_statement = grammar.rule("SwitchStatementSyntax").unwrap();
    assert!(
        switch_statement
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "IdentifierNameSyntax")
    );
    assert!(
        switch_statement
            .fields
            .iter()
            .any(|field| field.name == "Sections"
                && field.multiple
                && field.alternatives[0].node_kind == "SwitchSectionSyntax")
    );
    let switch_section = grammar.rule("SwitchSectionSyntax").unwrap();
    assert!(
        switch_section
            .fields
            .iter()
            .any(|field| field.name == "Labels"
                && field.multiple
                && field.alternatives[0].node_kind == "CaseSwitchLabelSyntax")
    );
    assert!(switch_section.fields.iter().any(|field| {
        field.name == "Statements"
            && field.multiple
            && field
                .alternatives
                .iter()
                .any(|alternative| alternative.node_kind == "BreakStatementSyntax")
    }));
    let case_label = grammar.rule("CaseSwitchLabelSyntax").unwrap();
    assert!(case_label.fields.iter().any(|field| field.name == "Value"
        && field.alternatives[0].node_kind == "LiteralExpressionSyntax"));
    assert!(
        grammar
            .rule("BreakStatementSyntax")
            .unwrap()
            .fields
            .is_empty()
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(token.alternatives[0].node_kind, "integer_literal");
    assert_eq!(
        token.alternatives[0].terminal,
        GrammarTerminalKind::IntegerPayload
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 19);
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
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SwitchSectionSyntax"
                && reference.field == "Labels"
                && reference.target_rule == "CaseSwitchLabelSyntax")
    );
    assert!(
        sidecar
            .structural_model
            .references
            .iter()
            .any(|reference| reference.owner_rule == "SwitchSectionSyntax"
                && reference.field == "Statements"
                && reference.target_rule == "BreakStatementSyntax")
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
}

#[test]
fn csharp_local_foreach_string_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_local_foreach_string_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(LOCAL_FOREACH_STRING_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_local_foreach_string_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(LOCAL_FOREACH_STRING_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(LOCAL_FOREACH_STRING_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(LOCAL_FOREACH_STRING_METADATA_PATH)
    );

    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(
        statements
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "LocalDeclarationStatementSyntax",
            "ForEachStatementSyntax",
            "ExpressionStatementSyntax",
            "ReturnStatementSyntax"
        ]
    );
    let predefined_type = grammar.rule("PredefinedTypeSyntax").unwrap();
    let keyword = predefined_type
        .fields
        .iter()
        .find(|field| field.name == "Keyword")
        .unwrap();
    assert_eq!(
        keyword
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["int_keyword", "char_keyword"]
    );
    let foreach_statement = grammar.rule("ForEachStatementSyntax").unwrap();
    assert!(
        foreach_statement
            .fields
            .iter()
            .any(|field| field.name == "Type"
                && field.alternatives[0].node_kind == "PredefinedTypeSyntax")
    );
    assert!(foreach_statement.fields.iter().any(|field| {
        field.name == "Identifier"
            && field.alternatives[0].terminal == GrammarTerminalKind::IdentifierPayload
    }));
    assert!(
        foreach_statement
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "LiteralExpressionSyntax")
    );
    assert!(
        foreach_statement
            .fields
            .iter()
            .any(|field| field.name == "Statement"
                && field.alternatives[0].node_kind == "BlockSyntax")
    );
    let literal = grammar.rule("LiteralExpressionSyntax").unwrap();
    let token = literal
        .fields
        .iter()
        .find(|field| field.name == "Token")
        .unwrap();
    assert_eq!(
        token
            .alternatives
            .iter()
            .map(|alternative| (alternative.node_kind.as_str(), alternative.terminal))
            .collect::<Vec<_>>(),
        vec![
            ("integer_literal", GrammarTerminalKind::IntegerPayload),
            ("string_literal", GrammarTerminalKind::StringPayload)
        ]
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 16);
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
}

#[test]
fn csharp_two_parameter_method_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_two_parameter_method_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(TWO_PARAMETER_METHOD_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_two_parameter_method_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(TWO_PARAMETER_METHOD_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(TWO_PARAMETER_METHOD_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(TWO_PARAMETER_METHOD_METADATA_PATH)
    );

    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    assert!(
        method
            .fields
            .iter()
            .any(|field| field.name == "ParameterList"
                && field.alternatives[0].node_kind == "ParameterListSyntax")
    );
    let parameter_list = grammar.rule("ParameterListSyntax").unwrap();
    let parameters = parameter_list
        .fields
        .iter()
        .find(|field| field.name == "Parameters")
        .unwrap();
    assert!(parameters.multiple);
    assert_eq!(parameters.alternatives[0].node_kind, "ParameterSyntax");
    let parameter = grammar.rule("ParameterSyntax").unwrap();
    assert!(
        parameter.fields.iter().any(|field| field.name == "Type"
            && field.alternatives[0].node_kind == "PredefinedTypeSyntax")
    );
    assert!(parameter.fields.iter().any(|field| {
        field.name == "Identifier"
            && field.alternatives[0].terminal == GrammarTerminalKind::IdentifierPayload
    }));
    let block = grammar.rule("BlockSyntax").unwrap();
    let statements = block
        .fields
        .iter()
        .find(|field| field.name == "Statements")
        .unwrap();
    assert_eq!(
        statements.alternatives[0].node_kind,
        "ReturnStatementSyntax"
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    assert!(
        return_statement
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "IdentifierNameSyntax")
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 10);
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
            .references
            .iter()
            .any(|reference| reference.owner_rule == "ParameterSyntax"
                && reference.field == "Type"
                && reference.target_rule == "PredefinedTypeSyntax")
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
}

#[test]
fn csharp_property_plus_method_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_property_plus_method_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(PROPERTY_PLUS_METHOD_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_property_plus_method_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(PROPERTY_PLUS_METHOD_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(PROPERTY_PLUS_METHOD_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(PROPERTY_PLUS_METHOD_METADATA_PATH)
    );

    let class = grammar.rule("ClassDeclarationSyntax").unwrap();
    let members = class
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert!(members.multiple);
    assert_eq!(
        members
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["PropertyDeclarationSyntax", "MethodDeclarationSyntax"]
    );
    let property = grammar.rule("PropertyDeclarationSyntax").unwrap();
    assert!(property.fields.iter().any(|field| {
        field.name == "Identifier"
            && field.alternatives[0].terminal == GrammarTerminalKind::IdentifierPayload
    }));
    assert!(
        property
            .fields
            .iter()
            .any(|field| field.name == "Initializer"
                && field.alternatives[0].terminal == GrammarTerminalKind::StringPayload)
    );
    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    assert!(method.fields.iter().any(|field| field.name == "ReturnType"
        && field.alternatives[0].node_kind == "PredefinedTypeSyntax"));
    assert!(
        method
            .fields
            .iter()
            .any(|field| field.name == "Body" && field.alternatives[0].node_kind == "BlockSyntax")
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    assert!(
        return_statement
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "IdentifierNameSyntax")
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 9);
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
}

#[test]
fn csharp_method_invocation_return_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_method_invocation_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(METHOD_INVOCATION_RETURN_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_method_invocation_return_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(METHOD_INVOCATION_RETURN_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(METHOD_INVOCATION_RETURN_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(METHOD_INVOCATION_RETURN_METADATA_PATH)
    );

    let class = grammar.rule("ClassDeclarationSyntax").unwrap();
    let members = class
        .fields
        .iter()
        .find(|field| field.name == "Members")
        .unwrap();
    assert!(members.multiple);
    assert_eq!(members.alternatives[0].node_kind, "MethodDeclarationSyntax");
    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    assert!(method.fields.iter().any(|field| field.name == "ReturnType"
        && field.alternatives[0].node_kind == "PredefinedTypeSyntax"));
    assert!(
        method
            .fields
            .iter()
            .any(|field| field.name == "Body" && field.alternatives[0].node_kind == "BlockSyntax")
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["InvocationExpressionSyntax", "LiteralExpressionSyntax"]
    );
    let invocation = grammar.rule("InvocationExpressionSyntax").unwrap();
    assert!(
        invocation
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "IdentifierNameSyntax")
    );
    assert!(
        invocation
            .fields
            .iter()
            .any(|field| field.name == "ArgumentList"
                && field.alternatives[0].node_kind == "ArgumentListSyntax")
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "string")
    );
}

#[test]
fn csharp_method_invocation_argument_return_metadata_lowers_to_structural_model_sidecar() {
    let adapter =
        CSharpSyntaxMetadataAdapter::new("csharp_method_invocation_argument_return_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_method_invocation_argument_return_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(METHOD_INVOCATION_ARGUMENT_RETURN_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(METHOD_INVOCATION_ARGUMENT_RETURN_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH)
    );

    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    assert!(
        method
            .fields
            .iter()
            .any(|field| field.name == "ParameterList"
                && field.alternatives[0].node_kind == "ParameterListSyntax")
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    let expression = return_statement
        .fields
        .iter()
        .find(|field| field.name == "Expression")
        .unwrap();
    assert_eq!(
        expression
            .alternatives
            .iter()
            .map(|alternative| alternative.node_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["InvocationExpressionSyntax", "IdentifierNameSyntax"]
    );
    let argument_list = grammar.rule("ArgumentListSyntax").unwrap();
    assert!(
        argument_list
            .fields
            .iter()
            .any(|field| field.name == "Arguments"
                && field.multiple
                && field.alternatives[0].node_kind == "ArgumentSyntax")
    );
    let argument = grammar.rule("ArgumentSyntax").unwrap();
    assert!(
        argument
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "LiteralExpressionSyntax")
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 14);
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
}

#[test]
fn csharp_collection_initializer_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_collection_initializer_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(COLLECTION_INITIALIZER_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(
        grammar.grammar_id,
        "csharp_collection_initializer_syntax_v0"
    );
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(COLLECTION_INITIALIZER_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(COLLECTION_INITIALIZER_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(COLLECTION_INITIALIZER_METADATA_PATH)
    );

    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    assert!(
        method.fields.iter().any(|field| field.name == "ReturnType"
            && field.alternatives[0].node_kind == "ArrayTypeSyntax")
    );
    let array_type = grammar.rule("ArrayTypeSyntax").unwrap();
    assert!(
        array_type
            .fields
            .iter()
            .any(|field| field.name == "ElementType"
                && field.alternatives[0].node_kind == "PredefinedTypeSyntax")
    );
    let array_creation = grammar.rule("ArrayCreationExpressionSyntax").unwrap();
    assert!(
        array_creation
            .fields
            .iter()
            .any(|field| field.name == "Type"
                && field.alternatives[0].node_kind == "ArrayTypeSyntax")
    );
    assert!(
        array_creation
            .fields
            .iter()
            .any(|field| field.name == "Initializer"
                && field.alternatives[0].node_kind == "InitializerExpressionSyntax")
    );
    let initializer = grammar.rule("InitializerExpressionSyntax").unwrap();
    assert!(initializer.fields.iter().any(|field| {
        field.name == "Expressions"
            && field.multiple
            && field.alternatives[0].node_kind == "LiteralExpressionSyntax"
    }));

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 11);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArrayTypeSyntax"
            && reference.field == "ElementType"
            && reference.target_rule == "PredefinedTypeSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArrayCreationExpressionSyntax"
            && reference.field == "Initializer"
            && reference.target_rule == "InitializerExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "InitializerExpressionSyntax"
            && reference.field == "Expressions"
            && reference.target_rule == "LiteralExpressionSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "LiteralExpressionSyntax"
                    && payload.field == "Token"
                    && payload.payload_kind == "integer"
            })
    );
}

#[test]
fn csharp_element_access_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_element_access_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(ELEMENT_ACCESS_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_element_access_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(ELEMENT_ACCESS_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(ELEMENT_ACCESS_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(ELEMENT_ACCESS_METADATA_PATH)
    );

    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    assert!(
        method
            .fields
            .iter()
            .any(|field| field.name == "ParameterList"
                && field.alternatives[0].node_kind == "ParameterListSyntax")
    );
    let parameter = grammar.rule("ParameterSyntax").unwrap();
    assert!(
        parameter
            .fields
            .iter()
            .any(|field| field.name == "Type"
                && field.alternatives[0].node_kind == "ArrayTypeSyntax")
    );
    let array_type = grammar.rule("ArrayTypeSyntax").unwrap();
    assert!(
        array_type
            .fields
            .iter()
            .any(|field| field.name == "ElementType"
                && field.alternatives[0].node_kind == "PredefinedTypeSyntax")
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    assert!(
        return_statement
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "ElementAccessExpressionSyntax")
    );
    let element_access = grammar.rule("ElementAccessExpressionSyntax").unwrap();
    assert!(element_access.fields.iter().any(|field| {
        field.name == "Expression" && field.alternatives[0].node_kind == "IdentifierNameSyntax"
    }));
    assert!(element_access.fields.iter().any(|field| {
        field.name == "ArgumentList"
            && field.alternatives[0].node_kind == "BracketedArgumentListSyntax"
    }));
    let bracketed_argument_list = grammar.rule("BracketedArgumentListSyntax").unwrap();
    assert!(bracketed_argument_list.fields.iter().any(|field| {
        field.name == "Arguments"
            && field.multiple
            && field.alternatives[0].node_kind == "ArgumentSyntax"
    }));
    let argument = grammar.rule("ArgumentSyntax").unwrap();
    assert!(
        argument
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "LiteralExpressionSyntax")
    );

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 15);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ParameterSyntax"
            && reference.field == "Type"
            && reference.target_rule == "ArrayTypeSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ReturnStatementSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "ElementAccessExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ElementAccessExpressionSyntax"
            && reference.field == "ArgumentList"
            && reference.target_rule == "BracketedArgumentListSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "BracketedArgumentListSyntax"
            && reference.field == "Arguments"
            && reference.target_rule == "ArgumentSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArgumentSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "LiteralExpressionSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer")
    );
}

#[test]
fn csharp_range_element_access_metadata_lowers_to_structural_model_sidecar() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_range_element_access_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(RANGE_ELEMENT_ACCESS_METADATA_PATH))
        .unwrap();

    assert_eq!(grammar.version, 1);
    assert_eq!(grammar.grammar_id, "csharp_range_element_access_syntax_v0");
    assert_eq!(grammar.provenance.source_kind, "csharp_syntax_metadata");
    assert_eq!(
        grammar.provenance.source_path.as_deref(),
        Some(RANGE_ELEMENT_ACCESS_SOURCE_PATH)
    );
    assert_eq!(
        grammar.provenance.source_sha256.as_deref(),
        Some(RANGE_ELEMENT_ACCESS_SOURCE_SHA256)
    );
    assert_eq!(
        grammar.provenance.metadata_path.as_deref(),
        Some(RANGE_ELEMENT_ACCESS_METADATA_PATH)
    );

    let method = grammar.rule("MethodDeclarationSyntax").unwrap();
    assert!(
        method.fields.iter().any(|field| field.name == "ReturnType"
            && field.alternatives[0].node_kind == "ArrayTypeSyntax")
    );
    assert!(
        method
            .fields
            .iter()
            .any(|field| field.name == "ParameterList"
                && field.alternatives[0].node_kind == "ParameterListSyntax")
    );
    let return_statement = grammar.rule("ReturnStatementSyntax").unwrap();
    assert!(
        return_statement
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "ElementAccessExpressionSyntax")
    );
    let argument = grammar.rule("ArgumentSyntax").unwrap();
    assert!(
        argument
            .fields
            .iter()
            .any(|field| field.name == "Expression"
                && field.alternatives[0].node_kind == "RangeExpressionSyntax")
    );
    let range = grammar.rule("RangeExpressionSyntax").unwrap();
    assert!(range.fields.iter().any(|field| {
        field.name == "LeftOperand" && field.alternatives[0].node_kind == "LiteralExpressionSyntax"
    }));
    assert!(range.fields.iter().any(|field| {
        field.name == "RightOperand" && field.alternatives[0].node_kind == "LiteralExpressionSyntax"
    }));

    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
    assert!(sidecar.proof.tokenizable_decisions_match);
    assert_eq!(sidecar.proof.grammar_shape.rules, 16);
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ArgumentSyntax"
            && reference.field == "Expression"
            && reference.target_rule == "RangeExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "RangeExpressionSyntax"
            && reference.field == "LeftOperand"
            && reference.target_rule == "LiteralExpressionSyntax"
    }));
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "RangeExpressionSyntax"
            && reference.field == "RightOperand"
            && reference.target_rule == "LiteralExpressionSyntax"
    }));
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| payload.owner_rule == "LiteralExpressionSyntax"
                && payload.field == "Token"
                && payload.payload_kind == "integer")
    );
}

#[test]
fn csharp_minimal_class_semantic_syntax_round_trips_sidecar_tokens() {
    let adapter = CSharpSyntaxMetadataAdapter::new("csharp_minimal_class_syntax_v0");
    let grammar = adapter
        .lower_metadata_file(Path::new(METADATA_PATH))
        .unwrap();
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout()).unwrap();
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
    assert!(
        sidecar
            .structural_model
            .payload_leaves
            .iter()
            .any(|payload| {
                payload.owner_rule == "FileScopedNamespaceDeclarationSyntax"
                    && payload.field == "Name"
                    && payload.payload_kind == "identifier"
            })
    );
    assert!(sidecar.structural_model.references.iter().any(|reference| {
        reference.owner_rule == "ClassDeclarationSyntax"
            && reference.field == "Members"
            && reference.target_rule == "PropertyDeclarationSyntax"
    }));

    let source = fs::read_to_string(SOURCE_PATH).unwrap();
    let rendered = fs::read_to_string(RENDERED_PATH).unwrap();
    assert_eq!(rendered, source);
    // Semantic fields match source.cs / rendered.cs directly; ArenaIR adapter
    // probes were retired with the C# arena frontend route.
    let syntax = CSharpMinimalClassSyntax {
        namespace: "Tct.Contracts".to_string(),
        class_name: "Greeter".to_string(),
        property_name: "Name".to_string(),
        property_type_keyword: "string".to_string(),
        initializer: "world".to_string(),
    };

    let tokens = encode_csharp_minimal_class_syntax(&sidecar, &syntax).unwrap();
    assert!(tokens.iter().all(|token| {
        *token < sidecar.structural_model.token_ranges.vocab_size
            && *token != sidecar.structural_model.token_ranges.pad_token
    }));
    let decoded = decode_csharp_minimal_class_syntax(&sidecar, &tokens).unwrap();
    assert_eq!(decoded, syntax);
}

#[test]
fn csharp_syntax_token_layout_reserves_bpe_boundary_without_collisions() {
    let layout = csharp_syntax_token_layout();
    let boundary = layout.reserve_bpe_boundary(16).unwrap();

    assert_eq!(boundary.raw_base_vocab_size, 576);
    assert_eq!(boundary.bpe_start, 576);
    assert_eq!(boundary.bpe_end_exclusive, 592);
    assert_eq!(boundary.pad_token, 592);
    assert_eq!(boundary.vocab_size, 593);
    assert!(!boundary.current_layout_hosts_bpe);
    assert!(
        layout
            .ranges
            .iter()
            .all(|range| range.end_exclusive <= boundary.bpe_start)
    );
    assert_eq!(layout.pad_token, boundary.bpe_start);
}

#[test]
fn csharp_syntax_metadata_rejects_empty_field_alternatives_with_context() {
    let adapter = CSharpSyntaxMetadataAdapter::new("bad_csharp");
    let error = adapter
        .lower_metadata_str(
            r#"{
              "artifact_kind": "tct_csharp_syntax_metadata_v0",
              "version": 0,
              "source": {
                "path": "tests/fixtures/csharp_syntax_sources/minimal_class/source.cs",
                "sha256": "baab4d0526ab5d0d9f33472fba4a90f6e3e11b2a719a47a60d94291ff37094f1"
              },
              "parser": {
                "tool": "dotnet",
                "version": "10.0.203"
              },
              "rules": [{
                "name": "ClassDeclarationSyntax",
                "node_kind": "ClassDeclarationSyntax",
                "fields": [{
                  "name": "Identifier",
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
    assert_eq!(error.rule.as_deref(), Some("ClassDeclarationSyntax"));
    assert_eq!(error.field.as_deref(), Some("Identifier"));
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpMinimalClassSyntax {
    namespace: String,
    class_name: String,
    property_name: String,
    property_type_keyword: String,
    initializer: String,
}

const CSHARP_PROGRAM_START_OFFSET: u32 = 0;
const CSHARP_NAMESPACE_OFFSET: u32 = 1;
const CSHARP_CLASS_OFFSET: u32 = 2;
const CSHARP_PROPERTY_OFFSET: u32 = 3;
const CSHARP_PREDEFINED_TYPE_OFFSET: u32 = 4;
const CSHARP_STRING_LITERAL_OFFSET: u32 = 5;
const CSHARP_PAYLOAD_END_OFFSET: u32 = 6;
const CSHARP_PROGRAM_END_OFFSET: u32 = 7;

fn encode_csharp_minimal_class_syntax(
    sidecar: &GrammarStructuralModelSidecar,
    syntax: &CSharpMinimalClassSyntax,
) -> Result<Vec<u32>, String> {
    let mut tokens = vec![csharp_structural_token(
        sidecar,
        CSHARP_PROGRAM_START_OFFSET,
    )?];
    push_csharp_field(
        sidecar,
        CSHARP_NAMESPACE_OFFSET,
        &syntax.namespace,
        &mut tokens,
    )?;
    push_csharp_field(
        sidecar,
        CSHARP_CLASS_OFFSET,
        &syntax.class_name,
        &mut tokens,
    )?;
    push_csharp_field(
        sidecar,
        CSHARP_PROPERTY_OFFSET,
        &syntax.property_name,
        &mut tokens,
    )?;
    push_csharp_field(
        sidecar,
        CSHARP_PREDEFINED_TYPE_OFFSET,
        &syntax.property_type_keyword,
        &mut tokens,
    )?;
    push_csharp_field(
        sidecar,
        CSHARP_STRING_LITERAL_OFFSET,
        &syntax.initializer,
        &mut tokens,
    )?;
    tokens.push(csharp_structural_token(sidecar, CSHARP_PROGRAM_END_OFFSET)?);
    Ok(tokens)
}

fn decode_csharp_minimal_class_syntax(
    sidecar: &GrammarStructuralModelSidecar,
    tokens: &[u32],
) -> Result<CSharpMinimalClassSyntax, String> {
    let mut position = 0;
    expect_csharp_token(
        tokens,
        &mut position,
        csharp_structural_token(sidecar, CSHARP_PROGRAM_START_OFFSET)?,
        "program_start",
    )?;
    let namespace = read_csharp_field(sidecar, tokens, &mut position, CSHARP_NAMESPACE_OFFSET)?;
    let class_name = read_csharp_field(sidecar, tokens, &mut position, CSHARP_CLASS_OFFSET)?;
    let property_name = read_csharp_field(sidecar, tokens, &mut position, CSHARP_PROPERTY_OFFSET)?;
    let property_type_keyword = read_csharp_field(
        sidecar,
        tokens,
        &mut position,
        CSHARP_PREDEFINED_TYPE_OFFSET,
    )?;
    let initializer =
        read_csharp_field(sidecar, tokens, &mut position, CSHARP_STRING_LITERAL_OFFSET)?;
    expect_csharp_token(
        tokens,
        &mut position,
        csharp_structural_token(sidecar, CSHARP_PROGRAM_END_OFFSET)?,
        "program_end",
    )?;
    if position == tokens.len() {
        Ok(CSharpMinimalClassSyntax {
            namespace,
            class_name,
            property_name,
            property_type_keyword,
            initializer,
        })
    } else {
        Err(format!(
            "decoded {position} of {} C# syntax tokens",
            tokens.len()
        ))
    }
}

fn push_csharp_field(
    sidecar: &GrammarStructuralModelSidecar,
    field_offset: u32,
    value: &str,
    tokens: &mut Vec<u32>,
) -> Result<(), String> {
    tokens.push(csharp_structural_token(sidecar, field_offset)?);
    let payload = csharp_token_range_by_role(sidecar, "utf8_byte_payload")?;
    for byte in value.as_bytes() {
        tokens.push(payload.start + u32::from(*byte));
    }
    tokens.push(csharp_structural_token(sidecar, CSHARP_PAYLOAD_END_OFFSET)?);
    Ok(())
}

fn read_csharp_field(
    sidecar: &GrammarStructuralModelSidecar,
    tokens: &[u32],
    position: &mut usize,
    field_offset: u32,
) -> Result<String, String> {
    expect_csharp_token(
        tokens,
        position,
        csharp_structural_token(sidecar, field_offset)?,
        "field",
    )?;
    let payload = csharp_token_range_by_role(sidecar, "utf8_byte_payload")?;
    let terminator = csharp_structural_token(sidecar, CSHARP_PAYLOAD_END_OFFSET)?;
    let mut bytes = Vec::new();
    loop {
        let token = *tokens
            .get(*position)
            .ok_or_else(|| "unexpected EOF while reading C# payload".to_string())?;
        *position += 1;
        if token == terminator {
            return String::from_utf8(bytes)
                .map_err(|error| format!("C# payload is not valid UTF-8: {error}"));
        }
        if !(payload.start..payload.end_exclusive).contains(&token) {
            return Err(format!(
                "C# payload token {token} is outside the UTF-8 range"
            ));
        }
        bytes.push((token - payload.start) as u8);
    }
}

fn expect_csharp_token(
    tokens: &[u32],
    position: &mut usize,
    expected: u32,
    rule: &str,
) -> Result<(), String> {
    let actual = *tokens
        .get(*position)
        .ok_or_else(|| format!("unexpected EOF in {rule}"))?;
    *position += 1;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected C# token {expected} in {rule}, got {actual}"
        ))
    }
}

fn csharp_structural_token(
    sidecar: &GrammarStructuralModelSidecar,
    offset: u32,
) -> Result<u32, String> {
    let structural = csharp_token_range_by_role(sidecar, "structural_decisions")?;
    let token = structural.start + offset;
    if token < structural.end_exclusive {
        Ok(token)
    } else {
        Err(format!(
            "C# structural token offset {offset} is outside the layout"
        ))
    }
}

fn csharp_token_range_by_role<'a>(
    sidecar: &'a GrammarStructuralModelSidecar,
    role: &str,
) -> Result<&'a GrammarStructuralTokenRange, String> {
    sidecar
        .structural_model
        .token_ranges
        .ranges
        .iter()
        .find(|range| range.role == role)
        .ok_or_else(|| format!("missing C# token range role {role}"))
}

fn csharp_syntax_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 577,
        pad_token: 576,
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
            GrammarStructuralTokenRange {
                id: "csharp.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 320,
                end_exclusive: 576,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "csharp.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}
