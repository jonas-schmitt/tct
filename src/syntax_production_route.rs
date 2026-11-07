//! Feature-limited stable syntax production route metadata gate.
//!
//! This module owns the parser-backed production-route metadata for syntax
//! backends. It accepts an already checked `GrammarIR -> StructuralModel`
//! sidecar and delegates to the library-owned syntax generated-code renderer
//! API. The stable claim is deliberately limited to this syntax-level route:
//! parser-backed metadata -> `GrammarIR` -> `StructuralModel` -> token layout
//! -> generated codec. Arena IR paths remain controls only.

use crate::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use crate::go_syntax_metadata::GoSyntaxMetadataAdapter;
use crate::grammar_ir::{GRAMMAR_IR_VERSION, GrammarIr};
use crate::grammar_ir_structural_model::{
    ARTIFACT_KIND as STRUCTURAL_MODEL_ARTIFACT_KIND, GrammarStructuralModelSidecar, MODEL_VERSION,
    lower_grammar_ir_to_structural_model,
};
use crate::provenance::json_sha256;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_generated_codec as syntax_codec;
use crate::syntax_generated_codec::{
    SyntaxGeneratedCodecArtifact, render_csharp_syntax_codec_fixture,
    render_go_syntax_codec_fixture, render_toy_syntax_codec_fixture,
    syntax_generated_codec_metadata,
};
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;

mod gate;
mod projector_metadata;
pub use gate::{
    SyntaxProductionRouteContract, SyntaxProductionRouteImplementationGate,
    SyntaxProductionRouteJsonBackendParity, SyntaxProductionRouteSetup,
    SyntaxProductionRouteSourceAuditCoverage, SyntaxProductionRouteSourceStablePromotionReview,
    SyntaxProductionRouteStableMetadata, SyntaxProductionRouteState,
};
pub use projector_metadata::{
    CSHARP_CHECKED_METADATA_PROJECTOR_COMMAND, CSHARP_CHECKED_METADATA_PROJECTOR_OUTPUT_PATH,
    GO_CHECKED_METADATA_PROJECTOR_COMMAND, GO_CHECKED_METADATA_PROJECTOR_COMMAND_TEMPLATE,
    GO_CHECKED_METADATA_PROJECTOR_OUTPUT_ROOT, SyntaxProductionRouteProjectorMetadataSetup,
    projector_metadata_setup_fields, syntax_production_route_projector_metadata_setup,
};

pub const TOOL: &str = "syntax_backend_production_route_implementation_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_production_route_implementation_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/production-route-implementation-gate-v1.json";
pub const STATUS: &str = "feature_limited_stable_metadata_promoted";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend production route implementation gate";
pub const STABLE_BACKEND_SCOPE: &str = "feature_limited_parser_backed_syntax_backend";
pub const STABLE_METADATA_PROMOTION_COMPLETED_CLASS: &str =
    "syntax backend stable metadata promotion implementation gate";
pub const STABLE_METADATA_PROMOTION_DECISION: &str =
    "ready_for_stable_metadata_promotion_implementation";
