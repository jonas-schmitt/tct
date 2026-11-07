// go_syntax_metadata_smoke reproduces the checked minimal Go syntax metadata
// projection from source through go/parser and go/ast.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strconv"
	"strings"
)

const (
	tool            = "go_syntax_metadata_smoke"
	reportVersion   = 1
	artifactKind    = "tct_go_syntax_metadata_v0"
	metadataVersion = 0
	defaultFixture  = "minimal-function"
)

type fixtureSpec struct {
	Name                   string
	DefaultSource          string
	DefaultCheckedMetadata string
	DefaultGeneratedOutput string
	Validate               func(*ast.File) error
	Rules                  func() []syntaxRule
	RulesFromFile          func(*ast.File, *token.FileSet) []syntaxRule
}

var fixtureSpecs = map[string]fixtureSpec{
	"minimal-function": {
		Name:                   "minimal-function",
		DefaultSource:          "tests/fixtures/go_syntax_sources/minimal/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/minimal_function/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/minimal-function.syntax-metadata-v0.json",
		Validate:               validateMinimalFunction,
		Rules:                  minimalFunctionRules,
	},
	"local-declaration": {
		Name:                   "local-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-declaration.syntax-metadata-v0.json",
		Validate:               validateLocalDeclaration,
		Rules:                  localDeclarationRules,
	},
	"local-assignment": {
		Name:                   "local-assignment",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_assignment/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_assignment/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-assignment.syntax-metadata-v0.json",
		Validate:               validateLocalAssignment,
		Rules:                  localAssignmentRules,
	},
	"local-escaped-assignment": {
		Name:                   "local-escaped-assignment",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_escaped_assignment/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_escaped_assignment/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-escaped-assignment.syntax-metadata-v0.json",
		Validate:               validateLocalEscapedAssignment,
		Rules:                  localAssignmentRules,
	},
	"local-short-var-declaration": {
		Name:                   "local-short-var-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_short_var_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_short_var_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-short-var-declaration.syntax-metadata-v0.json",
		Validate:               validateLocalShortVarDeclaration,
		Rules:                  localShortVarDeclarationRules,
	},
	"local-integer-short-var-declaration": {
		Name:                   "local-integer-short-var-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_integer_short_var_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_integer_short_var_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-integer-short-var-declaration.syntax-metadata-v0.json",
		Validate:               validateLocalIntegerShortVarDeclaration,
		Rules:                  localIntegerShortVarDeclarationRules,
	},
	"local-integer-return-literal": {
		Name:                   "local-integer-return-literal",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_integer_return_literal/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_integer_return_literal/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-integer-return-literal.syntax-metadata-v0.json",
		Validate:               validateLocalIntegerReturnLiteral,
		Rules:                  localIntegerReturnLiteralRules,
	},
	"local-parenthesized-integer-return-expression": {
		Name:                   "local-parenthesized-integer-return-expression",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_parenthesized_integer_return_expression/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_parenthesized_integer_return_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-parenthesized-integer-return-expression.syntax-metadata-v0.json",
		Validate:               validateLocalParenthesizedIntegerReturnExpression,
		Rules:                  localParenthesizedIntegerReturnExpressionRules,
	},
	"local-unary-integer-return-expression": {
		Name:                   "local-unary-integer-return-expression",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_unary_integer_return_expression/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_unary_integer_return_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-unary-integer-return-expression.syntax-metadata-v0.json",
		Validate:               validateLocalUnaryIntegerReturnExpression,
		Rules:                  localUnaryIntegerReturnExpressionRules,
	},
	"local-binary-integer-return-expression": {
		Name:                   "local-binary-integer-return-expression",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_binary_integer_return_expression/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_binary_integer_return_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-binary-integer-return-expression.syntax-metadata-v0.json",
		Validate:               validateLocalBinaryIntegerReturnExpression,
		Rules:                  localBinaryIntegerReturnExpressionRules,
	},
	"local-call-return-expression": {
		Name:                   "local-call-return-expression",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_call_return_expression/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_call_return_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-call-return-expression.syntax-metadata-v0.json",
		Validate:               validateLocalCallReturnExpression,
		Rules:                  localCallReturnExpressionRules,
	},
	"local-argument-call-return-expression": {
		Name:                   "local-argument-call-return-expression",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_argument_call_return_expression/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_argument_call_return_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-argument-call-return-expression.syntax-metadata-v0.json",
		Validate:               validateLocalArgumentCallReturnExpression,
		Rules:                  localArgumentCallReturnExpressionRules,
	},
	"local-string-return-literal": {
		Name:                   "local-string-return-literal",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_string_return_literal/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_string_return_literal/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-string-return-literal.syntax-metadata-v0.json",
		Validate:               validateLocalStringReturnLiteral,
		Rules:                  localStringReturnLiteralRules,
	},
	"local-float-return-literal": {
		Name:                   "local-float-return-literal",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_float_return_literal/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_float_return_literal/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-float-return-literal.syntax-metadata-v0.json",
		Validate:               validateLocalFloatReturnLiteral,
		Rules:                  localFloatReturnLiteralRules,
	},
	"local-boolean-return-identifier": {
		Name:                   "local-boolean-return-identifier",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_boolean_return_identifier/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_boolean_return_identifier/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-boolean-return-identifier.syntax-metadata-v0.json",
		Validate:               validateLocalBooleanReturnIdentifier,
		Rules:                  localBooleanReturnIdentifierRules,
	},
	"local-nil-return-identifier": {
		Name:                   "local-nil-return-identifier",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_nil_return_identifier/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_nil_return_identifier/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-nil-return-identifier.syntax-metadata-v0.json",
		Validate:               validateLocalNilReturnIdentifier,
		Rules:                  localNilReturnIdentifierRules,
	},
	"local-typed-declaration": {
		Name:                   "local-typed-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_typed_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_typed_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-typed-declaration.syntax-metadata-v0.json",
		Validate:               validateLocalTypedDeclaration,
		Rules:                  localTypedDeclarationRules,
	},
	"local-zero-value-typed-declaration": {
		Name:                   "local-zero-value-typed-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_zero_value_typed_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_zero_value_typed_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-zero-value-typed-declaration.syntax-metadata-v0.json",
		Validate:               validateLocalZeroValueTypedDeclaration,
		Rules:                  localZeroValueTypedDeclarationRules,
	},
	"local-integer-typed-declaration": {
		Name:                   "local-integer-typed-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_integer_typed_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_integer_typed_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-integer-typed-declaration.syntax-metadata-v0.json",
		Validate:               validateLocalIntegerTypedDeclaration,
		Rules:                  localIntegerTypedDeclarationRules,
	},
	"local-untyped-integer-declaration": {
		Name:                   "local-untyped-integer-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_untyped_integer_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_untyped_integer_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-untyped-integer-declaration.syntax-metadata-v0.json",
		Validate:               validateLocalUntypedIntegerDeclaration,
		Rules:                  localUntypedIntegerDeclarationRules,
	},
	"local-integer-assignment": {
		Name:                   "local-integer-assignment",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_integer_assignment/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_integer_assignment/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-integer-assignment.syntax-metadata-v0.json",
		Validate:               validateLocalIntegerAssignment,
		Rules:                  localIntegerAssignmentRules,
	},
	"local-compound-assignment": {
		Name:                   "local-compound-assignment",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_compound_assignment/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_compound_assignment/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-compound-assignment.syntax-metadata-v0.json",
		Validate:               validateLocalCompoundAssignment,
		Rules:                  localCompoundAssignmentRules,
	},
	"local-increment-decrement": {
		Name:                   "local-increment-decrement",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_increment_decrement/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_increment_decrement/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-increment-decrement.syntax-metadata-v0.json",
		Validate:               validateLocalIncrementDecrement,
		Rules:                  localIncrementDecrementRules,
	},
	"local-branch-statement": {
		Name:                   "local-branch-statement",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_branch_statement/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_branch_statement/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-branch-statement.syntax-metadata-v0.json",
		Validate:               validateLocalBranchStatement,
		Rules:                  localBranchStatementRules,
	},
	"local-if-else": {
		Name:                   "local-if-else",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_if_else/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_if_else/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-if-else.syntax-metadata-v0.json",
		Validate:               validateLocalIfElse,
		Rules:                  localIfElseRules,
	},
	"local-for-condition": {
		Name:                   "local-for-condition",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_for_condition/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_for_condition/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-for-condition.syntax-metadata-v0.json",
		Validate:               validateLocalForCondition,
		Rules:                  localForConditionRules,
	},
	"local-switch": {
		Name:                   "local-switch",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_switch/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_switch/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-switch.syntax-metadata-v0.json",
		Validate:               validateLocalSwitch,
		Rules:                  localSwitchRules,
	},
	"local-switch-init": {
		Name:                   "local-switch-init",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/local_switch_init/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_switch_init/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-switch-init.syntax-metadata-v0.json",
		Validate:               validateLocalSwitchInit,
		Rules:                  localSwitchInitRules,
	},
	"local-type-switch": {
		Name:                   "local-type-switch",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/local_type_switch/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_type_switch/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-type-switch.syntax-metadata-v0.json",
		Validate:               validateLocalTypeSwitch,
		Rules:                  localTypeSwitchRules,
	},
	"local-range-string": {
		Name:                   "local-range-string",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/local_range_string/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_range_string/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-range-string.syntax-metadata-v0.json",
		Validate:               validateLocalRangeString,
		Rules:                  localRangeStringRules,
	},
	"package-type-declaration": {
		Name:                   "package-type-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/package_type_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/package_type_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/package-type-declaration.syntax-metadata-v0.json",
		Validate:               validatePackageTypeDeclaration,
		Rules:                  packageTypeDeclarationRules,
	},
	"type-spec-assign": {
		Name:                   "type-spec-assign",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/type_spec_assign/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/type_spec_assign/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/type-spec-assign.syntax-metadata-v0.json",
		Validate:               validateTypeSpecAssign,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"type-spec-comment": {
		Name:                   "type-spec-comment",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/type_spec_comment/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/type_spec_comment/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/type-spec-comment.syntax-metadata-v0.json",
		Validate:               validateTypeSpecComment,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"type-spec-doc": {
		Name:                   "type-spec-doc",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/type_spec_doc/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/type_spec_doc/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/type-spec-doc.syntax-metadata-v0.json",
		Validate:               validateTypeSpecDoc,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"type-spec-type-params": {
		Name:                   "type-spec-type-params",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/type_spec_type_params/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/type_spec_type_params/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/type-spec-type-params.syntax-metadata-v0.json",
		Validate:               validateTypeSpecTypeParams,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"package-const-declaration": {
		Name:                   "package-const-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/package_const_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/package_const_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/package-const-declaration.syntax-metadata-v0.json",
		Validate:               validatePackageConstDeclaration,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"method-declaration": {
		Name:                   "method-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/method_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/method_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/method-declaration.syntax-metadata-v0.json",
		Validate:               validateMethodDeclaration,
		Rules:                  methodDeclarationRules,
	},
	"struct-type": {
		Name:                   "struct-type",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/struct_type/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/struct_type/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/struct-type.syntax-metadata-v0.json",
		Validate:               validateStructType,
		Rules:                  structTypeRules,
	},
	"composite-literal": {
		Name:                   "composite-literal",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/composite_literal/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/composite_literal/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/composite-literal.syntax-metadata-v0.json",
		Validate:               validateCompositeLiteral,
		Rules:                  compositeLiteralRules,
	},
	"key-value-composite-literal": {
		Name:                   "key-value-composite-literal",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/key_value_composite_literal/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/key_value_composite_literal/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/key-value-composite-literal.syntax-metadata-v0.json",
		Validate:               validateKeyValueCompositeLiteral,
		Rules:                  keyValueCompositeLiteralRules,
	},
	"array-composite-literal": {
		Name:                   "array-composite-literal",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/array_composite_literal/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/array_composite_literal/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/array-composite-literal.syntax-metadata-v0.json",
		Validate:               validateArrayCompositeLiteral,
		Rules:                  arrayCompositeLiteralRules,
	},
	"index-expression": {
		Name:                   "index-expression",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/index_expression/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/index_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/index-expression.syntax-metadata-v0.json",
		Validate:               validateIndexExpression,
		Rules:                  indexExpressionRules,
	},
	"index-list-expression": {
		Name:                   "index-list-expression",
		DefaultSource:          "tests/fixtures/go_syntax_sources/index_list/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/index_list_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/index-list-expression.syntax-metadata-v0.json",
		Validate:               validateIndexListExpression,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"slice-expression": {
		Name:                   "slice-expression",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/slice_expression/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/slice_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/slice-expression.syntax-metadata-v0.json",
		Validate:               validateSliceExpression,
		Rules:                  sliceExpressionRules,
	},
	"slice-expression-three-index": {
		Name:                   "slice-expression-three-index",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/slice_expression_three_index/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/slice_expression_three_index/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/slice-expression-three-index.syntax-metadata-v0.json",
		Validate:               validateSliceExpressionThreeIndex,
		Rules:                  sliceExpressionThreeIndexRules,
	},
	"star-expression": {
		Name:                   "star-expression",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/star_expression/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/star_expression/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/star-expression.syntax-metadata-v0.json",
		Validate:               validateStarExpression,
		Rules:                  starExpressionRules,
	},
	"package-var-declaration": {
		Name:                   "package-var-declaration",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/package_var_declaration/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/package_var_declaration/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/package-var-declaration.syntax-metadata-v0.json",
		Validate:               validatePackageVarDeclaration,
		Rules:                  packageVarDeclarationRules,
	},
	"value-spec-comment": {
		Name:                   "value-spec-comment",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/value_spec_comment/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/value_spec_comment/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/value-spec-comment.syntax-metadata-v0.json",
		Validate:               validateValueSpecComment,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"import-spec-name": {
		Name:                   "import-spec-name",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/import_spec_name/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/import_spec_name/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/import-spec-name.syntax-metadata-v0.json",
		Validate:               validateImportSpecName,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"import-spec-comment": {
		Name:                   "import-spec-comment",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/import_spec_comment/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/import_spec_comment/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/import-spec-comment.syntax-metadata-v0.json",
		Validate:               validateImportSpecComment,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"function-literal": {
		Name:                   "function-literal",
		DefaultSource:          "tests/fixtures/go_syntax_metadata/function_literal/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/function_literal/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/function-literal.syntax-metadata-v0.json",
		Validate:               validateFunctionLiteral,
		Rules:                  functionLiteralRules,
	},
	"local-generic-function": {
		Name:                   "local-generic-function",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/local_generic_function/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_generic_function/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-generic-function.syntax-metadata-v0.json",
		Validate:               validateLocalGenericFunction,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"concurrency-map-interface-type-assertion": {
		Name:                   "concurrency-map-interface-type-assertion",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/concurrency_map_interface_type_assertion/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/concurrency_map_interface_type_assertion/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/concurrency-map-interface-type-assertion.syntax-metadata-v0.json",
		Validate:               validateConcurrencyMapInterfaceTypeAssertion,
		Rules:                  concurrencyMapInterfaceTypeAssertionRules,
	},
	"local-variadic-ellipsis": {
		Name:                   "local-variadic-ellipsis",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/local_variadic_ellipsis/main.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/local_variadic_ellipsis/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/local-variadic-ellipsis.syntax-metadata-v0.json",
		Validate:               validateLocalVariadicEllipsis,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"generic-source-file": {
		Name:                   "generic-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/os/dir_unix.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/os_dir_unix_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/generic-source-file.syntax-metadata-v0.json",
		Validate:               validateGenericSourceFile,
		RulesFromFile:          parserBackedSourceFileRulesWithRetainedRootComments,
	},
	"strings-compare-source-file": {
		Name:                   "strings-compare-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/strings/compare.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/strings_compare_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/strings-compare-source-file.syntax-metadata-v0.json",
		Validate:               validateStringsCompareSourceFile,
		Rules:                  stringsCompareSourceFileRules,
	},
	"strings-clone-source-file": {
		Name:                   "strings-clone-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/strings/clone.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/strings_clone_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/strings-clone-source-file.syntax-metadata-v0.json",
		Validate:               validateStringsCloneSourceFile,
		Rules:                  stringsCloneSourceFileRules,
	},
	"sort-interface-source-file-declaration-prefix": {
		Name:                   "sort-interface-source-file-declaration-prefix",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/sort/example_interface_test.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/sort_interface_source_file_declaration_prefix/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/sort-interface-source-file-declaration-prefix.syntax-metadata-v0.json",
		Validate:               validateSortInterfaceSourceFileDeclarationPrefix,
		Rules:                  sortInterfaceSourceFileDeclarationPrefixRules,
	},
	"sort-interface-source-file": {
		Name:                   "sort-interface-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/sort/example_interface_test.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/sort_interface_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/sort-interface-source-file.syntax-metadata-v0.json",
		Validate:               validateSortInterfaceSourceFileDeclarationPrefix,
		Rules:                  sortInterfaceSourceFileRules,
	},
	"sort-search-source-file-example-prefix": {
		Name:                   "sort-search-source-file-example-prefix",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/sort_search_source_file_example_prefix/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/sort-search-source-file-example-prefix.syntax-metadata-v0.json",
		Validate:               validateSortSearchSourceFileExamplePrefix,
		Rules:                  sortSearchSourceFileExamplePrefixRules,
	},
	"sort-search-source-file": {
		Name:                   "sort-search-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/sort_search_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/sort-search-source-file.syntax-metadata-v0.json",
		Validate:               validateSortSearchSourceFile,
		Rules:                  sortSearchSourceFileRules,
	},
	"strings-search-source-file-finder-prefix": {
		Name:                   "strings-search-source-file-finder-prefix",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/strings/search.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/strings_search_source_file_finder_prefix/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/strings-search-source-file-finder-prefix.syntax-metadata-v0.json",
		Validate:               validateStringsSearchSourceFileFinderPrefix,
		Rules:                  stringsSearchSourceFileFinderPrefixRules,
	},
	"strings-search-source-file": {
		Name:                   "strings-search-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/strings/search.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/strings_search_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/strings-search-source-file.syntax-metadata-v0.json",
		Validate:               validateStringsSearchSourceFile,
		Rules:                  stringsSearchSourceFileRules,
	},
	"path-match-source-file-match-prefix": {
		Name:                   "path-match-source-file-match-prefix",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/path/match.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/path_match_source_file_match_prefix/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/path-match-source-file-match-prefix.syntax-metadata-v0.json",
		Validate:               validatePathMatchSourceFileMatchPrefix,
		Rules:                  pathMatchSourceFileMatchPrefixRules,
	},
	"path-match-source-file": {
		Name:                   "path-match-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/path/match.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/path_match_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/path-match-source-file.syntax-metadata-v0.json",
		Validate:               validatePathMatchSourceFile,
		Rules:                  pathMatchSourceFileRules,
	},
	"crypto-sysrand-rand-windows-source-file": {
		Name:                   "crypto-sysrand-rand-windows-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/crypto/internal/sysrand/rand_windows.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/crypto_sysrand_rand_windows_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/crypto-sysrand-rand-windows-source-file.syntax-metadata-v0.json",
		Validate:               validateCryptoSysrandRandWindowsSourceFile,
		Rules:                  cryptoSysrandRandWindowsSourceFileRules,
	},
	"crypto-subtle-xor-source-file": {
		Name:                   "crypto-subtle-xor-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/crypto/subtle/xor.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/crypto_subtle_xor_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/crypto-subtle-xor-source-file.syntax-metadata-v0.json",
		Validate:               validateCryptoSubtleXorSourceFile,
		Rules:                  cryptoSubtleXorSourceFileRules,
	},
	"internal-coverage-uleb128-source-file": {
		Name:                   "internal-coverage-uleb128-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/internal/coverage/uleb128/uleb128.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/internal_coverage_uleb128_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/internal-coverage-uleb128-source-file.syntax-metadata-v0.json",
		Validate:               validateInternalCoverageUleb128SourceFile,
		Rules:                  singleFunctionLoopSourceFileRules,
	},
	"net-lookup-plan9-source-file": {
		Name:                   "net-lookup-plan9-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/net/lookup_plan9.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/net_lookup_plan9_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/net-lookup-plan9-source-file.syntax-metadata-v0.json",
		Validate:               validateNetLookupPlan9SourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"syscall-fs-js-source-file": {
		Name:                   "syscall-fs-js-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/syscall/fs_js.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/syscall_fs_js_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/syscall-fs-js-source-file.syntax-metadata-v0.json",
		Validate:               validateSyscallFsJSSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"os-signal-notify-context-source-span": {
		Name:                   "os-signal-notify-context-source-span",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/os_signal_notify_context_source_span/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/os-signal-notify-context-source-span.syntax-metadata-v0.json",
		Validate:               validateOsSignalNotifyContextSourceSpan,
		RulesFromFile:          parserBackedNotifyContextSourceSpanRules,
	},
	"os-signal-source-file": {
		Name:                   "os-signal-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/os_signal_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/os-signal-source-file.syntax-metadata-v0.json",
		Validate:               validateOsSignalSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"httptest-server-source-file": {
		Name:                   "httptest-server-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/net/http/httptest/server.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/httptest_server_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/httptest-server-source-file.syntax-metadata-v0.json",
		Validate:               validateHttptestServerSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"net-nss-source-file": {
		Name:                   "net-nss-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/net/nss.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/net_nss_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/net-nss-source-file.syntax-metadata-v0.json",
		Validate:               validateNetNSSSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"net-ipsock-plan9-source-file": {
		Name:                   "net-ipsock-plan9-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/net/ipsock_plan9.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/net_ipsock_plan9_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/net-ipsock-plan9-source-file.syntax-metadata-v0.json",
		Validate:               validateNetIpsockPlan9SourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"os-dir-unix-source-file": {
		Name:                   "os-dir-unix-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/os/dir_unix.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/os_dir_unix_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/os-dir-unix-source-file.syntax-metadata-v0.json",
		Validate:               validateOsDirUnixSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"net-rpc-client-source-file": {
		Name:                   "net-rpc-client-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/net/rpc/client.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/net_rpc_client_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/net-rpc-client-source-file.syntax-metadata-v0.json",
		Validate:               validateNetRPCClientSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"internal-coverage-cfile-testsupport-source-file": {
		Name:                   "internal-coverage-cfile-testsupport-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/internal/coverage/cfile/testsupport.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/internal_coverage_cfile_testsupport_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/internal-coverage-cfile-testsupport-source-file.syntax-metadata-v0.json",
		Validate:               validateInternalCoverageCfileTestsupportSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"internal-fuzz-mutator-source-file": {
		Name:                   "internal-fuzz-mutator-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/internal/fuzz/mutator.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/internal_fuzz_mutator_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/internal-fuzz-mutator-source-file.syntax-metadata-v0.json",
		Validate:               validateInternalFuzzMutatorSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"runtime-debug-mod-source-file": {
		Name:                   "runtime-debug-mod-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/runtime/debug/mod.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/runtime_debug_mod_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/runtime-debug-mod-source-file.syntax-metadata-v0.json",
		Validate:               validateRuntimeDebugModSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"go-ast-commentmap-source-file": {
		Name:                   "go-ast-commentmap-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/go/ast/commentmap.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/go_ast_commentmap_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/go-ast-commentmap-source-file.syntax-metadata-v0.json",
		Validate:               validateGoAstCommentmapSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"go-types-gotype-source-file": {
		Name:                   "go-types-gotype-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/go/types/gotype.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/go_types_gotype_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/go-types-gotype-source-file.syntax-metadata-v0.json",
		Validate:               validateGoTypesGotypeSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"testing-match-source-file": {
		Name:                   "testing-match-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/testing/match.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/testing_match_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/testing-match-source-file.syntax-metadata-v0.json",
		Validate:               validateTestingMatchSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"encoding-xml-typeinfo-source-file": {
		Name:                   "encoding-xml-typeinfo-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/encoding/xml/typeinfo.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/encoding_xml_typeinfo_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/encoding-xml-typeinfo-source-file.syntax-metadata-v0.json",
		Validate:               validateEncodingXMLTypeinfoSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"go-doc-comment-text-source-file": {
		Name:                   "go-doc-comment-text-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/go/doc/comment/text.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/go_doc_comment_text_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/go-doc-comment-text-source-file.syntax-metadata-v0.json",
		Validate:               validateGoDocCommentTextSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"internal-buildcfg-exp-source-file": {
		Name:                   "internal-buildcfg-exp-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/internal/buildcfg/exp.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/internal_buildcfg_exp_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/internal-buildcfg-exp-source-file.syntax-metadata-v0.json",
		Validate:               validateInternalBuildcfgExpSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"runtime-error-source-file": {
		Name:                   "runtime-error-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/runtime/error.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/runtime_error_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/runtime-error-source-file.syntax-metadata-v0.json",
		Validate:               validateRuntimeErrorSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"syscall-netlink-linux-source-file": {
		Name:                   "syscall-netlink-linux-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/syscall/netlink_linux.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/syscall_netlink_linux_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/syscall-netlink-linux-source-file.syntax-metadata-v0.json",
		Validate:               validateSyscallNetlinkLinuxSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"debug-plan9obj-file-source-file": {
		Name:                   "debug-plan9obj-file-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/debug/plan9obj/file.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/debug_plan9obj_file_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/debug-plan9obj-file-source-file.syntax-metadata-v0.json",
		Validate:               validateDebugPlan9objFileSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"fmt-errors-source-file": {
		Name:                   "fmt-errors-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/fmt/errors.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/fmt_errors_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/fmt-errors-source-file.syntax-metadata-v0.json",
		Validate:               validateFmtErrorsSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"internal-poll-fd-fsync-windows-source-file": {
		Name:                   "internal-poll-fd-fsync-windows-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/internal/poll/fd_fsync_windows.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/internal_poll_fd_fsync_windows_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/internal-poll-fd-fsync-windows-source-file.syntax-metadata-v0.json",
		Validate:               validateInternalPollFdFsyncWindowsSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"internal-dag-parse-source-file": {
		Name:                   "internal-dag-parse-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/internal/dag/parse.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/internal_dag_parse_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/internal-dag-parse-source-file.syntax-metadata-v0.json",
		Validate:               validateInternalDagParseSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"internal-trace-reader-source-file": {
		Name:                   "internal-trace-reader-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/internal/trace/reader.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/internal_trace_reader_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/internal-trace-reader-source-file.syntax-metadata-v0.json",
		Validate:               validateInternalTraceReaderSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"mime-multipart-formdata-source-file": {
		Name:                   "mime-multipart-formdata-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/mime/multipart/formdata.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/mime_multipart_formdata_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/mime-multipart-formdata-source-file.syntax-metadata-v0.json",
		Validate:               validateMimeMultipartFormdataSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
	"compress-flate-huffman-code-source-file": {
		Name:                   "compress-flate-huffman-code-source-file",
		DefaultSource:          "tests/fixtures/go_syntax_corpus/src/compress/flate/huffman_code.go",
		DefaultCheckedMetadata: "tests/fixtures/go_syntax_metadata/compress_flate_huffman_code_source_file/syntax-metadata-v0.json",
		DefaultGeneratedOutput: "target/tct-analysis/go-syntax-metadata-smoke/compress-flate-huffman-code-source-file.syntax-metadata-v0.json",
		Validate:               validateCompressFlateHuffmanCodeSourceFile,
		RulesFromFile:          parserBackedSourceFileRules,
	},
}

type metadata struct {
	ArtifactKind string           `json:"artifact_kind"`
	Version      int              `json:"version"`
	Source       metadataSource   `json:"source"`
	Parser       parserProvenance `json:"parser"`
	Rules        []syntaxRule     `json:"rules"`
}

type metadataSource struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
}

type parserProvenance struct {
	Tool           string `json:"tool"`
	Version        string `json:"version"`
	Package        string `json:"package"`
	PackageVersion string `json:"package_version"`
}

type syntaxRule struct {
	Name     string        `json:"name"`
	NodeKind string        `json:"node_kind"`
	Fields   []syntaxField `json:"fields"`
}

type syntaxField struct {
	Name         string              `json:"name"`
	Required     bool                `json:"required"`
	Multiple     bool                `json:"multiple"`
	Alternatives []syntaxAlternative `json:"alternatives"`
}

type syntaxAlternative struct {
	NodeKind string `json:"node_kind"`
	Terminal string `json:"terminal"`
}

type report struct {
	Tool              string       `json:"tool"`
	Version           int          `json:"version"`
	Status            string       `json:"status"`
	Fixture           string       `json:"fixture"`
	ArtifactKind      string       `json:"artifact_kind"`
	MetadataVersion   int          `json:"metadata_version"`
	Source            fileArtifact `json:"source"`
	CheckedMetadata   fileArtifact `json:"checked_metadata"`
	GeneratedMetadata fileArtifact `json:"generated_metadata"`
	Go                goEvidence   `json:"go"`
	Comparison        comparison   `json:"comparison"`
	Blocker           *blocker     `json:"blocker"`
}

type fileArtifact struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int    `json:"bytes"`
}

type goEvidence struct {
	Available      bool   `json:"available"`
	Version        string `json:"version"`
	ParserPackage  string `json:"parser_package"`
	ASTPackage     string `json:"ast_package"`
	SourceParser   string `json:"source_parser"`
	GeneratedRules int    `json:"generated_rules"`
}

type comparison struct {
	Ready                     bool   `json:"ready"`
	GrammarProjectionMatches  bool   `json:"grammar_projection_matches"`
	ParserMatches             bool   `json:"parser_matches"`
	SourceMatches             bool   `json:"source_matches"`
	GeneratedSHA256           string `json:"generated_sha256"`
	CheckedProjectionSHA256   string `json:"checked_projection_sha256"`
	GeneratedProjectionSHA256 string `json:"generated_projection_sha256"`
	RuleCount                 int    `json:"rule_count"`
	FieldCount                int    `json:"field_count"`
}

type blocker struct {
	Kind    string `json:"kind"`
	Message string `json:"message"`
}

type projectionRule struct {
	Name     string            `json:"name"`
	NodeKind string            `json:"node_kind"`
	Fields   []projectionField `json:"fields"`
}

type projectionField struct {
	Name         string              `json:"name"`
	Required     bool                `json:"required"`
	Multiple     bool                `json:"multiple"`
	Alternatives []syntaxAlternative `json:"alternatives"`
}

func main() {
	fixture := flag.String("fixture", defaultFixture, "named Go syntax fixture")
	source := flag.String("source", "", "Go source fixture")
	checkedMetadata := flag.String("checked-metadata", "", "checked syntax metadata fixture")
	generatedOutput := flag.String("generated-output", "", "generated syntax metadata output")
	jsonOutput := flag.String("json-output", "", "write JSON report to this path instead of stdout")
	selfTest := flag.Bool("self-test", false, "fail unless generated metadata matches the checked projection")
	writeCheckedMetadata := flag.Bool("write-checked-metadata", false, "write generated metadata to the checked metadata fixture before comparing")
	flag.Parse()

	spec, ok := fixtureSpecs[*fixture]
	if !ok {
		exitf("unknown fixture %q; supported fixtures: %s", *fixture, supportedFixtures())
	}
	sourcePath := defaulted(*source, spec.DefaultSource)
	checkedMetadataPath := defaulted(*checkedMetadata, spec.DefaultCheckedMetadata)
	generatedOutputPath := defaulted(*generatedOutput, spec.DefaultGeneratedOutput)

	report := buildReport(spec, sourcePath, checkedMetadataPath, generatedOutputPath, *writeCheckedMetadata)
	if *selfTest {
		if report.Status != "ready" {
			encoded, _ := json.MarshalIndent(report, "", "  ")
			fmt.Fprintln(os.Stderr, string(encoded))
			os.Exit(1)
		}
		fmt.Println("go_syntax_metadata_smoke self-test passed")
		return
	}
	if err := writeJSON(*jsonOutput, report); err != nil {
		exitf("%v", err)
	}
}

func buildReport(spec fixtureSpec, sourcePath string, checkedMetadataPath string, generatedOutput string, writeCheckedMetadata bool) report {
	sourceDisplayPath := repoRelative(sourcePath)
	sourceInfo, sourceErr := fileInfo(sourcePath, sourceDisplayPath)
	checkedInfo, checkedErr := fileInfo(checkedMetadataPath, repoRelative(checkedMetadataPath))
	result := report{
		Tool:            tool,
		Version:         reportVersion,
		Status:          "blocked",
		Fixture:         spec.Name,
		ArtifactKind:    artifactKind,
		MetadataVersion: metadataVersion,
		Source:          sourceInfo,
		CheckedMetadata: checkedInfo,
		Go: goEvidence{
			Available:     true,
			Version:       runtime.Version(),
			ParserPackage: "go/parser",
			ASTPackage:    "go/ast",
			SourceParser:  "parser.ParseFile",
		},
	}
	if sourceErr != nil {
		result.Blocker = &blocker{Kind: "source_unavailable", Message: sourceErr.Error()}
		return result
	}
	generated, err := generateMetadata(spec, sourcePath, sourceDisplayPath)
	if err != nil {
		result.Blocker = &blocker{Kind: "go_parser_metadata_failed", Message: err.Error()}
		return result
	}
	result.Go.GeneratedRules = len(generated.Rules)
	if err := writeMetadata(generatedOutput, generated); err != nil {
		result.Blocker = &blocker{Kind: "generated_metadata_write_failed", Message: err.Error()}
		return result
	}
	generatedInfo, err := fileInfo(generatedOutput, repoRelative(generatedOutput))
	if err != nil {
		result.Blocker = &blocker{Kind: "generated_metadata_unavailable", Message: err.Error()}
		return result
	}
	result.GeneratedMetadata = generatedInfo

	if writeCheckedMetadata {
		if err := writeMetadata(checkedMetadataPath, generated); err != nil {
			result.Blocker = &blocker{Kind: "checked_metadata_write_failed", Message: err.Error()}
			return result
		}
		checkedInfo, checkedErr = fileInfo(checkedMetadataPath, repoRelative(checkedMetadataPath))
		result.CheckedMetadata = checkedInfo
	}
	if checkedErr != nil {
		result.Blocker = &blocker{Kind: "checked_metadata_unavailable", Message: checkedErr.Error()}
		return result
	}

	checked, err := loadMetadata(checkedMetadataPath)
	if err != nil {
		result.Blocker = &blocker{Kind: "checked_metadata_parse_failed", Message: err.Error()}
		return result
	}
	result.Comparison = compareMetadata(checked, generated)
	if result.Comparison.Ready {
		result.Status = "ready"
		return result
	}
	result.Blocker = &blocker{
		Kind:    "generated_metadata_mismatch",
		Message: "generated go/parser metadata did not match the checked GrammarIR projection",
	}
	return result
}

func generateMetadata(spec fixtureSpec, sourcePath string, sourceDisplayPath string) (*metadata, error) {
	source, err := os.ReadFile(sourcePath)
	if err != nil {
		return nil, fmt.Errorf("read source: %w", err)
	}
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, sourcePath, source, parser.ParseComments)
	if err != nil {
		return nil, fmt.Errorf("parse source: %w", err)
	}
	if err := spec.Validate(file); err != nil {
		return nil, err
	}
	rules, err := rulesForFixture(spec, file, fset)
	if err != nil {
		return nil, err
	}
	return &metadata{
		ArtifactKind: artifactKind,
		Version:      metadataVersion,
		Source: metadataSource{
			Path:   sourceDisplayPath,
			SHA256: sha256Hex(source),
		},
		Parser: parserProvenance{
			Tool:           "go",
			Version:        runtime.Version(),
			Package:        "go/parser",
			PackageVersion: runtime.Version(),
		},
		Rules: rules,
	}, nil
}

func rulesForFixture(spec fixtureSpec, file *ast.File, fset *token.FileSet) ([]syntaxRule, error) {
	if spec.RulesFromFile != nil {
		return spec.RulesFromFile(file, fset), nil
	}
	if spec.Rules == nil {
		return nil, fmt.Errorf("fixture %q has no rule projector", spec.Name)
	}
	return spec.Rules(), nil
}

type sourceFileRuleProjector struct {
	rules                              map[string]*sourceFileRuleAccumulator
	ruleOrder                          []string
	fset                               *token.FileSet
	comments                           []*ast.CommentGroup
	attachedComments                   map[token.Pos]struct{}
	ifStmtHeaderTrailingCommentAnchors map[token.Pos][]int
	ignoredCommentPositions            map[token.Pos]struct{}
}

type sourceFileRuleAccumulator struct {
	name       string
	observed   int
	fields     map[string]*sourceFileFieldAccumulator
	fieldOrder []string
}

type sourceFileFieldAccumulator struct {
	name         string
	present      int
	multiple     bool
	alternatives map[syntaxAlternative]struct{}
}

var (
	astNodeType    = reflect.TypeOf((*ast.Node)(nil)).Elem()
	tokenPosType   = reflect.TypeOf(token.NoPos)
	tokenTokenType = reflect.TypeOf(token.ILLEGAL)
	astChanDirType = reflect.TypeOf(ast.SEND)
)

const fieldListBlankLineBeforeField = "BlankLineBefore"
const compositeLitLineBreakBeforeField = "LineBreakBefore"
const callExprLineBreakBeforeField = "LineBreakBefore"
const blockStmtCommentBeforeField = "CommentBefore"
const blockStmtTrailingCommentField = "TrailingComment"
const blockStmtBlankLineBeforeField = "BlankLineBefore"
const ifStmtTrailingCommentField = "TrailingComment"
const statementListCommentBeforeField = "CommentBefore"
const statementListTrailingCommentField = "TrailingComment"
const statementListBlankLineBeforeField = "BlankLineBefore"

func parserBackedSourceFileRules(file *ast.File, fset *token.FileSet) []syntaxRule {
	attachedComments := attachedCommentGroupPositions(file)
	ifStmtHeaderTrailingCommentAnchors, ignoredCommentPositions :=
		ifStmtHeaderTrailingCommentAnchors(file, fset, file.Comments, attachedComments)
	projector := sourceFileRuleProjector{
		rules:                              map[string]*sourceFileRuleAccumulator{},
		fset:                               fset,
		comments:                           file.Comments,
		attachedComments:                   attachedComments,
		ifStmtHeaderTrailingCommentAnchors: ifStmtHeaderTrailingCommentAnchors,
		ignoredCommentPositions:            ignoredCommentPositions,
	}
	ast.Inspect(file, func(node ast.Node) bool {
		if node != nil {
			projector.observe(node)
		}
		return true
	})
	for _, commentGroup := range file.Comments {
		projector.observe(commentGroup)
		for _, comment := range commentGroup.List {
			projector.observe(comment)
		}
	}
	return projector.projectedRules()
}

func parserBackedSourceFileRulesWithRetainedRootComments(file *ast.File, fset *token.FileSet) []syntaxRule {
	return withRetainedRootFileComments(parserBackedSourceFileRules(file, fset))
}

func withRetainedRootFileComments(rules []syntaxRule) []syntaxRule {
	for ruleIndex := range rules {
		if rules[ruleIndex].Name != "File" {
			continue
		}
		hasComments := false
		for _, field := range rules[ruleIndex].Fields {
			if field.Name == "Comments" {
				hasComments = true
				break
			}
		}
		if !hasComments {
			rules[ruleIndex].Fields = append(
				rules[ruleIndex].Fields,
				field("Comments", false, true, alternative("CommentGroup", "nonterminal")),
			)
		}
		break
	}
	if !hasRuleNamed(rules, "CommentGroup") {
		rules = append(rules, rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))))
	}
	if !hasRuleNamed(rules, "Comment") {
		rules = append(rules, rule("Comment",
			field("Text", true, false, alternative("comment_text", "string_payload"))))
	}
	return rules
}

func hasRuleNamed(rules []syntaxRule, name string) bool {
	for _, rule := range rules {
		if rule.Name == name {
			return true
		}
	}
	return false
}

func validateGenericSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name == "" {
		return fmt.Errorf("generic Go source file must have a package clause")
	}
	if len(file.Decls) == 0 {
		return fmt.Errorf("generic Go source file must have at least one top-level declaration")
	}
	return nil
}

func parserBackedNotifyContextSourceSpanRules(file *ast.File, fset *token.FileSet) []syntaxRule {
	notifyContext := osSignalNotifyContextDecl(file)
	importDecl := osSignalContextOsImportDecl(file)
	projector := sourceFileRuleProjector{rules: map[string]*sourceFileRuleAccumulator{}, fset: fset}
	syntheticFile := &ast.File{
		Name:  file.Name,
		Decls: []ast.Decl{importDecl, notifyContext},
	}
	ast.Inspect(syntheticFile, func(node ast.Node) bool {
		if node != nil {
			projector.observe(node)
		}
		return true
	})
	return projector.projectedRules()
}

func (projector *sourceFileRuleProjector) observe(node ast.Node) {
	ruleName := astNodeKind(reflect.ValueOf(node))
	if ruleName == "" {
		return
	}
	rule := projector.rules[ruleName]
	if rule == nil {
		rule = &sourceFileRuleAccumulator{
			name:   ruleName,
			fields: map[string]*sourceFileFieldAccumulator{},
		}
		projector.rules[ruleName] = rule
		projector.ruleOrder = append(projector.ruleOrder, ruleName)
	}
	rule.observed++

	value := reflect.ValueOf(node)
	if value.Kind() == reflect.Ptr {
		value = value.Elem()
	}
	if !value.IsValid() || value.Kind() != reflect.Struct {
		return
	}
	valueType := value.Type()
	presentFields := map[string]struct{}{}
	for fieldIndex := 0; fieldIndex < value.NumField(); fieldIndex++ {
		structField := valueType.Field(fieldIndex)
		if !structField.IsExported() || skipProjectedASTField(ruleName, structField.Name) {
			continue
		}
		alternatives, multiple := projectedFieldAlternatives(ruleName, structField.Name, value.Field(fieldIndex), node)
		if len(alternatives) == 0 {
			continue
		}
		field := rule.fields[structField.Name]
		if field == nil {
			field = &sourceFileFieldAccumulator{
				name:         structField.Name,
				alternatives: map[syntaxAlternative]struct{}{},
			}
			rule.fields[structField.Name] = field
			rule.fieldOrder = append(rule.fieldOrder, structField.Name)
		}
		if _, alreadyPresent := presentFields[structField.Name]; !alreadyPresent {
			field.present++
			presentFields[structField.Name] = struct{}{}
		}
		field.multiple = field.multiple || multiple
		for _, alternative := range alternatives {
			field.alternatives[alternative] = struct{}{}
		}
	}
	projector.observeSyntheticFields(ruleName, node, rule, presentFields)
}

func (projector *sourceFileRuleProjector) observeSyntheticFields(ruleName string, node ast.Node, rule *sourceFileRuleAccumulator, presentFields map[string]struct{}) {
	if projector.fset == nil {
		return
	}
	switch typed := node.(type) {
	case *ast.FieldList:
		if ruleName != "FieldList" {
			return
		}
		blankLineBefore := fieldListBlankLineBeforeIndexes(typed, projector.fset)
		if len(blankLineBefore) > 0 {
			addSyntheticIntegerField(
				rule,
				presentFields,
				fieldListBlankLineBeforeField,
				"field_list_blank_line_before_index",
			)
		}
	case *ast.CompositeLit:
		if ruleName != "CompositeLit" {
			return
		}
		lineBreakBefore := compositeLitLineBreakBeforeIndexes(typed, projector.fset)
		if len(lineBreakBefore) > 0 {
			addSyntheticIntegerField(
				rule,
				presentFields,
				compositeLitLineBreakBeforeField,
				"composite_lit_line_break_before_index",
			)
		}
	case *ast.CallExpr:
		if ruleName != "CallExpr" {
			return
		}
		lineBreakBefore := callExprLineBreakBeforeIndexes(typed, projector.fset)
		if len(lineBreakBefore) > 0 {
			addSyntheticIntegerField(
				rule,
				presentFields,
				callExprLineBreakBeforeField,
				"call_expr_line_break_before_index",
			)
		}
	case *ast.BlockStmt:
		if ruleName != "BlockStmt" {
			return
		}
		commentBefore := blockStmtCommentBeforeIndexes(typed, projector.fset, projector.comments, projector.attachedComments, projector.ignoredCommentPositions)
		if len(commentBefore) > 0 {
			addSyntheticIntegerField(
				rule,
				presentFields,
				blockStmtCommentBeforeField,
				"block_stmt_comment_before_index",
			)
		}
		trailingComment := blockStmtTrailingCommentIndexes(typed, projector.fset, projector.comments, projector.attachedComments, projector.ignoredCommentPositions)
		if len(trailingComment) > 0 {
			addSyntheticIntegerField(
				rule,
				presentFields,
				blockStmtTrailingCommentField,
				"block_stmt_trailing_comment_index",
			)
		}
		blankLineBefore := blockStmtBlankLineBeforeIndexes(typed, projector.fset, projector.comments, projector.attachedComments, projector.ignoredCommentPositions)
		if len(blankLineBefore) > 0 {
			addSyntheticIntegerField(
				rule,
				presentFields,
				blockStmtBlankLineBeforeField,
				"block_stmt_blank_line_before_index",
			)
		}
	case *ast.IfStmt:
		if ruleName != "IfStmt" || typed.Body == nil {
			return
		}
		trailingComment := projector.ifStmtHeaderTrailingCommentAnchors[typed.Body.Lbrace]
		if len(trailingComment) > 0 {
			addSyntheticIntegerField(
				rule,
				presentFields,
				ifStmtTrailingCommentField,
				"if_stmt_trailing_comment_index",
			)
		}
	case *ast.CaseClause:
		if ruleName != "CaseClause" {
			return
		}
		projector.observeStatementListSyntheticFields(
			rule,
			presentFields,
			typed.Body,
			typed.Colon,
			"case_clause_body",
		)
	case *ast.CommClause:
		if ruleName != "CommClause" {
			return
		}
		projector.observeStatementListSyntheticFields(
			rule,
			presentFields,
			typed.Body,
			typed.Colon,
			"comm_clause_body",
		)
	}
}

func (projector *sourceFileRuleProjector) observeStatementListSyntheticFields(
	rule *sourceFileRuleAccumulator,
	presentFields map[string]struct{},
	statements []ast.Stmt,
	anchor token.Pos,
	nodeKindPrefix string,
) {
	commentBefore := statementListCommentBeforeIndexes(statements, anchor, projector.fset, projector.comments, projector.attachedComments, projector.ignoredCommentPositions)
	if len(commentBefore) > 0 {
		addSyntheticIntegerField(
			rule,
			presentFields,
			statementListCommentBeforeField,
			nodeKindPrefix+"_comment_before_index",
		)
	}
	trailingComment := statementListTrailingCommentIndexes(statements, projector.fset, projector.comments, projector.attachedComments, projector.ignoredCommentPositions)
	if len(trailingComment) > 0 {
		addSyntheticIntegerField(
			rule,
			presentFields,
			statementListTrailingCommentField,
			nodeKindPrefix+"_trailing_comment_index",
		)
	}
	blankLineBefore := statementListBlankLineBeforeIndexes(statements, anchor, projector.fset, projector.comments, projector.attachedComments, projector.ignoredCommentPositions)
	if len(blankLineBefore) > 0 {
		addSyntheticIntegerField(
			rule,
			presentFields,
			statementListBlankLineBeforeField,
			nodeKindPrefix+"_blank_line_before_index",
		)
	}
}

func addSyntheticIntegerField(rule *sourceFileRuleAccumulator, presentFields map[string]struct{}, fieldName string, nodeKind string) {
	field := rule.fields[fieldName]
	if field == nil {
		field = &sourceFileFieldAccumulator{
			name:         fieldName,
			alternatives: map[syntaxAlternative]struct{}{},
		}
		rule.fields[fieldName] = field
		rule.fieldOrder = append(rule.fieldOrder, fieldName)
	}
	if _, alreadyPresent := presentFields[fieldName]; !alreadyPresent {
		field.present++
		presentFields[fieldName] = struct{}{}
	}
	field.multiple = true
	field.alternatives[alternative(nodeKind, "integer_payload")] = struct{}{}
}

func fieldListBlankLineBeforeIndexes(fieldList *ast.FieldList, fset *token.FileSet) []int {
	if fieldList == nil || fset == nil || len(fieldList.List) < 2 {
		return nil
	}
	indexes := make([]int, 0)
	for index := 1; index < len(fieldList.List); index++ {
		previous := fieldList.List[index-1]
		current := fieldList.List[index]
		if previous == nil || current == nil {
			continue
		}
		previousEndLine := fset.Position(previous.End()).Line
		currentStartLine := fset.Position(current.Pos()).Line
		if previousEndLine > 0 && currentStartLine > previousEndLine+1 {
			indexes = append(indexes, index)
		}
	}
	return indexes
}

func compositeLitLineBreakBeforeIndexes(literal *ast.CompositeLit, fset *token.FileSet) []int {
	if literal == nil || fset == nil || len(literal.Elts) == 0 || !literal.Lbrace.IsValid() {
		return nil
	}
	indexes := make([]int, 0)
	previousLine := fset.Position(literal.Lbrace).Line
	for index, element := range literal.Elts {
		if element == nil {
			continue
		}
		elementStartLine := fset.Position(element.Pos()).Line
		if previousLine > 0 && elementStartLine > previousLine {
			indexes = append(indexes, index)
		}
		previousLine = fset.Position(element.End()).Line
	}
	return indexes
}

func callExprLineBreakBeforeIndexes(call *ast.CallExpr, fset *token.FileSet) []int {
	if call == nil || fset == nil || len(call.Args) == 0 || !call.Lparen.IsValid() {
		return nil
	}
	indexes := make([]int, 0)
	previousLine := fset.Position(call.Lparen).Line
	for index, argument := range call.Args {
		if argument == nil {
			continue
		}
		argumentStartLine := fset.Position(argument.Pos()).Line
		if previousLine > 0 && argumentStartLine > previousLine {
			indexes = append(indexes, index)
		}
		previousLine = fset.Position(argument.End()).Line
	}
	return indexes
}

func blockStmtCommentBeforeIndexes(block *ast.BlockStmt, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if block == nil {
		return nil
	}
	return statementListCommentBeforeIndexes(block.List, block.Lbrace, fset, comments, attachedComments, ignoredCommentPositions)
}

func blockStmtTrailingCommentIndexes(block *ast.BlockStmt, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if block == nil {
		return nil
	}
	return statementListTrailingCommentIndexes(block.List, fset, comments, attachedComments, ignoredCommentPositions)
}

func blockStmtBlankLineBeforeIndexes(block *ast.BlockStmt, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if block == nil {
		return nil
	}
	return statementListBlankLineBeforeIndexes(block.List, block.Lbrace, fset, comments, attachedComments, ignoredCommentPositions)
}

func statementListCommentBeforeIndexes(statements []ast.Stmt, anchor token.Pos, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	indexes, _ := statementListCommentBeforeIndexLines(statements, anchor, fset, comments, attachedComments, ignoredCommentPositions)
	return indexes
}

func statementListTrailingCommentIndexes(statements []ast.Stmt, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if fset == nil || len(statements) == 0 || len(comments) == 0 {
		return nil
	}
	indexes := make([]int, 0)
	seen := map[int]struct{}{}
	for _, group := range comments {
		if group == nil {
			continue
		}
		if _, attached := attachedComments[group.Pos()]; attached {
			continue
		}
		if _, ignored := ignoredCommentPositions[group.Pos()]; ignored {
			continue
		}
		for index, stmt := range statements {
			if stmt == nil || !commentGroupTrailsStatement(group, stmt, fset) {
				continue
			}
			if _, exists := seen[index]; !exists {
				indexes = append(indexes, index)
				seen[index] = struct{}{}
			}
			break
		}
	}
	return indexes
}

func statementListBlankLineBeforeIndexes(statements []ast.Stmt, anchor token.Pos, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if fset == nil || !anchor.IsValid() || len(statements) < 2 {
		return nil
	}
	_, commentStartLines := statementListCommentBeforeIndexLines(statements, anchor, fset, comments, attachedComments, ignoredCommentPositions)
	indexes := make([]int, 0)
	for index := 1; index < len(statements); index++ {
		previous := statements[index-1]
		current := statements[index]
		if previous == nil || current == nil {
			continue
		}
		previousEndLine := fset.Position(previous.End()).Line
		separatorStartLine := fset.Position(current.Pos()).Line
		if commentStartLine, ok := commentStartLines[index]; ok {
			separatorStartLine = commentStartLine
		}
		if previousEndLine > 0 && separatorStartLine > previousEndLine+1 {
			indexes = append(indexes, index)
		}
	}
	return indexes
}

func blockStmtCommentBeforeIndexLines(block *ast.BlockStmt, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) ([]int, map[int]int) {
	if block == nil {
		return nil, map[int]int{}
	}
	return statementListCommentBeforeIndexLines(block.List, block.Lbrace, fset, comments, attachedComments, ignoredCommentPositions)
}

func statementListCommentBeforeIndexLines(statements []ast.Stmt, anchor token.Pos, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) ([]int, map[int]int) {
	commentStartLines := map[int]int{}
	if fset == nil || !anchor.IsValid() || len(statements) == 0 || len(comments) == 0 {
		return nil, commentStartLines
	}
	indexes := make([]int, 0)
	for _, group := range comments {
		if group == nil || group.Pos() <= anchor {
			continue
		}
		if _, attached := attachedComments[group.Pos()]; attached {
			continue
		}
		if _, ignored := ignoredCommentPositions[group.Pos()]; ignored {
			continue
		}
		for index, stmt := range statements {
			if stmt == nil {
				continue
			}
			if commentGroupTrailsStatement(group, stmt, fset) {
				break
			}
			if group.End() >= stmt.Pos() {
				continue
			}
			previousEnd := anchor
			if index > 0 && statements[index-1] != nil {
				if commentGroupTrailsStatement(group, statements[index-1], fset) {
					break
				}
				previousEnd = statements[index-1].End()
			}
			if group.Pos() > previousEnd {
				if _, exists := commentStartLines[index]; !exists {
					indexes = append(indexes, index)
					commentStartLines[index] = fset.Position(group.Pos()).Line
				}
			}
			break
		}
	}
	sort.Ints(indexes)
	return indexes, commentStartLines
}

func commentGroupTrailsStatement(group *ast.CommentGroup, stmt ast.Stmt, fset *token.FileSet) bool {
	if group == nil || stmt == nil || fset == nil || group.Pos() <= stmt.End() {
		return false
	}
	statementEndLine := fset.Position(stmt.End()).Line
	groupStartLine := fset.Position(group.Pos()).Line
	groupEndLine := fset.Position(group.End()).Line
	return statementEndLine > 0 && groupStartLine == statementEndLine && groupEndLine == statementEndLine
}

func ifStmtHeaderTrailingCommentAnchors(root ast.Node, fset *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}) (map[token.Pos][]int, map[token.Pos]struct{}) {
	anchors := map[token.Pos][]int{}
	positions := map[token.Pos]struct{}{}
	if root == nil || fset == nil || len(comments) == 0 {
		return anchors, positions
	}
	ast.Inspect(root, func(node ast.Node) bool {
		ifStmt, ok := node.(*ast.IfStmt)
		if !ok || ifStmt.Body == nil || !ifStmt.Body.Lbrace.IsValid() {
			return true
		}
		for _, group := range comments {
			if group == nil || !commentGroupTrailsIfHeader(group, ifStmt, fset) {
				continue
			}
			if _, attached := attachedComments[group.Pos()]; attached {
				continue
			}
			if _, alreadyClaimed := positions[group.Pos()]; alreadyClaimed {
				continue
			}
			anchors[ifStmt.Body.Lbrace] = []int{0}
			positions[group.Pos()] = struct{}{}
			break
		}
		return true
	})
	return anchors, positions
}

func commentGroupTrailsIfHeader(group *ast.CommentGroup, ifStmt *ast.IfStmt, fset *token.FileSet) bool {
	if group == nil || ifStmt == nil || ifStmt.Body == nil || fset == nil {
		return false
	}
	if !ifStmt.Body.Lbrace.IsValid() || group.Pos() <= ifStmt.Body.Lbrace {
		return false
	}
	headerLine := fset.Position(ifStmt.Body.Lbrace).Line
	groupStartLine := fset.Position(group.Pos()).Line
	groupEndLine := fset.Position(group.End()).Line
	return headerLine > 0 && groupStartLine == headerLine && groupEndLine == headerLine
}

func attachedCommentGroupPositions(root ast.Node) map[token.Pos]struct{} {
	attached := map[token.Pos]struct{}{}
	ast.Inspect(root, func(node ast.Node) bool {
		if node == nil {
			return true
		}
		value := reflect.ValueOf(node)
		if value.Kind() == reflect.Ptr {
			value = value.Elem()
		}
		if !value.IsValid() || value.Kind() != reflect.Struct {
			return true
		}
		valueType := value.Type()
		for fieldIndex := 0; fieldIndex < value.NumField(); fieldIndex++ {
			structField := valueType.Field(fieldIndex)
			if !structField.IsExported() || (structField.Name != "Doc" && structField.Name != "Comment") {
				continue
			}
			field := value.Field(fieldIndex)
			if !field.IsValid() || field.IsNil() || !field.CanInterface() {
				continue
			}
			if group, ok := field.Interface().(*ast.CommentGroup); ok && group != nil {
				attached[group.Pos()] = struct{}{}
			}
		}
		return true
	})
	return attached
}

func (projector sourceFileRuleProjector) projectedRules() []syntaxRule {
	rules := make([]syntaxRule, 0, len(projector.ruleOrder))
	for _, ruleName := range projector.ruleOrder {
		accumulator := projector.rules[ruleName]
		fields := make([]syntaxField, 0, len(accumulator.fieldOrder))
		for _, fieldName := range accumulator.fieldOrder {
			fieldAccumulator := accumulator.fields[fieldName]
			fields = append(fields, syntaxField{
				Name:         fieldAccumulator.name,
				Required:     fieldAccumulator.present == accumulator.observed,
				Multiple:     fieldAccumulator.multiple,
				Alternatives: sortedAlternatives(fieldAccumulator.alternatives),
			})
		}
		rules = append(rules, syntaxRule{Name: accumulator.name, NodeKind: accumulator.name, Fields: fields})
	}
	return rules
}

func projectedFieldAlternatives(ruleName string, fieldName string, value reflect.Value, parent ast.Node) ([]syntaxAlternative, bool) {
	if !value.IsValid() {
		return nil, false
	}
	if value.Type() == tokenPosType {
		if ruleName == "CallExpr" && fieldName == "Ellipsis" && token.Pos(value.Int()).IsValid() {
			return []syntaxAlternative{alternative("ellipsis_token", "string_payload")}, false
		}
		if ruleName == "TypeSpec" && fieldName == "Assign" && token.Pos(value.Int()).IsValid() {
			return []syntaxAlternative{alternative("assign_token", "string_payload")}, false
		}
		return nil, false
	}
	if value.Type() == tokenTokenType {
		if token.Token(value.Int()) == token.ILLEGAL {
			return nil, false
		}
		return []syntaxAlternative{alternative(tokenFieldNodeKind(fieldName), "string_payload")}, false
	}
	if value.Type() == astChanDirType {
		if value.Int() == 0 {
			return nil, false
		}
		return []syntaxAlternative{alternative("channel_direction", "string_payload")}, false
	}
	if value.Kind() == reflect.String {
		return projectedStringFieldAlternative(ruleName, fieldName, parent)
	}
	if value.Kind() == reflect.Slice || value.Kind() == reflect.Array {
		alternatives := map[syntaxAlternative]struct{}{}
		for index := 0; index < value.Len(); index++ {
			if nodeKind := astNodeKind(value.Index(index)); nodeKind != "" {
				alternatives[alternative(nodeKind, "nonterminal")] = struct{}{}
			}
		}
		if len(alternatives) == 0 {
			return nil, true
		}
		return sortedAlternatives(alternatives), true
	}
	if nodeKind := astNodeKind(value); nodeKind != "" {
		return []syntaxAlternative{alternative(nodeKind, "nonterminal")}, false
	}
	return nil, false
}

func projectedStringFieldAlternative(ruleName string, fieldName string, parent ast.Node) ([]syntaxAlternative, bool) {
	switch {
	case ruleName == "Ident" && fieldName == "Name":
		return []syntaxAlternative{alternative("identifier", "identifier_payload")}, false
	case ruleName == "BasicLit" && fieldName == "Value":
		literal, ok := parent.(*ast.BasicLit)
		if ok && literal.Kind == token.INT {
			return []syntaxAlternative{alternative("integer_literal", "integer_payload")}, false
		}
		return []syntaxAlternative{alternative("string_literal", "string_payload")}, false
	case ruleName == "Comment" && fieldName == "Text":
		return []syntaxAlternative{alternative("comment_text", "string_payload")}, false
	default:
		return nil, false
	}
}

func sortedAlternatives(alternatives map[syntaxAlternative]struct{}) []syntaxAlternative {
	result := make([]syntaxAlternative, 0, len(alternatives))
	for alternative := range alternatives {
		result = append(result, alternative)
	}
	sort.Slice(result, func(left, right int) bool {
		if result[left].Terminal == result[right].Terminal {
			return result[left].NodeKind < result[right].NodeKind
		}
		return result[left].Terminal < result[right].Terminal
	})
	return result
}

func astNodeKind(value reflect.Value) string {
	value = unwrapReflectValue(value)
	if !value.IsValid() {
		return ""
	}
	if !value.Type().Implements(astNodeType) {
		return ""
	}
	if value.Kind() == reflect.Ptr {
		return value.Type().Elem().Name()
	}
	return value.Type().Name()
}

func unwrapReflectValue(value reflect.Value) reflect.Value {
	for value.IsValid() && (value.Kind() == reflect.Interface || value.Kind() == reflect.Ptr) {
		if value.IsNil() {
			return reflect.Value{}
		}
		value = value.Elem()
	}
	if value.IsValid() && value.CanAddr() && value.Addr().Type().Implements(astNodeType) {
		return value.Addr()
	}
	return value
}

func tokenFieldNodeKind(fieldName string) string {
	switch fieldName {
	case "Tok":
		return "token"
	case "Op":
		return "operator"
	case "Kind":
		return "literal_kind"
	default:
		return strings.ToLower(fieldName) + "_token"
	}
}

func skipProjectedASTField(ruleName string, fieldName string) bool {
	switch fieldName {
	case "Obj", "Scope", "Imports", "Unresolved":
		return true
	}
	return ruleName == "File" && fieldName == "Package"
}

func minimalFunctionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("import_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ImportSpec", "nonterminal"))),
		rule("ImportSpec",
			field("Path", true, false, alternative("string_literal", "string_payload"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", true, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CallExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false, alternative("SelectorExpr", "nonterminal")),
			field("Args", true, true, alternative("Ident", "nonterminal"))),
		rule("SelectorExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Sel", true, false, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("string_literal", "string_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localAssignmentRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assign_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("string_literal", "string_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localIntegerAssignmentRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assign_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localCompoundAssignmentRules() []syntaxRule {
	return localIntegerAssignmentRules()
}

func localIncrementDecrementRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("IncDecStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("IncDecStmt",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("increment_operator", "string_payload"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localBranchStatementRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("ForStmt", "nonterminal"),
				alternative("BranchStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("ForStmt",
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("BranchStmt",
			field("Tok", true, false, alternative("branch_keyword", "string_payload"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BasicLit", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localIfElseRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("IfStmt", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("IfStmt",
			field("Cond", true, false, alternative("BinaryExpr", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal")),
			field("Else", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("BinaryExpr",
			field("Op", true, false, alternative("binary_operator", "string_payload")),
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Y", true, false, alternative("BasicLit", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assign_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localForConditionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("ForStmt", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("ForStmt",
			field("Cond", true, false, alternative("BinaryExpr", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("BinaryExpr",
			field("Op", true, false, alternative("binary_operator", "string_payload")),
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Y", true, false, alternative("BasicLit", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assign_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localSwitchRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("SwitchStmt", "nonterminal"),
				alternative("CaseClause", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("SwitchStmt",
			field("Tag", true, false, alternative("Ident", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("CaseClause",
			field("List", true, true, alternative("BasicLit", "nonterminal")),
			field("Body", true, true, alternative("AssignStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assign_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localSwitchInitRules() []syntaxRule {
	return withRetainedRootFileComments([]syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("SwitchStmt", "nonterminal"),
				alternative("CaseClause", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("SwitchStmt",
			field("Init", true, false, alternative("AssignStmt", "nonterminal")),
			field("Tag", true, false, alternative("Ident", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("CaseClause",
			field("List", true, true, alternative("BasicLit", "nonterminal")),
			field("Body", true, true, alternative("AssignStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assign_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	})
}

func localTypeSwitchRules() []syntaxRule {
	return withRetainedRootFileComments([]syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("TypeSwitchStmt", "nonterminal"),
				alternative("CaseClause", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("TypeSwitchStmt",
			field("Assign", true, false, alternative("ExprStmt", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("ExprStmt",
			field("X", true, false, alternative("TypeAssertExpr", "nonterminal"))),
		rule("TypeAssertExpr",
			field("X", true, false, alternative("Ident", "nonterminal"))),
		rule("CaseClause",
			field("List", true, true, alternative("Ident", "nonterminal")),
			field("Body", true, true, alternative("AssignStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assign_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	})
}

func localShortVarDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("define_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("string_literal", "string_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localIntegerShortVarDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("define_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localIntegerReturnLiteralRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localParenthesizedIntegerReturnExpressionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("ParenExpr", "nonterminal"))),
		rule("ParenExpr",
			field("X", true, false, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localUnaryIntegerReturnExpressionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("UnaryExpr", "nonterminal"))),
		rule("UnaryExpr",
			field("Op", true, false, alternative("unary_operator", "string_payload")),
			field("X", true, false, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localBinaryIntegerReturnExpressionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BinaryExpr", "nonterminal"))),
		rule("BinaryExpr",
			field("Op", true, false, alternative("binary_operator", "string_payload")),
			field("X", true, false, alternative("BasicLit", "nonterminal")),
			field("Y", true, false, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localCallReturnExpressionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CallExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localArgumentCallReturnExpressionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CallExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false, alternative("Ident", "nonterminal")),
			field("Args", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("string_literal", "string_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localStringReturnLiteralRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("string_literal", "string_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localFloatReturnLiteralRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("float_literal", "string_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localBooleanReturnIdentifierRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localNilReturnIdentifierRules() []syntaxRule {
	return localBooleanReturnIdentifierRules()
}

func localTypedDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("string_literal", "string_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localZeroValueTypedDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localIntegerTypedDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localUntypedIntegerDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func localRangeStringRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("RangeStmt", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("RangeStmt",
			field("X", true, false, alternative("BasicLit", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assign_operator", "string_payload")),
			field("Rhs", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false,
				alternative("integer_literal", "integer_payload"),
				alternative("string_literal", "string_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func packageTypeDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("type_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("TypeSpec", "nonterminal"))),
		rule("TypeSpec",
			field("Name", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func methodDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("type_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("TypeSpec", "nonterminal"))),
		rule("TypeSpec",
			field("Name", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("FuncDecl",
			field("Recv", true, false, alternative("FieldList", "nonterminal")),
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func structTypeRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("type_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("TypeSpec", "nonterminal"))),
		rule("TypeSpec",
			field("Name", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("StructType", "nonterminal"))),
		rule("StructType",
			field("Fields", true, false, alternative("FieldList", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func compositeLiteralRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("type_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("TypeSpec", "nonterminal"))),
		rule("TypeSpec",
			field("Name", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("StructType", "nonterminal"))),
		rule("StructType",
			field("Fields", true, false, alternative("FieldList", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CompositeLit", "nonterminal"))),
		rule("CompositeLit",
			field("Type", true, false, alternative("Ident", "nonterminal")),
			field("Elts", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func keyValueCompositeLiteralRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("type_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("TypeSpec", "nonterminal"))),
		rule("TypeSpec",
			field("Name", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("StructType", "nonterminal"))),
		rule("StructType",
			field("Fields", true, false, alternative("FieldList", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CompositeLit", "nonterminal"))),
		rule("CompositeLit",
			field("Type", true, false, alternative("Ident", "nonterminal")),
			field("Elts", true, true, alternative("KeyValueExpr", "nonterminal"))),
		rule("KeyValueExpr",
			field("Key", true, false, alternative("Ident", "nonterminal")),
			field("Value", true, false, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func arrayCompositeLiteralRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("ArrayType", "nonterminal"))),
		rule("ArrayType",
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CompositeLit", "nonterminal"))),
		rule("CompositeLit",
			field("Type", true, false, alternative("ArrayType", "nonterminal")),
			field("Elts", true, true, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func indexExpressionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("ArrayType", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("ArrayType",
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("IndexExpr", "nonterminal"))),
		rule("IndexExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Index", true, false, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func sliceExpressionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("ArrayType", "nonterminal"))),
		rule("ArrayType",
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("SliceExpr", "nonterminal"))),
		rule("SliceExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Low", true, false, alternative("BasicLit", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func sliceExpressionThreeIndexRules() []syntaxRule {
	return withRetainedRootFileComments([]syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("ArrayType", "nonterminal"))),
		rule("ArrayType",
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("SliceExpr", "nonterminal"))),
		rule("SliceExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Low", true, false, alternative("BasicLit", "nonterminal")),
			field("High", true, false, alternative("BasicLit", "nonterminal")),
			field("Max", true, false, alternative("BasicLit", "nonterminal")),
			field("Slice3", true, false, alternative("slice3_marker", "boolean_payload"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	})
}

func starExpressionRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("StarExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("StarExpr",
			field("X", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("StarExpr", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func packageVarDeclarationRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ValueSpec", "nonterminal"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("BasicLit", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func functionLiteralRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Type", true, false,
				alternative("FuncType", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true,
				alternative("FuncLit", "nonterminal"),
				alternative("BasicLit", "nonterminal"))),
		rule("FuncLit",
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	}
}

func concurrencyMapInterfaceTypeAssertionRules() []syntaxRule {
	return withRetainedRootFileComments([]syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Decls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("type_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("TypeSpec", "nonterminal"))),
		rule("TypeSpec",
			field("Name", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("InterfaceType", "nonterminal"),
				alternative("StructType", "nonterminal"))),
		rule("InterfaceType",
			field("Methods", true, false, alternative("FieldList", "nonterminal"))),
		rule("StructType",
			field("Fields", true, false, alternative("FieldList", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", false, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("ChanType", "nonterminal"),
				alternative("FuncType", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("MapType", "nonterminal"))),
		rule("MapType",
			field("Key", true, false, alternative("Ident", "nonterminal")),
			field("Value", true, false,
				alternative("ChanType", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("ChanType",
			field("Dir", false, false, alternative("channel_direction", "string_payload")),
			field("Value", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("AssignStmt", "nonterminal"),
				alternative("CommClause", "nonterminal"),
				alternative("DeferStmt", "nonterminal"),
				alternative("GoStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"),
				alternative("SelectStmt", "nonterminal"),
				alternative("SendStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("define_operator", "string_payload")),
			field("Rhs", true, true,
				alternative("TypeAssertExpr", "nonterminal"),
				alternative("UnaryExpr", "nonterminal"))),
		rule("DeferStmt",
			field("Call", true, false, alternative("CallExpr", "nonterminal"))),
		rule("GoStmt",
			field("Call", true, false, alternative("CallExpr", "nonterminal"))),
		rule("SelectStmt",
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("CommClause",
			field("Comm", false, false,
				alternative("AssignStmt", "nonterminal"),
				alternative("SendStmt", "nonterminal")),
			field("Body", false, true, alternative("ReturnStmt", "nonterminal"))),
		rule("SendStmt",
			field("Chan", true, false, alternative("Ident", "nonterminal")),
			field("Value", true, false,
				alternative("CallExpr", "nonterminal"),
				alternative("UnaryExpr", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CompositeLit", "nonterminal"))),
		rule("CompositeLit",
			field("Type", true, false, alternative("MapType", "nonterminal")),
			field("Elts", false, true, alternative("KeyValueExpr", "nonterminal"))),
		rule("KeyValueExpr",
			field("Key", true, false, alternative("Ident", "nonterminal")),
			field("Value", true, false,
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false,
				alternative("FuncLit", "nonterminal"),
				alternative("Ident", "nonterminal")),
			field("Args", false, true, alternative("Ident", "nonterminal"))),
		rule("FuncLit",
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("UnaryExpr",
			field("Op", true, false, alternative("unary_operator", "string_payload")),
			field("X", true, false, alternative("Ident", "nonterminal"))),
		rule("TypeAssertExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
	})
}

func stringsCompareSourceFileRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("HeaderComments", true, true, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Imports", true, true, alternative("GenDecl", "nonterminal")),
			field("CompareDecl", true, false, alternative("FuncDecl", "nonterminal")),
			field("Comments", false, true, alternative("CommentGroup", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("import_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ImportSpec", "nonterminal"))),
		rule("ImportSpec",
			field("Path", true, false, alternative("string_literal", "string_payload"))),
		rule("FuncDecl",
			field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", true, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CallExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false, alternative("SelectorExpr", "nonterminal")),
			field("Args", true, true, alternative("Ident", "nonterminal"))),
		rule("SelectorExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Sel", true, false, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
		rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))),
		rule("Comment",
			field("Text", true, false, alternative("comment_text", "string_payload"))),
	}
}

func stringsCloneSourceFileRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("HeaderComments", true, true, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Imports", true, true, alternative("GenDecl", "nonterminal")),
			field("CloneDecl", true, false, alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("import_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ImportSpec", "nonterminal"))),
		rule("ImportSpec",
			field("Path", true, false, alternative("string_literal", "string_payload"))),
		rule("FuncDecl",
			field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", true, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CallExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false, alternative("SelectorExpr", "nonterminal")),
			field("Args", true, true, alternative("Ident", "nonterminal"))),
		rule("SelectorExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Sel", true, false, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
		rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))),
		rule("Comment",
			field("Text", true, false, alternative("comment_text", "string_payload"))),
	}
}

func sortInterfaceSourceFileDeclarationPrefixRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Imports", true, true, alternative("GenDecl", "nonterminal")),
			field("SupportedDecls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal")),
			field("OutputComments", true, false, alternative("CommentGroup", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false,
				alternative("import_keyword", "identifier_payload"),
				alternative("type_keyword", "identifier_payload")),
			field("Specs", true, true,
				alternative("ImportSpec", "nonterminal"),
				alternative("TypeSpec", "nonterminal"))),
		rule("ImportSpec",
			field("Path", true, false, alternative("string_literal", "string_payload"))),
		rule("TypeSpec",
			field("Name", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("StructType", "nonterminal"),
				alternative("ArrayType", "nonterminal"))),
		rule("StructType",
			field("Fields", true, false, alternative("FieldList", "nonterminal"))),
		rule("ArrayType",
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("FuncDecl",
			field("Recv", false, false, alternative("FieldList", "nonterminal")),
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", false, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("Ident", "nonterminal"),
				alternative("ArrayType", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("ReturnStmt", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("ExprStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true,
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal")),
			field("Tok", true, false,
				alternative("assign_operator", "string_payload"),
				alternative("define_operator", "string_payload")),
			field("Rhs", true, true,
				alternative("CompositeLit", "nonterminal"),
				alternative("IndexExpr", "nonterminal"))),
		rule("ExprStmt",
			field("X", true, false, alternative("CallExpr", "nonterminal"))),
		rule("BinaryExpr",
			field("X", true, false, alternative("SelectorExpr", "nonterminal")),
			field("Op", true, false, alternative("binary_operator", "string_payload")),
			field("Y", true, false, alternative("SelectorExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false,
				alternative("Ident", "nonterminal"),
				alternative("SelectorExpr", "nonterminal")),
			field("Args", false, true,
				alternative("BasicLit", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("FuncLit", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("SelectorExpr", "nonterminal"))),
		rule("FuncLit",
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("CompositeLit",
			field("Type", false, false,
				alternative("ArrayType", "nonterminal"),
				alternative("Ident", "nonterminal")),
			field("Elts", false, true,
				alternative("BasicLit", "nonterminal"),
				alternative("CompositeLit", "nonterminal"))),
		rule("SelectorExpr",
			field("X", true, false,
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal")),
			field("Sel", true, false, alternative("Ident", "nonterminal"))),
		rule("IndexExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Index", true, false, alternative("Ident", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false,
				alternative("string_literal", "string_payload"),
				alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
		rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))),
		rule("Comment",
			field("Text", true, false, alternative("line_comment_text", "string_payload"))),
	}
}

func sortInterfaceSourceFileRules() []syntaxRule {
	rules := sortInterfaceSourceFileDeclarationPrefixRules()
	if len(rules) > 0 && rules[0].Name == "File" {
		fileFields := append([]syntaxField{
			field("HeaderComments", true, true, alternative("CommentGroup", "nonterminal")),
		}, rules[0].Fields...)
		rules[0].Fields = fileFields
	}
	return rules
}

func sortSearchSourceFileExamplePrefixRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Imports", true, true, alternative("GenDecl", "nonterminal")),
			field("ExampleDecl", true, false, alternative("FuncDecl", "nonterminal")),
			field("OutputComments", true, false, alternative("CommentGroup", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("import_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ImportSpec", "nonterminal"))),
		rule("ImportSpec",
			field("Path", true, false, alternative("string_literal", "string_payload"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", false, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("AssignStmt", "nonterminal"),
				alternative("IfStmt", "nonterminal"),
				alternative("ExprStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false,
				alternative("assign_operator", "string_payload"),
				alternative("define_operator", "string_payload")),
			field("Rhs", true, true,
				alternative("BasicLit", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("CompositeLit", "nonterminal"))),
		rule("IfStmt",
			field("Cond", true, false, alternative("BinaryExpr", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal")),
			field("Else", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("ExprStmt",
			field("X", true, false, alternative("CallExpr", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("BinaryExpr", "nonterminal"))),
		rule("BinaryExpr",
			field("X", true, false,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal")),
			field("Op", true, false, alternative("binary_operator", "string_payload")),
			field("Y", true, false,
				alternative("BasicLit", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false,
				alternative("Ident", "nonterminal"),
				alternative("SelectorExpr", "nonterminal")),
			field("Args", false, true,
				alternative("BasicLit", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("FuncLit", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("FuncLit",
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("CompositeLit",
			field("Type", false, false, alternative("ArrayType", "nonterminal")),
			field("Elts", false, true, alternative("BasicLit", "nonterminal"))),
		rule("ArrayType",
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("SelectorExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Sel", true, false, alternative("Ident", "nonterminal"))),
		rule("IndexExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Index", true, false, alternative("Ident", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false,
				alternative("integer_literal", "integer_payload"),
				alternative("string_literal", "string_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
		rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))),
		rule("Comment",
			field("Text", true, false, alternative("comment_text", "string_payload"))),
	}
}

func sortSearchSourceFileRules() []syntaxRule {
	rules := sortSearchSourceFileExamplePrefixRules()
	if len(rules) > 0 && rules[0].Name == "File" {
		rules[0].Fields = []syntaxField{
			field("HeaderComments", true, true, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Imports", true, true, alternative("GenDecl", "nonterminal")),
			field("SupportedDecls", true, true, alternative("FuncDecl", "nonterminal")),
			field("OutputComments", true, true, alternative("CommentGroup", "nonterminal")),
		}
	}
	for ruleIndex := range rules {
		switch rules[ruleIndex].Name {
		case "BlockStmt":
			for fieldIndex := range rules[ruleIndex].Fields {
				if rules[ruleIndex].Fields[fieldIndex].Name == "List" {
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("RangeStmt", "nonterminal"),
					)
				}
			}
		case "ReturnStmt":
			rules[ruleIndex].Fields = []syntaxField{
				field("Results", true, true,
					alternative("BinaryExpr", "nonterminal"),
					alternative("CallExpr", "nonterminal")),
			}
		case "CallExpr":
			for fieldIndex := range rules[ruleIndex].Fields {
				if rules[ruleIndex].Fields[fieldIndex].Name == "Args" {
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("IndexExpr", "nonterminal"),
					)
				}
			}
		case "BasicLit":
			rules[ruleIndex].Fields = []syntaxField{
				field("Value", true, false,
					alternative("float_literal", "string_payload"),
					alternative("integer_literal", "integer_payload"),
					alternative("string_literal", "string_payload")),
			}
		}
	}
	rules = append(rules, rule("RangeStmt",
		field("Key", true, false, alternative("Ident", "nonterminal")),
		field("Value", true, false, alternative("Ident", "nonterminal")),
		field("Tok", true, false, alternative("define_operator", "string_payload")),
		field("X", true, false, alternative("CompositeLit", "nonterminal")),
		field("Body", true, false, alternative("BlockStmt", "nonterminal"))))
	return rules
}

func stringsSearchSourceFileFinderPrefixRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("FinderTypeDecl", true, false, alternative("GenDecl", "nonterminal")),
			field("MakeFinderDecl", true, false, alternative("FuncDecl", "nonterminal")),
			field("Comments", true, true, alternative("CommentGroup", "nonterminal"))),
		rule("GenDecl",
			field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
			field("Tok", true, false, alternative("type_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("TypeSpec", "nonterminal"))),
		rule("TypeSpec",
			field("Name", true, false, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("StructType", "nonterminal"))),
		rule("StructType",
			field("Fields", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("Ident", "nonterminal"),
				alternative("ArrayType", "nonterminal"))),
		rule("ArrayType",
			field("Len", false, false, alternative("BasicLit", "nonterminal")),
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("FuncDecl",
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("AssignStmt", "nonterminal"),
				alternative("RangeStmt", "nonterminal"),
				alternative("ForStmt", "nonterminal"),
				alternative("IfStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true,
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal")),
			field("Tok", true, false,
				alternative("assign_operator", "string_payload"),
				alternative("define_operator", "string_payload")),
			field("Rhs", true, true,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("UnaryExpr", "nonterminal"))),
		rule("RangeStmt",
			field("Key", true, false, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("define_operator", "string_payload")),
			field("X", true, false, alternative("SelectorExpr", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("ForStmt",
			field("Init", false, false, alternative("AssignStmt", "nonterminal")),
			field("Cond", false, false, alternative("BinaryExpr", "nonterminal")),
			field("Post", false, false, alternative("IncDecStmt", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("IncDecStmt",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Tok", true, false,
				alternative("increment_operator", "string_payload"),
				alternative("decrement_operator", "string_payload"))),
		rule("IfStmt",
			field("Cond", true, false,
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("UnaryExpr",
			field("Op", true, false, alternative("unary_operator", "string_payload")),
			field("X", true, false, alternative("CompositeLit", "nonterminal"))),
		rule("CompositeLit",
			field("Type", true, false, alternative("Ident", "nonterminal")),
			field("Elts", true, true, alternative("KeyValueExpr", "nonterminal"))),
		rule("KeyValueExpr",
			field("Key", true, false, alternative("Ident", "nonterminal")),
			field("Value", true, false,
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false,
				alternative("Ident", "nonterminal"),
				alternative("SelectorExpr", "nonterminal")),
			field("Args", false, true,
				alternative("ArrayType", "nonterminal"),
				alternative("BasicLit", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal"),
				alternative("SliceExpr", "nonterminal"))),
		rule("SelectorExpr",
			field("X", true, false,
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal"),
				alternative("SelectorExpr", "nonterminal")),
			field("Sel", true, false, alternative("Ident", "nonterminal"))),
		rule("IndexExpr",
			field("X", true, false,
				alternative("Ident", "nonterminal"),
				alternative("SelectorExpr", "nonterminal")),
			field("Index", true, false,
				alternative("BinaryExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("SliceExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Low", false, false,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal")),
			field("High", false, false, alternative("BinaryExpr", "nonterminal"))),
		rule("BinaryExpr",
			field("X", true, false,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal")),
			field("Op", true, false, alternative("binary_operator", "string_payload")),
			field("Y", true, false,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false,
				alternative("integer_literal", "integer_payload"),
				alternative("string_literal", "string_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
		rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))),
		rule("Comment",
			field("Text", true, false, alternative("comment_text", "string_payload"))),
	}
}

func stringsSearchSourceFileRules() []syntaxRule {
	rules := stringsSearchSourceFileFinderPrefixRules()
	if len(rules) > 0 && rules[0].Name == "File" {
		rules[0].Fields = []syntaxField{
			field("HeaderComments", true, true, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("SupportedDecls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal")),
			field("Comments", true, true, alternative("CommentGroup", "nonterminal")),
		}
	}
	for ruleIndex := range rules {
		switch rules[ruleIndex].Name {
		case "Field":
			for fieldIndex := range rules[ruleIndex].Fields {
				if rules[ruleIndex].Fields[fieldIndex].Name == "Type" {
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("StarExpr", "nonterminal"),
					)
				}
			}
		case "FuncDecl":
			rules[ruleIndex].Fields = append(
				[]syntaxField{
					field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
					field("Recv", false, false, alternative("FieldList", "nonterminal")),
				},
				rules[ruleIndex].Fields...,
			)
		case "BlockStmt":
			for fieldIndex := range rules[ruleIndex].Fields {
				if rules[ruleIndex].Fields[fieldIndex].Name == "List" {
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("BranchStmt", "nonterminal"),
						alternative("IncDecStmt", "nonterminal"),
					)
				}
			}
		case "AssignStmt":
			for fieldIndex := range rules[ruleIndex].Fields {
				switch rules[ruleIndex].Fields[fieldIndex].Name {
				case "Tok":
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("compound_assign_operator", "string_payload"),
					)
				case "Rhs":
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("SelectorExpr", "nonterminal"),
					)
				}
			}
		case "ReturnStmt":
			rules[ruleIndex].Fields = []syntaxField{
				field("Results", false, true,
					alternative("BinaryExpr", "nonterminal"),
					alternative("Ident", "nonterminal"),
					alternative("UnaryExpr", "nonterminal")),
			}
		case "UnaryExpr":
			rules[ruleIndex].Fields = []syntaxField{
				field("Op", true, false, alternative("unary_operator", "string_payload")),
				field("X", true, false,
					alternative("BasicLit", "nonterminal"),
					alternative("CompositeLit", "nonterminal")),
			}
		case "IndexExpr":
			for fieldIndex := range rules[ruleIndex].Fields {
				if rules[ruleIndex].Fields[fieldIndex].Name == "X" {
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("IndexExpr", "nonterminal"),
					)
				}
			}
		}
	}
	rules = append(rules,
		rule("BranchStmt",
			field("Tok", true, false, alternative("branch_keyword", "string_payload"))),
		rule("StarExpr",
			field("X", true, false, alternative("Ident", "nonterminal"))),
	)
	return rules
}

func pathMatchSourceFileMatchPrefixRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Imports", true, false, alternative("GenDecl", "nonterminal")),
			field("ErrBadPatternDecl", true, false, alternative("GenDecl", "nonterminal")),
			field("MatchDecl", true, false, alternative("FuncDecl", "nonterminal")),
			field("Comments", true, true, alternative("CommentGroup", "nonterminal"))),
		rule("GenDecl",
			field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
			field("Tok", true, false,
				alternative("import_keyword", "identifier_payload"),
				alternative("var_keyword", "identifier_payload")),
			field("Specs", true, true,
				alternative("ImportSpec", "nonterminal"),
				alternative("ValueSpec", "nonterminal"))),
		rule("ImportSpec",
			field("Path", true, false, alternative("string_literal", "string_payload"))),
		rule("ValueSpec",
			field("Names", true, true, alternative("Ident", "nonterminal")),
			field("Values", true, true, alternative("CallExpr", "nonterminal"))),
		rule("FuncDecl",
			field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", false, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("DeclStmt", "nonterminal"),
				alternative("AssignStmt", "nonterminal"),
				alternative("LabeledStmt", "nonterminal"),
				alternative("ForStmt", "nonterminal"),
				alternative("IfStmt", "nonterminal"),
				alternative("BranchStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("DeclStmt",
			field("Decl", true, false, alternative("GenDecl", "nonterminal"))),
		rule("LabeledStmt",
			field("Label", true, false, alternative("Ident", "nonterminal")),
			field("Stmt", true, false, alternative("ForStmt", "nonterminal"))),
		rule("ForStmt",
			field("Init", false, false, alternative("AssignStmt", "nonterminal")),
			field("Cond", false, false, alternative("BinaryExpr", "nonterminal")),
			field("Post", false, false, alternative("IncDecStmt", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("IncDecStmt",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("increment_operator", "string_payload"))),
		rule("IfStmt",
			field("Cond", true, false,
				alternative("BinaryExpr", "nonterminal"),
				alternative("Ident", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("BranchStmt",
			field("Tok", true, false, alternative("continue_keyword", "identifier_payload")),
			field("Label", false, false, alternative("Ident", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false,
				alternative("assign_operator", "string_payload"),
				alternative("define_operator", "string_payload")),
			field("Rhs", true, true,
				alternative("BasicLit", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("SliceExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false,
				alternative("Ident", "nonterminal"),
				alternative("SelectorExpr", "nonterminal")),
			field("Args", false, true,
				alternative("BasicLit", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("SliceExpr", "nonterminal"))),
		rule("SelectorExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Sel", true, false, alternative("Ident", "nonterminal"))),
		rule("IndexExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Index", true, false,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("SliceExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Low", false, false,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal")),
			field("High", false, false, alternative("BinaryExpr", "nonterminal"))),
		rule("BinaryExpr",
			field("X", true, false,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal")),
			field("Op", true, false, alternative("binary_operator", "string_payload")),
			field("Y", true, false,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"),
				alternative("IndexExpr", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false,
				alternative("integer_literal", "integer_payload"),
				alternative("string_literal", "string_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
		rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))),
		rule("Comment",
			field("Text", true, false, alternative("comment_text", "string_payload"))),
	}
}

func pathMatchSourceFileRules() []syntaxRule {
	rules := pathMatchSourceFileMatchPrefixRules()
	if len(rules) > 0 && rules[0].Name == "File" {
		rules[0].Fields = []syntaxField{
			field("HeaderComments", true, true, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Imports", true, false, alternative("GenDecl", "nonterminal")),
			field("SupportedDecls", true, true,
				alternative("GenDecl", "nonterminal"),
				alternative("FuncDecl", "nonterminal")),
			field("Comments", true, true, alternative("CommentGroup", "nonterminal")),
		}
	}
	for ruleIndex := range rules {
		switch rules[ruleIndex].Name {
		case "ValueSpec":
			rules[ruleIndex].Fields = []syntaxField{
				field("Names", true, true, alternative("Ident", "nonterminal")),
				field("Type", false, false, alternative("Ident", "nonterminal")),
				field("Values", false, true, alternative("CallExpr", "nonterminal")),
			}
		case "BlockStmt":
			for fieldIndex := range rules[ruleIndex].Fields {
				if rules[ruleIndex].Fields[fieldIndex].Name == "List" {
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("SwitchStmt", "nonterminal"),
						alternative("CaseClause", "nonterminal"),
						alternative("IncDecStmt", "nonterminal"),
					)
				}
			}
		case "IfStmt":
			rules[ruleIndex].Fields = []syntaxField{
				field("Init", false, false, alternative("AssignStmt", "nonterminal")),
				field("Cond", true, false,
					alternative("BinaryExpr", "nonterminal"),
					alternative("Ident", "nonterminal"),
					alternative("UnaryExpr", "nonterminal")),
				field("Body", true, false, alternative("BlockStmt", "nonterminal")),
			}
		case "BranchStmt":
			rules[ruleIndex].Fields = []syntaxField{
				field("Tok", true, false, alternative("branch_keyword", "string_payload")),
				field("Label", false, false, alternative("Ident", "nonterminal")),
			}
		case "ReturnStmt":
			rules[ruleIndex].Fields = []syntaxField{
				field("Results", false, true,
					alternative("BasicLit", "nonterminal"),
					alternative("BinaryExpr", "nonterminal"),
					alternative("Ident", "nonterminal"),
					alternative("SelectorExpr", "nonterminal")),
			}
		case "AssignStmt":
			for fieldIndex := range rules[ruleIndex].Fields {
				switch rules[ruleIndex].Fields[fieldIndex].Name {
				case "Rhs":
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("SelectorExpr", "nonterminal"),
					)
				}
			}
		case "SliceExpr":
			for fieldIndex := range rules[ruleIndex].Fields {
				switch rules[ruleIndex].Fields[fieldIndex].Name {
				case "Low", "High":
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("Ident", "nonterminal"),
					)
				}
			}
		case "BinaryExpr":
			for fieldIndex := range rules[ruleIndex].Fields {
				switch rules[ruleIndex].Fields[fieldIndex].Name {
				case "X", "Y":
					rules[ruleIndex].Fields[fieldIndex].Alternatives = append(
						rules[ruleIndex].Fields[fieldIndex].Alternatives,
						alternative("SelectorExpr", "nonterminal"),
					)
				}
			}
		case "BasicLit":
			rules[ruleIndex].Fields = []syntaxField{
				field("Value", true, false,
					alternative("char_literal", "string_payload"),
					alternative("integer_literal", "integer_payload"),
					alternative("string_literal", "string_payload")),
			}
		}
	}
	rules = append(rules,
		rule("SwitchStmt",
			field("Tag", true, false, alternative("IndexExpr", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("CaseClause",
			field("List", false, true, alternative("BasicLit", "nonterminal")),
			field("Body", false, true,
				alternative("AssignStmt", "nonterminal"),
				alternative("BranchStmt", "nonterminal"),
				alternative("IfStmt", "nonterminal"),
				alternative("IncDecStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("UnaryExpr",
			field("Op", true, false, alternative("unary_operator", "string_payload")),
			field("X", true, false, alternative("Ident", "nonterminal"))),
	)
	return rules
}

func cryptoSysrandRandWindowsSourceFileRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("HeaderComments", true, true, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("Imports", true, true, alternative("GenDecl", "nonterminal")),
			field("ReadDecl", true, false, alternative("FuncDecl", "nonterminal"))),
		rule("GenDecl",
			field("Tok", true, false, alternative("import_keyword", "identifier_payload")),
			field("Specs", true, true, alternative("ImportSpec", "nonterminal"))),
		rule("ImportSpec",
			field("Path", true, false, alternative("string_literal", "string_payload"))),
		rule("FuncDecl",
			field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", true, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("Ident", "nonterminal"),
				alternative("ArrayType", "nonterminal"))),
		rule("ArrayType",
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true, alternative("ReturnStmt", "nonterminal"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("CallExpr", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false, alternative("SelectorExpr", "nonterminal")),
			field("Args", true, true, alternative("Ident", "nonterminal"))),
		rule("SelectorExpr",
			field("X", true, false, alternative("Ident", "nonterminal")),
			field("Sel", true, false, alternative("Ident", "nonterminal"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
		rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))),
		rule("Comment",
			field("Text", true, false, alternative("comment_text", "string_payload"))),
	}
}

func cryptoSubtleXorSourceFileRules() []syntaxRule {
	rules := cryptoSysrandRandWindowsSourceFileRules()
	rules[0].Fields[3].Name = "XorDecl"
	return rules
}

func singleFunctionLoopSourceFileRules() []syntaxRule {
	return []syntaxRule{
		rule("File",
			field("HeaderComments", true, true, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("package_identifier", "identifier_payload")),
			field("FunctionDecl", true, false, alternative("FuncDecl", "nonterminal"))),
		rule("FuncDecl",
			field("Doc", false, false, alternative("CommentGroup", "nonterminal")),
			field("Name", true, false, alternative("identifier", "identifier_payload")),
			field("Type", true, false, alternative("FuncType", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("FuncType",
			field("Params", true, false, alternative("FieldList", "nonterminal")),
			field("Results", true, false, alternative("FieldList", "nonterminal"))),
		rule("FieldList",
			field("List", true, true, alternative("Field", "nonterminal"))),
		rule("Field",
			field("Names", false, true, alternative("Ident", "nonterminal")),
			field("Type", true, false,
				alternative("Ident", "nonterminal"),
				alternative("ArrayType", "nonterminal"))),
		rule("ArrayType",
			field("Elt", true, false, alternative("Ident", "nonterminal"))),
		rule("BlockStmt",
			field("List", true, true,
				alternative("AssignStmt", "nonterminal"),
				alternative("BranchStmt", "nonterminal"),
				alternative("ForStmt", "nonterminal"),
				alternative("IfStmt", "nonterminal"),
				alternative("ReturnStmt", "nonterminal"))),
		rule("ForStmt",
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("IfStmt",
			field("Cond", true, false, alternative("BinaryExpr", "nonterminal")),
			field("Body", true, false, alternative("BlockStmt", "nonterminal"))),
		rule("AssignStmt",
			field("Lhs", true, true, alternative("Ident", "nonterminal")),
			field("Tok", true, false, alternative("assignment_operator", "string_payload")),
			field("Rhs", true, true,
				alternative("BasicLit", "nonterminal"),
				alternative("BinaryExpr", "nonterminal"),
				alternative("CallExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("BranchStmt",
			field("Tok", true, false, alternative("branch_keyword", "string_payload"))),
		rule("ReturnStmt",
			field("Results", true, true, alternative("Ident", "nonterminal"))),
		rule("CallExpr",
			field("Fun", true, false, alternative("Ident", "nonterminal")),
			field("Args", true, true,
				alternative("BinaryExpr", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("BinaryExpr",
			field("Op", true, false, alternative("binary_operator", "string_payload")),
			field("X", true, false,
				alternative("BinaryExpr", "nonterminal"),
				alternative("Ident", "nonterminal")),
			field("Y", true, false,
				alternative("BasicLit", "nonterminal"),
				alternative("Ident", "nonterminal"))),
		rule("BasicLit",
			field("Value", true, false, alternative("integer_literal", "integer_payload"))),
		rule("Ident",
			field("Name", true, false, alternative("identifier", "identifier_payload"))),
		rule("CommentGroup",
			field("List", true, true, alternative("Comment", "nonterminal"))),
		rule("Comment",
			field("Text", true, false, alternative("comment_text", "string_payload"))),
	}
}

func validateInternalCoverageUleb128SourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "uleb128" {
		return fmt.Errorf("uleb128 source file must use package uleb128")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("uleb128 source file must contain one function declaration")
	}
	if len(file.Comments) != 1 || len(file.Comments[0].List) != 3 {
		return fmt.Errorf("uleb128 source file must preserve the three-line Go license header")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "AppendUleb128" {
		return fmt.Errorf("uleb128 source file must declare func AppendUleb128")
	}
	if funcDecl.Doc != nil {
		return fmt.Errorf("AppendUleb128 must not have a function doc comment in the selected source")
	}
	if !appendUleb128Signature(funcDecl.Type) {
		return fmt.Errorf("AppendUleb128 signature must be func AppendUleb128(b []byte, v uint) []byte")
	}
	if funcDecl.Body == nil || len(funcDecl.Body.List) != 2 {
		return fmt.Errorf("AppendUleb128 body must contain the for loop and final return")
	}
	forStmt, ok := funcDecl.Body.List[0].(*ast.ForStmt)
	if !ok || forStmt.Init != nil || forStmt.Cond != nil || forStmt.Post != nil || forStmt.Body == nil {
		return fmt.Errorf("AppendUleb128 first statement must be a bare for loop")
	}
	if len(forStmt.Body.List) != 5 {
		return fmt.Errorf("AppendUleb128 loop body must contain five statements")
	}
	if !assignDefineCFromUint8Mask(forStmt.Body.List[0]) {
		return fmt.Errorf("AppendUleb128 loop statement 1 must be c := uint8(v & 0x7f)")
	}
	if !compoundAssignIdentInteger(forStmt.Body.List[1], "v", token.SHR_ASSIGN, "7") {
		return fmt.Errorf("AppendUleb128 loop statement 2 must be v >>= 7")
	}
	if !ifCondBodyAssign(forStmt.Body.List[2], "v", token.NEQ, "0", "c", token.OR_ASSIGN, "0x80") {
		return fmt.Errorf("AppendUleb128 loop statement 3 must be if v != 0 { c |= 0x80 }")
	}
	if !assignBAppendBC(forStmt.Body.List[3]) {
		return fmt.Errorf("AppendUleb128 loop statement 4 must be b = append(b, c)")
	}
	if !ifBreakOnMaskedCZero(forStmt.Body.List[4]) {
		return fmt.Errorf("AppendUleb128 loop statement 5 must be if c&0x80 == 0 { break }")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "b") {
		return fmt.Errorf("AppendUleb128 final statement must return b")
	}
	return nil
}

func validateNetLookupPlan9SourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "net" {
		return fmt.Errorf("net lookup_plan9 source file must use package net")
	}
	if len(file.Decls) != 20 {
		return fmt.Errorf("net lookup_plan9 source file must keep the expected 20 top-level declarations")
	}
	if err := validateNetLookupPlan9ImportBlock(file.Decls[0]); err != nil {
		return err
	}
	if err := validateNetLookupPlan9ConstDecl(file.Decls[1]); err != nil {
		return err
	}
	for _, name := range []string{
		"cgoAvailable",
		"query",
		"queryCS",
		"queryCS1",
		"queryDNS",
		"handlePlan9DNSError",
		"toLower",
		"lookupProtocol",
		"lookupHost",
		"lookupIP",
		"lookupPort",
		"lookupPortWithNetwork",
		"lookupCNAME",
		"lookupSRV",
		"lookupMX",
		"lookupNS",
		"lookupTXT",
		"lookupAddr",
		"concurrentThreadsLimit",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("net lookup_plan9 source file must declare %s", name)
		}
	}
	return nil
}

func validateNetLookupPlan9ImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 7 {
		return fmt.Errorf("first net lookup_plan9 declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"context\"",
		"\"errors\"",
		"\"internal/bytealg\"",
		"\"internal/itoa\"",
		"\"internal/stringslite\"",
		"\"io\"",
		"\"os\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("net lookup_plan9 import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func validateNetLookupPlan9ConstDecl(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.CONST || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("second net lookup_plan9 declaration must be const cgoAvailable = true")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "cgoAvailable" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("net lookup_plan9 const declaration must name cgoAvailable with one value")
	}
	if valueSpec.Type != nil {
		return fmt.Errorf("net lookup_plan9 const declaration must omit ValueSpec.Type")
	}
	if !identNamed(valueSpec.Values[0], "true") {
		return fmt.Errorf("net lookup_plan9 const declaration value must be true")
	}
	return nil
}

func validateSyscallFsJSSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "syscall" {
		return fmt.Errorf("syscall fs_js source file must use package syscall")
	}
	if len(file.Decls) != 52 {
		return fmt.Errorf("syscall fs_js source file must keep the expected 52 top-level declarations")
	}
	if err := validateSyscallFsJSImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"now",
		"jsProcess",
		"jsPath",
		"jsFS",
		"constants",
		"uint8Array",
		"nodeWRONLY",
		"nodeRDWR",
		"nodeDIRECTORY",
		"init",
		"jsFile",
		"filesMu",
		"files",
		"fdToFile",
		"Open",
		"Close",
		"Read",
		"Write",
		"Seek",
		"Ftruncate",
		"Getcwd",
		"Chdir",
		"Mkdir",
		"Unlink",
		"Rmdir",
		"Rename",
		"Stat",
		"Lstat",
		"Fstat",
		"fsCall",
		"checkPath",
		"recoverErr",
		"mapJSError",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("syscall fs_js source file must declare %s", name)
		}
	}
	constructs := syscallFsJSSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"chan_type",
		"composite_literal",
		"defer_stmt",
		"for_stmt",
		"func_lit",
		"map_type",
		"range_stmt",
		"switch_stmt",
		"type_assert_expr",
	} {
		if !constructs[construct] {
			return fmt.Errorf("syscall fs_js source file must include %s", construct)
		}
	}
	return nil
}

func validateSyscallFsJSImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 3 {
		return fmt.Errorf("first syscall fs_js declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"errors\"",
		"\"sync\"",
		"\"syscall/js\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("syscall fs_js import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func syscallFsJSSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.ChanType:
			constructs["chan_type"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_stmt"] = true
		case *ast.ForStmt:
			constructs["for_stmt"] = true
		case *ast.FuncLit:
			constructs["func_lit"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_stmt"] = true
		case *ast.SwitchStmt:
			constructs["switch_stmt"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assert_expr"] = true
		}
		return true
	})
	return constructs
}

func validateOsSignalSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "signal" {
		return fmt.Errorf("os/signal source file must use package signal")
	}
	if len(file.Decls) != 21 {
		return fmt.Errorf("os/signal source file must keep the expected 21 top-level declarations")
	}
	if err := validateOsSignalImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"handlers",
		"stopping",
		"handler",
		"cancel",
		"Ignore",
		"Ignored",
		"watchSignalLoopOnce",
		"Notify",
		"Reset",
		"Stop",
		"signalWaitUntilIdle",
		"process",
		"NotifyContext",
		"signalCtx",
		"stringer",
	} {
		if !osSignalDeclares(file, name) {
			return fmt.Errorf("os/signal source file must declare %s", name)
		}
	}
	constructs := osSignalSyntaxConstructs(file)
	for _, construct := range []string{
		"map_type",
		"chan_type",
		"interface_type",
		"select_stmt",
		"type_assert_expr",
		"go_stmt",
	} {
		if !constructs[construct] {
			return fmt.Errorf("os/signal source file must include %s", construct)
		}
	}
	return nil
}

func validateHttptestServerSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "httptest" {
		return fmt.Errorf("net/http/httptest server source file must use package httptest")
	}
	if len(file.Decls) != 21 {
		return fmt.Errorf("net/http/httptest server source file must keep the expected 21 top-level declarations")
	}
	if err := validateHttptestServerImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"Server",
		"newLocalListener",
		"serveFlag",
		"strSliceContainsPrefix",
		"NewServer",
		"NewUnstartedServer",
		"Start",
		"StartTLS",
		"NewTLSServer",
		"closeIdleTransport",
		"Close",
		"logCloseHangDebugInfo",
		"CloseClientConnections",
		"Certificate",
		"Client",
		"goServe",
		"wrap",
		"closeConn",
		"closeConnChan",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("net/http/httptest server source file must declare %s", name)
		}
	}
	constructs := httptestServerSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"chan_type",
		"composite_literal",
		"defer_stmt",
		"for_stmt",
		"func_lit",
		"go_stmt",
		"map_type",
		"range_stmt",
		"select_stmt",
		"send_stmt",
		"switch_stmt",
		"type_assert_expr",
	} {
		if !constructs[construct] {
			return fmt.Errorf("net/http/httptest server source file must include %s", construct)
		}
	}
	return nil
}

func validateHttptestServerImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 12 {
		return fmt.Errorf("first net/http/httptest server declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"crypto/tls\"",
		"\"crypto/x509\"",
		"\"flag\"",
		"\"fmt\"",
		"\"log\"",
		"\"net\"",
		"\"net/http\"",
		"\"net/http/internal/testcert\"",
		"\"os\"",
		"\"strings\"",
		"\"sync\"",
		"\"time\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("net/http/httptest server import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func httptestServerSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.ChanType:
			constructs["chan_type"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_stmt"] = true
		case *ast.ForStmt:
			constructs["for_stmt"] = true
		case *ast.FuncLit:
			constructs["func_lit"] = true
		case *ast.GoStmt:
			constructs["go_stmt"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_stmt"] = true
		case *ast.SelectStmt:
			constructs["select_stmt"] = true
		case *ast.SendStmt:
			constructs["send_stmt"] = true
		case *ast.SwitchStmt:
			constructs["switch_stmt"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assert_expr"] = true
		}
		return true
	})
	return constructs
}

func validateNetNSSSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "net" {
		return fmt.Errorf("net/nss source file must use package net")
	}
	if len(file.Decls) != 18 {
		return fmt.Errorf("net/nss source file must keep the expected 18 top-level declarations")
	}
	if err := validateNetNSSImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"nssConfigPath",
		"nssConfig",
		"nsswitchConfig",
		"getSystemNSS",
		"init",
		"tryUpdate",
		"acquireSema",
		"tryAcquireSema",
		"releaseSema",
		"nssConf",
		"nssSource",
		"standardCriteria",
		"nssCriterion",
		"standardStatusAction",
		"parseNSSConfFile",
		"parseNSSConf",
		"parseCriteria",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("net/nss source file must declare %s", name)
		}
	}
	constructs := netNSSSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"channel_type",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"map_type",
		"range_statement",
		"select_statement",
		"send_statement",
		"slice_expression",
		"switch_statement",
	} {
		if !constructs[construct] {
			return fmt.Errorf("net/nss source file must include %s", construct)
		}
	}
	return nil
}

func validateNetNSSImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 5 {
		return fmt.Errorf("first net/nss declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"errors\"",
		"\"internal/bytealg\"",
		"\"os\"",
		"\"sync\"",
		"\"time\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("net/nss import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func netNSSSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.ChanType:
			constructs["channel_type"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SelectStmt:
			constructs["select_statement"] = true
		case *ast.SendStmt:
			constructs["send_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		}
		return true
	})
	return constructs
}

func validateNetIpsockPlan9SourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "net" {
		return fmt.Errorf("net/ipsock_plan9 source file must use package net")
	}
	if len(file.Decls) != 16 {
		return fmt.Errorf("net/ipsock_plan9 source file must keep the expected 16 top-level declarations")
	}
	if err := validateNetIpsockPlan9ImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"probe",
		"parsePlan9Addr",
		"readPlan9Addr",
		"startPlan9",
		"fixErr",
		"dialPlan9",
		"dialPlan9Blocking",
		"listenPlan9",
		"netFD",
		"acceptPlan9",
		"isWildcard",
		"toLocal",
		"plan9LocalAddr",
		"hangupCtlWrite",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("net/ipsock_plan9 source file must declare %s", name)
		}
	}
	constructs := netIpsockPlan9SyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"channel_type",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"go_statement",
		"select_statement",
		"send_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"type_assertion_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("net/ipsock_plan9 source file must include %s", construct)
		}
	}
	return nil
}

func validateNetIpsockPlan9ImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 6 {
		return fmt.Errorf("first net/ipsock_plan9 declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"context\"",
		"\"internal/bytealg\"",
		"\"internal/itoa\"",
		"\"io/fs\"",
		"\"os\"",
		"\"syscall\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("net/ipsock_plan9 import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func netIpsockPlan9SyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.ChanType:
			constructs["channel_type"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.GoStmt:
			constructs["go_statement"] = true
		case *ast.SelectStmt:
			constructs["select_statement"] = true
		case *ast.SendStmt:
			constructs["send_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assertion_expression"] = true
		}
		return true
	})
	return constructs
}

func validateOsDirUnixSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "os" {
		return fmt.Errorf("os/dir_unix source file must use package os")
	}
	if len(file.Decls) != 9 {
		return fmt.Errorf("os/dir_unix source file must keep the expected 9 top-level declarations")
	}
	if err := validateOsDirUnixImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"dirInfo",
		"blockSize",
		"dirBufPool",
		"close",
		"readdir",
		"readInt",
		"readIntBE",
		"readIntLE",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("os/dir_unix source file must declare %s", name)
		}
	}
	constructs := osDirUnixSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"range_statement",
		"slice_expression",
		"star_expression",
		"switch_statement",
		"type_assertion_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("os/dir_unix source file must include %s", construct)
		}
	}
	return nil
}

func validateOsDirUnixImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 7 {
		return fmt.Errorf("first os/dir_unix declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"internal/byteorder\"",
		"\"internal/goarch\"",
		"\"io\"",
		"\"runtime\"",
		"\"sync\"",
		"\"syscall\"",
		"\"unsafe\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("os/dir_unix import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func osDirUnixSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assertion_expression"] = true
		}
		return true
	})
	return constructs
}

func validateNetRPCClientSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "rpc" {
		return fmt.Errorf("net/rpc client source file must use package rpc")
	}
	if len(file.Decls) != 23 {
		return fmt.Errorf("net/rpc client source file must keep the expected 23 top-level declarations")
	}
	if err := validateNetRPCClientImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"ServerError",
		"Error",
		"ErrShutdown",
		"Call",
		"Client",
		"ClientCodec",
		"send",
		"input",
		"done",
		"NewClient",
		"NewClientWithCodec",
		"gobClientCodec",
		"WriteRequest",
		"ReadResponseHeader",
		"ReadResponseBody",
		"Close",
		"DialHTTP",
		"DialHTTPPath",
		"Dial",
		"Go",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("net/rpc client source file must declare %s", name)
		}
	}
	constructs := netRPCClientSyntaxConstructs(file)
	for _, construct := range []string{
		"channel_type",
		"composite_literal",
		"for_statement",
		"function_type",
		"go_statement",
		"interface_type",
		"map_type",
		"select_statement",
		"send_statement",
		"struct_type",
		"switch_statement",
	} {
		if !constructs[construct] {
			return fmt.Errorf("net/rpc client source file must include %s", construct)
		}
	}
	return nil
}

func validateNetRPCClientImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 8 {
		return fmt.Errorf("first net/rpc client declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"bufio\"",
		"\"encoding/gob\"",
		"\"errors\"",
		"\"io\"",
		"\"log\"",
		"\"net\"",
		"\"net/http\"",
		"\"sync\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("net/rpc client import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func netRPCClientSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ChanType:
			constructs["channel_type"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncType:
			constructs["function_type"] = true
		case *ast.GoStmt:
			constructs["go_statement"] = true
		case *ast.InterfaceType:
			constructs["interface_type"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.SelectStmt:
			constructs["select_statement"] = true
		case *ast.SendStmt:
			constructs["send_statement"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		}
		return true
	})
	return constructs
}

func validateInternalCoverageCfileTestsupportSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "cfile" {
		return fmt.Errorf("internal/coverage/cfile testsupport source file must use package cfile")
	}
	if len(file.Decls) != 7 {
		return fmt.Errorf("internal/coverage/cfile testsupport source file must keep the expected 7 top-level declarations")
	}
	if err := validateInternalCoverageCfileTestsupportImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"ProcessCoverTestDir",
		"tstate",
		"processPod",
		"pkfunc",
		"readAuxMetaFiles",
		"Snapshot",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("internal/coverage/cfile testsupport source file must declare %s", name)
		}
	}
	constructs := internalCoverageCfileTestsupportSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"map_type",
		"range_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
	} {
		if !constructs[construct] {
			return fmt.Errorf("internal/coverage/cfile testsupport source file must include %s", construct)
		}
	}
	return nil
}

func validateInternalCoverageCfileTestsupportImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 16 {
		return fmt.Errorf("first internal/coverage/cfile testsupport declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"encoding/json\"",
		"\"fmt\"",
		"\"internal/coverage\"",
		"\"internal/coverage/calloc\"",
		"\"internal/coverage/cformat\"",
		"\"internal/coverage/cmerge\"",
		"\"internal/coverage/decodecounter\"",
		"\"internal/coverage/decodemeta\"",
		"\"internal/coverage/pods\"",
		"\"internal/coverage/rtcov\"",
		"\"internal/runtime/atomic\"",
		"\"io\"",
		"\"os\"",
		"\"path/filepath\"",
		"\"strings\"",
		"\"unsafe\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("internal/coverage/cfile testsupport import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func internalCoverageCfileTestsupportSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		}
		return true
	})
	return constructs
}

func validateInternalFuzzMutatorSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "fuzz" {
		return fmt.Errorf("internal/fuzz mutator source file must use package fuzz")
	}
	if len(file.Decls) != 16 {
		return fmt.Errorf("internal/fuzz mutator source file must keep the expected 16 top-level declarations")
	}
	if err := validateInternalFuzzMutatorImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"mutator",
		"newMutator",
		"rand",
		"randByteOrder",
		"chooseLen",
		"mutate",
		"mutateInt",
		"mutateUInt",
		"mutateFloat",
		"byteSliceMutator",
		"byteSliceMutators",
		"mutateBytes",
		"interesting8",
		"interesting16",
		"interesting32",
		"maxUint",
		"maxInt",
		"init",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("internal/fuzz mutator source file must declare %s", name)
		}
	}
	constructs := internalFuzzMutatorSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"function_type",
		"range_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"type_assertion_expression",
		"type_switch_statement",
	} {
		if !constructs[construct] {
			return fmt.Errorf("internal/fuzz mutator source file must include %s", construct)
		}
	}
	return nil
}

func validateInternalFuzzMutatorImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 4 {
		return fmt.Errorf("first internal/fuzz mutator declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"encoding/binary\"",
		"\"fmt\"",
		"\"math\"",
		"\"unsafe\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("internal/fuzz mutator import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func internalFuzzMutatorSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.FuncType:
			constructs["function_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assertion_expression"] = true
		case *ast.TypeSwitchStmt:
			constructs["type_switch_statement"] = true
		}
		return true
	})
	return constructs
}

func validateRuntimeDebugModSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "debug" {
		return fmt.Errorf("runtime/debug mod source file must use package debug")
	}
	if len(file.Decls) != 10 {
		return fmt.Errorf("runtime/debug mod source file must keep the expected 10 top-level declarations")
	}
	if err := validateRuntimeDebugModImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"modinfo",
		"ReadBuildInfo",
		"BuildInfo",
		"Module",
		"BuildSetting",
		"quoteKey",
		"quoteValue",
		"String",
		"ParseBuildInfo",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("runtime/debug mod source file must declare %s", name)
		}
	}
	constructs := runtimeDebugModSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"function_type",
		"range_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
	} {
		if !constructs[construct] {
			return fmt.Errorf("runtime/debug mod source file must include %s", construct)
		}
	}
	return nil
}

func validateRuntimeDebugModImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 4 {
		return fmt.Errorf("first runtime/debug mod declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"fmt\"",
		"\"runtime\"",
		"\"strconv\"",
		"\"strings\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("runtime/debug mod import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func runtimeDebugModSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.FuncType:
			constructs["function_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		}
		return true
	})
	return constructs
}

func validateGoAstCommentmapSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "ast" {
		return fmt.Errorf("go/ast commentmap source file must use package ast")
	}
	if len(file.Decls) != 17 {
		return fmt.Errorf("go/ast commentmap source file must keep the expected 17 top-level declarations")
	}
	if err := validateGoAstCommentmapImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"sortComments",
		"CommentMap",
		"addComment",
		"nodeList",
		"commentListReader",
		"eol",
		"next",
		"nodeStack",
		"push",
		"pop",
		"NewCommentMap",
		"Update",
		"Filter",
		"Comments",
		"summary",
		"String",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("go/ast commentmap source file must declare %s", name)
		}
	}
	constructs := goAstCommentmapSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"for_statement",
		"function_literal",
		"map_type",
		"range_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"type_assertion_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("go/ast commentmap source file must include %s", construct)
		}
	}
	return nil
}

func validateGoAstCommentmapImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 6 {
		return fmt.Errorf("first go/ast commentmap declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"bytes\"",
		"\"cmp\"",
		"\"fmt\"",
		"\"go/token\"",
		"\"slices\"",
		"\"strings\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("go/ast commentmap import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func goAstCommentmapSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assertion_expression"] = true
		}
		return true
	})
	return constructs
}

func validateGoTypesGotypeSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("go/types gotype source file must use package main")
	}
	if len(file.Decls) != 15 {
		return fmt.Errorf("go/types gotype source file must keep the expected 15 top-level declarations")
	}
	if err := validateGoTypesGotypeImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"testFiles",
		"xtestFiles",
		"allErrors",
		"verbose",
		"compiler",
		"printAST",
		"printTrace",
		"parseComments",
		"panicOnError",
		"fset",
		"errorCount",
		"sequential",
		"parserMode",
		"initParserMode",
		"usageString",
		"usage",
		"report",
		"parse",
		"parseStdin",
		"parseFiles",
		"parseDir",
		"getPkgFiles",
		"checkPkgFiles",
		"printStats",
		"main",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("go/types gotype source file must declare %s", name)
		}
	}
	constructs := goTypesGotypeSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"function_literal",
		"function_type",
		"go_statement",
		"range_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
		"type_assertion_expression",
		"type_switch_statement",
	} {
		if !constructs[construct] {
			return fmt.Errorf("go/types gotype source file must include %s", construct)
		}
	}
	return nil
}

func validateGoTypesGotypeImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 14 {
		return fmt.Errorf("first go/types gotype declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"flag\"",
		"\"fmt\"",
		"\"go/ast\"",
		"\"go/build\"",
		"\"go/importer\"",
		"\"go/parser\"",
		"\"go/scanner\"",
		"\"go/token\"",
		"\"go/types\"",
		"\"io\"",
		"\"os\"",
		"\"path/filepath\"",
		"\"sync\"",
		"\"time\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("go/types gotype import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func goTypesGotypeSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.FuncType:
			constructs["function_type"] = true
		case *ast.GoStmt:
			constructs["go_statement"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assertion_expression"] = true
		case *ast.TypeSwitchStmt:
			constructs["type_switch_statement"] = true
		}
		return true
	})
	return constructs
}

func validateTestingMatchSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "testing" {
		return fmt.Errorf("testing/match source file must use package testing")
	}
	if len(file.Decls) != 19 {
		return fmt.Errorf("testing/match source file must keep the expected 19 top-level declarations")
	}
	if err := validateTestingMatchImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"matcher",
		"filterMatch",
		"simpleMatch",
		"alternationMatch",
		"matchMutex",
		"allMatcher",
		"newMatcher",
		"fullName",
		"clearSubNames",
		"matches",
		"verify",
		"splitRegexp",
		"unique",
		"parseSubtestNumber",
		"rewrite",
		"isSpace",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("testing/match source file must declare %s", name)
		}
	}
	constructs := testingMatchSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_type",
		"interface_type",
		"map_type",
		"range_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
	} {
		if !constructs[construct] {
			return fmt.Errorf("testing/match source file must include %s", construct)
		}
	}
	return nil
}

func validateTestingMatchImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 5 {
		return fmt.Errorf("first testing/match declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"fmt\"",
		"\"os\"",
		"\"strconv\"",
		"\"strings\"",
		"\"sync\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("testing/match import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func testingMatchSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncType:
			constructs["function_type"] = true
		case *ast.InterfaceType:
			constructs["interface_type"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		}
		return true
	})
	return constructs
}

func validateGoDocCommentTextSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "comment" {
		return fmt.Errorf("go/doc/comment text source file must use package comment")
	}
	if len(file.Decls) != 9 {
		return fmt.Errorf("go/doc/comment text source file must keep the expected 9 top-level declarations")
	}
	if err := validateGoDocCommentTextImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"textPrinter",
		"Text",
		"writeNL",
		"block",
		"text",
		"oneLongLine",
		"wrap",
		"wrapPenalty",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("go/doc/comment text source file must declare %s", name)
		}
	}
	constructs := goDocCommentTextSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"for_statement",
		"function_literal",
		"range_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"type_switch_statement",
	} {
		if !constructs[construct] {
			return fmt.Errorf("go/doc/comment text source file must include %s", construct)
		}
	}
	return nil
}

func validateGoDocCommentTextImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 5 {
		return fmt.Errorf("first go/doc/comment text declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"bytes\"",
		"\"fmt\"",
		"\"sort\"",
		"\"strings\"",
		"\"unicode/utf8\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("go/doc/comment text import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func goDocCommentTextSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeSwitchStmt:
			constructs["type_switch_statement"] = true
		}
		return true
	})
	return constructs
}

func validateInternalBuildcfgExpSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "buildcfg" {
		return fmt.Errorf("internal/buildcfg exp source file must use package buildcfg")
	}
	if len(file.Decls) != 10 {
		return fmt.Errorf("internal/buildcfg exp source file must keep the expected 10 top-level declarations")
	}
	if err := validateInternalBuildcfgExpImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"ExperimentFlags",
		"Experiment",
		"DefaultGOEXPERIMENT",
		"FramePointerEnabled",
		"ParseGOEXPERIMENT",
		"String",
		"expList",
		"Enabled",
		"All",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("internal/buildcfg exp source file must declare %s", name)
		}
	}
	constructs := internalBuildcfgExpSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"for_statement",
		"function_literal",
		"function_type",
		"map_type",
		"range_statement",
		"selector_expression",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("internal/buildcfg exp source file must include %s", construct)
		}
	}
	return nil
}

func validateInternalBuildcfgExpImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 4 {
		return fmt.Errorf("first internal/buildcfg exp declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"fmt\"",
		"\"reflect\"",
		"\"strings\"",
		"\"internal/goexperiment\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("internal/buildcfg exp import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func internalBuildcfgExpSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.FuncType:
			constructs["function_type"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateRuntimeErrorSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "runtime" {
		return fmt.Errorf("runtime/error source file must use package runtime")
	}
	if len(file.Decls) != 29 {
		return fmt.Errorf("runtime/error source file must keep the expected 29 top-level declarations")
	}
	if err := validateRuntimeErrorImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"Error",
		"TypeAssertionError",
		"itoa",
		"errorString",
		"errorAddressString",
		"plainError",
		"boundsError",
		"boundsErrorCode",
		"boundsErrorFmts",
		"boundsNegErrorFmts",
		"appendIntStr",
		"stringer",
		"printpanicval",
		"printanycustomtype",
		"printindented",
		"panicwrap",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("runtime/error source file must declare %s", name)
		}
	}
	constructs := runtimeErrorSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"for_statement",
		"function_type",
		"index_expression",
		"interface_type",
		"selector_expression",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"type_assertion_expression",
		"type_switch_statement",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("runtime/error source file must include %s", construct)
		}
	}
	return nil
}

func validateRuntimeErrorImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 3 {
		return fmt.Errorf("first runtime/error declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"internal/abi\"",
		"\"internal/bytealg\"",
		"\"internal/runtime/sys\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("runtime/error import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func runtimeErrorSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncType:
			constructs["function_type"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.InterfaceType:
			constructs["interface_type"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assertion_expression"] = true
		case *ast.TypeSwitchStmt:
			constructs["type_switch_statement"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateSyscallNetlinkLinuxSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "syscall" {
		return fmt.Errorf("syscall/netlink_linux source file must use package syscall")
	}
	if len(file.Decls) != 14 {
		return fmt.Errorf("syscall/netlink_linux source file must keep the expected 14 top-level declarations")
	}
	if err := validateSyscallNetlinkLinuxImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"nlmAlignOf",
		"rtaAlignOf",
		"NetlinkRouteRequest",
		"toWireFormat",
		"newNetlinkRouteRequest",
		"pageBufPool",
		"NetlinkRIB",
		"NetlinkMessage",
		"ParseNetlinkMessage",
		"netlinkMessageHeaderAndData",
		"NetlinkRouteAttr",
		"ParseNetlinkRouteAttr",
		"netlinkRouteAttrAndValue",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("syscall/netlink_linux source file must declare %s", name)
		}
	}
	constructs := syscallNetlinkLinuxSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"index_expression",
		"labeled_statement",
		"selector_expression",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("syscall/netlink_linux source file must include %s", construct)
		}
	}
	return nil
}

func validateSyscallNetlinkLinuxImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 2 {
		return fmt.Errorf("first syscall/netlink_linux declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"sync\"",
		"\"unsafe\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("syscall/netlink_linux import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func syscallNetlinkLinuxSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.LabeledStmt:
			constructs["labeled_statement"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateDebugPlan9objFileSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "plan9obj" {
		return fmt.Errorf("debug/plan9obj file source file must use package plan9obj")
	}
	if len(file.Decls) != 19 {
		return fmt.Errorf("debug/plan9obj file source file must keep the expected 19 top-level declarations")
	}
	if err := validateDebugPlan9objFileImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"FileHeader",
		"File",
		"SectionHeader",
		"Section",
		"Data",
		"Open",
		"Sym",
		"formatError",
		"parseMagic",
		"NewFile",
		"walksymtab",
		"newTable",
		"ErrNoSymbols",
		"Symbols",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("debug/plan9obj file source file must declare %s", name)
		}
	}
	constructs := debugPlan9objFileSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"for_statement",
		"function_literal",
		"function_type",
		"index_expression",
		"map_type",
		"range_statement",
		"selector_expression",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("debug/plan9obj file source file must include %s", construct)
		}
	}
	return nil
}

func validateDebugPlan9objFileImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 6 {
		return fmt.Errorf("first debug/plan9obj file declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"encoding/binary\"",
		"\"errors\"",
		"\"fmt\"",
		"\"internal/saferio\"",
		"\"io\"",
		"\"os\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("debug/plan9obj file import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func debugPlan9objFileSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.FuncType:
			constructs["function_type"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateFmtErrorsSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "fmt" {
		return fmt.Errorf("fmt/errors source file must use package fmt")
	}
	if len(file.Decls) != 8 {
		return fmt.Errorf("fmt/errors source file must keep the expected 8 top-level declarations")
	}
	if err := validateFmtErrorsImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"Errorf",
		"wrapError",
		"wrapErrors",
		"Error",
		"Unwrap",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("fmt/errors source file must declare %s", name)
		}
	}
	constructs := fmtErrorsSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"ellipsis",
		"index_expression",
		"range_statement",
		"selector_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"type_assertion_expression",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("fmt/errors source file must include %s", construct)
		}
	}
	return nil
}

func validateFmtErrorsImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 2 {
		return fmt.Errorf("first fmt/errors declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"errors\"",
		"\"slices\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("fmt/errors import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func validateInternalPollFdFsyncWindowsSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "poll" {
		return fmt.Errorf("internal/poll/fd_fsync_windows source file must use package poll")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("internal/poll/fd_fsync_windows source file must keep the expected import and Fsync declarations")
	}
	if !singleImportSpecPath(file.Decls[0], "\"syscall\"") {
		return fmt.Errorf("internal/poll/fd_fsync_windows source file must import syscall")
	}
	fsync, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || fsync.Name == nil || fsync.Name.Name != "Fsync" {
		return fmt.Errorf("internal/poll/fd_fsync_windows source file must declare Fsync")
	}
	if fsync.Recv == nil || len(fsync.Recv.List) != 1 {
		return fmt.Errorf("Fsync must keep the expected FD receiver")
	}
	if _, ok := fsync.Recv.List[0].Type.(*ast.StarExpr); !ok {
		return fmt.Errorf("Fsync receiver must be a pointer receiver")
	}
	if fsync.Type == nil || fsync.Type.Results == nil || len(fsync.Type.Results.List) != 1 {
		return fmt.Errorf("Fsync must keep one error result")
	}
	if result, ok := fsync.Type.Results.List[0].Type.(*ast.Ident); !ok || result.Name != "error" {
		return fmt.Errorf("Fsync result must be error")
	}
	if fsync.Body == nil || len(fsync.Body.List) != 3 {
		return fmt.Errorf("Fsync body must keep if/defer/return statements")
	}
	if _, ok := fsync.Body.List[0].(*ast.IfStmt); !ok {
		return fmt.Errorf("Fsync first statement must remain an if statement")
	}
	if _, ok := fsync.Body.List[1].(*ast.DeferStmt); !ok {
		return fmt.Errorf("Fsync second statement must remain a defer statement")
	}
	if _, ok := fsync.Body.List[2].(*ast.ReturnStmt); !ok {
		return fmt.Errorf("Fsync final statement must remain a return statement")
	}
	return nil
}

func validateInternalDagParseSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "dag" {
		return fmt.Errorf("internal/dag parse source file must use package dag")
	}
	if len(file.Decls) != 17 {
		return fmt.Errorf("internal/dag parse source file must keep the expected 17 top-level declarations")
	}
	if err := validateInternalDagParseImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"Graph",
		"newGraph",
		"addNode",
		"AddEdge",
		"DelEdge",
		"HasEdge",
		"Edges",
		"Parse",
		"rule",
		"syntaxError",
		"Error",
		"parseRules",
		"rulesParser",
		"nextList",
		"nextToken",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("internal/dag parse source file must declare %s", name)
		}
	}
	constructs := internalDagParseSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"index_expression",
		"map_type",
		"range_statement",
		"selector_expression",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"type_assertion_expression",
		"type_switch_statement",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("internal/dag parse source file must include %s", construct)
		}
	}
	return nil
}

func validateInternalDagParseImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 4 {
		return fmt.Errorf("first internal/dag parse declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"cmp\"",
		"\"fmt\"",
		"\"slices\"",
		"\"strings\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("internal/dag parse import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func internalDagParseSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assertion_expression"] = true
		case *ast.TypeSwitchStmt:
			constructs["type_switch_statement"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateInternalTraceReaderSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "trace" {
		return fmt.Errorf("internal/trace reader source file must use package trace")
	}
	if len(file.Decls) != 5 {
		return fmt.Errorf("internal/trace reader source file must keep the expected 5 top-level declarations")
	}
	if err := validateInternalTraceReaderImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"Reader",
		"NewReader",
		"ReadEvent",
		"dumpFrontier",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("internal/trace reader source file must declare %s", name)
		}
	}
	constructs := internalTraceReaderSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"defer_statement",
		"for_statement",
		"function_literal",
		"index_expression",
		"map_type",
		"selector_expression",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("internal/trace reader source file must include %s", construct)
		}
	}
	return nil
}

func validateInternalTraceReaderImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 8 {
		return fmt.Errorf("first internal/trace reader declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"bufio\"",
		"\"fmt\"",
		"\"io\"",
		"\"slices\"",
		"\"strings\"",
		"\"internal/trace/event/go122\"",
		"\"internal/trace/internal/oldtrace\"",
		"\"internal/trace/version\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("internal/trace reader import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func internalTraceReaderSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateMimeMultipartFormdataSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "multipart" {
		return fmt.Errorf("mime/multipart formdata source file must use package multipart")
	}
	if len(file.Decls) != 13 {
		return fmt.Errorf("mime/multipart formdata source file must keep the expected 13 top-level declarations")
	}
	if err := validateMimeMultipartFormdataImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"ErrMessageTooLarge",
		"ReadForm",
		"multipartfiles",
		"multipartmaxparts",
		"readForm",
		"mimeHeaderSize",
		"Form",
		"RemoveAll",
		"FileHeader",
		"Open",
		"File",
		"sectionReadCloser",
		"Close",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("mime/multipart formdata source file must declare %s", name)
		}
	}
	constructs := mimeMultipartFormdataSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"declaration_statement",
		"defer_statement",
		"for_statement",
		"function_literal",
		"index_expression",
		"interface_type",
		"map_type",
		"range_statement",
		"selector_expression",
		"star_expression",
		"struct_type",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("mime/multipart formdata source file must include %s", construct)
		}
	}
	return nil
}

func validateMimeMultipartFormdataImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 8 {
		return fmt.Errorf("first mime/multipart formdata declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"bytes\"",
		"\"errors\"",
		"\"internal/godebug\"",
		"\"io\"",
		"\"math\"",
		"\"net/textproto\"",
		"\"os\"",
		"\"strconv\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("mime/multipart formdata import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func mimeMultipartFormdataSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeclStmt:
			constructs["declaration_statement"] = true
		case *ast.DeferStmt:
			constructs["defer_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.FuncLit:
			constructs["function_literal"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.InterfaceType:
			constructs["interface_type"] = true
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateCompressFlateHuffmanCodeSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "flate" {
		return fmt.Errorf("compress/flate huffman_code source file must use package flate")
	}
	if len(file.Decls) != 28 {
		return fmt.Errorf("compress/flate huffman_code source file must keep the expected 28 top-level declarations")
	}
	if err := validateCompressFlateHuffmanCodeImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"hcode",
		"huffmanEncoder",
		"literalNode",
		"levelInfo",
		"set",
		"maxNode",
		"newHuffmanEncoder",
		"generateFixedLiteralEncoding",
		"generateFixedOffsetEncoding",
		"fixedLiteralEncoding",
		"fixedOffsetEncoding",
		"bitLength",
		"maxBitsLimit",
		"bitCounts",
		"assignEncodingAndSize",
		"generate",
		"byLiteral",
		"byFreq",
		"reverseBits",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("compress/flate huffman_code source file must declare %s", name)
		}
	}
	constructs := compressFlateHuffmanCodeSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"case_clause",
		"composite_literal",
		"declaration_statement",
		"for_statement",
		"index_expression",
		"parenthesized_expression",
		"range_statement",
		"selector_expression",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("compress/flate huffman_code source file must include %s", construct)
		}
	}
	return nil
}

func validateCompressFlateHuffmanCodeImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 3 {
		return fmt.Errorf("first compress/flate huffman_code declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"math\"",
		"\"math/bits\"",
		"\"sort\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("compress/flate huffman_code import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func compressFlateHuffmanCodeSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CaseClause:
			constructs["case_clause"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.DeclStmt:
			constructs["declaration_statement"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.ParenExpr:
			constructs["parenthesized_expression"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func singleImportSpecPath(decl ast.Decl, path string) bool {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || len(genDecl.Specs) != 1 {
		return false
	}
	return importSpecPath(genDecl.Specs[0], path)
}

func fmtErrorsSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.Ellipsis:
			constructs["ellipsis"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SelectorExpr:
			constructs["selector_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assertion_expression"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateEncodingXMLTypeinfoSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "xml" {
		return fmt.Errorf("encoding/xml typeinfo source file must use package xml")
	}
	if len(file.Decls) != 15 {
		return fmt.Errorf("encoding/xml typeinfo source file must keep the expected 15 top-level declarations")
	}
	if err := validateEncodingXMLTypeinfoImportBlock(file.Decls[0]); err != nil {
		return err
	}
	for _, name := range []string{
		"typeInfo",
		"fieldInfo",
		"fieldFlags",
		"fElement",
		"tinfoMap",
		"nameType",
		"getTypeInfo",
		"structFieldInfo",
		"lookupXMLName",
		"addFieldInfo",
		"TagPathError",
		"initNilPointers",
	} {
		if !goFileDeclares(file, name) {
			return fmt.Errorf("encoding/xml typeinfo source file must declare %s", name)
		}
	}
	constructs := encodingXMLTypeinfoSyntaxConstructs(file)
	for _, construct := range []string{
		"array_type",
		"branch_statement",
		"composite_literal",
		"for_statement",
		"index_expression",
		"range_statement",
		"slice_expression",
		"star_expression",
		"struct_type",
		"switch_statement",
		"type_assert_expression",
		"unary_expression",
	} {
		if !constructs[construct] {
			return fmt.Errorf("encoding/xml typeinfo source file must include %s", construct)
		}
	}
	return nil
}

func validateEncodingXMLTypeinfoImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 4 {
		return fmt.Errorf("first encoding/xml typeinfo declaration must be the expected import block")
	}
	for index, path := range []string{
		"\"fmt\"",
		"\"reflect\"",
		"\"strings\"",
		"\"sync\"",
	} {
		if !importSpecPath(genDecl.Specs[index], path) {
			return fmt.Errorf("encoding/xml typeinfo import block must import %s at position %d", path, index+1)
		}
	}
	return nil
}

func encodingXMLTypeinfoSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ArrayType:
			constructs["array_type"] = true
		case *ast.BranchStmt:
			constructs["branch_statement"] = true
		case *ast.CompositeLit:
			constructs["composite_literal"] = true
		case *ast.ForStmt:
			constructs["for_statement"] = true
		case *ast.IndexExpr:
			constructs["index_expression"] = true
		case *ast.RangeStmt:
			constructs["range_statement"] = true
		case *ast.SliceExpr:
			constructs["slice_expression"] = true
		case *ast.StarExpr:
			constructs["star_expression"] = true
		case *ast.StructType:
			constructs["struct_type"] = true
		case *ast.SwitchStmt:
			constructs["switch_statement"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assert_expression"] = true
		case *ast.UnaryExpr:
			constructs["unary_expression"] = true
		}
		return true
	})
	return constructs
}

func validateOsSignalNotifyContextSourceSpan(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "signal" {
		return fmt.Errorf("os/signal NotifyContext span must use package signal")
	}
	if err := validateOsSignalImportBlock(file.Decls[0]); err != nil {
		return err
	}
	notifyContext := osSignalNotifyContextDecl(file)
	if notifyContext == nil {
		return fmt.Errorf("os/signal source file must declare NotifyContext")
	}
	if !commentGroupTextEquals(notifyContext.Doc, []string{
		"// NotifyContext returns a copy of the parent context that is marked done",
		"// (its Done channel is closed) when one of the listed signals arrives,",
		"// when the returned stop function is called, or when the parent context's",
		"// Done channel is closed, whichever happens first.",
		"//",
		"// The stop function unregisters the signal behavior, which, like [signal.Reset],",
		"// may restore the default behavior for a given signal. For example, the default",
		"// behavior of a Go program receiving [os.Interrupt] is to exit. Calling",
		"// NotifyContext(parent, os.Interrupt) will change the behavior to cancel",
		"// the returned context. Future interrupts received will not trigger the default",
		"// (exit) behavior until the returned stop function is called.",
		"//",
		"// The stop function releases resources associated with it, so code should",
		"// call stop as soon as the operations running in this Context complete and",
		"// signals no longer need to be diverted to the context.",
	}) {
		return fmt.Errorf("NotifyContext doc comments must match the selected parser-backed source span")
	}
	if !notifyContextSignature(notifyContext.Type) {
		return fmt.Errorf("NotifyContext signature must be func NotifyContext(parent context.Context, signals ...os.Signal) (ctx context.Context, stop context.CancelFunc)")
	}
	if notifyContext.Body == nil || len(notifyContext.Body.List) != 6 {
		return fmt.Errorf("NotifyContext body must keep the selected six-statement parser-backed span")
	}
	if !declContainsNode(notifyContext, func(node ast.Node) bool {
		_, ok := node.(*ast.Ellipsis)
		return ok
	}) {
		return fmt.Errorf("NotifyContext span must include the variadic Ellipsis parameter")
	}
	if !blockContainsNode(notifyContext.Body, func(node ast.Node) bool {
		chanType, ok := node.(*ast.ChanType)
		return ok && chanType.Dir == ast.SEND|ast.RECV && selectorNamed(chanType.Value, "os", "Signal")
	}) {
		return fmt.Errorf("NotifyContext span must include make(chan os.Signal, 1)")
	}
	if !blockContainsNode(notifyContext.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.GoStmt)
		return ok
	}) {
		return fmt.Errorf("NotifyContext span must include the goroutine")
	}
	if !blockContainsNode(notifyContext.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.FuncLit)
		return ok
	}) {
		return fmt.Errorf("NotifyContext span must include the function literal")
	}
	if !blockContainsNode(notifyContext.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.SelectStmt)
		return ok
	}) {
		return fmt.Errorf("NotifyContext span must include select")
	}
	commClauseCount := 0
	ast.Inspect(notifyContext.Body, func(node ast.Node) bool {
		if _, ok := node.(*ast.CommClause); ok {
			commClauseCount++
		}
		return true
	})
	if commClauseCount != 2 {
		return fmt.Errorf("NotifyContext select must contain two communication clauses, got %d", commClauseCount)
	}
	return nil
}

func validateOsSignalImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 4 {
		return fmt.Errorf("first os/signal declaration must be the context/os/slices/sync import block")
	}
	if !importSpecPath(genDecl.Specs[0], "\"context\"") ||
		!importSpecPath(genDecl.Specs[1], "\"os\"") ||
		!importSpecPath(genDecl.Specs[2], "\"slices\"") ||
		!importSpecPath(genDecl.Specs[3], "\"sync\"") {
		return fmt.Errorf("os/signal import block must import context, os, slices, then sync")
	}
	return nil
}

func osSignalContextOsImportDecl(file *ast.File) ast.Decl {
	if len(file.Decls) == 0 {
		return nil
	}
	importDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || importDecl.Tok != token.IMPORT {
		return nil
	}
	specs := make([]ast.Spec, 0, 2)
	for _, spec := range importDecl.Specs {
		if importSpecPath(spec, "\"context\"") || importSpecPath(spec, "\"os\"") {
			specs = append(specs, spec)
		}
	}
	if len(specs) != 2 {
		return nil
	}
	return &ast.GenDecl{
		Doc:    importDecl.Doc,
		TokPos: importDecl.TokPos,
		Tok:    importDecl.Tok,
		Lparen: importDecl.Lparen,
		Specs:  specs,
		Rparen: importDecl.Rparen,
	}
}

func osSignalNotifyContextDecl(file *ast.File) *ast.FuncDecl {
	for _, decl := range file.Decls {
		funcDecl, ok := decl.(*ast.FuncDecl)
		if ok && funcDecl.Name != nil && funcDecl.Name.Name == "NotifyContext" {
			return funcDecl
		}
	}
	return nil
}

func notifyContextSignature(funcType *ast.FuncType) bool {
	if funcType == nil || funcType.Params == nil || len(funcType.Params.List) != 2 || funcType.Results == nil || len(funcType.Results.List) != 2 {
		return false
	}
	parent := funcType.Params.List[0]
	if len(parent.Names) != 1 || parent.Names[0].Name != "parent" || !selectorNamed(parent.Type, "context", "Context") {
		return false
	}
	signals := funcType.Params.List[1]
	ellipsis, ok := signals.Type.(*ast.Ellipsis)
	if !ok || len(signals.Names) != 1 || signals.Names[0].Name != "signals" || !selectorNamed(ellipsis.Elt, "os", "Signal") {
		return false
	}
	ctx := funcType.Results.List[0]
	if len(ctx.Names) != 1 || ctx.Names[0].Name != "ctx" || !selectorNamed(ctx.Type, "context", "Context") {
		return false
	}
	stop := funcType.Results.List[1]
	return len(stop.Names) == 1 && stop.Names[0].Name == "stop" && selectorNamed(stop.Type, "context", "CancelFunc")
}

func osSignalDeclares(file *ast.File, name string) bool {
	return goFileDeclares(file, name)
}

func goFileDeclares(file *ast.File, name string) bool {
	for _, decl := range file.Decls {
		switch decl := decl.(type) {
		case *ast.GenDecl:
			for _, spec := range decl.Specs {
				switch spec := spec.(type) {
				case *ast.TypeSpec:
					if spec.Name != nil && spec.Name.Name == name {
						return true
					}
				case *ast.ValueSpec:
					for _, declared := range spec.Names {
						if declared != nil && declared.Name == name {
							return true
						}
					}
				}
			}
		case *ast.FuncDecl:
			if decl.Name != nil && decl.Name.Name == name {
				return true
			}
		}
	}
	return false
}

func osSignalSyntaxConstructs(file *ast.File) map[string]bool {
	constructs := map[string]bool{}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.MapType:
			constructs["map_type"] = true
		case *ast.ChanType:
			constructs["chan_type"] = true
		case *ast.InterfaceType:
			constructs["interface_type"] = true
		case *ast.SelectStmt:
			constructs["select_stmt"] = true
		case *ast.TypeAssertExpr:
			constructs["type_assert_expr"] = true
		case *ast.GoStmt:
			constructs["go_stmt"] = true
		}
		return true
	})
	return constructs
}

func appendUleb128Signature(funcType *ast.FuncType) bool {
	if funcType == nil || funcType.Params == nil || len(funcType.Params.List) != 2 || funcType.Results == nil || len(funcType.Results.List) != 1 {
		return false
	}
	first := funcType.Params.List[0]
	second := funcType.Params.List[1]
	result := funcType.Results.List[0]
	return len(first.Names) == 1 &&
		first.Names[0].Name == "b" &&
		sliceArrayTypeNamed(first.Type, "byte") &&
		len(second.Names) == 1 &&
		second.Names[0].Name == "v" &&
		identNamed(second.Type, "uint") &&
		len(result.Names) == 0 &&
		sliceArrayTypeNamed(result.Type, "byte")
}

func assignDefineCFromUint8Mask(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], "c") {
		return false
	}
	call, ok := assign.Rhs[0].(*ast.CallExpr)
	return ok &&
		identNamed(call.Fun, "uint8") &&
		len(call.Args) == 1 &&
		binaryIdentInteger(call.Args[0], "v", token.AND, "0x7f")
}

func compoundAssignIdentInteger(stmt ast.Stmt, target string, op token.Token, value string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	return ok &&
		assign.Tok == op &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		identNamed(assign.Lhs[0], target) &&
		integerLiteral(assign.Rhs[0], value)
}

func ifCondBodyAssign(stmt ast.Stmt, condLeft string, condOp token.Token, condRight string, assignTarget string, assignOp token.Token, assignValue string) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Init != nil || ifStmt.Else != nil || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 {
		return false
	}
	return binaryIdentInteger(ifStmt.Cond, condLeft, condOp, condRight) &&
		compoundAssignIdentInteger(ifStmt.Body.List[0], assignTarget, assignOp, assignValue)
}

func assignBAppendBC(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], "b") {
		return false
	}
	call, ok := assign.Rhs[0].(*ast.CallExpr)
	return ok &&
		identNamed(call.Fun, "append") &&
		len(call.Args) == 2 &&
		identNamed(call.Args[0], "b") &&
		identNamed(call.Args[1], "c")
}

func ifBreakOnMaskedCZero(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Init != nil || ifStmt.Else != nil || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 {
		return false
	}
	cond, ok := ifStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.EQL || !integerLiteral(cond.Y, "0") {
		return false
	}
	mask, ok := cond.X.(*ast.BinaryExpr)
	if !ok || mask.Op != token.AND || !identNamed(mask.X, "c") || !integerLiteral(mask.Y, "0x80") {
		return false
	}
	branch, ok := ifStmt.Body.List[0].(*ast.BranchStmt)
	return ok && branch.Tok == token.BREAK && branch.Label == nil
}

func validateMinimalFunction(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("minimal fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("minimal fixture must contain one import declaration and one function declaration")
	}
	importDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || importDecl.Tok != token.IMPORT || len(importDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single import declaration")
	}
	importSpec, ok := importDecl.Specs[0].(*ast.ImportSpec)
	if !ok || importSpec.Path == nil || importSpec.Path.Value != "\"fmt\"" {
		return fmt.Errorf("import declaration must import fmt")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "hello" {
		return fmt.Errorf("second declaration must be func hello")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func hello must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 1 || len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func hello must have one parameter field and one result field")
	}
	param := funcDecl.Type.Params.List[0]
	if len(param.Names) != 1 || param.Names[0].Name != "name" || !identNamed(param.Type, "string") {
		return fmt.Errorf("func hello parameter must be name string")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func hello result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func hello body must contain one statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func hello body must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok || len(call.Args) != 2 {
		return fmt.Errorf("return expression must be a two-argument call")
	}
	selector, ok := call.Fun.(*ast.SelectorExpr)
	if !ok || !identNamed(selector.X, "fmt") || selector.Sel == nil || selector.Sel.Name != "Sprintf" {
		return fmt.Errorf("return call must target fmt.Sprintf")
	}
	literal, ok := call.Args[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"hello %s\"" {
		return fmt.Errorf("first fmt.Sprintf argument must be the hello format string")
	}
	if !identNamed(call.Args[1], "name") {
		return fmt.Errorf("second fmt.Sprintf argument must be name")
	}
	return nil
}

func validateLocalDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local declaration fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local declaration fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func build result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 2 {
		return fmt.Errorf("func build body must contain local declaration and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("local declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "name" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("local declaration must be var name = <string>")
	}
	literal, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"gopher\"" {
		return fmt.Errorf("local declaration value must be the gopher string literal")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "name") {
		return fmt.Errorf("second statement must return name")
	}
	return nil
}

func validateLocalAssignment(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local assignment fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local assignment fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func build result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 3 {
		return fmt.Errorf("func build body must contain local declaration, assignment, and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("local declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "name" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("local declaration must be var name = <string>")
	}
	literal, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"gopher\"" {
		return fmt.Errorf("local declaration value must be the gopher string literal")
	}
	assignStmt, ok := funcDecl.Body.List[1].(*ast.AssignStmt)
	if !ok {
		return fmt.Errorf("second statement must be an assignment statement")
	}
	if assignStmt.Tok != token.ASSIGN || len(assignStmt.Lhs) != 1 || len(assignStmt.Rhs) != 1 {
		return fmt.Errorf("assignment must use one target and one value with =")
	}
	if !identNamed(assignStmt.Lhs[0], "name") {
		return fmt.Errorf("assignment target must be name")
	}
	assignedLiteral, ok := assignStmt.Rhs[0].(*ast.BasicLit)
	if !ok || assignedLiteral.Kind != token.STRING || assignedLiteral.Value != "\"tct\"" {
		return fmt.Errorf("assignment value must be the tct string literal")
	}
	returnStmt, ok := funcDecl.Body.List[2].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "name") {
		return fmt.Errorf("third statement must return name")
	}
	return nil
}

func validateLocalEscapedAssignment(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local escaped assignment fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local escaped assignment fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func build result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 3 {
		return fmt.Errorf("func build body must contain local declaration, assignment, and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("local declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "name" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("local declaration must be var name = <string>")
	}
	literal, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"go\\\"pher\"" {
		return fmt.Errorf("local declaration value must be the escaped quote string literal")
	}
	assignStmt, ok := funcDecl.Body.List[1].(*ast.AssignStmt)
	if !ok {
		return fmt.Errorf("second statement must be an assignment statement")
	}
	if assignStmt.Tok != token.ASSIGN || len(assignStmt.Lhs) != 1 || len(assignStmt.Rhs) != 1 {
		return fmt.Errorf("assignment must use one target and one value with =")
	}
	if !identNamed(assignStmt.Lhs[0], "name") {
		return fmt.Errorf("assignment target must be name")
	}
	assignedLiteral, ok := assignStmt.Rhs[0].(*ast.BasicLit)
	if !ok || assignedLiteral.Kind != token.STRING || assignedLiteral.Value != "\"tc\\\\t\"" {
		return fmt.Errorf("assignment value must be the escaped backslash string literal")
	}
	returnStmt, ok := funcDecl.Body.List[2].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "name") {
		return fmt.Errorf("third statement must return name")
	}
	return nil
}

func validateLocalShortVarDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local short variable declaration fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local short variable declaration fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func build result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 2 {
		return fmt.Errorf("func build body must contain short variable declaration and return")
	}
	assignStmt, ok := funcDecl.Body.List[0].(*ast.AssignStmt)
	if !ok {
		return fmt.Errorf("first statement must be a short variable declaration")
	}
	if assignStmt.Tok != token.DEFINE || len(assignStmt.Lhs) != 1 || len(assignStmt.Rhs) != 1 {
		return fmt.Errorf("short variable declaration must use one target and one value with :=")
	}
	if !identNamed(assignStmt.Lhs[0], "name") {
		return fmt.Errorf("short variable declaration target must be name")
	}
	literal, ok := assignStmt.Rhs[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"tct\"" {
		return fmt.Errorf("short variable declaration value must be the tct string literal")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "name") {
		return fmt.Errorf("second statement must return name")
	}
	return nil
}

func validateLocalIntegerShortVarDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local integer short variable declaration fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local integer short variable declaration fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 2 {
		return fmt.Errorf("func build body must contain integer short variable declaration and return")
	}
	assignStmt, ok := funcDecl.Body.List[0].(*ast.AssignStmt)
	if !ok {
		return fmt.Errorf("first statement must be a short variable declaration")
	}
	if assignStmt.Tok != token.DEFINE || len(assignStmt.Lhs) != 1 || len(assignStmt.Rhs) != 1 {
		return fmt.Errorf("short variable declaration must use one target and one value with :=")
	}
	if !identNamed(assignStmt.Lhs[0], "count") {
		return fmt.Errorf("short variable declaration target must be count")
	}
	literal, ok := assignStmt.Rhs[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("short variable declaration value must be the integer literal 7")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "count") {
		return fmt.Errorf("second statement must return count")
	}
	return nil
}

func validateLocalIntegerReturnLiteral(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local integer return literal fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local integer return literal fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("return value must be the integer literal 7")
	}
	return nil
}

func validateLocalParenthesizedIntegerReturnExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local parenthesized integer return expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local parenthesized integer return expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	paren, ok := returnStmt.Results[0].(*ast.ParenExpr)
	if !ok {
		return fmt.Errorf("return value must be a parenthesized expression")
	}
	literal, ok := paren.X.(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("parenthesized return value must wrap the integer literal 7")
	}
	return nil
}

func validateLocalUnaryIntegerReturnExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local unary integer return expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local unary integer return expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	unary, ok := returnStmt.Results[0].(*ast.UnaryExpr)
	if !ok {
		return fmt.Errorf("return value must be a unary expression")
	}
	if unary.Op != token.SUB {
		return fmt.Errorf("unary return expression must use the - operator")
	}
	literal, ok := unary.X.(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("unary return value must wrap the integer literal 7")
	}
	return nil
}

func validateLocalBinaryIntegerReturnExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local binary integer return expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local binary integer return expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	binary, ok := returnStmt.Results[0].(*ast.BinaryExpr)
	if !ok {
		return fmt.Errorf("return value must be a binary expression")
	}
	if binary.Op != token.ADD {
		return fmt.Errorf("binary return expression must use the + operator")
	}
	left, ok := binary.X.(*ast.BasicLit)
	if !ok || left.Kind != token.INT || left.Value != "7" {
		return fmt.Errorf("binary return expression left operand must be the integer literal 7")
	}
	right, ok := binary.Y.(*ast.BasicLit)
	if !ok || right.Kind != token.INT || right.Value != "5" {
		return fmt.Errorf("binary return expression right operand must be the integer literal 5")
	}
	return nil
}

func validateLocalCallReturnExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local call return expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local call return expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "any") {
		return fmt.Errorf("func build result must be unnamed any")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok {
		return fmt.Errorf("return value must be a call expression")
	}
	if len(call.Args) != 0 {
		return fmt.Errorf("call return expression must not have arguments")
	}
	if !identNamed(call.Fun, "recover") {
		return fmt.Errorf("call return expression function must be recover")
	}
	return nil
}

func validateLocalArgumentCallReturnExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local argument call return expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local argument call return expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "any") {
		return fmt.Errorf("func build result must be unnamed any")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok {
		return fmt.Errorf("return value must be a call expression")
	}
	if !identNamed(call.Fun, "len") {
		return fmt.Errorf("argument call return expression function must be len")
	}
	if len(call.Args) != 1 {
		return fmt.Errorf("argument call return expression must have one argument")
	}
	arg, ok := call.Args[0].(*ast.BasicLit)
	if !ok || arg.Kind != token.STRING || arg.Value != `"world"` {
		return fmt.Errorf("argument call return expression argument must be the string literal %q", `"world"`)
	}
	return nil
}

func validateLocalStringReturnLiteral(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local string return literal fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local string return literal fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func build result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"ok\"" {
		return fmt.Errorf("return value must be the string literal \"ok\"")
	}
	return nil
}

func validateLocalFloatReturnLiteral(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local float return literal fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local float return literal fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "float64") {
		return fmt.Errorf("func build result must be unnamed float64")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.FLOAT || literal.Value != "3.5" {
		return fmt.Errorf("return value must be the float literal 3.5")
	}
	return nil
}

func validateLocalBooleanReturnIdentifier(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local boolean return identifier fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local boolean return identifier fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "bool") {
		return fmt.Errorf("func build result must be unnamed bool")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	ident, ok := returnStmt.Results[0].(*ast.Ident)
	if !ok || ident.Name != "true" {
		return fmt.Errorf("return value must be the identifier true")
	}
	return nil
}

func validateLocalNilReturnIdentifier(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local nil return identifier fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local nil return identifier fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "any") {
		return fmt.Errorf("func build result must be unnamed any")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("return statement must return one expression")
	}
	ident, ok := returnStmt.Results[0].(*ast.Ident)
	if !ok || ident.Name != "nil" {
		return fmt.Errorf("return value must be the identifier nil")
	}
	return nil
}

func validateLocalTypedDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local typed declaration fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local typed declaration fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func build result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 2 {
		return fmt.Errorf("func build body must contain typed local declaration and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("typed local declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "name" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("typed local declaration must be var name string = <string>")
	}
	if !identNamed(valueSpec.Type, "string") {
		return fmt.Errorf("typed local declaration type must be string")
	}
	literal, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"tct\"" {
		return fmt.Errorf("typed local declaration value must be the tct string literal")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "name") {
		return fmt.Errorf("second statement must return name")
	}
	return nil
}

func validateLocalZeroValueTypedDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local zero-value typed declaration fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local zero-value typed declaration fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func build result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 2 {
		return fmt.Errorf("func build body must contain zero-value typed local declaration and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("zero-value typed local declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "name" || len(valueSpec.Values) != 0 {
		return fmt.Errorf("zero-value typed local declaration must be var name string")
	}
	if !identNamed(valueSpec.Type, "string") {
		return fmt.Errorf("zero-value typed local declaration type must be string")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "name") {
		return fmt.Errorf("second statement must return name")
	}
	return nil
}

func validateLocalIntegerTypedDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local integer typed declaration fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local integer typed declaration fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 2 {
		return fmt.Errorf("func build body must contain integer typed local declaration and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("integer typed local declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "count" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("integer typed local declaration must be var count int = <integer>")
	}
	if !identNamed(valueSpec.Type, "int") {
		return fmt.Errorf("integer typed local declaration type must be int")
	}
	literal, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("integer typed local declaration value must be the integer literal 7")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "count") {
		return fmt.Errorf("second statement must return count")
	}
	return nil
}

func validateLocalUntypedIntegerDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local untyped integer declaration fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local untyped integer declaration fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 2 {
		return fmt.Errorf("func build body must contain untyped integer local declaration and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("untyped integer local declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "count" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("untyped integer local declaration must be var count = <integer>")
	}
	if valueSpec.Type != nil {
		return fmt.Errorf("untyped integer local declaration must omit ValueSpec.Type")
	}
	literal, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("untyped integer local declaration value must be the integer literal 7")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "count") {
		return fmt.Errorf("second statement must return count")
	}
	return nil
}

func validateLocalIntegerAssignment(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local integer assignment fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local integer assignment fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 3 {
		return fmt.Errorf("func build body must contain local integer declaration, assignment, and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("local integer declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "count" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("local integer declaration must be var count = <integer>")
	}
	if valueSpec.Type != nil {
		return fmt.Errorf("local integer declaration must omit ValueSpec.Type")
	}
	initialLiteral, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || initialLiteral.Kind != token.INT || initialLiteral.Value != "1" {
		return fmt.Errorf("local integer declaration value must be the integer literal 1")
	}
	assignStmt, ok := funcDecl.Body.List[1].(*ast.AssignStmt)
	if !ok {
		return fmt.Errorf("second statement must be an assignment statement")
	}
	if assignStmt.Tok != token.ASSIGN || len(assignStmt.Lhs) != 1 || len(assignStmt.Rhs) != 1 {
		return fmt.Errorf("integer assignment must use one target and one value with =")
	}
	if !identNamed(assignStmt.Lhs[0], "count") {
		return fmt.Errorf("integer assignment target must be count")
	}
	assignedLiteral, ok := assignStmt.Rhs[0].(*ast.BasicLit)
	if !ok || assignedLiteral.Kind != token.INT || assignedLiteral.Value != "7" {
		return fmt.Errorf("integer assignment value must be the integer literal 7")
	}
	returnStmt, ok := funcDecl.Body.List[2].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "count") {
		return fmt.Errorf("third statement must return count")
	}
	return nil
}

func validateLocalCompoundAssignment(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local compound assignment fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local compound assignment fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 3 {
		return fmt.Errorf("func build body must contain local integer declaration, compound assignment, and return")
	}
	declStmt, ok := funcDecl.Body.List[0].(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("first statement must be a declaration statement")
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("local integer declaration must be a single var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "count" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("local integer declaration must be var count = <integer>")
	}
	if valueSpec.Type != nil {
		return fmt.Errorf("local integer declaration must omit ValueSpec.Type")
	}
	initialLiteral, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || initialLiteral.Kind != token.INT || initialLiteral.Value != "1" {
		return fmt.Errorf("local integer declaration value must be the integer literal 1")
	}
	assignStmt, ok := funcDecl.Body.List[1].(*ast.AssignStmt)
	if !ok {
		return fmt.Errorf("second statement must be a compound assignment statement")
	}
	if assignStmt.Tok != token.ADD_ASSIGN || len(assignStmt.Lhs) != 1 || len(assignStmt.Rhs) != 1 {
		return fmt.Errorf("compound assignment must use one target and one value with +=")
	}
	if !identNamed(assignStmt.Lhs[0], "count") {
		return fmt.Errorf("compound assignment target must be count")
	}
	assignedLiteral, ok := assignStmt.Rhs[0].(*ast.BasicLit)
	if !ok || assignedLiteral.Kind != token.INT || assignedLiteral.Value != "7" {
		return fmt.Errorf("compound assignment value must be the integer literal 7")
	}
	returnStmt, ok := funcDecl.Body.List[2].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "count") {
		return fmt.Errorf("third statement must return count")
	}
	return nil
}

func validateLocalIncrementDecrement(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local increment/decrement fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local increment/decrement fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 3 {
		return fmt.Errorf("func build body must contain local integer declaration, increment/decrement, and return")
	}
	if err := validateInitialCountDeclaration(funcDecl.Body.List[0], "local increment/decrement"); err != nil {
		return err
	}
	incDecStmt, ok := funcDecl.Body.List[1].(*ast.IncDecStmt)
	if !ok {
		return fmt.Errorf("second statement must be an increment/decrement statement")
	}
	if incDecStmt.Tok != token.INC || !identNamed(incDecStmt.X, "count") {
		return fmt.Errorf("increment/decrement statement must increment count with ++")
	}
	returnStmt, ok := funcDecl.Body.List[2].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "count") {
		return fmt.Errorf("third statement must return count")
	}
	return nil
}

func validateLocalBranchStatement(file *ast.File) error {
	funcDecl, err := validateIntegerBuildFunction(file, "local branch statement", 2)
	if err != nil {
		return err
	}
	forStmt, ok := funcDecl.Body.List[0].(*ast.ForStmt)
	if !ok {
		return fmt.Errorf("first statement must be a for statement")
	}
	if forStmt.Init != nil || forStmt.Cond != nil || forStmt.Post != nil {
		return fmt.Errorf("branch fixture for statement must omit init, condition, and post")
	}
	if forStmt.Body == nil || len(forStmt.Body.List) != 1 {
		return fmt.Errorf("for body must contain one branch statement")
	}
	branchStmt, ok := forStmt.Body.List[0].(*ast.BranchStmt)
	if !ok {
		return fmt.Errorf("for body must contain a branch statement")
	}
	if branchStmt.Tok != token.BREAK || branchStmt.Label != nil {
		return fmt.Errorf("branch statement must be an unlabeled break")
	}
	returnStmt, ok := funcDecl.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("second statement must return one integer literal")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "1" {
		return fmt.Errorf("second statement must return the integer literal 1")
	}
	return nil
}

func validateLocalIfElse(file *ast.File) error {
	funcDecl, err := validateIntegerBuildFunction(file, "local if/else", 3)
	if err != nil {
		return err
	}
	if err := validateInitialCountDeclaration(funcDecl.Body.List[0], "local if/else"); err != nil {
		return err
	}
	ifStmt, ok := funcDecl.Body.List[1].(*ast.IfStmt)
	if !ok {
		return fmt.Errorf("second statement must be an if statement")
	}
	if ifStmt.Init != nil {
		return fmt.Errorf("if statement must not use an init statement")
	}
	if err := validateCountIntegerCondition(ifStmt.Cond, token.GTR, "0", "if condition"); err != nil {
		return err
	}
	if ifStmt.Body == nil || len(ifStmt.Body.List) != 1 {
		return fmt.Errorf("if body must contain one assignment")
	}
	if err := validateCountIntegerAssignment(ifStmt.Body.List[0], "7", "if body"); err != nil {
		return err
	}
	elseBlock, ok := ifStmt.Else.(*ast.BlockStmt)
	if !ok || len(elseBlock.List) != 1 {
		return fmt.Errorf("if statement must have a single-statement else block")
	}
	if err := validateCountIntegerAssignment(elseBlock.List[0], "0", "else body"); err != nil {
		return err
	}
	if err := validateCountReturn(funcDecl.Body.List[2], "third statement"); err != nil {
		return err
	}
	return nil
}

func validateLocalForCondition(file *ast.File) error {
	funcDecl, err := validateIntegerBuildFunction(file, "local condition-only for", 3)
	if err != nil {
		return err
	}
	if err := validateInitialCountDeclaration(funcDecl.Body.List[0], "local condition-only for"); err != nil {
		return err
	}
	forStmt, ok := funcDecl.Body.List[1].(*ast.ForStmt)
	if !ok {
		return fmt.Errorf("second statement must be a condition-only for statement")
	}
	if forStmt.Init != nil || forStmt.Post != nil {
		return fmt.Errorf("for statement must omit init and post statements")
	}
	if err := validateCountIntegerCondition(forStmt.Cond, token.LSS, "7", "for condition"); err != nil {
		return err
	}
	if forStmt.Body == nil || len(forStmt.Body.List) != 1 {
		return fmt.Errorf("for body must contain one assignment")
	}
	if err := validateCountIntegerAssignment(forStmt.Body.List[0], "7", "for body"); err != nil {
		return err
	}
	if err := validateCountReturn(funcDecl.Body.List[2], "third statement"); err != nil {
		return err
	}
	return nil
}

func validateLocalSwitch(file *ast.File) error {
	funcDecl, err := validateIntegerBuildFunction(file, "local switch", 3)
	if err != nil {
		return err
	}
	if err := validateInitialCountDeclaration(funcDecl.Body.List[0], "local switch"); err != nil {
		return err
	}
	switchStmt, ok := funcDecl.Body.List[1].(*ast.SwitchStmt)
	if !ok {
		return fmt.Errorf("second statement must be a switch statement")
	}
	if switchStmt.Init != nil {
		return fmt.Errorf("switch statement must not use an init statement")
	}
	if !identNamed(switchStmt.Tag, "count") {
		return fmt.Errorf("switch tag must be count")
	}
	if switchStmt.Body == nil || len(switchStmt.Body.List) != 1 {
		return fmt.Errorf("switch body must contain one case clause")
	}
	caseClause, ok := switchStmt.Body.List[0].(*ast.CaseClause)
	if !ok || len(caseClause.List) != 1 || len(caseClause.Body) != 1 {
		return fmt.Errorf("switch body must contain one single-value case with one assignment")
	}
	caseLiteral, ok := caseClause.List[0].(*ast.BasicLit)
	if !ok || caseLiteral.Kind != token.INT || caseLiteral.Value != "1" {
		return fmt.Errorf("switch case must match integer literal 1")
	}
	if err := validateCountIntegerAssignment(caseClause.Body[0], "7", "switch case"); err != nil {
		return err
	}
	if err := validateCountReturn(funcDecl.Body.List[2], "third statement"); err != nil {
		return err
	}
	return nil
}

func validateLocalSwitchInit(file *ast.File) error {
	funcDecl, err := validateIntegerBuildFunction(file, "local switch init", 3)
	if err != nil {
		return err
	}
	if err := validateInitialCountDeclaration(funcDecl.Body.List[0], "local switch init"); err != nil {
		return err
	}
	switchStmt, ok := funcDecl.Body.List[1].(*ast.SwitchStmt)
	if !ok {
		return fmt.Errorf("second statement must be a switch statement")
	}
	if err := validateCountIntegerAssignment(switchStmt.Init, "2", "switch init"); err != nil {
		return err
	}
	if !identNamed(switchStmt.Tag, "count") {
		return fmt.Errorf("switch tag must be count")
	}
	if switchStmt.Body == nil || len(switchStmt.Body.List) != 1 {
		return fmt.Errorf("switch body must contain one case clause")
	}
	caseClause, ok := switchStmt.Body.List[0].(*ast.CaseClause)
	if !ok || len(caseClause.List) != 1 || len(caseClause.Body) != 1 {
		return fmt.Errorf("switch body must contain one single-value case with one assignment")
	}
	caseLiteral, ok := caseClause.List[0].(*ast.BasicLit)
	if !ok || caseLiteral.Kind != token.INT || caseLiteral.Value != "2" {
		return fmt.Errorf("switch case must match integer literal 2")
	}
	if err := validateCountIntegerAssignment(caseClause.Body[0], "7", "switch case"); err != nil {
		return err
	}
	if err := validateCountReturn(funcDecl.Body.List[2], "third statement"); err != nil {
		return err
	}
	return nil
}

func validateLocalTypeSwitch(file *ast.File) error {
	funcDecl, err := validateIntegerBuildFunctionWithValueParam(file, "local type switch", 3)
	if err != nil {
		return err
	}
	if err := validateInitialCountDeclaration(funcDecl.Body.List[0], "local type switch"); err != nil {
		return err
	}
	typeSwitchStmt, ok := funcDecl.Body.List[1].(*ast.TypeSwitchStmt)
	if !ok {
		return fmt.Errorf("second statement must be a type switch statement")
	}
	if typeSwitchStmt.Init != nil {
		return fmt.Errorf("type switch statement must not use an init statement")
	}
	exprStmt, ok := typeSwitchStmt.Assign.(*ast.ExprStmt)
	if !ok {
		return fmt.Errorf("type switch assign must be an expression statement")
	}
	typeAssert, ok := exprStmt.X.(*ast.TypeAssertExpr)
	if !ok || !identNamed(typeAssert.X, "value") || typeAssert.Type != nil {
		return fmt.Errorf("type switch guard must be value.(type)")
	}
	if typeSwitchStmt.Body == nil || len(typeSwitchStmt.Body.List) != 1 {
		return fmt.Errorf("type switch body must contain one case clause")
	}
	caseClause, ok := typeSwitchStmt.Body.List[0].(*ast.CaseClause)
	if !ok || len(caseClause.List) != 1 || len(caseClause.Body) != 1 {
		return fmt.Errorf("type switch body must contain one single-type case with one assignment")
	}
	if !identNamed(caseClause.List[0], "int") {
		return fmt.Errorf("type switch case must match int")
	}
	if err := validateCountIntegerAssignment(caseClause.Body[0], "7", "type switch case"); err != nil {
		return err
	}
	if err := validateCountReturn(funcDecl.Body.List[2], "third statement"); err != nil {
		return err
	}
	return nil
}

func validateLocalRangeString(file *ast.File) error {
	funcDecl, err := validateIntegerBuildFunction(file, "local string range", 3)
	if err != nil {
		return err
	}
	if err := validateInitialCountDeclaration(funcDecl.Body.List[0], "local string range"); err != nil {
		return err
	}
	rangeStmt, ok := funcDecl.Body.List[1].(*ast.RangeStmt)
	if !ok {
		return fmt.Errorf("second statement must be a range statement")
	}
	if rangeStmt.Key != nil || rangeStmt.Value != nil || rangeStmt.Tok != token.ILLEGAL {
		return fmt.Errorf("range statement must omit key, value, and assignment token")
	}
	rangeLiteral, ok := rangeStmt.X.(*ast.BasicLit)
	if !ok || rangeLiteral.Kind != token.STRING || rangeLiteral.Value != "\"go\"" {
		return fmt.Errorf("range expression must be string literal \"go\"")
	}
	if rangeStmt.Body == nil || len(rangeStmt.Body.List) != 1 {
		return fmt.Errorf("range body must contain one assignment")
	}
	if err := validateCountIntegerAssignment(rangeStmt.Body.List[0], "7", "range body"); err != nil {
		return err
	}
	if err := validateCountReturn(funcDecl.Body.List[2], "third statement"); err != nil {
		return err
	}
	return nil
}

func validatePackageTypeDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("package type declaration fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("package type declaration fixture must contain one type declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level type declaration")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "Counter" || typeSpec.Assign.IsValid() {
		return fmt.Errorf("type declaration must be defined type Counter")
	}
	if !identNamed(typeSpec.Type, "int") {
		return fmt.Errorf("type declaration underlying type must be int")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("second declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func build body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("func build must return integer literal 7")
	}
	return nil
}

func validateTypeSpecAssign(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("type spec assign fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("type spec assign fixture must contain one type alias declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level type alias")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "Counter" || !typeSpec.Assign.IsValid() {
		return fmt.Errorf("type declaration must be alias type Counter = int")
	}
	if !identNamed(typeSpec.Type, "int") {
		return fmt.Errorf("type alias underlying type must be int")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("second declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func build body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("func build must return integer literal 7")
	}
	return nil
}

func validateTypeSpecTypeParams(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("type spec type params fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("type spec type params fixture must contain one type declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level generic type declaration")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "Box" {
		return fmt.Errorf("type declaration must name Box")
	}
	if typeSpec.Assign.IsValid() {
		return fmt.Errorf("generic type declaration must be a type definition, not an alias")
	}
	if typeSpec.TypeParams == nil || len(typeSpec.TypeParams.List) != 1 {
		return fmt.Errorf("generic type declaration must have one type parameter field")
	}
	typeParam := typeSpec.TypeParams.List[0]
	if len(typeParam.Names) != 1 || typeParam.Names[0].Name != "T" || !identNamed(typeParam.Type, "any") {
		return fmt.Errorf("generic type declaration type parameter must be [T any]")
	}
	structType, ok := typeSpec.Type.(*ast.StructType)
	if !ok || structType.Fields == nil || len(structType.Fields.List) != 1 {
		return fmt.Errorf("generic type declaration must use a one-field struct type")
	}
	field := structType.Fields.List[0]
	if len(field.Names) != 1 || field.Names[0].Name != "Value" || !identNamed(field.Type, "T") {
		return fmt.Errorf("generic type declaration field must be Value T")
	}
	return nil
}

func validateTypeSpecComment(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("type spec comment fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("type spec comment fixture must contain one type declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level type declaration")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "namedAnswer" {
		return fmt.Errorf("type declaration must name namedAnswer")
	}
	if typeSpec.Assign.IsValid() {
		return fmt.Errorf("type spec comment fixture must use a type definition, not an alias")
	}
	if !identNamed(typeSpec.Type, "int") {
		return fmt.Errorf("type declaration underlying type must be int")
	}
	if !commentGroupTextEquals(typeSpec.Comment, []string{"// retained type spec comment"}) {
		return fmt.Errorf("type declaration must attach trailing comment to TypeSpec.Comment")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "read" {
		return fmt.Errorf("second declaration must be func read")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func read must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func read must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func read must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "namedAnswer") {
		return fmt.Errorf("func read result must be unnamed namedAnswer")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func read body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func read body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "42" {
		return fmt.Errorf("func read must return integer literal 42")
	}
	return nil
}

func validateTypeSpecDoc(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("type spec doc fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("type spec doc fixture must contain one grouped type declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a grouped package-level type declaration")
	}
	if genDecl.Doc != nil {
		return fmt.Errorf("type spec doc fixture must not attach the comment to GenDecl.Doc")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "namedAnswer" {
		return fmt.Errorf("type declaration must name namedAnswer")
	}
	if typeSpec.Assign.IsValid() {
		return fmt.Errorf("type spec doc fixture must use a type definition, not an alias")
	}
	if !identNamed(typeSpec.Type, "int") {
		return fmt.Errorf("type declaration underlying type must be int")
	}
	if !commentGroupTextEquals(typeSpec.Doc, []string{"// retained type spec doc"}) {
		return fmt.Errorf("type declaration must attach leading comment to TypeSpec.Doc")
	}
	if typeSpec.Comment != nil {
		return fmt.Errorf("type spec doc fixture must not attach a trailing TypeSpec.Comment")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "read" {
		return fmt.Errorf("second declaration must be func read")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func read must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func read must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func read must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "namedAnswer") {
		return fmt.Errorf("func read result must be unnamed namedAnswer")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func read body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func read body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "42" {
		return fmt.Errorf("func read must return integer literal 42")
	}
	return nil
}

func validateMethodDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("method declaration fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("method declaration fixture must contain one type declaration and one method declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level type declaration")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "Counter" || typeSpec.Assign.IsValid() {
		return fmt.Errorf("type declaration must be defined type Counter")
	}
	if !identNamed(typeSpec.Type, "int") {
		return fmt.Errorf("type declaration underlying type must be int")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "value" {
		return fmt.Errorf("second declaration must be method value")
	}
	if funcDecl.Recv == nil || len(funcDecl.Recv.List) != 1 {
		return fmt.Errorf("method value must have one receiver field")
	}
	receiver := funcDecl.Recv.List[0]
	if len(receiver.Names) != 1 || receiver.Names[0].Name != "c" || !identNamed(receiver.Type, "Counter") {
		return fmt.Errorf("method receiver must be c Counter")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("method value must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("method value must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("method value must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("method value result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("method value body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("method value body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("method value must return integer literal 7")
	}
	return nil
}

func validateStructType(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("struct type fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("struct type fixture must contain one type declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level type declaration")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "Counter" || typeSpec.Assign.IsValid() {
		return fmt.Errorf("type declaration must be defined type Counter")
	}
	structType, ok := typeSpec.Type.(*ast.StructType)
	if !ok || structType.Fields == nil || len(structType.Fields.List) != 1 {
		return fmt.Errorf("type declaration underlying type must be a one-field struct")
	}
	field := structType.Fields.List[0]
	if len(field.Names) != 1 || field.Names[0].Name != "Value" || !identNamed(field.Type, "int") {
		return fmt.Errorf("struct field must be Value int")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("second declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func build body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("func build must return integer literal 7")
	}
	return nil
}

func validateCompositeLiteral(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("composite literal fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("composite literal fixture must contain one type declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level type declaration")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "Counter" || typeSpec.Assign.IsValid() {
		return fmt.Errorf("type declaration must be defined type Counter")
	}
	structType, ok := typeSpec.Type.(*ast.StructType)
	if !ok || structType.Fields == nil || len(structType.Fields.List) != 1 {
		return fmt.Errorf("type declaration underlying type must be a one-field struct")
	}
	field := structType.Fields.List[0]
	if len(field.Names) != 1 || field.Names[0].Name != "Value" || !identNamed(field.Type, "int") {
		return fmt.Errorf("struct field must be Value int")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("second declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "Counter") {
		return fmt.Errorf("func build result must be unnamed Counter")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func build body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.CompositeLit)
	if !ok || !identNamed(literal.Type, "Counter") || literal.Incomplete {
		return fmt.Errorf("func build must return a complete Counter composite literal")
	}
	if len(literal.Elts) != 1 {
		return fmt.Errorf("Counter literal must contain one element")
	}
	element, ok := literal.Elts[0].(*ast.BasicLit)
	if !ok || element.Kind != token.INT || element.Value != "7" {
		return fmt.Errorf("Counter literal element must be integer literal 7")
	}
	return nil
}

func validateKeyValueCompositeLiteral(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("key/value composite literal fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("key/value composite literal fixture must contain one type declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level type declaration")
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != "Counter" || typeSpec.Assign.IsValid() {
		return fmt.Errorf("type declaration must be defined type Counter")
	}
	structType, ok := typeSpec.Type.(*ast.StructType)
	if !ok || structType.Fields == nil || len(structType.Fields.List) != 1 {
		return fmt.Errorf("type declaration underlying type must be a one-field struct")
	}
	field := structType.Fields.List[0]
	if len(field.Names) != 1 || field.Names[0].Name != "Value" || !identNamed(field.Type, "int") {
		return fmt.Errorf("struct field must be Value int")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("second declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "Counter") {
		return fmt.Errorf("func build result must be unnamed Counter")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func build body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.CompositeLit)
	if !ok || !identNamed(literal.Type, "Counter") || literal.Incomplete {
		return fmt.Errorf("func build must return a complete Counter composite literal")
	}
	if len(literal.Elts) != 1 {
		return fmt.Errorf("Counter literal must contain one keyed element")
	}
	element, ok := literal.Elts[0].(*ast.KeyValueExpr)
	if !ok {
		return fmt.Errorf("Counter literal element must be a key/value expression")
	}
	if !identNamed(element.Key, "Value") {
		return fmt.Errorf("Counter literal key must be Value")
	}
	value, ok := element.Value.(*ast.BasicLit)
	if !ok || value.Kind != token.INT || value.Value != "7" {
		return fmt.Errorf("Counter literal value must be integer literal 7")
	}
	return nil
}

func validateArrayCompositeLiteral(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("array composite literal fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("array composite literal fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !sliceArrayTypeNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed []int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func build body must return one expression")
	}
	literal, ok := returnStmt.Results[0].(*ast.CompositeLit)
	if !ok || !sliceArrayTypeNamed(literal.Type, "int") || literal.Incomplete {
		return fmt.Errorf("func build must return a complete []int composite literal")
	}
	if len(literal.Elts) != 1 {
		return fmt.Errorf("[]int literal must contain one element")
	}
	element, ok := literal.Elts[0].(*ast.BasicLit)
	if !ok || element.Kind != token.INT || element.Value != "7" {
		return fmt.Errorf("[]int literal element must be integer literal 7")
	}
	return nil
}

func validateIndexExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("index expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("index expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "pick" {
		return fmt.Errorf("declaration must be func pick")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func pick must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 1 {
		return fmt.Errorf("func pick must have one parameter")
	}
	param := funcDecl.Type.Params.List[0]
	if len(param.Names) != 1 || param.Names[0].Name != "values" || !sliceArrayTypeNamed(param.Type, "int") {
		return fmt.Errorf("func pick parameter must be values []int")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func pick must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func pick result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func pick body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func pick body must return one expression")
	}
	indexExpr, ok := returnStmt.Results[0].(*ast.IndexExpr)
	if !ok {
		return fmt.Errorf("func pick must return an index expression")
	}
	if !identNamed(indexExpr.X, "values") {
		return fmt.Errorf("index expression target must be values")
	}
	index, ok := indexExpr.Index.(*ast.BasicLit)
	if !ok || index.Kind != token.INT || index.Value != "0" {
		return fmt.Errorf("index expression must use integer literal 0")
	}
	return nil
}

func validateIndexListExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("index list expression fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("index list expression fixture must contain two function declarations")
	}
	makePair, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || makePair.Name == nil || makePair.Name.Name != "makePair" {
		return fmt.Errorf("first declaration must be func makePair")
	}
	if makePair.Type == nil || makePair.Type.TypeParams == nil || makePair.Type.Params == nil || makePair.Type.Results == nil || makePair.Body == nil {
		return fmt.Errorf("func makePair must have type params, params, results, and body")
	}
	if len(makePair.Type.TypeParams.List) != 1 {
		return fmt.Errorf("func makePair must have one type parameter field")
	}
	typeParams := makePair.Type.TypeParams.List[0]
	if !fieldNamesType(typeParams, []string{"A", "B"}, "any") {
		return fmt.Errorf("func makePair type parameter field must be A, B any")
	}
	if len(makePair.Type.Params.List) != 2 {
		return fmt.Errorf("func makePair must have two value parameters")
	}
	if !fieldNamesType(makePair.Type.Params.List[0], []string{"a"}, "A") || !fieldNamesType(makePair.Type.Params.List[1], []string{"b"}, "B") {
		return fmt.Errorf("func makePair parameters must be a A, b B")
	}
	if len(makePair.Type.Results.List) != 1 || !identNamed(makePair.Type.Results.List[0].Type, "string") {
		return fmt.Errorf("func makePair result must be string")
	}
	if len(makePair.Body.List) != 1 {
		return fmt.Errorf("func makePair body must contain one return statement")
	}
	makePairReturn, ok := makePair.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(makePairReturn.Results) != 1 {
		return fmt.Errorf("func makePair must return one expression")
	}
	pairLiteral, ok := makePairReturn.Results[0].(*ast.BasicLit)
	if !ok || pairLiteral.Kind != token.STRING || pairLiteral.Value != strconv.Quote("pair") {
		return fmt.Errorf("func makePair must return string literal pair")
	}

	use, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || use.Name == nil || use.Name.Name != "use" {
		return fmt.Errorf("second declaration must be func use")
	}
	if use.Type == nil || use.Type.Params == nil || use.Type.Results == nil || use.Body == nil {
		return fmt.Errorf("func use must have params, results, and body")
	}
	if len(use.Type.Params.List) != 0 {
		return fmt.Errorf("func use must not have parameters")
	}
	if len(use.Type.Results.List) != 1 || !identNamed(use.Type.Results.List[0].Type, "string") {
		return fmt.Errorf("func use result must be string")
	}
	if len(use.Body.List) != 1 {
		return fmt.Errorf("func use body must contain one return statement")
	}
	useReturn, ok := use.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(useReturn.Results) != 1 {
		return fmt.Errorf("func use must return one expression")
	}
	call, ok := useReturn.Results[0].(*ast.CallExpr)
	if !ok {
		return fmt.Errorf("func use must return a call expression")
	}
	indexList, ok := call.Fun.(*ast.IndexListExpr)
	if !ok {
		return fmt.Errorf("func use call target must be an index list expression")
	}
	if !identNamed(indexList.X, "makePair") {
		return fmt.Errorf("index list target must be makePair")
	}
	if len(indexList.Indices) != 2 || !identNamed(indexList.Indices[0], "int") || !identNamed(indexList.Indices[1], "string") {
		return fmt.Errorf("index list must instantiate [int, string]")
	}
	if len(call.Args) != 2 {
		return fmt.Errorf("makePair instantiation must have two call arguments")
	}
	firstArg, ok := call.Args[0].(*ast.BasicLit)
	if !ok || firstArg.Kind != token.INT || firstArg.Value != "1" {
		return fmt.Errorf("first makePair argument must be integer literal 1")
	}
	secondArg, ok := call.Args[1].(*ast.BasicLit)
	if !ok || secondArg.Kind != token.STRING || secondArg.Value != strconv.Quote("x") {
		return fmt.Errorf("second makePair argument must be string literal x")
	}
	return nil
}

func validateSliceExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("slice expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("slice expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "tail" {
		return fmt.Errorf("declaration must be func tail")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func tail must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 1 {
		return fmt.Errorf("func tail must have one parameter")
	}
	param := funcDecl.Type.Params.List[0]
	if len(param.Names) != 1 || param.Names[0].Name != "values" || !sliceArrayTypeNamed(param.Type, "int") {
		return fmt.Errorf("func tail parameter must be values []int")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func tail must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !sliceArrayTypeNamed(result.Type, "int") {
		return fmt.Errorf("func tail result must be unnamed []int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func tail body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func tail body must return one expression")
	}
	sliceExpr, ok := returnStmt.Results[0].(*ast.SliceExpr)
	if !ok {
		return fmt.Errorf("func tail must return a slice expression")
	}
	if !identNamed(sliceExpr.X, "values") {
		return fmt.Errorf("slice expression target must be values")
	}
	if sliceExpr.High != nil || sliceExpr.Max != nil || sliceExpr.Slice3 {
		return fmt.Errorf("slice expression must use only a low bound")
	}
	low, ok := sliceExpr.Low.(*ast.BasicLit)
	if !ok || low.Kind != token.INT || low.Value != "1" {
		return fmt.Errorf("slice expression low bound must be integer literal 1")
	}
	return nil
}

func validateSliceExpressionThreeIndex(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("three-index slice expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("three-index slice expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "window" {
		return fmt.Errorf("declaration must be func window")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func window must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 1 {
		return fmt.Errorf("func window must have one parameter")
	}
	param := funcDecl.Type.Params.List[0]
	if len(param.Names) != 1 || param.Names[0].Name != "values" || !sliceArrayTypeNamed(param.Type, "int") {
		return fmt.Errorf("func window parameter must be values []int")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func window must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !sliceArrayTypeNamed(result.Type, "int") {
		return fmt.Errorf("func window result must be unnamed []int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func window body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func window body must return one expression")
	}
	sliceExpr, ok := returnStmt.Results[0].(*ast.SliceExpr)
	if !ok {
		return fmt.Errorf("func window must return a slice expression")
	}
	if !identNamed(sliceExpr.X, "values") {
		return fmt.Errorf("three-index slice expression target must be values")
	}
	if !sliceExpr.Slice3 {
		return fmt.Errorf("three-index slice expression must set Slice3")
	}
	if !integerLiteral(sliceExpr.Low, "1") ||
		!integerLiteral(sliceExpr.High, "2") ||
		!integerLiteral(sliceExpr.Max, "3") {
		return fmt.Errorf("three-index slice expression must use integer bounds 1, 2, and 3")
	}
	return nil
}

func validateStarExpression(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("star expression fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("star expression fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "value" {
		return fmt.Errorf("declaration must be func value")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func value must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 1 {
		return fmt.Errorf("func value must have one parameter")
	}
	param := funcDecl.Type.Params.List[0]
	if len(param.Names) != 1 || param.Names[0].Name != "ptr" || !starExprIdent(param.Type, "int") {
		return fmt.Errorf("func value parameter must be ptr *int")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func value must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func value result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func value body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func value body must return one expression")
	}
	if !starExprIdent(returnStmt.Results[0], "ptr") {
		return fmt.Errorf("func value must return *ptr")
	}
	return nil
}

func validatePackageVarDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("package var declaration fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("package var declaration fixture must contain one var declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "count" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("var declaration must be var count int = <integer>")
	}
	if !identNamed(valueSpec.Type, "int") {
		return fmt.Errorf("var declaration type must be int")
	}
	literal, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("var declaration value must be integer literal 7")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("second declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "count") {
		return fmt.Errorf("func build must return count")
	}
	return nil
}

func validateValueSpecComment(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("value spec comment fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("value spec comment fixture must contain one var declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "answer" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("var declaration must be var answer = 42 with one name and value")
	}
	if valueSpec.Type != nil {
		return fmt.Errorf("value spec comment fixture must omit ValueSpec.Type")
	}
	literal, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "42" {
		return fmt.Errorf("var declaration value must be integer literal 42")
	}
	if !commentGroupTextEquals(valueSpec.Comment, []string{"// retained value spec comment"}) {
		return fmt.Errorf("var declaration must attach trailing comment to ValueSpec.Comment")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "read" {
		return fmt.Errorf("second declaration must be func read")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func read must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func read must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func read must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return fmt.Errorf("func read result must be unnamed int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func read body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "answer") {
		return fmt.Errorf("func read must return answer")
	}
	return nil
}

func validatePackageConstDeclaration(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("package const declaration fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("package const declaration fixture must contain one const declaration and one function declaration")
	}
	genDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || genDecl.Tok != token.CONST || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single package-level const declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "cgoAvailable" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("const declaration must be const cgoAvailable = true")
	}
	if valueSpec.Type != nil {
		return fmt.Errorf("const declaration must omit ValueSpec.Type")
	}
	if !identNamed(valueSpec.Values[0], "true") {
		return fmt.Errorf("const declaration value must be true")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("second declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "bool") {
		return fmt.Errorf("func build result must be unnamed bool")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "cgoAvailable") {
		return fmt.Errorf("func build must return cgoAvailable")
	}
	return nil
}

func validateImportSpecName(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("import spec name fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("import spec name fixture must contain one import declaration and one function declaration")
	}
	importDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || importDecl.Tok != token.IMPORT || importDecl.Lparen.IsValid() || len(importDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single import declaration")
	}
	importSpec, ok := importDecl.Specs[0].(*ast.ImportSpec)
	if !ok || importSpec.Name == nil || importSpec.Name.Name != "_" {
		return fmt.Errorf("import declaration must use a blank ImportSpec.Name")
	}
	if importSpec.Path == nil || importSpec.Path.Value != "\"fmt\"" {
		return fmt.Errorf("import declaration must import fmt")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "read" {
		return fmt.Errorf("second declaration must be func read")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func read must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func read must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func read must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func read result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func read body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func read must return one result")
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"ok\"" {
		return fmt.Errorf("func read must return string literal ok")
	}
	return nil
}

func validateImportSpecComment(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("import spec comment fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("import spec comment fixture must contain one import declaration and one function declaration")
	}
	importDecl, ok := file.Decls[0].(*ast.GenDecl)
	if !ok || importDecl.Tok != token.IMPORT || importDecl.Lparen.IsValid() || len(importDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single import declaration")
	}
	importSpec, ok := importDecl.Specs[0].(*ast.ImportSpec)
	if !ok || importSpec.Name != nil {
		return fmt.Errorf("import declaration must omit ImportSpec.Name")
	}
	if importSpec.Path == nil || importSpec.Path.Value != "\"fmt\"" {
		return fmt.Errorf("import declaration must import fmt")
	}
	if !commentGroupTextEquals(importSpec.Comment, []string{"// retained import spec comment"}) {
		return fmt.Errorf("import declaration must attach trailing comment to ImportSpec.Comment")
	}
	funcDecl, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "read" {
		return fmt.Errorf("second declaration must be func read")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func read must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func read must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func read must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "string") {
		return fmt.Errorf("func read result must be unnamed string")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func read body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func read must return one result")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "fmt", "Sprint") || len(call.Args) != 1 {
		return fmt.Errorf("func read must return fmt.Sprint with one argument")
	}
	literal, ok := call.Args[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"ok\"" {
		return fmt.Errorf("func read must call fmt.Sprint with string literal ok")
	}
	return nil
}

func validateFunctionLiteral(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("function literal fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("function literal fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return fmt.Errorf("declaration must be func build")
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 {
		return fmt.Errorf("func build result must be unnamed")
	}
	resultFunc, ok := result.Type.(*ast.FuncType)
	if !ok || !functionTypeReturnsUnnamedInt(resultFunc) {
		return fmt.Errorf("func build result must be func() int")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func build body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func build body must return one expression")
	}
	funcLit, ok := returnStmt.Results[0].(*ast.FuncLit)
	if !ok || funcLit.Type == nil || funcLit.Body == nil {
		return fmt.Errorf("func build must return a function literal")
	}
	if !functionTypeReturnsUnnamedInt(funcLit.Type) {
		return fmt.Errorf("function literal type must be func() int")
	}
	if len(funcLit.Body.List) != 1 {
		return fmt.Errorf("function literal body must contain one return statement")
	}
	innerReturn, ok := funcLit.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(innerReturn.Results) != 1 {
		return fmt.Errorf("function literal body must return one expression")
	}
	literal, ok := innerReturn.Results[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != "7" {
		return fmt.Errorf("function literal must return integer literal 7")
	}
	return nil
}

func validateLocalGenericFunction(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local generic function fixture must use package main")
	}
	if len(file.Decls) != 1 {
		return fmt.Errorf("local generic function fixture must contain one function declaration")
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "identity" {
		return fmt.Errorf("declaration must be func identity")
	}
	if funcDecl.Type == nil || funcDecl.Type.TypeParams == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return fmt.Errorf("func identity must have type params, params, results, and body")
	}
	if len(funcDecl.Type.TypeParams.List) != 1 {
		return fmt.Errorf("func identity must have one type parameter field")
	}
	typeParam := funcDecl.Type.TypeParams.List[0]
	if len(typeParam.Names) != 1 || typeParam.Names[0].Name != "T" || !identNamed(typeParam.Type, "any") {
		return fmt.Errorf("func identity type parameter must be [T any]")
	}
	if !funcTypeHasOneNamedParam(funcDecl.Type, "value", "T") {
		return fmt.Errorf("func identity parameter must be value T")
	}
	if !funcTypeReturns(funcDecl.Type, "T") {
		return fmt.Errorf("func identity result must be T")
	}
	if len(funcDecl.Body.List) != 1 {
		return fmt.Errorf("func identity body must contain one return statement")
	}
	returnStmt, ok := funcDecl.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "value") {
		return fmt.Errorf("func identity must return value")
	}
	return nil
}

func validateConcurrencyMapInterfaceTypeAssertion(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("concurrency map/interface/type assertion fixture must use package main")
	}
	if len(file.Decls) != 3 {
		return fmt.Errorf("concurrency map/interface/type assertion fixture must contain two type declarations and one function declaration")
	}
	subscriberSpec, err := singleTypeSpec(file.Decls[0], "subscriber")
	if err != nil {
		return err
	}
	interfaceType, ok := subscriberSpec.Type.(*ast.InterfaceType)
	if !ok || interfaceType.Methods == nil || len(interfaceType.Methods.List) != 1 {
		return fmt.Errorf("subscriber must be a one-method interface type")
	}
	done := interfaceType.Methods.List[0]
	if len(done.Names) != 1 || done.Names[0].Name != "Done" {
		return fmt.Errorf("subscriber method must be named Done")
	}
	doneType, ok := done.Type.(*ast.FuncType)
	if !ok || doneType.Results == nil || len(doneType.Results.List) != 1 {
		return fmt.Errorf("subscriber Done method must return one value")
	}
	if !chanTypeNamed(doneType.Results.List[0].Type, ast.RECV, "int") {
		return fmt.Errorf("subscriber Done method must return <-chan int")
	}

	registrySpec, err := singleTypeSpec(file.Decls[1], "registry")
	if err != nil {
		return err
	}
	structType, ok := registrySpec.Type.(*ast.StructType)
	if !ok || structType.Fields == nil || len(structType.Fields.List) != 1 {
		return fmt.Errorf("registry must be a one-field struct type")
	}
	if !mapChannelField(structType.Fields.List[0], "values", "string", ast.SEND|ast.RECV, "int") {
		return fmt.Errorf("registry field must be values map[string]chan int")
	}

	funcDecl, ok := file.Decls[2].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "classify" || funcDecl.Type == nil || funcDecl.Body == nil {
		return fmt.Errorf("third declaration must be func classify")
	}
	required := map[string]bool{
		"channel_type":              false,
		"defer_statement":           false,
		"go_statement":              false,
		"interface_type":            false,
		"map_type":                  false,
		"select_statement":          false,
		"send_statement":            false,
		"type_assertion_expression": false,
	}
	ast.Inspect(file, func(node ast.Node) bool {
		switch node.(type) {
		case *ast.ChanType:
			required["channel_type"] = true
		case *ast.DeferStmt:
			required["defer_statement"] = true
		case *ast.GoStmt:
			required["go_statement"] = true
		case *ast.InterfaceType:
			required["interface_type"] = true
		case *ast.MapType:
			required["map_type"] = true
		case *ast.SelectStmt:
			required["select_statement"] = true
		case *ast.SendStmt:
			required["send_statement"] = true
		case *ast.TypeAssertExpr:
			required["type_assertion_expression"] = true
		}
		return true
	})
	for construct, seen := range required {
		if !seen {
			return fmt.Errorf("fixture must contain %s", construct)
		}
	}
	return nil
}

func validateLocalVariadicEllipsis(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "main" {
		return fmt.Errorf("local variadic ellipsis fixture must use package main")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("local variadic ellipsis fixture must contain build and total functions")
	}
	build, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || build.Name == nil || build.Name.Name != "build" {
		return fmt.Errorf("first declaration must be func build")
	}
	total, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || total.Name == nil || total.Name.Name != "total" {
		return fmt.Errorf("second declaration must be func total")
	}
	if !variadicIntFunction(build, "build") || !variadicIntFunction(total, "total") {
		return fmt.Errorf("build and total must accept values ...int and return int")
	}
	if len(build.Body.List) != 1 {
		return fmt.Errorf("build body must contain one return statement")
	}
	buildReturn, ok := build.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(buildReturn.Results) != 1 {
		return fmt.Errorf("build must return one expression")
	}
	buildCall, ok := buildReturn.Results[0].(*ast.CallExpr)
	if !ok || !identNamed(buildCall.Fun, "total") || len(buildCall.Args) != 1 || !identNamed(buildCall.Args[0], "values") {
		return fmt.Errorf("build must return total(values...)")
	}
	if !buildCall.Ellipsis.IsValid() {
		return fmt.Errorf("build call must use values... spread")
	}
	if len(total.Body.List) != 1 {
		return fmt.Errorf("total body must contain one return statement")
	}
	totalReturn, ok := total.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(totalReturn.Results) != 1 {
		return fmt.Errorf("total must return one expression")
	}
	totalCall, ok := totalReturn.Results[0].(*ast.CallExpr)
	if !ok || !identNamed(totalCall.Fun, "len") || len(totalCall.Args) != 1 || !identNamed(totalCall.Args[0], "values") {
		return fmt.Errorf("total must return len(values)")
	}
	if totalCall.Ellipsis.IsValid() {
		return fmt.Errorf("len(values) must not use call ellipsis")
	}
	ellipsisCount := 0
	ast.Inspect(file, func(node ast.Node) bool {
		if _, ok := node.(*ast.Ellipsis); ok {
			ellipsisCount++
		}
		return true
	})
	if ellipsisCount != 2 {
		return fmt.Errorf("fixture must contain exactly two parser-backed Ellipsis nodes, got %d", ellipsisCount)
	}
	return nil
}

func validateStringsCompareSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "strings" {
		return fmt.Errorf("strings compare source file must use package strings")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("strings compare source file must contain import and Compare declarations")
	}
	if err := validateStringsCompareHeaderComments(file); err != nil {
		return err
	}
	if err := validateStringsCompareImportDecl(file.Decls[0]); err != nil {
		return err
	}
	compare, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || compare.Recv != nil || compare.Name == nil || compare.Name.Name != "Compare" {
		return fmt.Errorf("second declaration must be func Compare")
	}
	if err := validateStringsCompareDoc(compare); err != nil {
		return err
	}
	if compare.Type == nil || compare.Type.Params == nil || compare.Type.Results == nil || compare.Body == nil {
		return fmt.Errorf("func Compare must have params, results, and body")
	}
	if !funcTypeHasNamedParams(compare.Type, "a", "b", "string") {
		return fmt.Errorf("func Compare must accept a, b string")
	}
	if !funcTypeReturns(compare.Type, "int") {
		return fmt.Errorf("func Compare must return unnamed int")
	}
	if len(compare.Body.List) != 1 {
		return fmt.Errorf("func Compare body must contain one return statement")
	}
	returnStmt, ok := compare.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func Compare body must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "bytealg", "CompareString") || len(call.Args) != 2 {
		return fmt.Errorf("func Compare must return bytealg.CompareString(a, b)")
	}
	if !identNamed(call.Args[0], "a") || !identNamed(call.Args[1], "b") {
		return fmt.Errorf("bytealg.CompareString arguments must be a, b")
	}
	return nil
}

func validateStringsCloneSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "strings" {
		return fmt.Errorf("strings clone source file must use package strings")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("strings clone source file must contain import and Clone declarations")
	}
	if err := validateStringsCloneHeaderComments(file); err != nil {
		return err
	}
	if err := validateStringsCloneImportDecl(file.Decls[0]); err != nil {
		return err
	}
	clone, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || clone.Recv != nil || clone.Name == nil || clone.Name.Name != "Clone" {
		return fmt.Errorf("second declaration must be func Clone")
	}
	if err := validateStringsCloneDoc(clone); err != nil {
		return err
	}
	if clone.Type == nil || clone.Type.Params == nil || clone.Type.Results == nil || clone.Body == nil {
		return fmt.Errorf("func Clone must have params, results, and body")
	}
	if !funcTypeHasOneNamedParam(clone.Type, "s", "string") {
		return fmt.Errorf("func Clone must accept s string")
	}
	if !funcTypeReturns(clone.Type, "string") {
		return fmt.Errorf("func Clone must return unnamed string")
	}
	if len(clone.Body.List) != 1 {
		return fmt.Errorf("func Clone body must contain one return statement")
	}
	returnStmt, ok := clone.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func Clone body must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "stringslite", "Clone") || len(call.Args) != 1 {
		return fmt.Errorf("func Clone must return stringslite.Clone(s)")
	}
	if !identNamed(call.Args[0], "s") {
		return fmt.Errorf("stringslite.Clone argument must be s")
	}
	return nil
}

func validateSortInterfaceSourceFileDeclarationPrefix(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "sort_test" {
		return fmt.Errorf("sort interface source file must use package sort_test")
	}
	if len(file.Decls) != 8 {
		return fmt.Errorf("sort interface source file must keep the expected eight top-level declarations")
	}
	if err := validateSortInterfaceImportBlock(file.Decls[0]); err != nil {
		return err
	}
	if err := validatePersonStructDeclaration(file.Decls[1]); err != nil {
		return err
	}
	if err := validatePersonStringMethod(file.Decls[2]); err != nil {
		return err
	}
	if err := validateByAgeTypeDeclaration(file.Decls[3]); err != nil {
		return err
	}
	if err := validateByAgeLenMethod(file.Decls[4]); err != nil {
		return err
	}
	if err := validateByAgeSwapMethod(file.Decls[5]); err != nil {
		return err
	}
	if err := validateByAgeLessMethod(file.Decls[6]); err != nil {
		return err
	}
	example, ok := file.Decls[7].(*ast.FuncDecl)
	if !ok || example.Recv != nil || example.Name == nil || example.Name.Name != "Example" {
		return fmt.Errorf("last declaration must be the Example function")
	}
	if err := validateSortInterfaceExampleBodyPrefix(example); err != nil {
		return err
	}
	if err := validateSortInterfaceOutputCommentSuffix(file, example); err != nil {
		return err
	}
	if !blockContainsNode(example.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.CompositeLit)
		return ok
	}) {
		return fmt.Errorf("Example body prefix must include the composite literal collection")
	}
	if !blockContainsNode(example.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.FuncLit)
		return ok
	}) {
		return fmt.Errorf("Example executable suffix must include the function literal sort closure")
	}
	return nil
}

func validateSortSearchSourceFileExamplePrefix(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "sort_test" {
		return fmt.Errorf("sort search source file must use package sort_test")
	}
	if len(file.Decls) != 7 {
		return fmt.Errorf("sort search source file must keep the expected seven top-level declarations")
	}
	if err := validateSortSearchImportBlock(file.Decls[0]); err != nil {
		return err
	}
	example, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || example.Recv != nil || example.Name == nil || example.Name.Name != "ExampleSearch" {
		return fmt.Errorf("first supported declaration must be the ExampleSearch function")
	}
	if err := validateSortSearchExampleBodyPrefix(example); err != nil {
		return err
	}
	if err := validateSortSearchOutputCommentSuffix(file, example); err != nil {
		return err
	}
	if !blockContainsNode(example.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.CompositeLit)
		return ok
	}) {
		return fmt.Errorf("ExampleSearch body must include the sorted integer slice literal")
	}
	if !blockContainsNode(example.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.FuncLit)
		return ok
	}) {
		return fmt.Errorf("ExampleSearch body must include the sort.Search predicate function literal")
	}
	return nil
}

func validateSortSearchSourceFile(file *ast.File) error {
	if err := validateSortSearchSourceFileExamplePrefix(file); err != nil {
		return err
	}
	if err := validateSortSearchHeaderComments(file); err != nil {
		return err
	}
	if err := validateSortSearchSupportedFunctions(file); err != nil {
		return err
	}
	if err := validateSortSearchAllOutputComments(file); err != nil {
		return err
	}
	return nil
}

func validateStringsSearchSourceFileFinderPrefix(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "strings" {
		return fmt.Errorf("strings search source file must use package strings")
	}
	if len(file.Decls) != 4 {
		return fmt.Errorf("strings search source file must keep the expected four top-level declarations")
	}
	if err := validateStringFinderTypeDeclaration(file.Decls[0]); err != nil {
		return err
	}
	makeFinder, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || makeFinder.Recv != nil || makeFinder.Name == nil || makeFinder.Name.Name != "makeStringFinder" {
		return fmt.Errorf("second declaration must be makeStringFinder")
	}
	if err := validateMakeStringFinderBody(makeFinder); err != nil {
		return err
	}
	if err := validateStringsSearchFinderComments(file); err != nil {
		return err
	}
	if !blockContainsNode(makeFinder.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.RangeStmt)
		return ok
	}) {
		return fmt.Errorf("makeStringFinder must include the range loop over f.badCharSkip")
	}
	if !blockContainsNode(makeFinder.Body, func(node ast.Node) bool {
		index, ok := node.(*ast.IndexExpr)
		return ok && selectorNamed(index.X, "f", "badCharSkip")
	}) {
		return fmt.Errorf("makeStringFinder must include indexed f.badCharSkip assignments")
	}
	if !blockContainsNode(makeFinder.Body, func(node ast.Node) bool {
		slice, ok := node.(*ast.SliceExpr)
		return ok && identNamed(slice.X, "pattern")
	}) {
		return fmt.Errorf("makeStringFinder must include pattern slice expressions")
	}
	return nil
}

func validateStringsSearchSourceFile(file *ast.File) error {
	if err := validateStringsSearchSourceFileFinderPrefix(file); err != nil {
		return err
	}
	if err := validateStringsSearchHeaderComments(file); err != nil {
		return err
	}
	if err := validateStringsSearchSupportedDeclarations(file); err != nil {
		return err
	}
	if err := validateStringsSearchNextComments(file); err != nil {
		return err
	}
	return nil
}

func validatePathMatchSourceFileMatchPrefix(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "path" {
		return fmt.Errorf("path match source file must use package path")
	}
	if len(file.Decls) != 6 {
		return fmt.Errorf("path match source file must keep the expected six top-level declarations")
	}
	if err := validatePathMatchImportBlock(file.Decls[0]); err != nil {
		return err
	}
	if err := validateErrBadPatternDeclaration(file.Decls[1]); err != nil {
		return err
	}
	match, ok := file.Decls[2].(*ast.FuncDecl)
	if !ok || match.Recv != nil || match.Name == nil || match.Name.Name != "Match" {
		return fmt.Errorf("third declaration must be the Match function")
	}
	if err := validatePathMatchFunctionBody(match); err != nil {
		return err
	}
	if err := validatePathMatchComments(file); err != nil {
		return err
	}
	if !blockContainsNode(match.Body, func(node ast.Node) bool {
		labeled, ok := node.(*ast.LabeledStmt)
		return ok && labeled.Label != nil && labeled.Label.Name == "Pattern"
	}) {
		return fmt.Errorf("Match must preserve the Pattern label")
	}
	if !blockContainsNode(match.Body, func(node ast.Node) bool {
		branch, ok := node.(*ast.BranchStmt)
		return ok && branch.Tok == token.CONTINUE && branch.Label != nil && branch.Label.Name == "Pattern"
	}) {
		return fmt.Errorf("Match must preserve labeled continue Pattern")
	}
	if !blockContainsNode(match.Body, func(node ast.Node) bool {
		_, ok := node.(*ast.SliceExpr)
		return ok
	}) {
		return fmt.Errorf("Match must include slice expressions for skipped-name matching")
	}
	return nil
}

func validatePathMatchSourceFile(file *ast.File) error {
	if err := validatePathMatchSourceFileMatchPrefix(file); err != nil {
		return err
	}
	if err := validatePathMatchHeaderComments(file); err != nil {
		return err
	}
	if err := validatePathMatchSupportedDeclarations(file); err != nil {
		return err
	}
	if err := validatePathMatchCompleteComments(file); err != nil {
		return err
	}
	return nil
}

func validateCryptoSysrandRandWindowsSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "sysrand" {
		return fmt.Errorf("crypto sysrand rand_windows source file must use package sysrand")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("crypto sysrand rand_windows source file must contain import and read declarations")
	}
	if err := validateCryptoSysrandRandWindowsHeaderComments(file); err != nil {
		return err
	}
	if err := validateCryptoSysrandRandWindowsImportDecl(file.Decls[0]); err != nil {
		return err
	}
	read, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || read.Recv != nil || read.Name == nil || read.Name.Name != "read" {
		return fmt.Errorf("second declaration must be func read")
	}
	if read.Doc != nil {
		return fmt.Errorf("func read must not have doc comments in the checked source file")
	}
	if read.Type == nil || read.Type.Params == nil || read.Type.Results == nil || read.Body == nil {
		return fmt.Errorf("func read must have params, results, and body")
	}
	if !funcTypeHasOneNamedSliceParam(read.Type, "b", "byte") {
		return fmt.Errorf("func read must accept b []byte")
	}
	if !funcTypeReturns(read.Type, "error") {
		return fmt.Errorf("func read must return unnamed error")
	}
	if len(read.Body.List) != 1 {
		return fmt.Errorf("func read body must contain one return statement")
	}
	returnStmt, ok := read.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func read body must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "windows", "ProcessPrng") || len(call.Args) != 1 {
		return fmt.Errorf("func read must return windows.ProcessPrng(b)")
	}
	if !identNamed(call.Args[0], "b") {
		return fmt.Errorf("windows.ProcessPrng argument must be b")
	}
	return nil
}

func validateCryptoSubtleXorSourceFile(file *ast.File) error {
	if file.Name == nil || file.Name.Name != "subtle" {
		return fmt.Errorf("crypto subtle xor source file must use package subtle")
	}
	if len(file.Decls) != 2 {
		return fmt.Errorf("crypto subtle xor source file must contain import and XORBytes declarations")
	}
	if err := validateCryptoSubtleXorHeaderComments(file); err != nil {
		return err
	}
	if err := validateCryptoSubtleXorImportDecl(file.Decls[0]); err != nil {
		return err
	}
	xorBytes, ok := file.Decls[1].(*ast.FuncDecl)
	if !ok || xorBytes.Recv != nil || xorBytes.Name == nil || xorBytes.Name.Name != "XORBytes" {
		return fmt.Errorf("second declaration must be func XORBytes")
	}
	if !commentGroupTextEquals(xorBytes.Doc, []string{
		"// XORBytes sets dst[i] = x[i] ^ y[i] for all i < n = min(len(x), len(y)),",
		"// returning n, the number of bytes written to dst.",
		"//",
		"// If dst does not have length at least n,",
		"// XORBytes panics without writing anything to dst.",
		"//",
		"// dst and x or y may overlap exactly or not at all,",
		"// otherwise XORBytes may panic.",
	}) {
		return fmt.Errorf("func XORBytes doc comments must match the parser-backed source file")
	}
	if xorBytes.Type == nil || xorBytes.Type.Params == nil || xorBytes.Type.Results == nil || xorBytes.Body == nil {
		return fmt.Errorf("func XORBytes must have params, results, and body")
	}
	if !funcTypeHasThreeNamedSliceParams(xorBytes.Type, "dst", "x", "y", "byte") {
		return fmt.Errorf("func XORBytes must accept dst, x, y []byte")
	}
	if !funcTypeReturns(xorBytes.Type, "int") {
		return fmt.Errorf("func XORBytes must return unnamed int")
	}
	if len(xorBytes.Body.List) != 1 {
		return fmt.Errorf("func XORBytes body must contain one return statement")
	}
	returnStmt, ok := xorBytes.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("func XORBytes body must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "subtle", "XORBytes") || len(call.Args) != 3 {
		return fmt.Errorf("func XORBytes must return subtle.XORBytes(dst, x, y)")
	}
	for index, want := range []string{"dst", "x", "y"} {
		if !identNamed(call.Args[index], want) {
			return fmt.Errorf("subtle.XORBytes argument %d must be %s", index, want)
		}
	}
	return nil
}

func validateSortInterfaceOutputCommentSuffix(file *ast.File, example *ast.FuncDecl) error {
	expected := []string{
		"// Output:",
		"// [Bob: 31 John: 42 Michael: 17 Jenny: 26]",
		"// [Michael: 17 Jenny: 26 Bob: 31 John: 42]",
		"// [John: 42 Bob: 31 Jenny: 26 Michael: 17]",
	}
	if file == nil || example == nil || example.Body == nil || len(example.Body.List) == 0 {
		return fmt.Errorf("sort interface output comments require the parsed Example body")
	}
	var outputGroup *ast.CommentGroup
	for _, group := range file.Comments {
		if group == nil || len(group.List) == 0 {
			continue
		}
		if group.List[0].Text == "// Output:" {
			outputGroup = group
			break
		}
	}
	if outputGroup == nil {
		return fmt.Errorf("Example must preserve the parser-backed // Output: comment group")
	}
	lastStatement := example.Body.List[len(example.Body.List)-1]
	if outputGroup.Pos() <= lastStatement.End() || outputGroup.End() >= example.Body.Rbrace {
		return fmt.Errorf("Example output comment group must follow the final statement inside the function body")
	}
	if len(outputGroup.List) != len(expected) {
		return fmt.Errorf("Example output comment group must contain the marker and three output lines")
	}
	for index, want := range expected {
		if outputGroup.List[index].Text != want {
			return fmt.Errorf("Example output comment %d must be %q", index, want)
		}
		if !strings.HasPrefix(outputGroup.List[index].Text, "//") {
			return fmt.Errorf("Example output comment %d must be a line comment from go/parser metadata", index)
		}
	}
	return nil
}

func validateSortSearchOutputCommentSuffix(file *ast.File, example *ast.FuncDecl) error {
	expected := []string{
		"// Output:",
		"// found 6 at index 2 in [1 3 6 10 15 21 28 36 45 55]",
	}
	if file == nil || example == nil || example.Body == nil || len(example.Body.List) == 0 {
		return fmt.Errorf("sort search output comments require the parsed ExampleSearch body")
	}
	var outputGroup *ast.CommentGroup
	for _, group := range file.Comments {
		if group == nil || len(group.List) == 0 {
			continue
		}
		if group.List[0].Text == "// Output:" && group.Pos() > example.Body.Pos() && group.End() < example.Body.End() {
			outputGroup = group
			break
		}
	}
	if outputGroup == nil {
		return fmt.Errorf("ExampleSearch must preserve the parser-backed // Output: comment group")
	}
	lastStatement := example.Body.List[len(example.Body.List)-1]
	if outputGroup.Pos() <= lastStatement.End() || outputGroup.End() >= example.Body.Rbrace {
		return fmt.Errorf("ExampleSearch output comment group must follow the final statement inside the function body")
	}
	if len(outputGroup.List) != len(expected) {
		return fmt.Errorf("ExampleSearch output comment group must contain the marker and one output line")
	}
	for index, want := range expected {
		if outputGroup.List[index].Text != want {
			return fmt.Errorf("ExampleSearch output comment %d must be %q", index, want)
		}
		if !strings.HasPrefix(outputGroup.List[index].Text, "//") {
			return fmt.Errorf("ExampleSearch output comment %d must be a line comment from go/parser metadata", index)
		}
	}
	return nil
}

func validateSortSearchHeaderComments(file *ast.File) error {
	expected := []string{
		"// Copyright 2016 The Go Authors. All rights reserved.",
		"// Use of this source code is governed by a BSD-style",
		"// license that can be found in the LICENSE file.",
	}
	if len(file.Comments) < 1 || !commentGroupTextEquals(file.Comments[0], expected) {
		return fmt.Errorf("sort search source file must preserve the parser-backed header comments")
	}
	return nil
}

func validateSortSearchSupportedFunctions(file *ast.File) error {
	expected := []string{
		"ExampleSearch",
		"ExampleSearch_descendingOrder",
		"ExampleFind",
		"ExampleSearchFloat64s",
		"ExampleSearchInts",
		"ExampleSearchStrings",
	}
	if len(file.Decls) != len(expected)+1 {
		return fmt.Errorf("sort search source file must keep the import block and six example functions")
	}
	for index, name := range expected {
		funcDecl, ok := file.Decls[index+1].(*ast.FuncDecl)
		if !ok || funcDecl.Recv != nil || funcDecl.Name == nil || funcDecl.Name.Name != name {
			return fmt.Errorf("sort search declaration %d must be func %s", index+1, name)
		}
		if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results != nil || funcDecl.Body == nil {
			return fmt.Errorf("sort search function %s must have no params, no results, and a body", name)
		}
		if len(funcDecl.Type.Params.List) != 0 {
			return fmt.Errorf("sort search function %s must not declare parameters", name)
		}
	}
	return nil
}

func validateSortSearchAllOutputComments(file *ast.File) error {
	expectedGroups := [][]string{
		{
			"// Output:",
			"// found 6 at index 2 in [1 3 6 10 15 21 28 36 45 55]",
		},
		{
			"// Output:",
			"// found 6 at index 7 in [55 45 36 28 21 15 10 6 3 1]",
		},
		{
			"// Output:",
			"// found banana at index 1",
			"// orange not found, would insert at 4",
		},
		{
			"// Output:",
			"// found 2 at index 1 in [1 2 3.3 4.6 6.1 7.2 8]",
			"// 0.5 not found, can be inserted at index 0 in [1 2 3.3 4.6 6.1 7.2 8]",
		},
		{
			"// Output:",
			"// found 2 at index 1 in [1 2 3 4 6 7 8]",
			"// 5 not found, can be inserted at index 4 in [1 2 3 4 6 7 8]",
		},
		{
			"// Output:",
			"// found banana at index 1 in [apple banana cherry date fig grape]",
			"// coconut not found, can be inserted at index 3 in [apple banana cherry date fig grape]",
		},
	}
	groups := make([]*ast.CommentGroup, 0, len(expectedGroups))
	for _, group := range file.Comments {
		if group == nil || len(group.List) == 0 || group.List[0].Text != "// Output:" {
			continue
		}
		groups = append(groups, group)
	}
	if len(groups) != len(expectedGroups) {
		return fmt.Errorf("sort search source file must preserve six parser-backed output comment groups")
	}
	for index, expected := range expectedGroups {
		if !commentGroupTextEquals(groups[index], expected) {
			return fmt.Errorf("sort search output comment group %d must match the parser-backed source file", index)
		}
	}
	return nil
}

func validateSortInterfaceExampleBodyPrefix(example *ast.FuncDecl) error {
	if example.Type == nil || example.Type.Params == nil || example.Type.Results != nil || example.Body == nil {
		return fmt.Errorf("Example must have no parameters, no results, and a body")
	}
	if len(example.Type.Params.List) != 0 {
		return fmt.Errorf("Example must not declare parameters")
	}
	if len(example.Body.List) != 6 {
		return fmt.Errorf("Example body must contain the people declaration and five executable call statements")
	}
	assign, ok := example.Body.List[0].(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 {
		return fmt.Errorf("Example first statement must be people := []Person{...}")
	}
	if !identNamed(assign.Lhs[0], "people") {
		return fmt.Errorf("Example short variable target must be people")
	}
	peopleLiteral, ok := assign.Rhs[0].(*ast.CompositeLit)
	if !ok || !sliceArrayTypeNamed(peopleLiteral.Type, "Person") || peopleLiteral.Incomplete {
		return fmt.Errorf("Example people value must be a complete []Person composite literal")
	}
	expectedPeople := []struct {
		name string
		age  string
	}{
		{name: "\"Bob\"", age: "31"},
		{name: "\"John\"", age: "42"},
		{name: "\"Michael\"", age: "17"},
		{name: "\"Jenny\"", age: "26"},
	}
	if len(peopleLiteral.Elts) != len(expectedPeople) {
		return fmt.Errorf("Example people literal must contain four Person elements")
	}
	for index, expected := range expectedPeople {
		personLiteral, ok := peopleLiteral.Elts[index].(*ast.CompositeLit)
		if !ok || personLiteral.Type != nil || personLiteral.Incomplete || len(personLiteral.Elts) != 2 {
			return fmt.Errorf("Example people element %d must be an untyped Person composite literal", index)
		}
		nameLiteral, ok := personLiteral.Elts[0].(*ast.BasicLit)
		if !ok || nameLiteral.Kind != token.STRING || nameLiteral.Value != expected.name {
			return fmt.Errorf("Example people element %d must carry name %s", index, expected.name)
		}
		ageLiteral, ok := personLiteral.Elts[1].(*ast.BasicLit)
		if !ok || ageLiteral.Kind != token.INT || ageLiteral.Value != expected.age {
			return fmt.Errorf("Example people element %d must carry age %s", index, expected.age)
		}
	}
	if !exprStmtCall(example.Body.List[1], "fmt", "Println", "people") {
		return fmt.Errorf("Example second statement must be fmt.Println(people)")
	}
	if !sortSortByAgePeopleCall(example.Body.List[2]) {
		return fmt.Errorf("Example third statement must be sort.Sort(ByAge(people))")
	}
	if !exprStmtCall(example.Body.List[3], "fmt", "Println", "people") {
		return fmt.Errorf("Example fourth statement must be fmt.Println(people)")
	}
	if !sortSlicePeopleClosureCall(example.Body.List[4]) {
		return fmt.Errorf("Example fifth statement must be sort.Slice with the age-descending closure")
	}
	if !exprStmtCall(example.Body.List[5], "fmt", "Println", "people") {
		return fmt.Errorf("Example sixth statement must be fmt.Println(people)")
	}
	return nil
}

func validateSortSearchExampleBodyPrefix(example *ast.FuncDecl) error {
	if example.Type == nil || example.Type.Params == nil || example.Type.Results != nil || example.Body == nil {
		return fmt.Errorf("ExampleSearch must have no parameters, no results, and a body")
	}
	if len(example.Type.Params.List) != 0 {
		return fmt.Errorf("ExampleSearch must not declare parameters")
	}
	if len(example.Body.List) != 4 {
		return fmt.Errorf("ExampleSearch body must contain the three declarations and one if/else statement")
	}
	if !shortVarIntegerSlice(example.Body.List[0], "a", []string{"1", "3", "6", "10", "15", "21", "28", "36", "45", "55"}) {
		return fmt.Errorf("ExampleSearch first statement must be the ascending []int slice literal")
	}
	if !shortVarIntegerLiteral(example.Body.List[1], "x", "6") {
		return fmt.Errorf("ExampleSearch second statement must be x := 6")
	}
	if !sortSearchShortVarCall(example.Body.List[2]) {
		return fmt.Errorf("ExampleSearch third statement must be i := sort.Search(len(a), func(i int) bool { return a[i] >= x })")
	}
	if !sortSearchIfElse(example.Body.List[3]) {
		return fmt.Errorf("ExampleSearch final statement must be the parser-backed found/not-found if/else")
	}
	return nil
}

func validateStringFinderTypeDeclaration(decl ast.Decl) error {
	typeSpec, err := singleTypeSpec(decl, "stringFinder")
	if err != nil {
		return err
	}
	structType, ok := typeSpec.Type.(*ast.StructType)
	if !ok || structType.Fields == nil || len(structType.Fields.List) != 3 {
		return fmt.Errorf("stringFinder must be a three-field struct")
	}
	if !namedField(structType.Fields.List[0], "pattern", "string") {
		return fmt.Errorf("stringFinder first field must be pattern string")
	}
	if !fixedArrayField(structType.Fields.List[1], "badCharSkip", "256", "int") {
		return fmt.Errorf("stringFinder second field must be badCharSkip [256]int")
	}
	if !sliceArrayField(structType.Fields.List[2], "goodSuffixSkip", "int") {
		return fmt.Errorf("stringFinder third field must be goodSuffixSkip []int")
	}
	return nil
}

func validateMakeStringFinderBody(makeFinder *ast.FuncDecl) error {
	if makeFinder.Type == nil || makeFinder.Type.Params == nil || makeFinder.Type.Results == nil || makeFinder.Body == nil {
		return fmt.Errorf("makeStringFinder must have params, results, and body")
	}
	if !funcTypeHasOneNamedParam(makeFinder.Type, "pattern", "string") || !funcTypeReturnsPointer(makeFinder.Type, "stringFinder") {
		return fmt.Errorf("makeStringFinder must have signature func makeStringFinder(pattern string) *stringFinder")
	}
	if len(makeFinder.Body.List) != 8 {
		return fmt.Errorf("makeStringFinder body must contain the expected eight statements")
	}
	validators := []struct {
		statement string
		validate  func(ast.Stmt) bool
	}{
		{"f := &stringFinder{...}", stringFinderCompositeAssignment},
		{"last := len(pattern) - 1", lastIndexAssignment},
		{"badCharSkip range loop", badCharSkipRangeLoop},
		{"badCharSkip forward loop", badCharSkipForwardLoop},
		{"lastPrefix := last", lastPrefixAssignment},
		{"goodSuffixSkip prefix loop", goodSuffixPrefixLoop},
		{"goodSuffixSkip repeat loop", goodSuffixRepeatLoop},
		{"return f", returnIdentifierF},
	}
	for index, validator := range validators {
		if !validator.validate(makeFinder.Body.List[index]) {
			return fmt.Errorf("makeStringFinder statement %d must be %s", index+1, validator.statement)
		}
	}
	return nil
}

func validateStringsSearchFinderComments(file *ast.File) error {
	requiredComments := []string{
		"// stringFinder efficiently finds strings in a source text. It's implemented",
		"// pattern is the string that we are searching for in the text.",
		"// badCharSkip[b] contains the distance between the last byte of pattern",
		"// goodSuffixSkip[i] defines how far we can shift the matching frame given",
		"// last is the index of the last character in the pattern.",
		"// Build bad character table.",
		"// Build good suffix table.",
		"// Second pass: find repeats of pattern's suffix starting from the front.",
	}
	for _, required := range requiredComments {
		found := false
		for _, group := range file.Comments {
			for _, comment := range group.List {
				if comment.Text == required {
					found = true
					break
				}
			}
			if found {
				break
			}
		}
		if !found {
			return fmt.Errorf("strings/search finder prefix must preserve parser-backed comment %q", required)
		}
	}
	return nil
}

func validateStringsSearchHeaderComments(file *ast.File) error {
	expected := []string{
		"// Copyright 2012 The Go Authors. All rights reserved.",
		"// Use of this source code is governed by a BSD-style",
		"// license that can be found in the LICENSE file.",
	}
	if len(file.Comments) < 1 || !commentGroupTextEquals(file.Comments[0], expected) {
		return fmt.Errorf("strings/search source file must preserve the parser-backed header comments")
	}
	return nil
}

func validateStringsSearchSupportedDeclarations(file *ast.File) error {
	if len(file.Decls) != 4 {
		return fmt.Errorf("strings/search source file must keep type plus three function declarations")
	}
	longest, ok := file.Decls[2].(*ast.FuncDecl)
	if !ok || longest.Recv != nil || longest.Name == nil || longest.Name.Name != "longestCommonSuffix" {
		return fmt.Errorf("third declaration must be longestCommonSuffix")
	}
	if err := validateLongestCommonSuffixBody(longest); err != nil {
		return err
	}
	next, ok := file.Decls[3].(*ast.FuncDecl)
	if !ok || next.Name == nil || next.Name.Name != "next" {
		return fmt.Errorf("fourth declaration must be (*stringFinder).next")
	}
	if err := validateStringFinderNextBody(next); err != nil {
		return err
	}
	return nil
}

func validateLongestCommonSuffixBody(longest *ast.FuncDecl) error {
	if longest.Type == nil || longest.Type.Params == nil || longest.Type.Results == nil || longest.Body == nil {
		return fmt.Errorf("longestCommonSuffix must have params, results, and body")
	}
	if !funcTypeHasTwoNamedParamsOneType(longest.Type, "a", "b", "string") ||
		!funcTypeReturnsNamedSingle(longest.Type, "i", "int") {
		return fmt.Errorf("longestCommonSuffix must have signature func longestCommonSuffix(a, b string) (i int)")
	}
	if len(longest.Body.List) != 2 {
		return fmt.Errorf("longestCommonSuffix body must keep the for loop and bare return")
	}
	loop, ok := longest.Body.List[0].(*ast.ForStmt)
	if !ok || loop.Init != nil || loop.Body == nil || !incDecIdent(loop.Post, "i", token.INC) {
		return fmt.Errorf("longestCommonSuffix first statement must be the suffix-counting for loop")
	}
	if !stringSuffixLoopCondition(loop.Cond) {
		return fmt.Errorf("longestCommonSuffix loop condition must compare i against len(a) and len(b)")
	}
	if len(loop.Body.List) != 1 {
		return fmt.Errorf("longestCommonSuffix loop body must contain the mismatch guard")
	}
	ifStmt, ok := loop.Body.List[0].(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 || !longestCommonSuffixMismatchCondition(ifStmt.Cond) {
		return fmt.Errorf("longestCommonSuffix loop body must break on mismatched suffix bytes")
	}
	branch, ok := ifStmt.Body.List[0].(*ast.BranchStmt)
	if !ok || branch.Tok != token.BREAK || branch.Label != nil {
		return fmt.Errorf("longestCommonSuffix mismatch branch must be a bare break")
	}
	returnStmt, ok := longest.Body.List[1].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 0 {
		return fmt.Errorf("longestCommonSuffix final statement must be a bare return")
	}
	return nil
}

func validateStringFinderNextBody(next *ast.FuncDecl) error {
	if next.Type == nil || next.Type.Params == nil || next.Type.Results == nil || next.Body == nil {
		return fmt.Errorf("next must have params, results, and body")
	}
	if !pointerMethodDecl(next, "next", "f", "stringFinder") {
		return fmt.Errorf("next must have receiver func (f *stringFinder)")
	}
	if !funcTypeHasOneNamedParam(next.Type, "text", "string") || !funcTypeReturns(next.Type, "int") {
		return fmt.Errorf("next must have signature func (f *stringFinder) next(text string) int")
	}
	if len(next.Body.List) != 3 {
		return fmt.Errorf("next body must keep the initial index, loop, and not-found return")
	}
	if !assignLastPatternIndex(next.Body.List[0]) {
		return fmt.Errorf("next first statement must initialize i from len(f.pattern) - 1")
	}
	loop, ok := next.Body.List[1].(*ast.ForStmt)
	if !ok || loop.Init != nil || loop.Post != nil || loop.Body == nil || !binaryIdentLenCall(loop.Cond, "i", token.LSS, "text") {
		return fmt.Errorf("next loop must iterate while i < len(text)")
	}
	if len(loop.Body.List) != 4 {
		return fmt.Errorf("next loop body must keep j init, compare loop, match return, and skip assignment")
	}
	if !assignLastPatternIndexToName(loop.Body.List[0], "j") {
		return fmt.Errorf("next loop first statement must initialize j from len(f.pattern) - 1")
	}
	if !backwardPatternCompareLoop(loop.Body.List[1]) {
		return fmt.Errorf("next loop second statement must compare text and pattern backwards")
	}
	if !nextMatchReturnIf(loop.Body.List[2]) {
		return fmt.Errorf("next loop third statement must return i + 1 on match")
	}
	if !nextSkipAssignment(loop.Body.List[3]) {
		return fmt.Errorf("next loop fourth statement must add max bad-char/good-suffix skip")
	}
	if !returnNegativeOne(next.Body.List[2]) {
		return fmt.Errorf("next final statement must return -1")
	}
	return nil
}

func validateStringsSearchNextComments(file *ast.File) error {
	expected := []string{
		"// next returns the index in text of the first occurrence of the pattern. If",
		"// the pattern is not found, it returns -1.",
	}
	for _, group := range file.Comments {
		if commentGroupTextEquals(group, expected) {
			return nil
		}
	}
	return fmt.Errorf("strings/search source file must preserve the parser-backed next doc comments")
}

func validatePathMatchComments(file *ast.File) error {
	requiredComments := []string{
		"// ErrBadPattern indicates a pattern was malformed.",
		"// Match reports whether name matches the shell pattern.",
		"// The pattern syntax is:",
		"// Match requires pattern to match all of name, not just a substring.",
		"// Trailing * matches rest of string unless it has a /.",
		"// Look for match at current position.",
		"// check that the remainder of the pattern is syntactically valid.",
	}
	for _, required := range requiredComments {
		found := false
		for _, group := range file.Comments {
			for _, comment := range group.List {
				if comment.Text == required {
					found = true
					break
				}
			}
			if found {
				break
			}
		}
		if !found {
			return fmt.Errorf("path/match prefix must preserve parser-backed comment %q", required)
		}
	}
	return nil
}

func validatePathMatchHeaderComments(file *ast.File) error {
	expected := []string{
		"// Copyright 2010 The Go Authors. All rights reserved.",
		"// Use of this source code is governed by a BSD-style",
		"// license that can be found in the LICENSE file.",
	}
	if len(file.Comments) < 1 || !commentGroupTextEquals(file.Comments[0], expected) {
		return fmt.Errorf("path/match source file must preserve the parser-backed header comments")
	}
	return nil
}

func validatePathMatchSupportedDeclarations(file *ast.File) error {
	if len(file.Decls) != 6 {
		return fmt.Errorf("path/match source file must keep import, ErrBadPattern, Match, scanChunk, matchChunk, and getEsc")
	}
	scanChunk, ok := file.Decls[3].(*ast.FuncDecl)
	if !ok || scanChunk.Recv != nil || scanChunk.Name == nil || scanChunk.Name.Name != "scanChunk" {
		return fmt.Errorf("fourth declaration must be scanChunk")
	}
	if err := validateScanChunkBody(scanChunk); err != nil {
		return err
	}
	matchChunk, ok := file.Decls[4].(*ast.FuncDecl)
	if !ok || matchChunk.Recv != nil || matchChunk.Name == nil || matchChunk.Name.Name != "matchChunk" {
		return fmt.Errorf("fifth declaration must be matchChunk")
	}
	if err := validateMatchChunkBody(matchChunk); err != nil {
		return err
	}
	getEsc, ok := file.Decls[5].(*ast.FuncDecl)
	if !ok || getEsc.Recv != nil || getEsc.Name == nil || getEsc.Name.Name != "getEsc" {
		return fmt.Errorf("sixth declaration must be getEsc")
	}
	if err := validateGetEscBody(getEsc); err != nil {
		return err
	}
	return nil
}

func validatePathMatchCompleteComments(file *ast.File) error {
	requiredComments := []string{
		"// scanChunk gets the next segment of pattern, which is a non-star string",
		"// possibly preceded by a star.",
		"// matchChunk checks whether chunk matches the beginning of s.",
		"// Chunk is all single-character operators: literals, char classes, and ?.",
		"// failed records whether the match has failed.",
		"// character class",
		"// possibly negated",
		"// parse all ranges",
		"// getEsc gets a possibly-escaped character from chunk, for a character class.",
	}
	for _, required := range requiredComments {
		found := false
		for _, group := range file.Comments {
			for _, comment := range group.List {
				if comment.Text == required {
					found = true
					break
				}
			}
			if found {
				break
			}
		}
		if !found {
			return fmt.Errorf("path/match source file must preserve parser-backed comment %q", required)
		}
	}
	return nil
}

func validateStringsCompareHeaderComments(file *ast.File) error {
	if len(file.Comments) < 2 {
		return fmt.Errorf("strings compare source file must preserve header and Compare doc comments")
	}
	expected := []string{
		"// Copyright 2015 The Go Authors. All rights reserved.",
		"// Use of this source code is governed by a BSD-style",
		"// license that can be found in the LICENSE file.",
	}
	if !commentGroupTextEquals(file.Comments[0], expected) {
		return fmt.Errorf("strings compare header comments must match the parser-backed source file")
	}
	return nil
}

func validateStringsCompareDoc(compare *ast.FuncDecl) error {
	expected := []string{
		"// Compare returns an integer comparing two strings lexicographically.",
		"// The result will be 0 if a == b, -1 if a < b, and +1 if a > b.",
		"//",
		"// Use Compare when you need to perform a three-way comparison (with",
		"// [slices.SortFunc], for example). It is usually clearer and always faster",
		"// to use the built-in string comparison operators ==, <, >, and so on.",
	}
	if !commentGroupTextEquals(compare.Doc, expected) {
		return fmt.Errorf("func Compare doc comments must match the parser-backed source file")
	}
	return nil
}

func validateStringsCloneHeaderComments(file *ast.File) error {
	if len(file.Comments) < 2 {
		return fmt.Errorf("strings clone source file must preserve header and Clone doc comments")
	}
	expected := []string{
		"// Copyright 2021 The Go Authors. All rights reserved.",
		"// Use of this source code is governed by a BSD-style",
		"// license that can be found in the LICENSE file.",
	}
	if !commentGroupTextEquals(file.Comments[0], expected) {
		return fmt.Errorf("strings clone header comments must match the parser-backed source file")
	}
	return nil
}

func validateCryptoSysrandRandWindowsHeaderComments(file *ast.File) error {
	if len(file.Comments) < 1 {
		return fmt.Errorf("crypto sysrand rand_windows source file must preserve header comments")
	}
	expected := []string{
		"// Copyright 2010 The Go Authors. All rights reserved.",
		"// Use of this source code is governed by a BSD-style",
		"// license that can be found in the LICENSE file.",
	}
	if !commentGroupTextEquals(file.Comments[0], expected) {
		return fmt.Errorf("crypto sysrand rand_windows header comments must match the parser-backed source file")
	}
	return nil
}

func validateCryptoSubtleXorHeaderComments(file *ast.File) error {
	if len(file.Comments) < 1 {
		return fmt.Errorf("crypto subtle xor source file must preserve header comments")
	}
	expected := []string{
		"// Copyright 2022 The Go Authors. All rights reserved.",
		"// Use of this source code is governed by a BSD-style",
		"// license that can be found in the LICENSE file.",
	}
	if !commentGroupTextEquals(file.Comments[0], expected) {
		return fmt.Errorf("crypto subtle xor header comments must match the parser-backed source file")
	}
	return nil
}

func validateStringsCloneDoc(clone *ast.FuncDecl) error {
	expected := []string{
		"// Clone returns a fresh copy of s.",
		"// It guarantees to make a copy of s into a new allocation,",
		"// which can be important when retaining only a small substring",
		"// of a much larger string. Using Clone can help such programs",
		"// use less memory. Of course, since using Clone makes a copy,",
		"// overuse of Clone can make programs use more memory.",
		"// Clone should typically be used only rarely, and only when",
		"// profiling indicates that it is needed.",
		"// For strings of length zero the string \"\" will be returned",
		"// and no allocation is made.",
	}
	if !commentGroupTextEquals(clone.Doc, expected) {
		return fmt.Errorf("func Clone doc comments must match the parser-backed source file")
	}
	return nil
}

func commentGroupTextEquals(group *ast.CommentGroup, expected []string) bool {
	if group == nil || len(group.List) != len(expected) {
		return false
	}
	for index, want := range expected {
		if group.List[index].Text != want {
			return false
		}
	}
	return true
}

func validateSortInterfaceImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 2 {
		return fmt.Errorf("first declaration must be the fmt/sort import block")
	}
	if !importSpecPath(genDecl.Specs[0], "\"fmt\"") || !importSpecPath(genDecl.Specs[1], "\"sort\"") {
		return fmt.Errorf("import block must import fmt then sort")
	}
	return nil
}

func validateSortSearchImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 3 {
		return fmt.Errorf("first declaration must be the fmt/sort/strings import block")
	}
	if !importSpecPath(genDecl.Specs[0], "\"fmt\"") ||
		!importSpecPath(genDecl.Specs[1], "\"sort\"") ||
		!importSpecPath(genDecl.Specs[2], "\"strings\"") {
		return fmt.Errorf("import block must import fmt, sort, then strings")
	}
	return nil
}

func validateStringsCompareImportDecl(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single internal/bytealg import")
	}
	if !importSpecPath(genDecl.Specs[0], "\"internal/bytealg\"") {
		return fmt.Errorf("strings compare import must be internal/bytealg")
	}
	return nil
}

func validateStringsCloneImportDecl(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be the internal/stringslite import block")
	}
	if !importSpecPath(genDecl.Specs[0], "\"internal/stringslite\"") {
		return fmt.Errorf("strings clone import must be internal/stringslite")
	}
	return nil
}

func validateCryptoSysrandRandWindowsImportDecl(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single internal/syscall/windows import")
	}
	if !importSpecPath(genDecl.Specs[0], "\"internal/syscall/windows\"") {
		return fmt.Errorf("crypto sysrand rand_windows import must be internal/syscall/windows")
	}
	return nil
}

func validateCryptoSubtleXorImportDecl(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("first declaration must be a single crypto/internal/fips140/subtle import")
	}
	if !importSpecPath(genDecl.Specs[0], "\"crypto/internal/fips140/subtle\"") {
		return fmt.Errorf("crypto subtle xor import must be crypto/internal/fips140/subtle")
	}
	return nil
}

func validatePathMatchImportBlock(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.IMPORT || !genDecl.Lparen.IsValid() || len(genDecl.Specs) != 3 {
		return fmt.Errorf("first declaration must be the errors/bytealg/utf8 import block")
	}
	if !importSpecPath(genDecl.Specs[0], "\"errors\"") ||
		!importSpecPath(genDecl.Specs[1], "\"internal/bytealg\"") ||
		!importSpecPath(genDecl.Specs[2], "\"unicode/utf8\"") {
		return fmt.Errorf("import block must import errors, internal/bytealg, then unicode/utf8")
	}
	return nil
}

func validateErrBadPatternDeclaration(decl ast.Decl) error {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return fmt.Errorf("ErrBadPattern must be a single package-level var declaration")
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "ErrBadPattern" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("ErrBadPattern ValueSpec must name ErrBadPattern with one initializer")
	}
	call, ok := valueSpec.Values[0].(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "errors", "New") || len(call.Args) != 1 {
		return fmt.Errorf("ErrBadPattern initializer must call errors.New")
	}
	literal, ok := call.Args[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.STRING || literal.Value != "\"syntax error in pattern\"" {
		return fmt.Errorf("ErrBadPattern initializer must use the expected error message")
	}
	return nil
}

func validatePathMatchFunctionBody(match *ast.FuncDecl) error {
	if match.Type == nil || match.Type.Params == nil || match.Type.Results == nil || match.Body == nil {
		return fmt.Errorf("Match must have params, results, and body")
	}
	if !funcTypeHasTwoNamedParamsOneType(match.Type, "pattern", "name", "string") {
		return fmt.Errorf("Match must have signature func Match(pattern, name string) ...")
	}
	if !funcTypeReturnsNamedPair(match.Type, "matched", "bool", "err", "error") {
		return fmt.Errorf("Match must return named matched bool and err error")
	}
	if len(match.Body.List) != 2 {
		return fmt.Errorf("Match body must contain the Pattern loop and final return")
	}
	labeled, ok := match.Body.List[0].(*ast.LabeledStmt)
	if !ok || labeled.Label == nil || labeled.Label.Name != "Pattern" {
		return fmt.Errorf("Match first statement must be the Pattern label")
	}
	loop, ok := labeled.Stmt.(*ast.ForStmt)
	if !ok || loop.Body == nil || !binaryLenIdentInteger(loop.Cond, "pattern", token.GTR, "0") {
		return fmt.Errorf("Pattern label must wrap for len(pattern) > 0")
	}
	if len(loop.Body.List) != 10 {
		return fmt.Errorf("Pattern loop must keep the expected ten statements")
	}
	if !varDeclarationNamedType(loop.Body.List[0], "star", "bool") {
		return fmt.Errorf("Pattern loop first statement must be var star bool")
	}
	if !varDeclarationNamedType(loop.Body.List[1], "chunk", "string") {
		return fmt.Errorf("Pattern loop second statement must be var chunk string")
	}
	if !assignFromCall(loop.Body.List[2], token.ASSIGN, []string{"star", "chunk", "pattern"}, "scanChunk", []string{"pattern"}) {
		return fmt.Errorf("Pattern loop third statement must assign star, chunk, pattern = scanChunk(pattern)")
	}
	if !pathMatchTrailingStarReturn(loop.Body.List[3]) {
		return fmt.Errorf("Pattern loop fourth statement must handle trailing star return")
	}
	if !assignFromCall(loop.Body.List[4], token.DEFINE, []string{"t", "ok", "err"}, "matchChunk", []string{"chunk", "name"}) {
		return fmt.Errorf("Pattern loop fifth statement must assign t, ok, err := matchChunk(chunk, name)")
	}
	if !pathMatchCurrentPositionIf(loop.Body.List[5]) {
		return fmt.Errorf("Pattern loop sixth statement must update name and continue on current-position match")
	}
	if !pathMatchErrReturnIf(loop.Body.List[6]) {
		return fmt.Errorf("Pattern loop seventh statement must return false, err")
	}
	if !pathMatchStarSearchIf(loop.Body.List[7]) {
		return fmt.Errorf("Pattern loop eighth statement must search after star with labeled continue")
	}
	if !pathMatchRemainderValidationLoop(loop.Body.List[8]) {
		return fmt.Errorf("Pattern loop ninth statement must validate the remaining pattern before false nil")
	}
	if !returnFalseNil(loop.Body.List[9]) {
		return fmt.Errorf("Pattern loop tenth statement must return false, nil")
	}
	if !returnLenNameZeroNil(match.Body.List[1]) {
		return fmt.Errorf("Match final statement must return len(name) == 0, nil")
	}
	return nil
}

func validateScanChunkBody(scanChunk *ast.FuncDecl) error {
	if scanChunk.Type == nil || scanChunk.Type.Params == nil || scanChunk.Type.Results == nil || scanChunk.Body == nil {
		return fmt.Errorf("scanChunk must have params, results, and body")
	}
	if !funcTypeHasOneNamedParam(scanChunk.Type, "pattern", "string") || !funcTypeReturnsScanChunk(scanChunk.Type) {
		return fmt.Errorf("scanChunk must have signature func scanChunk(pattern string) (star bool, chunk, rest string)")
	}
	if len(scanChunk.Body.List) != 5 {
		return fmt.Errorf("scanChunk body must keep star trim loop, inrange init, i decl, Scan loop, and return")
	}
	if !scanChunkStarTrimLoop(scanChunk.Body.List[0]) {
		return fmt.Errorf("scanChunk first statement must trim leading stars")
	}
	if !shortVarBooleanLiteral(scanChunk.Body.List[1], "inrange", "false") {
		return fmt.Errorf("scanChunk second statement must initialize inrange := false")
	}
	if !varDeclarationNamedType(scanChunk.Body.List[2], "i", "int") {
		return fmt.Errorf("scanChunk third statement must declare var i int")
	}
	if !scanChunkLabeledScanLoop(scanChunk.Body.List[3]) {
		return fmt.Errorf("scanChunk fourth statement must be the labeled Scan loop with switch cases")
	}
	if !scanChunkReturn(scanChunk.Body.List[4]) {
		return fmt.Errorf("scanChunk final statement must return star, pattern[0:i], pattern[i:]")
	}
	return nil
}

func validateMatchChunkBody(matchChunk *ast.FuncDecl) error {
	if matchChunk.Type == nil || matchChunk.Type.Params == nil || matchChunk.Type.Results == nil || matchChunk.Body == nil {
		return fmt.Errorf("matchChunk must have params, results, and body")
	}
	if !funcTypeHasTwoNamedParamsOneType(matchChunk.Type, "chunk", "s", "string") || !funcTypeReturnsMatchChunk(matchChunk.Type) {
		return fmt.Errorf("matchChunk must have signature func matchChunk(chunk, s string) (rest string, ok bool, err error)")
	}
	if len(matchChunk.Body.List) != 4 {
		return fmt.Errorf("matchChunk body must keep failed init, chunk loop, failed return, and success return")
	}
	if !shortVarBooleanLiteral(matchChunk.Body.List[0], "failed", "false") {
		return fmt.Errorf("matchChunk first statement must initialize failed := false")
	}
	loop, ok := matchChunk.Body.List[1].(*ast.ForStmt)
	if !ok || loop.Body == nil || !binaryLenIdentInteger(loop.Cond, "chunk", token.GTR, "0") {
		return fmt.Errorf("matchChunk second statement must loop while len(chunk) > 0")
	}
	if !blockContainsNode(loop.Body, chunkSwitchWithCharacterCases) {
		return fmt.Errorf("matchChunk loop must switch on chunk[0] with character-class, wildcard, escape, and default cases")
	}
	if !blockContainsNode(loop.Body, func(node ast.Node) bool {
		return callNamed(node, "utf8", "DecodeRuneInString")
	}) {
		return fmt.Errorf("matchChunk loop must decode UTF-8 runes through utf8.DecodeRuneInString")
	}
	if !blockContainsNode(loop.Body, func(node ast.Node) bool {
		return callNamed(node, "", "getEsc")
	}) {
		return fmt.Errorf("matchChunk loop must call getEsc for character ranges")
	}
	if !blockContainsNode(loop.Body, func(node ast.Node) bool {
		branch, ok := node.(*ast.BranchStmt)
		return ok && branch.Tok == token.FALLTHROUGH
	}) {
		return fmt.Errorf("matchChunk escape case must preserve fallthrough")
	}
	if !failedReturn(matchChunk.Body.List[2]) {
		return fmt.Errorf("matchChunk third statement must return empty rest, false, nil when failed")
	}
	if !returnIdentTrueNil(matchChunk.Body.List[3], "s") {
		return fmt.Errorf("matchChunk final statement must return s, true, nil")
	}
	return nil
}

func validateGetEscBody(getEsc *ast.FuncDecl) error {
	if getEsc.Type == nil || getEsc.Type.Params == nil || getEsc.Type.Results == nil || getEsc.Body == nil {
		return fmt.Errorf("getEsc must have params, results, and body")
	}
	if !funcTypeHasOneNamedParam(getEsc.Type, "chunk", "string") || !funcTypeReturnsGetEsc(getEsc.Type) {
		return fmt.Errorf("getEsc must have signature func getEsc(chunk string) (r rune, nchunk string, err error)")
	}
	if len(getEsc.Body.List) != 7 {
		return fmt.Errorf("getEsc body must keep malformed-start check, escape check, decode, RuneError check, nchunk slice, empty-tail check, and bare return")
	}
	if !getEscMalformedStartIf(getEsc.Body.List[0]) {
		return fmt.Errorf("getEsc first statement must reject empty, '-' and ']' chunks")
	}
	if !getEscBackslashIf(getEsc.Body.List[1]) {
		return fmt.Errorf("getEsc second statement must consume an escaped character and reject EOF")
	}
	if !decodeRuneShortVar(getEsc.Body.List[2], "chunk") {
		return fmt.Errorf("getEsc third statement must decode r, n := utf8.DecodeRuneInString(chunk)")
	}
	if !getEscRuneErrorIf(getEsc.Body.List[3]) {
		return fmt.Errorf("getEsc fourth statement must reject utf8.RuneError width one")
	}
	if !assignSliceFromIndex(getEsc.Body.List[4], "nchunk", "chunk", "n") {
		return fmt.Errorf("getEsc fifth statement must set nchunk = chunk[n:]")
	}
	if !getEscEmptyTailIf(getEsc.Body.List[5]) {
		return fmt.Errorf("getEsc sixth statement must reject empty nchunk")
	}
	if !bareReturn(getEsc.Body.List[6]) {
		return fmt.Errorf("getEsc final statement must be a bare return")
	}
	return nil
}

func validatePersonStructDeclaration(decl ast.Decl) error {
	typeSpec, err := singleTypeSpec(decl, "Person")
	if err != nil {
		return err
	}
	structType, ok := typeSpec.Type.(*ast.StructType)
	if !ok || structType.Fields == nil || len(structType.Fields.List) != 2 {
		return fmt.Errorf("Person must be a two-field struct")
	}
	if !namedField(structType.Fields.List[0], "Name", "string") ||
		!namedField(structType.Fields.List[1], "Age", "int") {
		return fmt.Errorf("Person fields must be Name string and Age int")
	}
	return nil
}

func validateByAgeTypeDeclaration(decl ast.Decl) error {
	typeSpec, err := singleTypeSpec(decl, "ByAge")
	if err != nil {
		return err
	}
	if !sliceArrayTypeNamed(typeSpec.Type, "Person") {
		return fmt.Errorf("ByAge must be declared as []Person")
	}
	return nil
}

func validatePersonStringMethod(decl ast.Decl) error {
	method, err := methodDecl(decl, "String", "p", "Person")
	if err != nil {
		return err
	}
	if !funcTypeHasParams(method.Type) || !funcTypeReturns(method.Type, "string") {
		return fmt.Errorf("String must have signature func (p Person) String() string")
	}
	if len(method.Body.List) != 1 {
		return fmt.Errorf("String body must contain one return statement")
	}
	returnStmt, ok := method.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("String body must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "fmt", "Sprintf") || len(call.Args) != 3 {
		return fmt.Errorf("String must return fmt.Sprintf with three arguments")
	}
	return nil
}

func validateByAgeLenMethod(decl ast.Decl) error {
	method, err := methodDecl(decl, "Len", "a", "ByAge")
	if err != nil {
		return err
	}
	if !funcTypeHasParams(method.Type) || !funcTypeReturns(method.Type, "int") {
		return fmt.Errorf("Len must have signature func (a ByAge) Len() int")
	}
	if len(method.Body.List) != 1 {
		return fmt.Errorf("Len body must contain one return statement")
	}
	returnStmt, ok := method.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("Len body must return one expression")
	}
	call, ok := returnStmt.Results[0].(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, "len") || len(call.Args) != 1 || !identNamed(call.Args[0], "a") {
		return fmt.Errorf("Len must return len(a)")
	}
	return nil
}

func validateByAgeSwapMethod(decl ast.Decl) error {
	method, err := methodDecl(decl, "Swap", "a", "ByAge")
	if err != nil {
		return err
	}
	if !funcTypeHasNamedParams(method.Type, "i", "j", "int") || !funcTypeHasNoResults(method.Type) {
		return fmt.Errorf("Swap must have signature func (a ByAge) Swap(i, j int)")
	}
	if len(method.Body.List) != 1 {
		return fmt.Errorf("Swap body must contain one assignment statement")
	}
	assign, ok := method.Body.List[0].(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 2 || len(assign.Rhs) != 2 {
		return fmt.Errorf("Swap body must assign two indexed values")
	}
	if !indexExprNamed(assign.Lhs[0], "a", "i") || !indexExprNamed(assign.Lhs[1], "a", "j") ||
		!indexExprNamed(assign.Rhs[0], "a", "j") || !indexExprNamed(assign.Rhs[1], "a", "i") {
		return fmt.Errorf("Swap assignment must swap a[i] and a[j]")
	}
	return nil
}

func validateByAgeLessMethod(decl ast.Decl) error {
	method, err := methodDecl(decl, "Less", "a", "ByAge")
	if err != nil {
		return err
	}
	if !funcTypeHasNamedParams(method.Type, "i", "j", "int") || !funcTypeReturns(method.Type, "bool") {
		return fmt.Errorf("Less must have signature func (a ByAge) Less(i, j int) bool")
	}
	if len(method.Body.List) != 1 {
		return fmt.Errorf("Less body must contain one return statement")
	}
	returnStmt, ok := method.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return fmt.Errorf("Less body must return one expression")
	}
	binary, ok := returnStmt.Results[0].(*ast.BinaryExpr)
	if !ok || binary.Op != token.LSS {
		return fmt.Errorf("Less must return a less-than comparison")
	}
	if !selectorIndexExprNamed(binary.X, "a", "i", "Age") ||
		!selectorIndexExprNamed(binary.Y, "a", "j", "Age") {
		return fmt.Errorf("Less must compare a[i].Age and a[j].Age")
	}
	return nil
}

func functionTypeReturnsUnnamedInt(funcType *ast.FuncType) bool {
	if funcType == nil || funcType.Params == nil || funcType.Results == nil {
		return false
	}
	if len(funcType.Params.List) != 0 || len(funcType.Results.List) != 1 {
		return false
	}
	result := funcType.Results.List[0]
	return len(result.Names) == 0 && identNamed(result.Type, "int")
}

func variadicIntFunction(funcDecl *ast.FuncDecl, name string) bool {
	if funcDecl == nil || funcDecl.Name == nil || funcDecl.Name.Name != name {
		return false
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return false
	}
	if len(funcDecl.Type.Params.List) != 1 || len(funcDecl.Type.Results.List) != 1 {
		return false
	}
	param := funcDecl.Type.Params.List[0]
	if len(param.Names) != 1 || param.Names[0].Name != "values" {
		return false
	}
	ellipsis, ok := param.Type.(*ast.Ellipsis)
	if !ok || !identNamed(ellipsis.Elt, "int") {
		return false
	}
	result := funcDecl.Type.Results.List[0]
	return len(result.Names) == 0 && identNamed(result.Type, "int")
}

func validateIntegerBuildFunction(file *ast.File, fixture string, statementCount int) (*ast.FuncDecl, error) {
	if file.Name == nil || file.Name.Name != "main" {
		return nil, fmt.Errorf("%s fixture must use package main", fixture)
	}
	if len(file.Decls) != 1 {
		return nil, fmt.Errorf("%s fixture must contain one function declaration", fixture)
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return nil, fmt.Errorf("%s declaration must be func build", fixture)
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return nil, fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 0 {
		return nil, fmt.Errorf("func build must not have parameters")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return nil, fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return nil, fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != statementCount {
		return nil, fmt.Errorf("func build body must contain %d statements", statementCount)
	}
	return funcDecl, nil
}

func validateIntegerBuildFunctionWithValueParam(file *ast.File, fixture string, statementCount int) (*ast.FuncDecl, error) {
	if file.Name == nil || file.Name.Name != "main" {
		return nil, fmt.Errorf("%s fixture must use package main", fixture)
	}
	if len(file.Decls) != 1 {
		return nil, fmt.Errorf("%s fixture must contain one function declaration", fixture)
	}
	funcDecl, ok := file.Decls[0].(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != "build" {
		return nil, fmt.Errorf("%s declaration must be func build", fixture)
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil || funcDecl.Type.Results == nil || funcDecl.Body == nil {
		return nil, fmt.Errorf("func build must have params, results, and body")
	}
	if len(funcDecl.Type.Params.List) != 1 {
		return nil, fmt.Errorf("func build must have one parameter")
	}
	param := funcDecl.Type.Params.List[0]
	if len(param.Names) != 1 || param.Names[0].Name != "value" || !identNamed(param.Type, "any") {
		return nil, fmt.Errorf("func build parameter must be value any")
	}
	if len(funcDecl.Type.Results.List) != 1 {
		return nil, fmt.Errorf("func build must have one result field")
	}
	result := funcDecl.Type.Results.List[0]
	if len(result.Names) != 0 || !identNamed(result.Type, "int") {
		return nil, fmt.Errorf("func build result must be unnamed int")
	}
	if len(funcDecl.Body.List) != statementCount {
		return nil, fmt.Errorf("func build body must contain %d statements", statementCount)
	}
	return funcDecl, nil
}

func validateInitialCountDeclaration(stmt ast.Stmt, fixture string) error {
	declStmt, ok := stmt.(*ast.DeclStmt)
	if !ok {
		return fmt.Errorf("%s first statement must be a declaration statement", fixture)
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return fmt.Errorf("%s local integer declaration must be a single var declaration", fixture)
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	if !ok || len(valueSpec.Names) != 1 || valueSpec.Names[0].Name != "count" || len(valueSpec.Values) != 1 {
		return fmt.Errorf("%s local integer declaration must be var count = <integer>", fixture)
	}
	if valueSpec.Type != nil {
		return fmt.Errorf("%s local integer declaration must omit ValueSpec.Type", fixture)
	}
	initialLiteral, ok := valueSpec.Values[0].(*ast.BasicLit)
	if !ok || initialLiteral.Kind != token.INT || initialLiteral.Value != "1" {
		return fmt.Errorf("%s local integer declaration value must be the integer literal 1", fixture)
	}
	return nil
}

func validateCountIntegerCondition(expr ast.Expr, operator token.Token, value string, fixture string) error {
	binary, ok := expr.(*ast.BinaryExpr)
	if !ok {
		return fmt.Errorf("%s must be a binary expression", fixture)
	}
	if !identNamed(binary.X, "count") {
		return fmt.Errorf("%s left operand must be count", fixture)
	}
	if binary.Op != operator {
		return fmt.Errorf("%s must use the %s operator", fixture, operator.String())
	}
	literal, ok := binary.Y.(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != value {
		return fmt.Errorf("%s right operand must be the integer literal %s", fixture, value)
	}
	return nil
}

func validateCountIntegerAssignment(stmt ast.Stmt, value string, fixture string) error {
	assignStmt, ok := stmt.(*ast.AssignStmt)
	if !ok {
		return fmt.Errorf("%s statement must be an assignment", fixture)
	}
	if assignStmt.Tok != token.ASSIGN || len(assignStmt.Lhs) != 1 || len(assignStmt.Rhs) != 1 {
		return fmt.Errorf("%s assignment must use one target and one value with =", fixture)
	}
	if !identNamed(assignStmt.Lhs[0], "count") {
		return fmt.Errorf("%s assignment target must be count", fixture)
	}
	literal, ok := assignStmt.Rhs[0].(*ast.BasicLit)
	if !ok || literal.Kind != token.INT || literal.Value != value {
		return fmt.Errorf("%s assignment value must be the integer literal %s", fixture, value)
	}
	return nil
}

func validateCountReturn(stmt ast.Stmt, position string) error {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 || !identNamed(returnStmt.Results[0], "count") {
		return fmt.Errorf("%s must return count", position)
	}
	return nil
}

func identNamed(expr ast.Expr, name string) bool {
	expr = unparenExpr(expr)
	ident, ok := expr.(*ast.Ident)
	return ok && ident.Name == name
}

func fieldNamesType(field *ast.Field, names []string, typeName string) bool {
	if field == nil || len(field.Names) != len(names) || !identNamed(field.Type, typeName) {
		return false
	}
	for index, name := range names {
		if field.Names[index] == nil || field.Names[index].Name != name {
			return false
		}
	}
	return true
}

func unparenExpr(expr ast.Expr) ast.Expr {
	for {
		paren, ok := expr.(*ast.ParenExpr)
		if !ok {
			return expr
		}
		expr = paren.X
	}
}

func starExprIdent(expr ast.Expr, name string) bool {
	expr = unparenExpr(expr)
	star, ok := expr.(*ast.StarExpr)
	return ok && identNamed(star.X, name)
}

func sliceArrayTypeNamed(expr ast.Expr, elem string) bool {
	arrayType, ok := expr.(*ast.ArrayType)
	return ok && arrayType.Len == nil && identNamed(arrayType.Elt, elem)
}

func fixedArrayTypeNamed(expr ast.Expr, length string, elem string) bool {
	arrayType, ok := expr.(*ast.ArrayType)
	if !ok || !identNamed(arrayType.Elt, elem) {
		return false
	}
	literal, ok := arrayType.Len.(*ast.BasicLit)
	return ok && literal.Kind == token.INT && literal.Value == length
}

func fixedArrayField(field *ast.Field, name string, length string, elem string) bool {
	return field != nil &&
		len(field.Names) == 1 &&
		field.Names[0].Name == name &&
		fixedArrayTypeNamed(field.Type, length, elem)
}

func sliceArrayField(field *ast.Field, name string, elem string) bool {
	return field != nil &&
		len(field.Names) == 1 &&
		field.Names[0].Name == name &&
		sliceArrayTypeNamed(field.Type, elem)
}

func chanTypeNamed(expr ast.Expr, dir ast.ChanDir, elem string) bool {
	chanType, ok := expr.(*ast.ChanType)
	return ok && chanType.Dir == dir && identNamed(chanType.Value, elem)
}

func mapChannelField(field *ast.Field, name string, key string, dir ast.ChanDir, elem string) bool {
	if field == nil || len(field.Names) != 1 || field.Names[0].Name != name {
		return false
	}
	mapType, ok := field.Type.(*ast.MapType)
	return ok && identNamed(mapType.Key, key) && chanTypeNamed(mapType.Value, dir, elem)
}

func importSpecPath(spec ast.Spec, quotedPath string) bool {
	importSpec, ok := spec.(*ast.ImportSpec)
	return ok && importSpec.Path != nil && importSpec.Path.Value == quotedPath
}

func singleTypeSpec(decl ast.Decl, name string) (*ast.TypeSpec, error) {
	genDecl, ok := decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.TYPE || genDecl.Lparen.IsValid() || len(genDecl.Specs) != 1 {
		return nil, fmt.Errorf("%s must be a single package-level type declaration", name)
	}
	typeSpec, ok := genDecl.Specs[0].(*ast.TypeSpec)
	if !ok || typeSpec.Name == nil || typeSpec.Name.Name != name || typeSpec.Assign.IsValid() {
		return nil, fmt.Errorf("type declaration must define %s", name)
	}
	return typeSpec, nil
}

func namedField(field *ast.Field, name string, typ string) bool {
	return field != nil &&
		len(field.Names) == 1 &&
		field.Names[0].Name == name &&
		identNamed(field.Type, typ)
}

func methodDecl(decl ast.Decl, name string, receiverName string, receiverType string) (*ast.FuncDecl, error) {
	funcDecl, ok := decl.(*ast.FuncDecl)
	if !ok || funcDecl.Name == nil || funcDecl.Name.Name != name || funcDecl.Body == nil {
		return nil, fmt.Errorf("declaration must be method %s", name)
	}
	if funcDecl.Recv == nil || len(funcDecl.Recv.List) != 1 {
		return nil, fmt.Errorf("method %s must have one receiver field", name)
	}
	receiver := funcDecl.Recv.List[0]
	if len(receiver.Names) != 1 || receiver.Names[0].Name != receiverName || !identNamed(receiver.Type, receiverType) {
		return nil, fmt.Errorf("method %s receiver must be %s %s", name, receiverName, receiverType)
	}
	if funcDecl.Type == nil || funcDecl.Type.Params == nil {
		return nil, fmt.Errorf("method %s must have a function type with parameters", name)
	}
	return funcDecl, nil
}

func funcTypeHasParams(funcType *ast.FuncType) bool {
	return funcType != nil && funcType.Params != nil && len(funcType.Params.List) == 0
}

func funcTypeHasNamedParams(funcType *ast.FuncType, firstName string, secondName string, typ string) bool {
	if funcType == nil || funcType.Params == nil || len(funcType.Params.List) != 1 {
		return false
	}
	field := funcType.Params.List[0]
	return len(field.Names) == 2 &&
		field.Names[0].Name == firstName &&
		field.Names[1].Name == secondName &&
		identNamed(field.Type, typ)
}

func funcTypeReturns(funcType *ast.FuncType, typ string) bool {
	if funcType == nil || funcType.Results == nil || len(funcType.Results.List) != 1 {
		return false
	}
	result := funcType.Results.List[0]
	return len(result.Names) == 0 && identNamed(result.Type, typ)
}

func funcTypeReturnsPointer(funcType *ast.FuncType, typ string) bool {
	if funcType == nil || funcType.Results == nil || len(funcType.Results.List) != 1 {
		return false
	}
	result := funcType.Results.List[0]
	if len(result.Names) != 0 {
		return false
	}
	star, ok := result.Type.(*ast.StarExpr)
	return ok && identNamed(star.X, typ)
}

func funcTypeHasNoResults(funcType *ast.FuncType) bool {
	return funcType != nil && (funcType.Results == nil || len(funcType.Results.List) == 0)
}

func selectorNamed(expr ast.Expr, receiver string, selector string) bool {
	selectorExpr, ok := expr.(*ast.SelectorExpr)
	return ok && identNamed(selectorExpr.X, receiver) && selectorExpr.Sel != nil && selectorExpr.Sel.Name == selector
}

func exprStmtCall(stmt ast.Stmt, receiver string, selector string, arg string) bool {
	exprStmt, ok := stmt.(*ast.ExprStmt)
	if !ok {
		return false
	}
	call, ok := exprStmt.X.(*ast.CallExpr)
	return ok &&
		selectorNamed(call.Fun, receiver, selector) &&
		len(call.Args) == 1 &&
		identNamed(call.Args[0], arg)
}

func sortSortByAgePeopleCall(stmt ast.Stmt) bool {
	exprStmt, ok := stmt.(*ast.ExprStmt)
	if !ok {
		return false
	}
	call, ok := exprStmt.X.(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "sort", "Sort") || len(call.Args) != 1 {
		return false
	}
	conversion, ok := call.Args[0].(*ast.CallExpr)
	return ok && identNamed(conversion.Fun, "ByAge") && len(conversion.Args) == 1 && identNamed(conversion.Args[0], "people")
}

func sortSlicePeopleClosureCall(stmt ast.Stmt) bool {
	exprStmt, ok := stmt.(*ast.ExprStmt)
	if !ok {
		return false
	}
	call, ok := exprStmt.X.(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "sort", "Slice") || len(call.Args) != 2 || !identNamed(call.Args[0], "people") {
		return false
	}
	funcLit, ok := call.Args[1].(*ast.FuncLit)
	if !ok || !funcTypeHasNamedParams(funcLit.Type, "i", "j", "int") || !funcTypeReturns(funcLit.Type, "bool") {
		return false
	}
	if funcLit.Body == nil || len(funcLit.Body.List) != 1 {
		return false
	}
	returnStmt, ok := funcLit.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return false
	}
	binary, ok := returnStmt.Results[0].(*ast.BinaryExpr)
	return ok &&
		binary.Op == token.GTR &&
		selectorIndexExprNamed(binary.X, "people", "i", "Age") &&
		selectorIndexExprNamed(binary.Y, "people", "j", "Age")
}

func shortVarIntegerSlice(stmt ast.Stmt, target string, values []string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], target) {
		return false
	}
	literal, ok := assign.Rhs[0].(*ast.CompositeLit)
	if !ok || !sliceArrayTypeNamed(literal.Type, "int") || len(literal.Elts) != len(values) {
		return false
	}
	for index, value := range values {
		basicLit, ok := literal.Elts[index].(*ast.BasicLit)
		if !ok || basicLit.Kind != token.INT || basicLit.Value != value {
			return false
		}
	}
	return true
}

func shortVarIntegerLiteral(stmt ast.Stmt, target string, value string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], target) {
		return false
	}
	literal, ok := assign.Rhs[0].(*ast.BasicLit)
	return ok && literal.Kind == token.INT && literal.Value == value
}

func sortSearchShortVarCall(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], "i") {
		return false
	}
	call, ok := assign.Rhs[0].(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "sort", "Search") || len(call.Args) != 2 {
		return false
	}
	if !lenCallArg(call.Args[0], "a") {
		return false
	}
	funcLit, ok := call.Args[1].(*ast.FuncLit)
	if !ok || !funcTypeHasOneNamedParam(funcLit.Type, "i", "int") || !funcTypeReturns(funcLit.Type, "bool") {
		return false
	}
	if funcLit.Body == nil || len(funcLit.Body.List) != 1 {
		return false
	}
	returnStmt, ok := funcLit.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return false
	}
	binary, ok := returnStmt.Results[0].(*ast.BinaryExpr)
	return ok && binary.Op == token.GEQ && indexExprNamed(binary.X, "a", "i") && identNamed(binary.Y, "x")
}

func sortSearchIfElse(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Init != nil || ifStmt.Body == nil || ifStmt.Else == nil {
		return false
	}
	cond, ok := ifStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.LAND {
		return false
	}
	left, ok := cond.X.(*ast.BinaryExpr)
	if !ok || left.Op != token.LSS || !identNamed(left.X, "i") || !lenCallArg(left.Y, "a") {
		return false
	}
	right, ok := unparenExpr(cond.Y).(*ast.BinaryExpr)
	if !ok || right.Op != token.EQL || !indexExprNamed(right.X, "a", "i") || !identNamed(right.Y, "x") {
		return false
	}
	if len(ifStmt.Body.List) != 1 || !printfCall(ifStmt.Body.List[0], "found %d at index %d in %v\n", []string{"x", "i", "a"}) {
		return false
	}
	elseBlock, ok := ifStmt.Else.(*ast.BlockStmt)
	return ok &&
		len(elseBlock.List) == 1 &&
		printfCall(elseBlock.List[0], "%d not found in %v\n", []string{"x", "a"})
}

func varDeclarationNamedType(stmt ast.Stmt, name string, typ string) bool {
	declStmt, ok := stmt.(*ast.DeclStmt)
	if !ok {
		return false
	}
	genDecl, ok := declStmt.Decl.(*ast.GenDecl)
	if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
		return false
	}
	valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)
	return ok &&
		len(valueSpec.Names) == 1 &&
		valueSpec.Names[0].Name == name &&
		identNamed(valueSpec.Type, typ) &&
		len(valueSpec.Values) == 0
}

func assignFromCall(stmt ast.Stmt, tok token.Token, lhs []string, function string, args []string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != tok || len(assign.Lhs) != len(lhs) || len(assign.Rhs) != 1 {
		return false
	}
	for index, name := range lhs {
		if !identNamed(assign.Lhs[index], name) {
			return false
		}
	}
	call, ok := assign.Rhs[0].(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, function) || len(call.Args) != len(args) {
		return false
	}
	for index, name := range args {
		if !identNamed(call.Args[index], name) {
			return false
		}
	}
	return true
}

func pathMatchTrailingStarReturn(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 {
		return false
	}
	cond, ok := ifStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.LAND || !identNamed(cond.X, "star") {
		return false
	}
	right, ok := unparenExpr(cond.Y).(*ast.BinaryExpr)
	if !ok || right.Op != token.EQL || !identNamed(right.X, "chunk") {
		return false
	}
	emptyLiteral, ok := right.Y.(*ast.BasicLit)
	if !ok || emptyLiteral.Kind != token.STRING || emptyLiteral.Value != "\"\"" {
		return false
	}
	returnStmt, ok := ifStmt.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 2 || !identNamed(returnStmt.Results[1], "nil") {
		return false
	}
	binary, ok := returnStmt.Results[0].(*ast.BinaryExpr)
	return ok && binary.Op == token.LSS && bytealgIndexSlashCall(binary.X) && integerLiteral(binary.Y, "0")
}

func pathMatchCurrentPositionIf(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 2 {
		return false
	}
	cond, ok := ifStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.LAND || !identNamed(cond.X, "ok") {
		return false
	}
	right, ok := unparenExpr(cond.Y).(*ast.BinaryExpr)
	if !ok || right.Op != token.LOR {
		return false
	}
	leftCall, ok := right.X.(*ast.BinaryExpr)
	if !ok || leftCall.Op != token.EQL || !lenCallArg(leftCall.X, "t") || !integerLiteral(leftCall.Y, "0") {
		return false
	}
	rightCall, ok := right.Y.(*ast.BinaryExpr)
	if !ok || rightCall.Op != token.GTR || !lenCallArg(rightCall.X, "pattern") || !integerLiteral(rightCall.Y, "0") {
		return false
	}
	assign, ok := ifStmt.Body.List[0].(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 ||
		!identNamed(assign.Lhs[0], "name") || !identNamed(assign.Rhs[0], "t") {
		return false
	}
	branch, ok := ifStmt.Body.List[1].(*ast.BranchStmt)
	return ok && branch.Tok == token.CONTINUE && branch.Label == nil
}

func pathMatchErrReturnIf(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 {
		return false
	}
	cond, ok := ifStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.NEQ || !identNamed(cond.X, "err") || !identNamed(cond.Y, "nil") {
		return false
	}
	returnStmt, ok := ifStmt.Body.List[0].(*ast.ReturnStmt)
	return ok &&
		len(returnStmt.Results) == 2 &&
		identNamed(returnStmt.Results[1], "err") &&
		identNamed(returnStmt.Results[0], "false")
}

func pathMatchStarSearchIf(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 || !identNamed(ifStmt.Cond, "star") {
		return false
	}
	forStmt, ok := ifStmt.Body.List[0].(*ast.ForStmt)
	if !ok || forStmt.Body == nil || len(forStmt.Body.List) != 3 {
		return false
	}
	if !forInitInteger(forStmt.Init, "i", "0") || !incDecIdent(forStmt.Post, "i", token.INC) {
		return false
	}
	cond, ok := forStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.LAND {
		return false
	}
	left, ok := cond.X.(*ast.BinaryExpr)
	if !ok || left.Op != token.LSS || !identNamed(left.X, "i") || !lenCallArg(left.Y, "name") {
		return false
	}
	right, ok := cond.Y.(*ast.BinaryExpr)
	if !ok || right.Op != token.NEQ || !indexExprNamed(right.X, "name", "i") {
		return false
	}
	slash, ok := right.Y.(*ast.BasicLit)
	if !ok || slash.Kind != token.CHAR || slash.Value != "'/'" {
		return false
	}
	if !assignFromMatchChunkNameSlice(forStmt.Body.List[0]) {
		return false
	}
	okIf, ok := forStmt.Body.List[1].(*ast.IfStmt)
	if !ok || okIf.Body == nil || len(okIf.Body.List) != 3 || !identNamed(okIf.Cond, "ok") {
		return false
	}
	if !pathMatchLastChunkContinue(okIf.Body.List[0]) {
		return false
	}
	assign, ok := okIf.Body.List[1].(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 ||
		!identNamed(assign.Lhs[0], "name") || !identNamed(assign.Rhs[0], "t") {
		return false
	}
	branch, ok := okIf.Body.List[2].(*ast.BranchStmt)
	if !ok || branch.Tok != token.CONTINUE || branch.Label == nil || branch.Label.Name != "Pattern" {
		return false
	}
	errIf, ok := forStmt.Body.List[2].(*ast.IfStmt)
	if !ok || !pathMatchErrReturnIf(errIf) {
		return false
	}
	return true
}

func pathMatchRemainderValidationLoop(stmt ast.Stmt) bool {
	forStmt, ok := stmt.(*ast.ForStmt)
	if !ok || forStmt.Body == nil || len(forStmt.Body.List) != 2 || !binaryLenIdentInteger(forStmt.Cond, "pattern", token.GTR, "0") {
		return false
	}
	assign, ok := forStmt.Body.List[0].(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 3 || len(assign.Rhs) != 1 ||
		!identNamed(assign.Lhs[0], "_") || !identNamed(assign.Lhs[1], "chunk") || !identNamed(assign.Lhs[2], "pattern") {
		return false
	}
	call, ok := assign.Rhs[0].(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, "scanChunk") || len(call.Args) != 1 || !identNamed(call.Args[0], "pattern") {
		return false
	}
	ifStmt, ok := forStmt.Body.List[1].(*ast.IfStmt)
	if !ok || ifStmt.Init == nil || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 {
		return false
	}
	init, ok := ifStmt.Init.(*ast.AssignStmt)
	if !ok || init.Tok != token.DEFINE || len(init.Lhs) != 3 || len(init.Rhs) != 1 ||
		!identNamed(init.Lhs[0], "_") || !identNamed(init.Lhs[1], "_") || !identNamed(init.Lhs[2], "err") {
		return false
	}
	initCall, ok := init.Rhs[0].(*ast.CallExpr)
	if !ok || !identNamed(initCall.Fun, "matchChunk") || len(initCall.Args) != 2 || !identNamed(initCall.Args[0], "chunk") {
		return false
	}
	emptyLit, ok := initCall.Args[1].(*ast.BasicLit)
	if !ok || emptyLit.Kind != token.STRING || emptyLit.Value != "\"\"" {
		return false
	}
	cond, ok := ifStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.NEQ || !identNamed(cond.X, "err") || !identNamed(cond.Y, "nil") {
		return false
	}
	returnStmt, ok := ifStmt.Body.List[0].(*ast.ReturnStmt)
	return ok && len(returnStmt.Results) == 2 && identNamed(returnStmt.Results[0], "false") && identNamed(returnStmt.Results[1], "err")
}

func returnLenNameZeroNil(stmt ast.Stmt) bool {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 2 || !identNamed(returnStmt.Results[1], "nil") {
		return false
	}
	binary, ok := returnStmt.Results[0].(*ast.BinaryExpr)
	return ok && binary.Op == token.EQL && lenCallArg(binary.X, "name") && integerLiteral(binary.Y, "0")
}

func returnFalseNil(stmt ast.Stmt) bool {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	return ok &&
		len(returnStmt.Results) == 2 &&
		identNamed(returnStmt.Results[0], "false") &&
		identNamed(returnStmt.Results[1], "nil")
}

func bytealgIndexSlashCall(expr ast.Expr) bool {
	call, ok := expr.(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "bytealg", "IndexByteString") || len(call.Args) != 2 || !identNamed(call.Args[0], "name") {
		return false
	}
	literal, ok := call.Args[1].(*ast.BasicLit)
	return ok && literal.Kind == token.CHAR && literal.Value == "'/'"
}

func integerLiteral(expr ast.Expr, value string) bool {
	literal, ok := expr.(*ast.BasicLit)
	return ok && literal.Kind == token.INT && literal.Value == value
}

func binaryLenIdentInteger(expr ast.Expr, name string, op token.Token, value string) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	return ok && binary.Op == op && lenCallArg(binary.X, name) && integerLiteral(binary.Y, value)
}

func assignFromMatchChunkNameSlice(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 3 || len(assign.Rhs) != 1 ||
		!identNamed(assign.Lhs[0], "t") || !identNamed(assign.Lhs[1], "ok") || !identNamed(assign.Lhs[2], "err") {
		return false
	}
	call, ok := assign.Rhs[0].(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, "matchChunk") || len(call.Args) != 2 || !identNamed(call.Args[0], "chunk") {
		return false
	}
	slice, ok := call.Args[1].(*ast.SliceExpr)
	return ok && identNamed(slice.X, "name") && slice.Low != nil && binaryIdentInteger(slice.Low, "i", token.ADD, "1") && slice.High == nil && slice.Max == nil
}

func pathMatchLastChunkContinue(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 {
		return false
	}
	cond, ok := ifStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.LAND {
		return false
	}
	left, ok := cond.X.(*ast.BinaryExpr)
	if !ok || left.Op != token.EQL || !lenCallArg(left.X, "pattern") || !integerLiteral(left.Y, "0") {
		return false
	}
	right, ok := cond.Y.(*ast.BinaryExpr)
	if !ok || right.Op != token.GTR || !lenCallArg(right.X, "t") || !integerLiteral(right.Y, "0") {
		return false
	}
	branch, ok := ifStmt.Body.List[0].(*ast.BranchStmt)
	return ok && branch.Tok == token.CONTINUE && branch.Label == nil
}

func funcTypeReturnsScanChunk(funcType *ast.FuncType) bool {
	if funcType == nil || funcType.Results == nil || len(funcType.Results.List) != 2 {
		return false
	}
	first := funcType.Results.List[0]
	second := funcType.Results.List[1]
	return len(first.Names) == 1 &&
		first.Names[0].Name == "star" &&
		identNamed(first.Type, "bool") &&
		len(second.Names) == 2 &&
		second.Names[0].Name == "chunk" &&
		second.Names[1].Name == "rest" &&
		identNamed(second.Type, "string")
}

func funcTypeReturnsMatchChunk(funcType *ast.FuncType) bool {
	if funcType == nil || funcType.Results == nil || len(funcType.Results.List) != 3 {
		return false
	}
	expected := []struct {
		name string
		typ  string
	}{
		{name: "rest", typ: "string"},
		{name: "ok", typ: "bool"},
		{name: "err", typ: "error"},
	}
	for index, want := range expected {
		field := funcType.Results.List[index]
		if len(field.Names) != 1 || field.Names[0].Name != want.name || !identNamed(field.Type, want.typ) {
			return false
		}
	}
	return true
}

func funcTypeReturnsGetEsc(funcType *ast.FuncType) bool {
	if funcType == nil || funcType.Results == nil || len(funcType.Results.List) != 3 {
		return false
	}
	expected := []struct {
		name string
		typ  string
	}{
		{name: "r", typ: "rune"},
		{name: "nchunk", typ: "string"},
		{name: "err", typ: "error"},
	}
	for index, want := range expected {
		field := funcType.Results.List[index]
		if len(field.Names) != 1 || field.Names[0].Name != want.name || !identNamed(field.Type, want.typ) {
			return false
		}
	}
	return true
}

func shortVarBooleanLiteral(stmt ast.Stmt, target string, value string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.DEFINE &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		identNamed(assign.Lhs[0], target) &&
		identNamed(assign.Rhs[0], value)
}

func scanChunkStarTrimLoop(stmt ast.Stmt) bool {
	forStmt, ok := stmt.(*ast.ForStmt)
	if !ok || forStmt.Init != nil || forStmt.Post != nil || forStmt.Body == nil || len(forStmt.Body.List) != 2 {
		return false
	}
	cond, ok := forStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.LAND {
		return false
	}
	if !binaryLenIdentInteger(cond.X, "pattern", token.GTR, "0") ||
		!binaryIndexChar(cond.Y, "pattern", "0", token.EQL, "'*'") {
		return false
	}
	return assignSliceFromLiteral(forStmt.Body.List[0], "pattern", "pattern", "1") &&
		assignIdentBool(forStmt.Body.List[1], "star", "true")
}

func scanChunkLabeledScanLoop(stmt ast.Stmt) bool {
	labeled, ok := stmt.(*ast.LabeledStmt)
	if !ok || labeled.Label == nil || labeled.Label.Name != "Scan" {
		return false
	}
	forStmt, ok := labeled.Stmt.(*ast.ForStmt)
	if !ok || forStmt.Body == nil || len(forStmt.Body.List) != 1 {
		return false
	}
	if !forInitAssignInteger(forStmt.Init, "i", "0") ||
		!binaryIdentLenCall(forStmt.Cond, "i", token.LSS, "pattern") ||
		!incDecIdent(forStmt.Post, "i", token.INC) {
		return false
	}
	switchStmt, ok := forStmt.Body.List[0].(*ast.SwitchStmt)
	if !ok || !switchTagIndex(switchStmt, "pattern", "i") || switchStmt.Body == nil || len(switchStmt.Body.List) != 4 {
		return false
	}
	return switchHasCaseLiteral(switchStmt, "'\\\\'") &&
		switchHasCaseLiteral(switchStmt, "'['") &&
		switchHasCaseLiteral(switchStmt, "']'") &&
		switchHasCaseLiteral(switchStmt, "'*'") &&
		switchContainsLabeledBreak(switchStmt, "Scan")
}

func scanChunkReturn(stmt ast.Stmt) bool {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 3 || !identNamed(returnStmt.Results[0], "star") {
		return false
	}
	firstSlice, ok := returnStmt.Results[1].(*ast.SliceExpr)
	if !ok || !identNamed(firstSlice.X, "pattern") || !integerLiteral(firstSlice.Low, "0") || !identNamed(firstSlice.High, "i") || firstSlice.Max != nil {
		return false
	}
	secondSlice, ok := returnStmt.Results[2].(*ast.SliceExpr)
	return ok && identNamed(secondSlice.X, "pattern") && identNamed(secondSlice.Low, "i") && secondSlice.High == nil && secondSlice.Max == nil
}

func chunkSwitchWithCharacterCases(node ast.Node) bool {
	switchStmt, ok := node.(*ast.SwitchStmt)
	if !ok || !switchTagIndex(switchStmt, "chunk", "0") || switchStmt.Body == nil {
		return false
	}
	return switchHasCaseLiteral(switchStmt, "'['") &&
		switchHasCaseLiteral(switchStmt, "'?'") &&
		switchHasCaseLiteral(switchStmt, "'\\\\'") &&
		switchHasDefault(switchStmt)
}

func failedReturn(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 || !identNamed(ifStmt.Cond, "failed") {
		return false
	}
	return returnStringFalseNil(ifStmt.Body.List[0], "\"\"")
}

func returnIdentTrueNil(stmt ast.Stmt, name string) bool {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	return ok &&
		len(returnStmt.Results) == 3 &&
		identNamed(returnStmt.Results[0], name) &&
		identNamed(returnStmt.Results[1], "true") &&
		identNamed(returnStmt.Results[2], "nil")
}

func getEscMalformedStartIf(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 2 {
		return false
	}
	hasEmpty := exprContains(ifStmt.Cond, func(expr ast.Expr) bool {
		binary, ok := expr.(*ast.BinaryExpr)
		return ok && binary.Op == token.EQL && lenCallArg(binary.X, "chunk") && integerLiteral(binary.Y, "0")
	})
	hasDash := exprContains(ifStmt.Cond, func(expr ast.Expr) bool {
		return binaryIndexChar(expr, "chunk", "0", token.EQL, "'-'")
	})
	hasBracket := exprContains(ifStmt.Cond, func(expr ast.Expr) bool {
		return binaryIndexChar(expr, "chunk", "0", token.EQL, "']'")
	})
	return hasEmpty && hasDash && hasBracket &&
		assignErrBadPattern(ifStmt.Body.List[0]) &&
		bareReturn(ifStmt.Body.List[1])
}

func getEscBackslashIf(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 2 || !binaryIndexChar(ifStmt.Cond, "chunk", "0", token.EQL, "'\\\\'") {
		return false
	}
	if !assignSliceFromLiteral(ifStmt.Body.List[0], "chunk", "chunk", "1") {
		return false
	}
	nested, ok := ifStmt.Body.List[1].(*ast.IfStmt)
	if !ok || nested.Body == nil || len(nested.Body.List) != 2 || !binaryLenIdentInteger(nested.Cond, "chunk", token.EQL, "0") {
		return false
	}
	return assignErrBadPattern(nested.Body.List[0]) && bareReturn(nested.Body.List[1])
}

func decodeRuneShortVar(stmt ast.Stmt, arg string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 2 || len(assign.Rhs) != 1 ||
		!identNamed(assign.Lhs[0], "r") || !identNamed(assign.Lhs[1], "n") {
		return false
	}
	return callNamed(assign.Rhs[0], "utf8", "DecodeRuneInString") &&
		len(assign.Rhs[0].(*ast.CallExpr).Args) == 1 &&
		identNamed(assign.Rhs[0].(*ast.CallExpr).Args[0], arg)
}

func getEscRuneErrorIf(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 {
		return false
	}
	cond, ok := ifStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.LAND {
		return false
	}
	left, ok := cond.X.(*ast.BinaryExpr)
	if !ok || left.Op != token.EQL || !identNamed(left.X, "r") || !selectorNamed(left.Y, "utf8", "RuneError") {
		return false
	}
	return binaryIdentInteger(cond.Y, "n", token.EQL, "1") && assignErrBadPattern(ifStmt.Body.List[0])
}

func getEscEmptyTailIf(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 || !binaryLenIdentInteger(ifStmt.Cond, "nchunk", token.EQL, "0") {
		return false
	}
	return assignErrBadPattern(ifStmt.Body.List[0])
}

func callNamed(node ast.Node, receiver string, name string) bool {
	call, ok := node.(*ast.CallExpr)
	if !ok {
		return false
	}
	if receiver == "" {
		return identNamed(call.Fun, name)
	}
	return selectorNamed(call.Fun, receiver, name)
}

func exprContains(expr ast.Expr, matches func(ast.Expr) bool) bool {
	found := false
	ast.Inspect(expr, func(node ast.Node) bool {
		if node == nil {
			return true
		}
		expr, ok := node.(ast.Expr)
		if ok && matches(expr) {
			found = true
			return false
		}
		return !found
	})
	return found
}

func binaryIndexChar(expr ast.Expr, target string, indexValue string, op token.Token, charValue string) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	if !ok || binary.Op != op {
		return false
	}
	index, ok := binary.X.(*ast.IndexExpr)
	if !ok || !identNamed(index.X, target) || !integerLiteral(index.Index, indexValue) {
		return false
	}
	literal, ok := binary.Y.(*ast.BasicLit)
	return ok && literal.Kind == token.CHAR && literal.Value == charValue
}

func assignSliceFromLiteral(stmt ast.Stmt, target string, source string, low string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], target) {
		return false
	}
	slice, ok := assign.Rhs[0].(*ast.SliceExpr)
	return ok && identNamed(slice.X, source) && integerLiteral(slice.Low, low) && slice.High == nil && slice.Max == nil
}

func assignSliceFromIndex(stmt ast.Stmt, target string, source string, lowName string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], target) {
		return false
	}
	slice, ok := assign.Rhs[0].(*ast.SliceExpr)
	return ok && identNamed(slice.X, source) && identNamed(slice.Low, lowName) && slice.High == nil && slice.Max == nil
}

func assignIdentBool(stmt ast.Stmt, target string, value string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.ASSIGN &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		identNamed(assign.Lhs[0], target) &&
		identNamed(assign.Rhs[0], value)
}

func assignErrBadPattern(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.ASSIGN &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		identNamed(assign.Lhs[0], "err") &&
		identNamed(assign.Rhs[0], "ErrBadPattern")
}

func forInitAssignInteger(stmt ast.Stmt, target string, value string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], target) {
		return false
	}
	return integerLiteral(assign.Rhs[0], value)
}

func switchTagIndex(switchStmt *ast.SwitchStmt, target string, index string) bool {
	indexExpr, ok := switchStmt.Tag.(*ast.IndexExpr)
	if !ok || !identNamed(indexExpr.X, target) {
		return false
	}
	if index == "0" {
		return integerLiteral(indexExpr.Index, "0")
	}
	return identNamed(indexExpr.Index, index)
}

func switchHasCaseLiteral(switchStmt *ast.SwitchStmt, value string) bool {
	if switchStmt == nil || switchStmt.Body == nil {
		return false
	}
	for _, stmt := range switchStmt.Body.List {
		clause, ok := stmt.(*ast.CaseClause)
		if !ok {
			continue
		}
		for _, expr := range clause.List {
			literal, ok := expr.(*ast.BasicLit)
			if ok && literal.Kind == token.CHAR && literal.Value == value {
				return true
			}
		}
	}
	return false
}

func switchHasDefault(switchStmt *ast.SwitchStmt) bool {
	if switchStmt == nil || switchStmt.Body == nil {
		return false
	}
	for _, stmt := range switchStmt.Body.List {
		clause, ok := stmt.(*ast.CaseClause)
		if ok && len(clause.List) == 0 {
			return true
		}
	}
	return false
}

func switchContainsLabeledBreak(switchStmt *ast.SwitchStmt, label string) bool {
	if switchStmt == nil || switchStmt.Body == nil {
		return false
	}
	found := false
	ast.Inspect(switchStmt, func(node ast.Node) bool {
		branch, ok := node.(*ast.BranchStmt)
		if ok && branch.Tok == token.BREAK && branch.Label != nil && branch.Label.Name == label {
			found = true
			return false
		}
		return !found
	})
	return found
}

func returnStringFalseNil(stmt ast.Stmt, stringValue string) bool {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 3 {
		return false
	}
	literal, ok := returnStmt.Results[0].(*ast.BasicLit)
	return ok &&
		literal.Kind == token.STRING &&
		literal.Value == stringValue &&
		identNamed(returnStmt.Results[1], "false") &&
		identNamed(returnStmt.Results[2], "nil")
}

func bareReturn(stmt ast.Stmt) bool {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	return ok && len(returnStmt.Results) == 0
}

func stringFinderCompositeAssignment(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], "f") {
		return false
	}
	unary, ok := assign.Rhs[0].(*ast.UnaryExpr)
	if !ok || unary.Op != token.AND {
		return false
	}
	literal, ok := unary.X.(*ast.CompositeLit)
	if !ok || !identNamed(literal.Type, "stringFinder") || len(literal.Elts) != 2 {
		return false
	}
	first, ok := literal.Elts[0].(*ast.KeyValueExpr)
	if !ok || !identNamed(first.Key, "pattern") || !identNamed(first.Value, "pattern") {
		return false
	}
	second, ok := literal.Elts[1].(*ast.KeyValueExpr)
	return ok &&
		identNamed(second.Key, "goodSuffixSkip") &&
		makeIntSliceLenPattern(second.Value)
}

func lastIndexAssignment(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.DEFINE &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		identNamed(assign.Lhs[0], "last") &&
		binaryLenPatternMinusOne(assign.Rhs[0])
}

func badCharSkipRangeLoop(stmt ast.Stmt) bool {
	rangeStmt, ok := stmt.(*ast.RangeStmt)
	if !ok || rangeStmt.Tok != token.DEFINE || rangeStmt.Value != nil || rangeStmt.Body == nil {
		return false
	}
	if !identNamed(rangeStmt.Key, "i") || !selectorNamed(rangeStmt.X, "f", "badCharSkip") {
		return false
	}
	if len(rangeStmt.Body.List) != 1 {
		return false
	}
	assign, ok := rangeStmt.Body.List[0].(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.ASSIGN &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		selectorIndexedExprNamed(assign.Lhs[0], "f", "badCharSkip", "i") &&
		lenCallArg(assign.Rhs[0], "pattern")
}

func badCharSkipForwardLoop(stmt ast.Stmt) bool {
	forStmt, ok := stmt.(*ast.ForStmt)
	if !ok || forStmt.Body == nil || len(forStmt.Body.List) != 1 {
		return false
	}
	if !forInitInteger(forStmt.Init, "i", "0") || !binaryIdentIdent(forStmt.Cond, "i", token.LSS, "last") || !incDecIdent(forStmt.Post, "i", token.INC) {
		return false
	}
	assign, ok := forStmt.Body.List[0].(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.ASSIGN &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		nestedBadCharIndex(assign.Lhs[0]) &&
		binaryIdentIdent(assign.Rhs[0], "last", token.SUB, "i")
}

func lastPrefixAssignment(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.DEFINE &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		identNamed(assign.Lhs[0], "lastPrefix") &&
		identNamed(assign.Rhs[0], "last")
}

func goodSuffixPrefixLoop(stmt ast.Stmt) bool {
	forStmt, ok := stmt.(*ast.ForStmt)
	if !ok || forStmt.Body == nil || len(forStmt.Body.List) != 2 {
		return false
	}
	if !forInitIdent(forStmt.Init, "i", "last") || !binaryIdentInteger(forStmt.Cond, "i", token.GEQ, "0") || !incDecIdent(forStmt.Post, "i", token.DEC) {
		return false
	}
	ifStmt, ok := forStmt.Body.List[0].(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 || !hasPrefixPatternSliceCall(ifStmt.Cond) {
		return false
	}
	assign, ok := ifStmt.Body.List[0].(*ast.AssignStmt)
	if !ok || assign.Tok != token.ASSIGN || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 ||
		!identNamed(assign.Lhs[0], "lastPrefix") ||
		!binaryIdentInteger(assign.Rhs[0], "i", token.ADD, "1") {
		return false
	}
	outerAssign, ok := forStmt.Body.List[1].(*ast.AssignStmt)
	return ok &&
		outerAssign.Tok == token.ASSIGN &&
		len(outerAssign.Lhs) == 1 &&
		len(outerAssign.Rhs) == 1 &&
		selectorIndexedExprNamed(outerAssign.Lhs[0], "f", "goodSuffixSkip", "i") &&
		lastPrefixShiftExpression(outerAssign.Rhs[0])
}

func goodSuffixRepeatLoop(stmt ast.Stmt) bool {
	forStmt, ok := stmt.(*ast.ForStmt)
	if !ok || forStmt.Body == nil || len(forStmt.Body.List) != 2 {
		return false
	}
	if !forInitInteger(forStmt.Init, "i", "0") || !binaryIdentIdent(forStmt.Cond, "i", token.LSS, "last") || !incDecIdent(forStmt.Post, "i", token.INC) {
		return false
	}
	assign, ok := forStmt.Body.List[0].(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 ||
		!identNamed(assign.Lhs[0], "lenSuffix") ||
		!longestCommonSuffixPatternCall(assign.Rhs[0]) {
		return false
	}
	ifStmt, ok := forStmt.Body.List[1].(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 || !goodSuffixMismatchCondition(ifStmt.Cond) {
		return false
	}
	update, ok := ifStmt.Body.List[0].(*ast.AssignStmt)
	return ok &&
		update.Tok == token.ASSIGN &&
		len(update.Lhs) == 1 &&
		len(update.Rhs) == 1 &&
		goodSuffixLastMinusLenSuffixIndex(update.Lhs[0]) &&
		lenSuffixShiftExpression(update.Rhs[0])
}

func returnIdentifierF(stmt ast.Stmt) bool {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	return ok && len(returnStmt.Results) == 1 && identNamed(returnStmt.Results[0], "f")
}

func funcTypeHasOneNamedParam(funcType *ast.FuncType, name string, typ string) bool {
	if funcType == nil || funcType.Params == nil || len(funcType.Params.List) != 1 {
		return false
	}
	field := funcType.Params.List[0]
	return len(field.Names) == 1 && field.Names[0].Name == name && identNamed(field.Type, typ)
}

func funcTypeHasOneNamedSliceParam(funcType *ast.FuncType, name string, elem string) bool {
	if funcType == nil || funcType.Params == nil || len(funcType.Params.List) != 1 {
		return false
	}
	field := funcType.Params.List[0]
	return len(field.Names) == 1 && field.Names[0].Name == name && sliceArrayTypeNamed(field.Type, elem)
}

func funcTypeHasThreeNamedSliceParams(funcType *ast.FuncType, firstName string, secondName string, thirdName string, elem string) bool {
	if funcType == nil || funcType.Params == nil || len(funcType.Params.List) != 1 {
		return false
	}
	field := funcType.Params.List[0]
	return len(field.Names) == 3 &&
		field.Names[0].Name == firstName &&
		field.Names[1].Name == secondName &&
		field.Names[2].Name == thirdName &&
		sliceArrayTypeNamed(field.Type, elem)
}

func funcTypeHasTwoNamedParamsOneType(funcType *ast.FuncType, firstName string, secondName string, typ string) bool {
	if funcType == nil || funcType.Params == nil || len(funcType.Params.List) != 1 {
		return false
	}
	field := funcType.Params.List[0]
	return len(field.Names) == 2 &&
		field.Names[0].Name == firstName &&
		field.Names[1].Name == secondName &&
		identNamed(field.Type, typ)
}

func funcTypeReturnsNamedPair(funcType *ast.FuncType, firstName string, firstType string, secondName string, secondType string) bool {
	if funcType == nil || funcType.Results == nil || len(funcType.Results.List) != 2 {
		return false
	}
	first := funcType.Results.List[0]
	second := funcType.Results.List[1]
	return len(first.Names) == 1 &&
		first.Names[0].Name == firstName &&
		identNamed(first.Type, firstType) &&
		len(second.Names) == 1 &&
		second.Names[0].Name == secondName &&
		identNamed(second.Type, secondType)
}

func funcTypeReturnsNamedSingle(funcType *ast.FuncType, name string, typ string) bool {
	if funcType == nil || funcType.Results == nil || len(funcType.Results.List) != 1 {
		return false
	}
	result := funcType.Results.List[0]
	return len(result.Names) == 1 &&
		result.Names[0].Name == name &&
		identNamed(result.Type, typ)
}

func pointerMethodDecl(funcDecl *ast.FuncDecl, name string, receiverName string, receiverType string) bool {
	if funcDecl == nil || funcDecl.Name == nil || funcDecl.Name.Name != name || funcDecl.Recv == nil || len(funcDecl.Recv.List) != 1 {
		return false
	}
	receiver := funcDecl.Recv.List[0]
	return len(receiver.Names) == 1 &&
		receiver.Names[0].Name == receiverName &&
		starExprIdent(receiver.Type, receiverType)
}

func lenCallArg(expr ast.Expr, arg string) bool {
	call, ok := expr.(*ast.CallExpr)
	return ok && identNamed(call.Fun, "len") && len(call.Args) == 1 && identNamed(call.Args[0], arg)
}

func printfCall(stmt ast.Stmt, format string, args []string) bool {
	exprStmt, ok := stmt.(*ast.ExprStmt)
	if !ok {
		return false
	}
	call, ok := exprStmt.X.(*ast.CallExpr)
	if !ok || !selectorNamed(call.Fun, "fmt", "Printf") || len(call.Args) != len(args)+1 {
		return false
	}
	formatLit, ok := call.Args[0].(*ast.BasicLit)
	if !ok || formatLit.Kind != token.STRING || formatLit.Value != strconv.Quote(format) {
		return false
	}
	for index, arg := range args {
		if !identNamed(call.Args[index+1], arg) {
			return false
		}
	}
	return true
}

func indexExprNamed(expr ast.Expr, target string, index string) bool {
	indexExpr, ok := expr.(*ast.IndexExpr)
	return ok && identNamed(indexExpr.X, target) && identNamed(indexExpr.Index, index)
}

func selectorIndexExprNamed(expr ast.Expr, target string, index string, selector string) bool {
	selectorExpr, ok := expr.(*ast.SelectorExpr)
	return ok && indexExprNamed(selectorExpr.X, target, index) && selectorExpr.Sel != nil && selectorExpr.Sel.Name == selector
}

func makeIntSliceLenPattern(expr ast.Expr) bool {
	call, ok := expr.(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, "make") || len(call.Args) != 2 {
		return false
	}
	arrayType, ok := call.Args[0].(*ast.ArrayType)
	return ok && arrayType.Len == nil && identNamed(arrayType.Elt, "int") && lenCallArg(call.Args[1], "pattern")
}

func binaryLenPatternMinusOne(expr ast.Expr) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	if !ok || binary.Op != token.SUB || !lenCallArg(binary.X, "pattern") {
		return false
	}
	literal, ok := binary.Y.(*ast.BasicLit)
	return ok && literal.Kind == token.INT && literal.Value == "1"
}

func selectorIndexedExprNamed(expr ast.Expr, target string, selector string, indexName string) bool {
	index, ok := expr.(*ast.IndexExpr)
	return ok && selectorNamed(index.X, target, selector) && identNamed(index.Index, indexName)
}

func nestedBadCharIndex(expr ast.Expr) bool {
	index, ok := expr.(*ast.IndexExpr)
	return ok && selectorNamed(index.X, "f", "badCharSkip") && indexExprNamed(index.Index, "pattern", "i")
}

func forInitInteger(stmt ast.Stmt, target string, value string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.DEFINE || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], target) {
		return false
	}
	literal, ok := assign.Rhs[0].(*ast.BasicLit)
	return ok && literal.Kind == token.INT && literal.Value == value
}

func forInitIdent(stmt ast.Stmt, target string, value string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.DEFINE &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		identNamed(assign.Lhs[0], target) &&
		identNamed(assign.Rhs[0], value)
}

func binaryIdentIdent(expr ast.Expr, left string, op token.Token, right string) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	return ok && binary.Op == op && identNamed(binary.X, left) && identNamed(binary.Y, right)
}

func binaryIdentInteger(expr ast.Expr, left string, op token.Token, right string) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	if !ok || binary.Op != op || !identNamed(binary.X, left) {
		return false
	}
	literal, ok := binary.Y.(*ast.BasicLit)
	return ok && literal.Kind == token.INT && literal.Value == right
}

func binaryIdentLenCall(expr ast.Expr, left string, op token.Token, rightLenArg string) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	return ok && binary.Op == op && identNamed(binary.X, left) && lenCallArg(binary.Y, rightLenArg)
}

func incDecIdent(stmt ast.Stmt, name string, op token.Token) bool {
	incDec, ok := stmt.(*ast.IncDecStmt)
	return ok && incDec.Tok == op && identNamed(incDec.X, name)
}

func hasPrefixPatternSliceCall(expr ast.Expr) bool {
	call, ok := expr.(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, "HasPrefix") || len(call.Args) != 2 || !identNamed(call.Args[0], "pattern") {
		return false
	}
	slice, ok := call.Args[1].(*ast.SliceExpr)
	return ok &&
		identNamed(slice.X, "pattern") &&
		slice.Low != nil &&
		binaryIdentInteger(slice.Low, "i", token.ADD, "1") &&
		slice.High == nil &&
		slice.Max == nil
}

func lastPrefixShiftExpression(expr ast.Expr) bool {
	outer, ok := expr.(*ast.BinaryExpr)
	if !ok || outer.Op != token.SUB || !identNamed(outer.Y, "i") {
		return false
	}
	inner, ok := outer.X.(*ast.BinaryExpr)
	return ok && inner.Op == token.ADD && identNamed(inner.X, "lastPrefix") && identNamed(inner.Y, "last")
}

func longestCommonSuffixPatternCall(expr ast.Expr) bool {
	call, ok := expr.(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, "longestCommonSuffix") || len(call.Args) != 2 || !identNamed(call.Args[0], "pattern") {
		return false
	}
	slice, ok := call.Args[1].(*ast.SliceExpr)
	if !ok || !identNamed(slice.X, "pattern") || slice.Max != nil {
		return false
	}
	low, ok := slice.Low.(*ast.BasicLit)
	return ok &&
		low.Kind == token.INT &&
		low.Value == "1" &&
		slice.High != nil &&
		binaryIdentInteger(slice.High, "i", token.ADD, "1")
}

func patternIndexWithMinus(expr ast.Expr, left string, right string) bool {
	index, ok := expr.(*ast.IndexExpr)
	if !ok || !identNamed(index.X, "pattern") {
		return false
	}
	return binaryIdentIdent(index.Index, left, token.SUB, right)
}

func goodSuffixMismatchCondition(expr ast.Expr) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	return ok &&
		binary.Op == token.NEQ &&
		patternIndexWithMinus(binary.X, "i", "lenSuffix") &&
		patternIndexWithMinus(binary.Y, "last", "lenSuffix")
}

func goodSuffixLastMinusLenSuffixIndex(expr ast.Expr) bool {
	index, ok := expr.(*ast.IndexExpr)
	return ok &&
		selectorNamed(index.X, "f", "goodSuffixSkip") &&
		binaryIdentIdent(index.Index, "last", token.SUB, "lenSuffix")
}

func lenSuffixShiftExpression(expr ast.Expr) bool {
	outer, ok := expr.(*ast.BinaryExpr)
	if !ok || outer.Op != token.SUB || !identNamed(outer.Y, "i") {
		return false
	}
	inner, ok := outer.X.(*ast.BinaryExpr)
	return ok && inner.Op == token.ADD && identNamed(inner.X, "lenSuffix") && identNamed(inner.Y, "last")
}

func stringSuffixLoopCondition(expr ast.Expr) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	if !ok || binary.Op != token.LAND {
		return false
	}
	return binaryIdentLenCall(binary.X, "i", token.LSS, "a") &&
		binaryIdentLenCall(binary.Y, "i", token.LSS, "b")
}

func longestCommonSuffixMismatchCondition(expr ast.Expr) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	return ok &&
		binary.Op == token.NEQ &&
		indexLenMinusOneMinusIdent(binary.X, "a", "i") &&
		indexLenMinusOneMinusIdent(binary.Y, "b", "i")
}

func indexLenMinusOneMinusIdent(expr ast.Expr, target string, indexName string) bool {
	index, ok := expr.(*ast.IndexExpr)
	return ok && identNamed(index.X, target) && lenMinusOneMinusIdent(index.Index, target, indexName)
}

func lenMinusOneMinusIdent(expr ast.Expr, target string, indexName string) bool {
	outer, ok := expr.(*ast.BinaryExpr)
	if !ok || outer.Op != token.SUB || !identNamed(outer.Y, indexName) {
		return false
	}
	inner, ok := outer.X.(*ast.BinaryExpr)
	return ok && inner.Op == token.SUB && lenCallArg(inner.X, target) && integerLiteral(inner.Y, "1")
}

func assignLastPatternIndex(stmt ast.Stmt) bool {
	return assignLastPatternIndexToName(stmt, "i")
}

func assignLastPatternIndexToName(stmt ast.Stmt, name string) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	return ok &&
		assign.Tok == token.DEFINE &&
		len(assign.Lhs) == 1 &&
		len(assign.Rhs) == 1 &&
		identNamed(assign.Lhs[0], name) &&
		binarySelectorLenMinusOne(assign.Rhs[0], "f", "pattern")
}

func binarySelectorLenMinusOne(expr ast.Expr, receiver string, selector string) bool {
	binary, ok := expr.(*ast.BinaryExpr)
	if !ok || binary.Op != token.SUB {
		return false
	}
	call, ok := binary.X.(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, "len") || len(call.Args) != 1 {
		return false
	}
	return selectorNamed(call.Args[0], receiver, selector) && integerLiteral(binary.Y, "1")
}

func backwardPatternCompareLoop(stmt ast.Stmt) bool {
	forStmt, ok := stmt.(*ast.ForStmt)
	if !ok || forStmt.Init != nil || forStmt.Post != nil || forStmt.Body == nil || len(forStmt.Body.List) != 2 {
		return false
	}
	cond, ok := forStmt.Cond.(*ast.BinaryExpr)
	if !ok || cond.Op != token.LAND {
		return false
	}
	if !binaryIdentInteger(cond.X, "j", token.GEQ, "0") {
		return false
	}
	right, ok := cond.Y.(*ast.BinaryExpr)
	if !ok || right.Op != token.EQL ||
		!indexExprNamed(right.X, "text", "i") ||
		!selectorIndexedExprNamed(right.Y, "f", "pattern", "j") {
		return false
	}
	return incDecIdent(forStmt.Body.List[0], "i", token.DEC) &&
		incDecIdent(forStmt.Body.List[1], "j", token.DEC)
}

func nextMatchReturnIf(stmt ast.Stmt) bool {
	ifStmt, ok := stmt.(*ast.IfStmt)
	if !ok || ifStmt.Body == nil || len(ifStmt.Body.List) != 1 || !binaryIdentInteger(ifStmt.Cond, "j", token.LSS, "0") {
		return false
	}
	returnStmt, ok := ifStmt.Body.List[0].(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return false
	}
	return binaryIdentInteger(returnStmt.Results[0], "i", token.ADD, "1")
}

func nextSkipAssignment(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)
	if !ok || assign.Tok != token.ADD_ASSIGN || len(assign.Lhs) != 1 || len(assign.Rhs) != 1 || !identNamed(assign.Lhs[0], "i") {
		return false
	}
	call, ok := assign.Rhs[0].(*ast.CallExpr)
	if !ok || !identNamed(call.Fun, "max") || len(call.Args) != 2 {
		return false
	}
	return selectorIndexedByTextIndex(call.Args[0], "f", "badCharSkip", "text", "i") &&
		selectorIndexedExprNamed(call.Args[1], "f", "goodSuffixSkip", "j")
}

func selectorIndexedByTextIndex(expr ast.Expr, receiver string, selector string, target string, indexName string) bool {
	index, ok := expr.(*ast.IndexExpr)
	return ok &&
		selectorNamed(index.X, receiver, selector) &&
		indexExprNamed(index.Index, target, indexName)
}

func returnNegativeOne(stmt ast.Stmt) bool {
	returnStmt, ok := stmt.(*ast.ReturnStmt)
	if !ok || len(returnStmt.Results) != 1 {
		return false
	}
	unary, ok := returnStmt.Results[0].(*ast.UnaryExpr)
	return ok && unary.Op == token.SUB && integerLiteral(unary.X, "1")
}

func blockContainsNode(block *ast.BlockStmt, matches func(ast.Node) bool) bool {
	if block == nil {
		return false
	}
	found := false
	ast.Inspect(block, func(node ast.Node) bool {
		if node != nil && matches(node) {
			found = true
			return false
		}
		return !found
	})
	return found
}

func declContainsNode(decl ast.Decl, matches func(ast.Node) bool) bool {
	if decl == nil {
		return false
	}
	found := false
	ast.Inspect(decl, func(node ast.Node) bool {
		if node != nil && matches(node) {
			found = true
			return false
		}
		return !found
	})
	return found
}

func rule(nodeKind string, fields ...syntaxField) syntaxRule {
	return syntaxRule{Name: nodeKind, NodeKind: nodeKind, Fields: fields}
}

func field(name string, required bool, multiple bool, alternatives ...syntaxAlternative) syntaxField {
	return syntaxField{Name: name, Required: required, Multiple: multiple, Alternatives: alternatives}
}

func alternative(nodeKind string, terminal string) syntaxAlternative {
	return syntaxAlternative{NodeKind: nodeKind, Terminal: terminal}
}

func compareMetadata(checked *metadata, generated *metadata) comparison {
	checkedProjection := grammarProjection(checked)
	generatedProjection := grammarProjection(generated)
	return comparison{
		Ready: reflect.DeepEqual(checkedProjection, generatedProjection) &&
			checked.Source == generated.Source &&
			checked.Parser == generated.Parser,
		GrammarProjectionMatches:  reflect.DeepEqual(checkedProjection, generatedProjection),
		ParserMatches:             checked.Parser == generated.Parser,
		SourceMatches:             checked.Source == generated.Source,
		GeneratedSHA256:           jsonSHA256(generated),
		CheckedProjectionSHA256:   jsonSHA256(checkedProjection),
		GeneratedProjectionSHA256: jsonSHA256(generatedProjection),
		RuleCount:                 len(generatedProjection),
		FieldCount:                fieldCount(generatedProjection),
	}
}

func grammarProjection(data *metadata) []projectionRule {
	projected := make([]projectionRule, 0, len(data.Rules))
	for _, rule := range data.Rules {
		fields := make([]projectionField, 0, len(rule.Fields))
		for _, field := range rule.Fields {
			fields = append(fields, projectionField{
				Name:         field.Name,
				Required:     field.Required,
				Multiple:     field.Multiple,
				Alternatives: field.Alternatives,
			})
		}
		projected = append(projected, projectionRule{
			Name:     rule.Name,
			NodeKind: rule.NodeKind,
			Fields:   fields,
		})
	}
	return projected
}

func fieldCount(rules []projectionRule) int {
	total := 0
	for _, rule := range rules {
		total += len(rule.Fields)
	}
	return total
}

func loadMetadata(path string) (*metadata, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read %s: %w", path, err)
	}
	var data metadata
	if err := json.Unmarshal(raw, &data); err != nil {
		return nil, fmt.Errorf("parse %s: %w", path, err)
	}
	return &data, nil
}

func writeMetadata(path string, data *metadata) error {
	return writeJSON(path, data)
}

func writeJSON(path string, data any) error {
	encoded, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		return err
	}
	encoded = append(encoded, '\n')
	if path == "" {
		_, err = os.Stdout.Write(encoded)
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	return os.WriteFile(path, encoded, 0o644)
}

func fileInfo(path string, displayPath string) (fileArtifact, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return fileArtifact{Path: displayPath}, fmt.Errorf("read %s: %w", path, err)
	}
	return fileArtifact{
		Path:   displayPath,
		SHA256: sha256Hex(raw),
		Bytes:  len(raw),
	}, nil
}

func sha256Hex(raw []byte) string {
	sum := sha256.Sum256(raw)
	return hex.EncodeToString(sum[:])
}

func jsonSHA256(data any) string {
	encoded, err := json.Marshal(data)
	if err != nil {
		return ""
	}
	return sha256Hex(encoded)
}

func repoRelative(path string) string {
	absolute, err := filepath.Abs(path)
	if err != nil {
		return filepath.ToSlash(path)
	}
	workingDir, err := os.Getwd()
	if err != nil {
		return filepath.ToSlash(path)
	}
	relative, err := filepath.Rel(workingDir, absolute)
	if err != nil || strings.HasPrefix(relative, "..") {
		return filepath.ToSlash(path)
	}
	return filepath.ToSlash(relative)
}

func defaulted(value string, fallback string) string {
	if value == "" {
		return fallback
	}
	return value
}

func supportedFixtures() string {
	names := make([]string, 0, len(fixtureSpecs))
	for name := range fixtureSpecs {
		names = append(names, name)
	}
	return strings.Join(names, ", ")
}

func exitf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