pub const STABLE_METADATA_PROMOTION_DECISION_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/final-stable-promotion-decision-v1.json";
pub const GO_MINIMAL_FUNCTION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go minimal function production route tranche";
pub const GO_LOCAL_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go local declaration route tranche";
pub const GO_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go local assignment route tranche";
pub const GO_ESCAPED_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go escaped local assignment route tranche";
pub const GO_LOCAL_SHORT_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go short variable declaration route tranche";
pub const GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go integer short variable declaration route tranche";
pub const GO_LOCAL_INTEGER_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go integer return literal route tranche";
pub const GO_LOCAL_STRING_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go string return literal route tranche";
pub const GO_LOCAL_FLOAT_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go float return literal route tranche";
pub const GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go boolean return identifier route tranche";
pub const GO_LOCAL_NIL_RETURN_IDENTIFIER_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go nil return identifier route tranche";
pub const GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go parenthesized integer return expression route tranche";
pub const GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go unary integer return expression route tranche";
pub const GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go binary integer return expression route tranche";
pub const GO_LOCAL_CALL_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go call return expression route tranche";
pub const GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go argument call return expression route tranche";
pub const GO_LOCAL_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go typed local declaration route tranche";
pub const GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go zero-value typed local declaration route tranche";
pub const GO_LOCAL_INTEGER_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go integer typed local declaration route tranche";
pub const GO_LOCAL_UNTYPED_INTEGER_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go untyped integer local declaration route tranche";
pub const GO_LOCAL_INTEGER_ASSIGNMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go integer local assignment route tranche";
pub const GO_LOCAL_COMPOUND_ASSIGNMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go local compound assignment route tranche";
pub const GO_LOCAL_INCREMENT_DECREMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go local increment/decrement route tranche";
pub const GO_LOCAL_BRANCH_STATEMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go local branch statement route tranche";
pub const GO_LOCAL_IF_ELSE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go if/else route tranche";
pub const GO_LOCAL_FOR_CONDITION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go condition-only for route tranche";
pub const GO_LOCAL_SWITCH_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go single-case switch route tranche";
pub const GO_LOCAL_SWITCH_INIT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go switch init route tranche";
pub const GO_LOCAL_TYPE_SWITCH_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go type-switch route tranche";
pub const GO_LOCAL_RANGE_STRING_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go string range route tranche";
pub const GO_PACKAGE_TYPE_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go package type declaration route tranche";
pub const GO_TYPE_SPEC_ASSIGN_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go TypeSpec.Assign route tranche";
pub const GO_TYPE_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go TypeSpec.Comment route tranche";
pub const GO_TYPE_SPEC_DOC_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go TypeSpec.Doc route tranche";
pub const GO_PACKAGE_CONST_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go package const declaration route tranche";
pub const GO_METHOD_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go method declaration route tranche";
pub const GO_STRUCT_TYPE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go struct type route tranche";
pub const GO_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go composite literal route tranche";
pub const GO_KEY_VALUE_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go key/value composite literal route tranche";
pub const GO_ARRAY_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go array composite literal route tranche";
pub const GO_INDEX_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go index expression route tranche";
pub const GO_SLICE_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go slice expression route tranche";
pub const GO_SLICE_EXPRESSION_THREE_INDEX_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go three-index slice expression route tranche";
pub const GO_STAR_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go star expression route tranche";
pub const GO_PACKAGE_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go package var declaration route tranche";
pub const GO_VALUE_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go ValueSpec.Comment route tranche";
pub const GO_IMPORT_SPEC_NAME_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go ImportSpec.Name route tranche";
pub const GO_IMPORT_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go ImportSpec.Comment route tranche";
pub const GO_FUNCTION_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go function literal route tranche";
pub const GO_LOCAL_GENERIC_FUNCTION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go generic function type parameters route tranche";
pub const GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go concurrency/map/interface/type-assertion route tranche";
pub const GO_LOCAL_VARIADIC_ELLIPSIS_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go variadic ellipsis route tranche";
pub const GO_STRINGS_COMPARE_SOURCE_FILE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go strings compare source-file route tranche";
pub const GO_OS_DIR_UNIX_SOURCE_FILE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go os dir_unix source-driven tokenizer generator route tranche";
pub const CSHARP_MINIMAL_CLASS_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# architecture-parity route tranche";
pub const CSHARP_BLOCK_SCOPED_NAMESPACE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# block-scoped namespace route tranche";
pub const CSHARP_USING_DIRECTIVE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# using directive route tranche";
pub const CSHARP_QUALIFIED_NAME_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# qualified name route tranche";
pub const CSHARP_GENERIC_NAME_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# generic name route tranche";
pub const CSHARP_STRUCT_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# struct declaration route tranche";
pub const CSHARP_ENUM_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# enum declaration route tranche";
pub const CSHARP_EVENT_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# event declaration route tranche";
pub const CSHARP_CONSTRUCTOR_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# constructor declaration route tranche";
pub const CSHARP_WHILE_POSTFIX_LOOP_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# while/postfix loop route tranche";
pub const CSHARP_INTERPOLATED_STRING_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# interpolated string route tranche";
pub const CSHARP_MEMBER_ACCESS_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# member access expression route tranche";
pub const CSHARP_FIELD_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# field declaration route tranche";
pub const CSHARP_BASE_LIST_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# base list route tranche";
pub const CSHARP_CONDITIONAL_ACCESS_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# conditional access route tranche";
pub const CSHARP_ARRAY_RANK_OBJECT_CREATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# array-rank/object-creation route tranche";
pub const CSHARP_THIS_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# this-expression route tranche";
pub const CSHARP_PATTERN_EXPRESSION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# pattern-expression route tranche";
pub const CSHARP_ATTRIBUTE_SYNTAX_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# attribute route tranche";
pub const CSHARP_ARROW_EXPRESSION_CLAUSE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# arrow-expression-clause route tranche";
pub const CSHARP_LOCAL_STRING_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# local string return literal route tranche";
pub const CSHARP_LOCAL_INTEGER_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# integer return literal route tranche";
pub const CSHARP_LOCAL_FLOAT_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# float return literal route tranche";
pub const CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# boolean return literal route tranche";
pub const CSHARP_LOCAL_NULL_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# null return literal route tranche";
pub const CSHARP_LOCAL_NULLABLE_RETURN_TYPE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# nullable return type route tranche";
pub const CSHARP_LOCAL_NULLABLE_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# nullable local declaration route tranche";
pub const CSHARP_NULLABLE_PROPERTY_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# nullable property declaration route tranche";
pub const CSHARP_NULLABLE_PARAMETER_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# nullable parameter declaration route tranche";
pub const CSHARP_LOCAL_UNARY_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# prefix unary integer return route tranche";
pub const CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# parenthesized integer return route tranche";
pub const CSHARP_LOCAL_BINARY_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# binary integer return route tranche";
pub const CSHARP_LOCAL_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# local typed declaration route tranche";
pub const CSHARP_LOCAL_IMPLICIT_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# implicit local declaration route tranche";
pub const CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# integer implicit local declaration route tranche";
pub const CSHARP_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# local assignment route tranche";
pub const CSHARP_LOCAL_ESCAPED_ASSIGNMENT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# escaped local assignment route tranche";
pub const CSHARP_LOCAL_IF_ELSE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# local if/else route tranche";
pub const CSHARP_LOCAL_FOR_CONDITION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# condition-only for route tranche";
pub const CSHARP_LOCAL_SWITCH_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# single-case switch route tranche";
pub const CSHARP_LOCAL_FOREACH_STRING_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# local foreach string route tranche";
pub const CSHARP_TWO_PARAMETER_METHOD_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# two-parameter method route tranche";
pub const CSHARP_PROPERTY_PLUS_METHOD_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# property plus method route tranche";
pub const CSHARP_METHOD_INVOCATION_RETURN_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# method invocation return route tranche";
pub const CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# invocation-with-argument return route tranche";
pub const CSHARP_COLLECTION_INITIALIZER_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# collection initializer route tranche";
pub const CSHARP_ELEMENT_ACCESS_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# element access route tranche";
pub const CSHARP_RANGE_ELEMENT_ACCESS_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# range element access route tranche";
pub const CSHARP_COMPILATION_UNIT_SOURCE_FILE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# whole-source compilation-unit generated-codec route tranche";
pub const CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# Logger whole-source compilation-unit registry coverage tranche";
pub const CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# VersionOne whole-source compilation-unit registry coverage tranche";
pub const CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# IteratorMethods whole-source compilation-unit registry coverage tranche";
pub const CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# Loan whole-source compilation-unit registry coverage tranche";
pub const ROUTE_SETUP_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend route setup normalization tranche";
pub const PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend projector-driven metadata route setup tranche";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const ROUTE_ENTRYPOINT: &str = "compile_checked_syntax_production_route";
pub const SYNTAX_COMPILER_ROUTE_ENTRYPOINT: &str = "compile_checked_syntax_compiler_route";
pub const SYNTAX_COMPILER_ROUTE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend target-agnostic syntax compiler routing slice";
pub const SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend target-agnostic generated-code gate ownership tranche";
pub const SYNTAX_COMPILER_STRICT_RECOVERY_DECODE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend target-agnostic strict/recovery decode route ownership tranche";
pub const SYNTAX_COMPILER_CONSUMED_TOKEN_ACCOUNTING_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend target-agnostic consumed-token accounting route ownership tranche";
pub const SYNTAX_COMPILER_TOKEN_VOCABULARY_BPE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend target-agnostic token/vocabulary/BPE route ownership tranche";
pub const SYNTAX_COMPILER_VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend JSON-backend-grade validation matrix route ownership gate";
pub const SYNTAX_COMPILER_VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend validation matrix execution evidence preservation gate";
pub const SYNTAX_COMPILER_STATE_API_ROUTE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend generated-codec state API route gate";
pub const SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS;
pub const SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend generated-codec admissible-token mask route gate";
pub const SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS;
pub const SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend generated-codec prefix-state continuation route gate";
pub const SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS:
    &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_RESPONSE_EVIDENCE_FIXTURE: &str =
    "tests/fixtures/syntax_backend/generated-codec-state-api-route-gate-v1.json";
pub const ROUTE_GATE: &str = "syntax-production-route-v0";
pub const ROUTE_SETUP_SOURCE_OF_TRUTH: &str =
    "src/syntax_production_route.rs::SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS";
pub const CODEC_REGISTRY_SOURCE_OF_TRUTH: &str =
    "src/syntax_production_route.rs::SYNTAX_CODEC_REGISTRY";
pub const ROUTE_SETUP_NEXT_MISSING_CLASS_SOURCE: &str =
    "src/syntax_production_route.rs::NEXT_MISSING_STABLE_PROMOTION_CLASS";
pub const PROJECTOR_METADATA_SETUP_SOURCE_OF_TRUTH: &str =
    "src/syntax_production_route.rs::syntax_production_route_projector_metadata_setup";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_ROUTE_SURFACES: &[&str] = &[
    "checked_syntax_input",
    "structural_model_boundary",
    "syntax_generated_codec_renderer_api",
    "generated_codec_artifact_metadata",
    "strict_recovery_decode_policy",
    "consumed_token_accounting",
    "token_vocabulary_bpe_invariants",
    "generated_artifact_reproducibility",
    "audit_markers",
    "json_backend_parity",
    "validation_matrix",
];
const REQUIRED_JSON_BACKEND_GRADE_CATEGORIES: &[&str] = &[
    "input_validation",
    "generated_code_compilation",
    "semantic_round_trips",
    "strict_recovery_decode",
    "consumed_token_accounting",
    "token_vocabulary_bpe",
    "generated_artifact_reproducibility",
    "maintained_documentation",
    "audit_coverage",
    "json_backend_parity",
];
const REQUIRED_VALIDATION_MATRIX_ROUTE_EVIDENCE: &[&str] = &[
    "csharp_minimal_class_syntax_v0",
    "csharp_local_string_return_literal_syntax_v0",
    "go_minimal_function_syntax_v0",
    "go_local_declaration_syntax_v0",
    "generated_go_minimal_function_real_source_strings_compare_demo",
];
const REQUIRED_RENDERER_FUNCTIONS: &[&str] = &[
    "render_toy_syntax_codec_fixture",
    "render_csharp_syntax_codec_fixture",
    "render_go_syntax_codec_fixture",
];
const REQUIRED_STABLE_METADATA_CLAIMS: &[&str] = &[
    "feature_limited_parser_backed_syntax_backend",
    "stable_route_metadata_promoted",
    "no_arena_ir_promotion",
    "no_generated_json_schema_output_change",
    "no_core_compiler_path_change",
    "no_json_schema_compiler_pass_change",
    "no_generated_source_substring_dependency",
];
const REQUIRED_JSON_BACKEND_CHECKS: &[&str] = &[
    "cargo test --test round_trip_test test_round_trip",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "./scripts/testing/test-syntax-backend-gates.sh",
];
const REQUIRED_ROUTE_SETUP_FIELDS: &[&str] = &[
    "target_id",
    "grammar_id",
    "completed_promotion_class",
    "metadata_path",
    "generated_test_markers",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxProductionRouteConstruct {
    pub target_id: &'static str,
    pub grammar_id: &'static str,
    pub completed_promotion_class: &'static str,
    pub metadata_path: &'static str,
    pub generated_test_markers: &'static [&'static str],
}

pub type SyntaxCodecRegistryEntry = SyntaxProductionRouteConstruct;

pub const SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS: &[SyntaxProductionRouteConstruct] = &[
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_minimal_class_syntax_v0",
        completed_promotion_class: CSHARP_MINIMAL_CLASS_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_MINIMAL_CLASS_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_using_directive_syntax_v0",
        completed_promotion_class: CSHARP_USING_DIRECTIVE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_USING_DIRECTIVE_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_USING_DIRECTIVE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_qualified_name_syntax_v0",
        completed_promotion_class: CSHARP_QUALIFIED_NAME_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_QUALIFIED_NAME_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_QUALIFIED_NAME_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_generic_name_syntax_v0",
        completed_promotion_class: CSHARP_GENERIC_NAME_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_GENERIC_NAME_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_GENERIC_NAME_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_block_scoped_namespace_syntax_v0",
        completed_promotion_class: CSHARP_BLOCK_SCOPED_NAMESPACE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_BLOCK_SCOPED_NAMESPACE_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_BLOCK_SCOPED_NAMESPACE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_struct_declaration_syntax_v0",
        completed_promotion_class: CSHARP_STRUCT_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_STRUCT_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_STRUCT_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_enum_declaration_syntax_v0",
        completed_promotion_class: CSHARP_ENUM_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_ENUM_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_ENUM_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_event_declaration_syntax_v0",
        completed_promotion_class: CSHARP_EVENT_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_EVENT_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_EVENT_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_constructor_declaration_syntax_v0",
        completed_promotion_class: CSHARP_CONSTRUCTOR_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_CONSTRUCTOR_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_CONSTRUCTOR_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_while_postfix_loop_syntax_v0",
        completed_promotion_class: CSHARP_WHILE_POSTFIX_LOOP_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_WHILE_POSTFIX_LOOP_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_WHILE_POSTFIX_LOOP_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_interpolated_string_syntax_v0",
        completed_promotion_class: CSHARP_INTERPOLATED_STRING_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_INTERPOLATED_STRING_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_INTERPOLATED_STRING_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_member_access_expression_syntax_v0",
        completed_promotion_class: CSHARP_MEMBER_ACCESS_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_MEMBER_ACCESS_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_field_declaration_syntax_v0",
        completed_promotion_class: CSHARP_FIELD_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_FIELD_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_FIELD_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_base_list_syntax_v0",
        completed_promotion_class: CSHARP_BASE_LIST_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_BASE_LIST_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_BASE_LIST_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_conditional_access_syntax_v0",
        completed_promotion_class: CSHARP_CONDITIONAL_ACCESS_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_CONDITIONAL_ACCESS_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_CONDITIONAL_ACCESS_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_array_rank_object_creation_syntax_v0",
        completed_promotion_class: CSHARP_ARRAY_RANK_OBJECT_CREATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_ARRAY_RANK_OBJECT_CREATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_ARRAY_RANK_OBJECT_CREATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_this_expression_syntax_v0",
        completed_promotion_class: CSHARP_THIS_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_THIS_EXPRESSION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_THIS_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_pattern_expression_syntax_v0",
        completed_promotion_class: CSHARP_PATTERN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_PATTERN_EXPRESSION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_PATTERN_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_attribute_syntax_v0",
        completed_promotion_class: CSHARP_ATTRIBUTE_SYNTAX_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_ATTRIBUTE_SYNTAX_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_ATTRIBUTE_SYNTAX_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_arrow_expression_clause_syntax_v0",
        completed_promotion_class: CSHARP_ARROW_EXPRESSION_CLAUSE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_ARROW_EXPRESSION_CLAUSE_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_ARROW_EXPRESSION_CLAUSE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_string_return_literal_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_STRING_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_STRING_RETURN_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_integer_return_literal_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_INTEGER_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_float_return_literal_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_FLOAT_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_FLOAT_RETURN_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_boolean_return_literal_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_BOOLEAN_RETURN_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_null_return_literal_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_NULL_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_NULL_RETURN_LITERAL_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_NULL_RETURN_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_nullable_return_type_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_NULLABLE_RETURN_TYPE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_NULLABLE_RETURN_TYPE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_nullable_declaration_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_NULLABLE_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_NULLABLE_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_NULLABLE_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_nullable_property_declaration_syntax_v0",
        completed_promotion_class: CSHARP_NULLABLE_PROPERTY_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_NULLABLE_PROPERTY_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_NULLABLE_PROPERTY_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_nullable_parameter_declaration_syntax_v0",
        completed_promotion_class: CSHARP_NULLABLE_PARAMETER_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_NULLABLE_PARAMETER_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_NULLABLE_PARAMETER_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_unary_integer_return_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_UNARY_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_UNARY_INTEGER_RETURN_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_parenthesized_integer_return_syntax_v0",
        completed_promotion_class:
            CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_PARENTHESIZED_INTEGER_RETURN_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_binary_integer_return_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_BINARY_INTEGER_RETURN_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_BINARY_INTEGER_RETURN_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_BINARY_INTEGER_RETURN_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_typed_declaration_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_TYPED_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_LOCAL_TYPED_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_implicit_declaration_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_IMPLICIT_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_IMPLICIT_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_IMPLICIT_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_integer_implicit_declaration_syntax_v0",
        completed_promotion_class:
            CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_INTEGER_IMPLICIT_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_assignment_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_ASSIGNMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_LOCAL_ASSIGNMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_if_else_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_IF_ELSE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_IF_ELSE_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_LOCAL_IF_ELSE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_for_condition_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_FOR_CONDITION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_FOR_CONDITION_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_LOCAL_FOR_CONDITION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_switch_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_SWITCH_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_SWITCH_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_LOCAL_SWITCH_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_foreach_string_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_FOREACH_STRING_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_FOREACH_STRING_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_LOCAL_FOREACH_STRING_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_two_parameter_method_syntax_v0",
        completed_promotion_class: CSHARP_TWO_PARAMETER_METHOD_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_TWO_PARAMETER_METHOD_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_TWO_PARAMETER_METHOD_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_property_plus_method_syntax_v0",
        completed_promotion_class: CSHARP_PROPERTY_PLUS_METHOD_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_PROPERTY_PLUS_METHOD_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_PROPERTY_PLUS_METHOD_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_method_invocation_return_syntax_v0",
        completed_promotion_class: CSHARP_METHOD_INVOCATION_RETURN_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_METHOD_INVOCATION_RETURN_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_METHOD_INVOCATION_RETURN_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_method_invocation_argument_return_syntax_v0",
        completed_promotion_class:
            CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_METHOD_INVOCATION_ARGUMENT_RETURN_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_collection_initializer_syntax_v0",
        completed_promotion_class: CSHARP_COLLECTION_INITIALIZER_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_COLLECTION_INITIALIZER_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_COLLECTION_INITIALIZER_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_element_access_syntax_v0",
        completed_promotion_class: CSHARP_ELEMENT_ACCESS_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_ELEMENT_ACCESS_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_ELEMENT_ACCESS_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_range_element_access_syntax_v0",
        completed_promotion_class: CSHARP_RANGE_ELEMENT_ACCESS_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_RANGE_ELEMENT_ACCESS_METADATA_PATH,
        generated_test_markers: syntax_codec::CSHARP_RANGE_ELEMENT_ACCESS_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_local_escaped_assignment_syntax_v0",
        completed_promotion_class: CSHARP_LOCAL_ESCAPED_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_LOCAL_ESCAPED_ASSIGNMENT_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_LOCAL_ESCAPED_ASSIGNMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_compilation_unit_source_file_syntax_v0",
        completed_promotion_class: CSHARP_COMPILATION_UNIT_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_COMPILATION_UNIT_SOURCE_FILE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_compilation_unit_logger_source_file_syntax_v0",
        completed_promotion_class:
            CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_compilation_unit_version_one_source_file_syntax_v0",
        completed_promotion_class:
            CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_compilation_unit_iterator_methods_source_file_syntax_v0",
        completed_promotion_class:
            CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
        metadata_path:
            syntax_codec::CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "csharp",
        grammar_id: "csharp_compilation_unit_loan_source_file_syntax_v0",
        completed_promotion_class:
            CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH,
        generated_test_markers:
            syntax_codec::CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_minimal_function_syntax_v0",
        completed_promotion_class: GO_MINIMAL_FUNCTION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_MINIMAL_FUNCTION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_declaration_syntax_v0",
        completed_promotion_class: GO_LOCAL_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_assignment_syntax_v0",
        completed_promotion_class: GO_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_ASSIGNMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_ASSIGNMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_escaped_local_assignment_syntax_v0",
        completed_promotion_class: GO_ESCAPED_LOCAL_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_ESCAPED_LOCAL_ASSIGNMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_ESCAPED_LOCAL_ASSIGNMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_short_var_declaration_syntax_v0",
        completed_promotion_class: GO_LOCAL_SHORT_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_SHORT_VAR_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_SHORT_VAR_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_integer_short_var_declaration_syntax_v0",
        completed_promotion_class: GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_INTEGER_SHORT_VAR_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_integer_return_literal_syntax_v0",
        completed_promotion_class: GO_LOCAL_INTEGER_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_INTEGER_RETURN_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_parenthesized_integer_return_expression_syntax_v0",
        completed_promotion_class:
            GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_PARENTHESIZED_INTEGER_RETURN_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_unary_integer_return_expression_syntax_v0",
        completed_promotion_class:
            GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_UNARY_INTEGER_RETURN_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_binary_integer_return_expression_syntax_v0",
        completed_promotion_class:
            GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_BINARY_INTEGER_RETURN_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_call_return_expression_syntax_v0",
        completed_promotion_class: GO_LOCAL_CALL_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_CALL_RETURN_EXPRESSION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_CALL_RETURN_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_argument_call_return_expression_syntax_v0",
        completed_promotion_class:
            GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_ARGUMENT_CALL_RETURN_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_string_return_literal_syntax_v0",
        completed_promotion_class: GO_LOCAL_STRING_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_STRING_RETURN_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_float_return_literal_syntax_v0",
        completed_promotion_class: GO_LOCAL_FLOAT_RETURN_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_FLOAT_RETURN_LITERAL_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_FLOAT_RETURN_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_boolean_return_identifier_syntax_v0",
        completed_promotion_class: GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_BOOLEAN_RETURN_IDENTIFIER_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_nil_return_identifier_syntax_v0",
        completed_promotion_class: GO_LOCAL_NIL_RETURN_IDENTIFIER_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_NIL_RETURN_IDENTIFIER_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_NIL_RETURN_IDENTIFIER_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_typed_declaration_syntax_v0",
        completed_promotion_class: GO_LOCAL_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_TYPED_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_TYPED_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_zero_value_typed_declaration_syntax_v0",
        completed_promotion_class: GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_ZERO_VALUE_TYPED_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_integer_typed_declaration_syntax_v0",
        completed_promotion_class: GO_LOCAL_INTEGER_TYPED_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_INTEGER_TYPED_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_INTEGER_TYPED_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_untyped_integer_declaration_syntax_v0",
        completed_promotion_class: GO_LOCAL_UNTYPED_INTEGER_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_UNTYPED_INTEGER_DECLARATION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_LOCAL_UNTYPED_INTEGER_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_integer_assignment_syntax_v0",
        completed_promotion_class: GO_LOCAL_INTEGER_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_INTEGER_ASSIGNMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_INTEGER_ASSIGNMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_compound_assignment_syntax_v0",
        completed_promotion_class: GO_LOCAL_COMPOUND_ASSIGNMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_COMPOUND_ASSIGNMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_COMPOUND_ASSIGNMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_increment_decrement_syntax_v0",
        completed_promotion_class: GO_LOCAL_INCREMENT_DECREMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_INCREMENT_DECREMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_INCREMENT_DECREMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_branch_statement_syntax_v0",
        completed_promotion_class: GO_LOCAL_BRANCH_STATEMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_BRANCH_STATEMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_BRANCH_STATEMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_if_else_syntax_v0",
        completed_promotion_class: GO_LOCAL_IF_ELSE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_IF_ELSE_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_IF_ELSE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_for_condition_syntax_v0",
        completed_promotion_class: GO_LOCAL_FOR_CONDITION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_FOR_CONDITION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_FOR_CONDITION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_switch_syntax_v0",
        completed_promotion_class: GO_LOCAL_SWITCH_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_SWITCH_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_SWITCH_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_switch_init_syntax_v0",
        completed_promotion_class: GO_LOCAL_SWITCH_INIT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_SWITCH_INIT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_SWITCH_INIT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_type_switch_syntax_v0",
        completed_promotion_class: GO_LOCAL_TYPE_SWITCH_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_TYPE_SWITCH_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_TYPE_SWITCH_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_range_string_syntax_v0",
        completed_promotion_class: GO_LOCAL_RANGE_STRING_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_RANGE_STRING_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_RANGE_STRING_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_package_type_declaration_syntax_v0",
        completed_promotion_class: GO_PACKAGE_TYPE_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_PACKAGE_TYPE_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_PACKAGE_TYPE_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_type_spec_assign_syntax_v0",
        completed_promotion_class: GO_TYPE_SPEC_ASSIGN_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_TYPE_SPEC_ASSIGN_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_TYPE_SPEC_ASSIGN_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_type_spec_comment_syntax_v0",
        completed_promotion_class: GO_TYPE_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_TYPE_SPEC_COMMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_TYPE_SPEC_COMMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_type_spec_doc_syntax_v0",
        completed_promotion_class: GO_TYPE_SPEC_DOC_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_TYPE_SPEC_DOC_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_TYPE_SPEC_DOC_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_package_const_declaration_syntax_v0",
        completed_promotion_class: GO_PACKAGE_CONST_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_PACKAGE_CONST_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_PACKAGE_CONST_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_method_declaration_syntax_v0",
        completed_promotion_class: GO_METHOD_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_METHOD_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_METHOD_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_struct_type_syntax_v0",
        completed_promotion_class: GO_STRUCT_TYPE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_STRUCT_TYPE_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_STRUCT_TYPE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_composite_literal_syntax_v0",
        completed_promotion_class: GO_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_COMPOSITE_LITERAL_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_COMPOSITE_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_key_value_composite_literal_syntax_v0",
        completed_promotion_class: GO_KEY_VALUE_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_KEY_VALUE_COMPOSITE_LITERAL_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_KEY_VALUE_COMPOSITE_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_array_composite_literal_syntax_v0",
        completed_promotion_class: GO_ARRAY_COMPOSITE_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_ARRAY_COMPOSITE_LITERAL_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_ARRAY_COMPOSITE_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_index_expression_syntax_v0",
        completed_promotion_class: GO_INDEX_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_INDEX_EXPRESSION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_INDEX_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_slice_expression_syntax_v0",
        completed_promotion_class: GO_SLICE_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_SLICE_EXPRESSION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_SLICE_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_slice_expression_three_index_syntax_v0",
        completed_promotion_class: GO_SLICE_EXPRESSION_THREE_INDEX_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_SLICE_EXPRESSION_THREE_INDEX_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_SLICE_EXPRESSION_THREE_INDEX_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_star_expression_syntax_v0",
        completed_promotion_class: GO_STAR_EXPRESSION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_STAR_EXPRESSION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_STAR_EXPRESSION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_package_var_declaration_syntax_v0",
        completed_promotion_class: GO_PACKAGE_VAR_DECLARATION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_PACKAGE_VAR_DECLARATION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_PACKAGE_VAR_DECLARATION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_value_spec_comment_syntax_v0",
        completed_promotion_class: GO_VALUE_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_VALUE_SPEC_COMMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_VALUE_SPEC_COMMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_import_spec_name_syntax_v0",
        completed_promotion_class: GO_IMPORT_SPEC_NAME_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_IMPORT_SPEC_NAME_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_IMPORT_SPEC_NAME_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_import_spec_comment_syntax_v0",
        completed_promotion_class: GO_IMPORT_SPEC_COMMENT_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_IMPORT_SPEC_COMMENT_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_IMPORT_SPEC_COMMENT_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_function_literal_syntax_v0",
        completed_promotion_class: GO_FUNCTION_LITERAL_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_FUNCTION_LITERAL_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_FUNCTION_LITERAL_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_generic_function_syntax_v0",
        completed_promotion_class: GO_LOCAL_GENERIC_FUNCTION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_GENERIC_FUNCTION_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_GENERIC_FUNCTION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_concurrency_map_interface_type_assertion_syntax_v0",
        completed_promotion_class:
            GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_METADATA_PATH,
        generated_test_markers:
            syntax_codec::GO_CONCURRENCY_MAP_INTERFACE_TYPE_ASSERTION_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_local_variadic_ellipsis_syntax_v0",
        completed_promotion_class: GO_LOCAL_VARIADIC_ELLIPSIS_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_LOCAL_VARIADIC_ELLIPSIS_GENERATED_TEST_MARKERS,
    },
    // Registry freeze (M1 forward gate): the two go_*_source_file_syntax_v0
    // entries below are the only historical per-file Go registrations and the
    // set must not grow. New Go source files route through the unified pinned
    // inventory grammar artifact (tct-go-grammar-artifact-v1, grammar id
    // go_ast_schema_inventory_go1_24_3); the freeze is pinned by
    // syntax_codec_registry_freezes_per_file_go_grammar_entries_to_the_historical_pair
    // in tests/syntax_generated_codec_gate_test.rs.
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_strings_compare_source_file_syntax_v0",
        completed_promotion_class: GO_STRINGS_COMPARE_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_STRINGS_COMPARE_SOURCE_FILE_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_STRINGS_COMPARE_SOURCE_FILE_GENERATED_TEST_MARKERS,
    },
    SyntaxProductionRouteConstruct {
        target_id: "go",
        grammar_id: "go_os_dir_unix_source_file_syntax_v0",
        completed_promotion_class: GO_OS_DIR_UNIX_SOURCE_FILE_COMPLETED_PROMOTION_CLASS,
        metadata_path: syntax_codec::GO_OS_DIR_UNIX_SOURCE_FILE_METADATA_PATH,
        generated_test_markers: syntax_codec::GO_OS_DIR_UNIX_SOURCE_FILE_GENERATED_TEST_MARKERS,
    },
];

pub const SYNTAX_CODEC_REGISTRY: &[SyntaxCodecRegistryEntry] = SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS;

impl SyntaxProductionRouteConstruct {
    pub fn target(self) -> Result<SyntaxProductionRouteTarget, SyntaxBackendBoundaryError> {
        SyntaxProductionRouteTarget::from_target_id(self.target_id)
    }

    pub fn source_kind(self) -> Result<&'static str, SyntaxBackendBoundaryError> {
        Ok(self.target()?.source_kind())
    }

    pub fn renderer_family(self) -> Result<&'static str, SyntaxBackendBoundaryError> {
        Ok(self.target()?.renderer_family())
    }

    pub fn renderer(self) -> Result<&'static str, SyntaxBackendBoundaryError> {
        Ok(self.target()?.renderer())
    }

    pub fn crate_name(self) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(syntax_production_route_crate_name(
            self.target()?,
            self.grammar_id,
        ))
    }

    pub fn construct_class(self) -> String {
        syntax_production_route_construct_class(self.grammar_id)
    }
}

pub fn syntax_codec_registry_entry(grammar_id: &str) -> Option<&'static SyntaxCodecRegistryEntry> {
    SYNTAX_CODEC_REGISTRY
        .iter()
        .find(|entry| entry.grammar_id == grammar_id)
}

pub fn syntax_production_route_construct(
    grammar_id: &str,
) -> Option<&'static SyntaxProductionRouteConstruct> {
    syntax_codec_registry_entry(grammar_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxProductionRouteTarget {
    Toy,
    CSharp,
    Go,
}

impl SyntaxProductionRouteTarget {
    pub fn target_id(self) -> &'static str {
        match self {
            Self::Toy => "toy",
            Self::CSharp => "csharp",
            Self::Go => "go",
        }
    }

    fn source_kind(self) -> &'static str {
        match self {
            Self::Toy => "hand_written_toy_grammar",
            Self::CSharp => "csharp_syntax_metadata",
            Self::Go => "go_syntax_metadata",
        }
    }

    fn renderer_family(self) -> &'static str {
        match self {
            Self::Toy => "toy_syntax_codec_renderer",
            Self::CSharp => "csharp_syntax_codec_renderer",
            Self::Go => "go_syntax_codec_renderer",
        }
    }

    fn renderer(self) -> &'static str {
        match self {
            Self::Toy => "render_toy_syntax_codec_fixture",
            Self::CSharp => "render_csharp_syntax_codec_fixture",
            Self::Go => "render_go_syntax_codec_fixture",
        }
    }

    fn crate_name(self) -> &'static str {
        match self {
            Self::Toy => "toy-syntax-generated-codec",
            Self::CSharp => "csharp-syntax-generated-codec",
            Self::Go => "go-syntax-generated-codec",
        }
    }

    fn from_target_id(target_id: &str) -> Result<Self, SyntaxBackendBoundaryError> {
        match target_id {
            "csharp" => Ok(Self::CSharp),
            "go" => Ok(Self::Go),
            "toy" => Ok(Self::Toy),
            other => Err(SyntaxBackendBoundaryError::new(
                "route_construct.target_id",
                format!("unsupported syntax production route target {other}"),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SyntaxProductionRouteRequest<'a> {
    pub target: SyntaxProductionRouteTarget,
    pub grammar: &'a GrammarIr,
    pub sidecar: &'a GrammarStructuralModelSidecar,
}

#[derive(Debug, Clone, Copy)]
pub struct SyntaxCompilerRouteRequest<'a> {
    pub grammar: &'a GrammarIr,
    pub sidecar: &'a GrammarStructuralModelSidecar,
}

pub fn build_csharp_minimal_class_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact("csharp_minimal_class_syntax_v0")
}

pub fn build_csharp_local_string_return_literal_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_local_string_return_literal_syntax_v0",
    )
}

pub fn build_csharp_local_integer_return_literal_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_local_integer_return_literal_syntax_v0",
    )
}

pub fn build_csharp_local_boolean_return_literal_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_local_boolean_return_literal_syntax_v0",
    )
}

pub fn build_csharp_local_null_return_literal_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_local_null_return_literal_syntax_v0",
    )
}

pub fn build_csharp_local_nullable_return_type_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_local_nullable_return_type_syntax_v0",
    )
}

pub fn build_csharp_local_nullable_declaration_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_local_nullable_declaration_syntax_v0",
    )
}

pub fn build_csharp_nullable_property_declaration_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_nullable_property_declaration_syntax_v0",
    )
}

pub fn build_csharp_nullable_parameter_declaration_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_nullable_parameter_declaration_syntax_v0",
    )
}

pub fn build_csharp_compilation_unit_source_file_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact(
        "csharp_compilation_unit_source_file_syntax_v0",
    )
}

pub fn build_go_minimal_function_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact("go_minimal_function_syntax_v0")
}

pub fn build_go_local_declaration_compiler_route_artifact()
-> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    build_registered_parser_backed_syntax_compiler_route_artifact("go_local_declaration_syntax_v0")
}

pub fn build_registered_parser_backed_syntax_compiler_route_artifact(
    grammar_id: &str,
) -> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    let entry = syntax_codec_registry_entry(grammar_id).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "syntax_codec_registry.grammar_id",
            format!("grammar {grammar_id} is not registered in the syntax codec registry"),
        )
    })?;
    build_parser_backed_syntax_compiler_route_artifact(entry)
}

pub fn build_parser_backed_syntax_compiler_route_artifact(
    entry: &SyntaxCodecRegistryEntry,
) -> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    let (grammar, sidecar) = lower_registered_parser_backed_syntax_entry(entry)?;
    compile_checked_syntax_compiler_route(SyntaxCompilerRouteRequest {
        grammar: &grammar,
        sidecar: &sidecar,
    })
}

pub fn compile_checked_syntax_compiler_route(
    request: SyntaxCompilerRouteRequest<'_>,
) -> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    let target = syntax_compiler_route_target(request.grammar)?;
    let mut artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target,
        grammar: request.grammar,
        sidecar: request.sidecar,
    })?;
    artifact.metadata["target_agnostic_compiler_route"] = json!({
        "entrypoint": SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        "delegates_to": ROUTE_ENTRYPOINT,
        "route_gate": ROUTE_GATE,
        "route_setup_source_of_truth": ROUTE_SETUP_SOURCE_OF_TRUTH,
        "target_inference": "checked GrammarIR grammar_id via syntax production route construct table",
        "target": target.target_id(),
        "grammar_id": request.grammar.grammar_id,
        "accepted_input": "checked GrammarIR -> StructuralModel",
        "output_artifact": "SyntaxGeneratedCodecArtifact",
        "rejects_arena_ir_boundaries": true,
        "stable_backend_promoted": true,
        "stable_backend_scope": STABLE_BACKEND_SCOPE,
        "stable_metadata_promotion_completed_class": STABLE_METADATA_PROMOTION_COMPLETED_CLASS,
        "stable_promotion_decision": STABLE_METADATA_PROMOTION_DECISION,
        "stable_promotion_decision_fixture": STABLE_METADATA_PROMOTION_DECISION_FIXTURE_PATH,
        "generated_json_schema_output_changed": false,
        "core_compiler_path_changed": false,
        "paired_route_evidence": {
            "csharp_fixture": syntax_codec::CSHARP_METADATA_PATH,
            "csharp_non_minimal_fixture": syntax_codec::CSHARP_LOCAL_STRING_RETURN_LITERAL_METADATA_PATH,
            "go_fixture": syntax_codec::GO_METADATA_PATH,
            "go_non_minimal_fixture": syntax_codec::GO_LOCAL_DECLARATION_METADATA_PATH,
            "go_real_source_path": "tests/fixtures/go_syntax_corpus/src/strings/compare.go",
            "go_real_source_marker": "generated_go_minimal_function_real_source_strings_compare_demo"
        },
        "generated_code_gate_representatives": [
            "csharp_minimal_class_syntax_v0",
            "csharp_local_string_return_literal_syntax_v0",
            "go_minimal_function_syntax_v0",
            "go_local_declaration_syntax_v0",
            "generated_go_minimal_function_real_source_strings_compare_demo"
        ],
        "parser_backed_reproducibility_evidence": [
            "csharp_minimal_class_syntax_v0",
            "csharp_local_string_return_literal_syntax_v0",
            "go_minimal_function_syntax_v0",
            "go_local_declaration_syntax_v0"
        ],
        "strict_recovery_decode_route_evidence": [
            "csharp_minimal_class_syntax_v0",
            "csharp_local_string_return_literal_syntax_v0",
            "go_minimal_function_syntax_v0",
            "go_local_declaration_syntax_v0",
            "generated_go_minimal_function_real_source_strings_compare_demo"
        ],
        "consumed_token_accounting_route_evidence": [
            "csharp_minimal_class_syntax_v0",
            "csharp_local_string_return_literal_syntax_v0",
            "go_minimal_function_syntax_v0",
            "go_local_declaration_syntax_v0",
            "generated_go_minimal_function_real_source_strings_compare_demo"
        ],
        "token_vocabulary_bpe_route_evidence": [
            "csharp_minimal_class_syntax_v0",
            "csharp_local_string_return_literal_syntax_v0",
            "go_minimal_function_syntax_v0",
            "go_local_declaration_syntax_v0",
            "generated_go_minimal_function_real_source_strings_compare_demo"
        ],
        "validation_matrix_categories": REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
        "validation_matrix_route_evidence": REQUIRED_VALIDATION_MATRIX_ROUTE_EVIDENCE,
        "validation_matrix_completed_promotion_class": SYNTAX_COMPILER_VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
        "validation_matrix_next_missing_stable_promotion_class": SYNTAX_COMPILER_VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        "generated_codec_state_api_route": {
            "source_integration_fixture": "tests/fixtures/syntax_backend/generated-codec-state-api-integration-v1.json",
            "source_integration_artifact_kind": "tct_syntax_generated_codec_state_api_integration_gate_v1",
            "source_integration_completed_promotion_class": "syntax backend target-neutral generated-codec state API integration gate",
            "target_neutral_surface_id": "target-neutral-generated-codec-state-api-integration-surface-v1",
            "completed_promotion_class": SYNTAX_COMPILER_STATE_API_ROUTE_COMPLETED_PROMOTION_CLASS,
            "next_missing_stable_promotion_class": SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
            "target_ids": ["csharp", "go"],
            "capabilities": [
                "state_after_raw_prefix_query",
                "candidate_raw_token_transition_query",
                "eof_validity_query",
                "strict_decode_query",
                "recovery_decode_query",
                "consumed_raw_token_accounting_query",
                "admissible_next_raw_token_ids_query",
                "admissible_next_raw_token_ranges_query",
                "bpe_raw_expansion_query",
                "multi_token_bpe_raw_expansion_replay_query",
            ],
            "route_evidence": [
                "go_os_dir_unix_source_file_syntax_v0",
                "go_net_rpc_client_source_file_syntax_v0",
                "go_internal_coverage_cfile_testsupport_source_file_syntax_v0",
                "go_net_ipsock_plan9_source_file_syntax_v0",
                "go_go_ast_commentmap_source_file_syntax_v0",
                "csharp_compilation_unit_source_file_syntax_v0",
            ],
            "go_source_instance_records": [
                "tests/fixtures/go_syntax_corpus/src/os/dir_unix.go",
                "tests/fixtures/go_syntax_corpus/src/net/rpc/client.go",
                "tests/fixtures/go_syntax_corpus/src/internal/coverage/cfile/testsupport.go",
                "tests/fixtures/go_syntax_corpus/src/net/ipsock_plan9.go",
                "tests/fixtures/go_syntax_corpus/src/go/ast/commentmap.go",
            ],
            "csharp_real_source_records": [
                "pattern-matching/Shapes.cs",
                "delegates-and-events/Logger.cs",
                "events/VersionOne.cs",
                "iterators/IteratorMethods.cs",
                "serialization/Loan.cs",
            ],
            "record_query_count": 10,
            "query_contract_consumed_from_integration": true,
            "prefix_state_queries_ready": true,
            "transition_queries_ready": true,
            "eof_queries_ready": true,
            "strict_recovery_decode_queries_ready": true,
            "consumed_token_accounting_queries_ready": true,
            "admissible_raw_token_set_queries_ready": true,
            "admissible_raw_token_range_queries_ready": true,
            "one_token_bpe_raw_expansion_probe_ready": true,
            "multi_token_bpe_raw_expansion_runtime_replay_ready": true,
            "multi_token_bpe_raw_expansion_replay_case_count": 12,
            "next_generated_codec_state_api_work": SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
            "admissible_token_mask_contract": {
                "mask_contract_id": "target-neutral-generated-codec-admissible-token-mask-v1",
                "completed_promotion_class": SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS,
                "next_missing_stable_promotion_class": SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
                "target_ids": ["csharp", "go"],
                "source_query_contract": "target-neutral generated-codec state API integration record queries",
                "target_neutral_state_api_authority": "target-neutral raw transition API",
                "record_query_count": 10,
                "finite_mask_record_count": 10,
                "payload_range_mask_record_count": 10,
                "eof_validity_record_count": 10,
                "bpe_raw_expansion_replay_case_count": 12,
                "bpe_raw_expansion_accepted_case_count": 6,
                "bpe_raw_expansion_first_failure_case_count": 6,
                "requires_first_failure_diagnostics": true,
                "bpe_ids_used_as_syntax_validity_authority": false,
                "bpe_layer_defines_validity": false,
                "source_program_validity_claim_allowed": false,
                "stable_backend_promotion_allowed": false,
            },
            "prefix_state_continuation_contract": {
                "continuation_contract_id": "target-neutral-generated-codec-prefix-state-continuation-v1",
                "completed_promotion_class": SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_COMPLETED_PROMOTION_CLASS,
                "next_missing_stable_promotion_class": SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
                "source_admissible_token_mask_contract_id": "target-neutral-generated-codec-admissible-token-mask-v1",
                "source_admissible_token_mask_completed_promotion_class": SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS,
                "target_ids": ["csharp", "go"],
                "source_query_contract": "target-neutral generated-codec state API integration record queries",
                "target_neutral_state_api_authority": "target-neutral raw transition API",
                "request_fields": [
                    "target",
                    "record_id",
                    "raw_prefix_token_count",
                    "raw_prefix_token_ids_sha256",
                    "prefix_state_case_id",
                ],
                "response_fields": [
                    "raw_prefix_token_ids",
                    "consumed_raw_tokens",
                    "prefix_status",
                    "eof_valid",
                    "finite_admissible_raw_token_ids",
                    "payload_token_ranges",
                    "bpe_raw_expansion_first_failure_diagnostic",
                ],
                "record_query_count": 10,
                "raw_prefix_request_count": 40,
                "finite_admissible_response_record_count": 10,
                "payload_range_response_record_count": 10,
                "eof_validity_response_record_count": 10,
                "bpe_raw_expansion_replay_case_count": 12,
                "bpe_raw_expansion_first_failure_case_count": 6,
                "consumes_route_owned_admissible_mask_contract": true,
                "requires_consumed_raw_token_count": true,
                "requires_prefix_status": true,
                "requires_eof_validity": true,
                "requires_first_failure_diagnostics": true,
                "bpe_ids_used_as_syntax_validity_authority": false,
                "bpe_layer_defines_validity": false,
                "source_program_validity_claim_allowed": false,
                "stable_backend_promotion_allowed": false,
            },
            "state_api_stable_backend_promotion_allowed": false,
            "source_program_validity_claim_allowed": false,
            "arena_ir_proof_used": false,
        },
        "strict_recovery_decode_policy": "syntax-parser-backed-recovery-v1",
        "consumed_token_accounting_cases": [
            "complete_stream",
            "trailing_pad_recovery",
            "eof",
            "non_pad_surplus",
            "malformed_structural_tokens",
            "malformed_payload_tokens",
            "invalid_parser_backed_payloads",
            "partial_prefix_failures"
        ],
        "token_vocabulary_bpe_invariant_cases": [
            "pad_token_below_vocab_size",
            "pad_token_outside_raw_base_ranges",
            "structural_ranges_do_not_overlap_payload_ranges",
            "bpe_range_starts_at_raw_base_vocab_size",
            "bpe_range_below_vocab_size_or_relocates_pad",
            "bpe_ids_do_not_collide_with_pad_token",
            "emitted_tokens_below_vocab_size",
            "emitted_tokens_exclude_pad_token"
        ],
        "completed_promotion_class": SYNTAX_COMPILER_ROUTE_COMPLETED_PROMOTION_CLASS,
        "generated_code_gate_completed_promotion_class": SYNTAX_COMPILER_GENERATED_CODE_GATE_COMPLETED_PROMOTION_CLASS,
        "strict_recovery_decode_completed_promotion_class": SYNTAX_COMPILER_STRICT_RECOVERY_DECODE_COMPLETED_PROMOTION_CLASS,
        "consumed_token_accounting_completed_promotion_class": SYNTAX_COMPILER_CONSUMED_TOKEN_ACCOUNTING_COMPLETED_PROMOTION_CLASS,
        "token_vocabulary_bpe_completed_promotion_class": SYNTAX_COMPILER_TOKEN_VOCABULARY_BPE_COMPLETED_PROMOTION_CLASS,
        "completed_construct_promotion_class": completed_construct_promotion_class(request.grammar),
        "next_missing_stable_promotion_class": NEXT_MISSING_STABLE_PROMOTION_CLASS,
    });
    let continuation_contract = artifact.metadata["target_agnostic_compiler_route"]
        ["generated_codec_state_api_route"]["prefix_state_continuation_contract"]
        .as_object_mut()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "target_agnostic_compiler_route.generated_codec_state_api_route.prefix_state_continuation_contract",
                "prefix-state continuation contract metadata must be an object",
            )
        })?;
    continuation_contract.insert(
        "response_evidence_fixture".to_string(),
        json!(SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_RESPONSE_EVIDENCE_FIXTURE),
    );
    continuation_contract.insert("response_evidence_record_count".to_string(), json!(10));
    continuation_contract.insert(
        "response_evidence_raw_prefix_response_count".to_string(),
        json!(40),
    );
    continuation_contract.insert(
        "response_evidence_bpe_replay_response_count".to_string(),
        json!(12),
    );
    continuation_contract.insert(
        "response_evidence_checked_against_target_neutral_api".to_string(),
        json!(true),
    );
    artifact.metadata_sha256 = json_sha256(&artifact.metadata).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "target_agnostic_compiler_route.metadata",
            format!("failed to hash route metadata: {error}"),
        )
    })?;
    Ok(artifact)
}

fn syntax_compiler_route_target(
    grammar: &GrammarIr,
) -> Result<SyntaxProductionRouteTarget, SyntaxBackendBoundaryError> {
    if grammar.provenance.source_kind == "hand_written_toy_grammar" {
        return Ok(SyntaxProductionRouteTarget::Toy);
    }
    let construct = syntax_production_route_construct(&grammar.grammar_id).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "grammar.grammar_id",
            format!(
                "grammar {} is not registered in the syntax production route construct table",
                grammar.grammar_id
            ),
        )
    })?;
    construct.target()
}

pub fn compile_checked_syntax_production_route(
    request: SyntaxProductionRouteRequest<'_>,
) -> Result<SyntaxGeneratedCodecArtifact, SyntaxBackendBoundaryError> {
    validate_checked_route_input(request)?;
    let target = request.target;
    let registry_entry = syntax_codec_registry_entry(&request.grammar.grammar_id);
    if let Some(entry) = registry_entry
        && entry.target()? != target
    {
        return Err(SyntaxBackendBoundaryError::new(
            "syntax_codec_registry.target_id",
            format!(
                "registry target {} does not match requested target {} for {}",
                entry.target_id,
                target.target_id(),
                request.grammar.grammar_id
            ),
        ));
    }
    let source = match registry_entry {
        Some(entry) => render_registered_syntax_codec_fixture(entry, request.sidecar)?,
        None => match target {
            SyntaxProductionRouteTarget::Toy => render_toy_syntax_codec_fixture(request.sidecar),
            SyntaxProductionRouteTarget::CSharp => {
                render_csharp_syntax_codec_fixture(request.sidecar)
            }
            SyntaxProductionRouteTarget::Go => render_go_syntax_codec_fixture(request.sidecar),
        },
    };
    let crate_name = match registry_entry {
        Some(entry) => entry.crate_name()?,
        None => syntax_production_route_crate_name(target, &request.grammar.grammar_id),
    };
    let renderer = match registry_entry {
        Some(entry) => entry.renderer()?,
        None => target.renderer(),
    };
    let renderer_family = match registry_entry {
        Some(entry) => entry.renderer_family()?,
        None => target.renderer_family(),
    };
    let construct_class = registry_entry
        .map(|entry| entry.construct_class())
        .unwrap_or_else(|| syntax_production_route_construct_class(&request.grammar.grammar_id));
    let generation_options = json!({
        "renderer": renderer,
        "renderer_version": 1,
        "crate_name": crate_name.clone(),
        "crate_target": "bin",
        "compiler_path": "sidecar-only",
    });
    let mut metadata = syntax_generated_codec_metadata(
        target.target_id(),
        request.grammar,
        request.sidecar,
        &source,
        &generation_options,
    );
    let projector_metadata_setup = syntax_production_route_construct(&request.grammar.grammar_id)
        .map(syntax_production_route_projector_metadata_setup);
    metadata["production_route"] = json!({
        "artifact_kind": ARTIFACT_KIND,
        "version": VERSION,
        "tool": TOOL,
        "status": STATUS,
        "route_entrypoint": ROUTE_ENTRYPOINT,
        "route_gate": ROUTE_GATE,
        "route_setup_source_of_truth": ROUTE_SETUP_SOURCE_OF_TRUTH,
        "syntax_codec_registry_source_of_truth": CODEC_REGISTRY_SOURCE_OF_TRUTH,
        "projector_metadata_setup_source": PROJECTOR_METADATA_SETUP_SOURCE_OF_TRUTH,
        "target": target.target_id(),
        "accepted_input": "checked GrammarIR -> StructuralModel",
        "renderer": renderer,
        "renderer_family": renderer_family,
        "crate_name": crate_name,
        "construct_class": construct_class,
        "registry_entry_resolved": registry_entry.is_some(),
        "output_artifact": "SyntaxGeneratedCodecArtifact",
        "stable_backend_promoted": true,
        "stable_backend_scope": STABLE_BACKEND_SCOPE,
        "stable_metadata_promotion_completed_class": STABLE_METADATA_PROMOTION_COMPLETED_CLASS,
        "stable_promotion_decision": STABLE_METADATA_PROMOTION_DECISION,
        "stable_promotion_decision_fixture": STABLE_METADATA_PROMOTION_DECISION_FIXTURE_PATH,
        "generated_json_schema_output_changed": false,
        "core_compiler_path_changed": false,
        "completed_promotion_class": COMPLETED_PROMOTION_CLASS,
        "completed_construct_promotion_class": completed_construct_promotion_class(request.grammar),
        "completed_route_setup_promotion_class": ROUTE_SETUP_COMPLETED_PROMOTION_CLASS,
        "completed_projector_metadata_setup_class": PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS,
        "projector_metadata_setup": projector_metadata_setup,
        "next_missing_stable_promotion_class": NEXT_MISSING_STABLE_PROMOTION_CLASS,
    });
    let metadata_sha256 = json_sha256(&metadata).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "production_route.metadata",
            format!("failed to hash route metadata: {error}"),
        )
    })?;

    Ok(SyntaxGeneratedCodecArtifact {
        target: target.target_id(),
        source,
        metadata,
        metadata_sha256,
    })
}

fn completed_construct_promotion_class(grammar: &GrammarIr) -> Option<&'static str> {
    syntax_production_route_construct(&grammar.grammar_id)
        .map(|construct| construct.completed_promotion_class)
}

pub fn lower_registered_parser_backed_syntax_entry(
    entry: &SyntaxCodecRegistryEntry,
) -> Result<(GrammarIr, GrammarStructuralModelSidecar), SyntaxBackendBoundaryError> {
    let grammar = match entry.target()? {
        SyntaxProductionRouteTarget::CSharp => CSharpSyntaxMetadataAdapter::new(entry.grammar_id)
            .lower_metadata_file(Path::new(entry.metadata_path))
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "csharp_syntax_metadata.metadata_path",
                    format!(
                        "failed to lower {} from {}: {error}",
                        entry.grammar_id, entry.metadata_path
                    ),
                )
            })?,
        SyntaxProductionRouteTarget::Go => GoSyntaxMetadataAdapter::new(entry.grammar_id)
            .lower_metadata_file(Path::new(entry.metadata_path))
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "go_syntax_metadata.metadata_path",
                    format!(
                        "failed to lower {} from {}: {error}",
                        entry.grammar_id, entry.metadata_path
                    ),
                )
            })?,
        SyntaxProductionRouteTarget::Toy => {
            return Err(SyntaxBackendBoundaryError::new(
                "syntax_codec_registry.target_id",
                "toy is hand-written and is not a parser-backed registry entry",
            ));
        }
    };
    let sidecar = match entry.target()? {
        SyntaxProductionRouteTarget::CSharp => lower_grammar_ir_to_structural_model(
            &grammar,
            syntax_codec::csharp_syntax_token_layout(),
        ),
        SyntaxProductionRouteTarget::Go => {
            lower_grammar_ir_to_structural_model(&grammar, syntax_codec::go_syntax_token_layout())
        }
        SyntaxProductionRouteTarget::Toy => unreachable!("toy target rejected above"),
    }
    .map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "syntax_codec_registry.structural_model",
            format!(
                "failed to lower {} structural model: {error}",
                entry.grammar_id
            ),
        )
    })?;
    Ok((grammar, sidecar))
}

pub fn render_registered_syntax_codec_fixture(
    entry: &SyntaxCodecRegistryEntry,
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<String, SyntaxBackendBoundaryError> {
    Ok(match entry.target()? {
        SyntaxProductionRouteTarget::CSharp => render_csharp_syntax_codec_fixture(sidecar),
        SyntaxProductionRouteTarget::Go => render_go_syntax_codec_fixture(sidecar),
        SyntaxProductionRouteTarget::Toy => render_toy_syntax_codec_fixture(sidecar),
    })
}

fn syntax_production_route_crate_name(
    target: SyntaxProductionRouteTarget,
    grammar_id: &str,
) -> String {
    match grammar_id {
        "csharp_minimal_class_syntax_v0" | "go_minimal_function_syntax_v0" => {
            target.crate_name().to_string()
        }
        grammar_id if grammar_id.ends_with("_syntax_v0") => format!(
            "{}-syntax-generated-codec",
            grammar_id.trim_end_matches("_syntax_v0").replace('_', "-")
        ),
        _ => target.crate_name().to_string(),
    }
}

fn syntax_production_route_construct_class(grammar_id: &str) -> String {
    grammar_id
        .strip_suffix("_syntax_v0")
        .unwrap_or(grammar_id)
        .to_string()
}

fn validate_checked_route_input(
    request: SyntaxProductionRouteRequest<'_>,
) -> Result<(), SyntaxBackendBoundaryError> {
    let grammar = request.grammar;
    let sidecar = request.sidecar;
    require_u32("grammar.version", grammar.version, GRAMMAR_IR_VERSION)?;
    require_string(
        "sidecar.artifact_kind",
        &sidecar.artifact_kind,
        STRUCTURAL_MODEL_ARTIFACT_KIND,
    )?;
    require_string(
        "sidecar.model_version",
        &sidecar.model_version,
        MODEL_VERSION,
    )?;
    require_string(
        "sidecar.input.grammar_id",
        &sidecar.input.grammar_id,
        &grammar.grammar_id,
    )?;
    require_u32(
        "sidecar.input.grammar_version",
        sidecar.input.grammar_version,
        grammar.version,
    )?;
    if sidecar.input.provenance != grammar.provenance {
        return Err(SyntaxBackendBoundaryError::new(
            "sidecar.input.provenance",
            "sidecar provenance must match GrammarIR provenance",
        ));
    }
    require_string(
        "sidecar.structural_model.frontend_id",
        &sidecar.structural_model.frontend_id,
        &grammar.grammar_id,
    )?;
    require_bool(
        "sidecar.boundary.generated_json_schema_output_changed",
        sidecar
            .structural_model
            .boundary
            .generated_json_schema_output_changed,
        false,
    )?;
    require_bool(
        "sidecar.boundary.core_compiler_path_changed",
        sidecar.structural_model.boundary.core_compiler_path_changed,
        false,
    )?;
    require_bool(
        "sidecar.boundary.production_grammar_compiler_path_changed",
        sidecar
            .structural_model
            .boundary
            .production_grammar_compiler_path_changed,
        false,
    )?;
    require_bool(
        "sidecar.proof.tokenizable_decisions_match",
        sidecar.proof.tokenizable_decisions_match,
        true,
    )?;
    require_string(
        "grammar.provenance.source_kind",
        &grammar.provenance.source_kind,
        request.target.source_kind(),
    )
}

fn require_string(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_u32(
    field: &'static str,
    actual: u32,
    expected: u32,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_bool(
    field: &'static str,
    actual: bool,
    expected: bool,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_all(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for required in expected {
        if !actual.contains(required) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("missing required value {required}"),
            ));
        }
    }
    Ok(())
}
