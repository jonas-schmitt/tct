#!/usr/bin/env python3
"""Tests for the Roslyn-backed C# syntax metadata smoke helper."""

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/csharp_syntax_metadata_smoke.py"
CHECKED_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/minimal_class/syntax-metadata-v0.json"
)
BLOCK_SCOPED_NAMESPACE_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/block_scoped_namespace/syntax-metadata-v0.json"
)
USING_DIRECTIVE_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/using_directive/syntax-metadata-v0.json"
)
QUALIFIED_NAME_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/qualified_name/syntax-metadata-v0.json"
)
CONSTRUCTOR_DECLARATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/constructor_declaration/syntax-metadata-v0.json"
)
WHILE_POSTFIX_LOOP_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/while_postfix_loop/syntax-metadata-v0.json"
)
INTERPOLATED_STRING_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/interpolated_string/syntax-metadata-v0.json"
)
MEMBER_ACCESS_EXPRESSION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/member_access_expression/syntax-metadata-v0.json"
)
FIELD_DECLARATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/field_declaration/syntax-metadata-v0.json"
)
BASE_LIST_METADATA = (
    REPO_ROOT / "tests/fixtures/csharp_syntax_metadata/base_list/syntax-metadata-v0.json"
)
CONDITIONAL_ACCESS_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/conditional_access/syntax-metadata-v0.json"
)
ARRAY_RANK_OBJECT_CREATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/array_rank_object_creation/syntax-metadata-v0.json"
)
THIS_EXPRESSION_METADATA = (
    REPO_ROOT / "tests/fixtures/csharp_syntax_metadata/this_expression/syntax-metadata-v0.json"
)
PATTERN_EXPRESSION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/pattern_expression/syntax-metadata-v0.json"
)
ARROW_EXPRESSION_CLAUSE_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/arrow_expression_clause/syntax-metadata-v0.json"
)
STRUCT_DECLARATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/struct_declaration/syntax-metadata-v0.json"
)
ENUM_DECLARATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/enum_declaration/syntax-metadata-v0.json"
)
EVENT_DECLARATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/event_declaration/syntax-metadata-v0.json"
)
YIELD_STATEMENT_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/yield_statement/syntax-metadata-v0.json"
)
ATTRIBUTE_SYNTAX_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/attribute_syntax/syntax-metadata-v0.json"
)
GENERIC_NAME_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/generic_name/syntax-metadata-v0.json"
)
LOCAL_STRING_RETURN_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_string_return_literal/syntax-metadata-v0.json"
)
LOCAL_INTEGER_RETURN_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_integer_return_literal/syntax-metadata-v0.json"
)
LOCAL_TYPED_DECLARATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_typed_declaration/syntax-metadata-v0.json"
)
LOCAL_IMPLICIT_DECLARATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_implicit_declaration/syntax-metadata-v0.json"
)
LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_integer_implicit_declaration/syntax-metadata-v0.json"
)
LOCAL_ASSIGNMENT_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_assignment/syntax-metadata-v0.json"
)
LOCAL_ESCAPED_ASSIGNMENT_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_escaped_assignment/syntax-metadata-v0.json"
)
LOCAL_IF_ELSE_METADATA = (
    REPO_ROOT / "tests/fixtures/csharp_syntax_metadata/local_if_else/syntax-metadata-v0.json"
)
LOCAL_FOR_CONDITION_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_for_condition/syntax-metadata-v0.json"
)
LOCAL_SWITCH_METADATA = (
    REPO_ROOT / "tests/fixtures/csharp_syntax_metadata/local_switch/syntax-metadata-v0.json"
)
LOCAL_FOREACH_STRING_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/local_foreach_string/syntax-metadata-v0.json"
)
TWO_PARAMETER_METHOD_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/two_parameter_method/syntax-metadata-v0.json"
)
PROPERTY_PLUS_METHOD_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/property_plus_method/syntax-metadata-v0.json"
)
METHOD_INVOCATION_RETURN_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/method_invocation_return/syntax-metadata-v0.json"
)
METHOD_INVOCATION_ARGUMENT_RETURN_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/method_invocation_argument_return/syntax-metadata-v0.json"
)
COLLECTION_INITIALIZER_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/collection_initializer/syntax-metadata-v0.json"
)
ELEMENT_ACCESS_METADATA = (
    REPO_ROOT / "tests/fixtures/csharp_syntax_metadata/element_access/syntax-metadata-v0.json"
)
RANGE_ELEMENT_ACCESS_METADATA = (
    REPO_ROOT
    / "tests/fixtures/csharp_syntax_metadata/range_element_access/syntax-metadata-v0.json"
)


def load_script_module():
    spec = importlib.util.spec_from_file_location(
        "csharp_syntax_metadata_smoke", SCRIPT
    )
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class TestCSharpSyntaxMetadataSmoke(unittest.TestCase):
    def test_compare_metadata_treats_parser_sdk_patch_drift_as_diagnostic(self):
        module = load_script_module()
        checked = json.loads(CHECKED_METADATA.read_text())
        generated = json.loads(json.dumps(checked))
        generated["parser"]["version"] = "10.0.300"

        comparison = module.compare_metadata(checked, generated)

        self.assertTrue(comparison["ready"])
        self.assertTrue(comparison["grammar_projection_matches"])
        self.assertTrue(comparison["source_matches"])
        self.assertTrue(comparison["parser_contract_matches"])
        self.assertFalse(comparison["parser_matches"])
        self.assertFalse(comparison["parser_version_matches"])

    def test_checked_fixture_projection_pins_minimal_csharp_shape(self):
        module = load_script_module()
        checked = json.loads(CHECKED_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(len(projection), 5)
        property_rule = next(
            rule for rule in projection if rule["name"] == "PropertyDeclarationSyntax"
        )
        fields = {field["name"]: field for field in property_rule["fields"]}
        self.assertEqual(
            fields["Type"]["alternatives"][0],
            {"node_kind": "PredefinedTypeSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            fields["Initializer"]["alternatives"][0],
            {
                "node_kind": "StringLiteralExpressionSyntax",
                "terminal": "string_payload",
            },
        )

    def test_checked_fixture_projection_pins_block_scoped_namespace_shape(self):
        module = load_script_module()
        checked = json.loads(BLOCK_SCOPED_NAMESPACE_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/block_scoped_namespace/source.cs",
        )
        self.assertEqual(len(projection), 5)
        root_rule = next(
            rule for rule in projection if rule["name"] == "CompilationUnitSyntax"
        )
        self.assertEqual(
            root_rule["fields"][0]["alternatives"][0],
            {"node_kind": "NamespaceDeclarationSyntax", "terminal": "nonterminal"},
        )
        namespace_rule = next(
            rule for rule in projection if rule["name"] == "NamespaceDeclarationSyntax"
        )
        namespace_fields = {field["name"]: field for field in namespace_rule["fields"]}
        self.assertEqual(
            namespace_fields["Name"]["alternatives"][0],
            {"node_kind": "qualified_identifier", "terminal": "identifier_payload"},
        )
        self.assertEqual(
            namespace_fields["Members"]["alternatives"][0],
            {"node_kind": "ClassDeclarationSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_using_directive_shape(self):
        module = load_script_module()
        checked = json.loads(USING_DIRECTIVE_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/using_directive/source.cs",
        )
        self.assertEqual(len(projection), 6)
        root_rule = next(
            rule for rule in projection if rule["name"] == "CompilationUnitSyntax"
        )
        root_fields = {field["name"]: field for field in root_rule["fields"]}
        self.assertEqual(
            root_fields["Usings"]["alternatives"][0],
            {"node_kind": "UsingDirectiveSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            root_fields["Members"]["alternatives"][0],
            {"node_kind": "NamespaceDeclarationSyntax", "terminal": "nonterminal"},
        )
        using_rule = next(
            rule for rule in projection if rule["name"] == "UsingDirectiveSyntax"
        )
        self.assertEqual(
            using_rule["fields"][0]["alternatives"][0],
            {"node_kind": "qualified_identifier", "terminal": "identifier_payload"},
        )

    def test_checked_fixture_projection_pins_qualified_name_shape(self):
        module = load_script_module()
        checked = json.loads(QUALIFIED_NAME_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/qualified_name/source.cs",
        )
        self.assertEqual(len(projection), 8)
        using_rule = next(
            rule for rule in projection if rule["name"] == "UsingDirectiveSyntax"
        )
        self.assertEqual(
            using_rule["fields"][0]["alternatives"][0],
            {"node_kind": "QualifiedNameSyntax", "terminal": "nonterminal"},
        )
        qualified_rule = next(
            rule for rule in projection if rule["name"] == "QualifiedNameSyntax"
        )
        qualified_fields = {field["name"]: field for field in qualified_rule["fields"]}
        self.assertEqual(
            qualified_fields["Left"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            qualified_fields["Right"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_constructor_declaration_shape(self):
        module = load_script_module()
        checked = json.loads(CONSTRUCTOR_DECLARATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/constructor_declaration/source.cs",
        )
        self.assertEqual(len(projection), 14)
        class_rule = next(
            rule for rule in projection if rule["name"] == "ClassDeclarationSyntax"
        )
        class_members = next(
            field for field in class_rule["fields"] if field["name"] == "Members"
        )
        self.assertEqual(
            [alternative["node_kind"] for alternative in class_members["alternatives"]],
            ["PropertyDeclarationSyntax", "ConstructorDeclarationSyntax"],
        )
        constructor_rule = next(
            rule for rule in projection if rule["name"] == "ConstructorDeclarationSyntax"
        )
        constructor_fields = {
            field["name"]: field for field in constructor_rule["fields"]
        }
        self.assertEqual(
            constructor_fields["ParameterList"]["alternatives"][0],
            {"node_kind": "ParameterListSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            constructor_fields["Body"]["alternatives"][0],
            {"node_kind": "BlockSyntax", "terminal": "nonterminal"},
        )
        assignment_rule = next(
            rule for rule in projection if rule["name"] == "AssignmentExpressionSyntax"
        )
        assignment_fields = {
            field["name"]: field for field in assignment_rule["fields"]
        }
        self.assertEqual(
            assignment_fields["Left"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            assignment_fields["Right"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_while_postfix_loop_shape(self):
        module = load_script_module()
        checked = json.loads(WHILE_POSTFIX_LOOP_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/while_postfix_loop/source.cs",
        )
        self.assertEqual(len(projection), 17)
        block_rule = next(
            rule for rule in projection if rule["name"] == "BlockSyntax"
        )
        block_fields = {field["name"]: field for field in block_rule["fields"]}
        self.assertEqual(
            [
                alternative["node_kind"]
                for alternative in block_fields["Statements"]["alternatives"]
            ],
            [
                "LocalDeclarationStatementSyntax",
                "WhileStatementSyntax",
                "ExpressionStatementSyntax",
                "ReturnStatementSyntax",
            ],
        )
        while_rule = next(
            rule for rule in projection if rule["name"] == "WhileStatementSyntax"
        )
        while_fields = {field["name"]: field for field in while_rule["fields"]}
        self.assertEqual(
            while_fields["Condition"]["alternatives"][0],
            {"node_kind": "BinaryExpressionSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            while_fields["Statement"]["alternatives"][0],
            {"node_kind": "BlockSyntax", "terminal": "nonterminal"},
        )
        postfix_rule = next(
            rule for rule in projection if rule["name"] == "PostfixUnaryExpressionSyntax"
        )
        postfix_fields = {field["name"]: field for field in postfix_rule["fields"]}
        self.assertEqual(
            postfix_fields["Operand"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            postfix_fields["OperatorToken"]["alternatives"][0],
            {"node_kind": "postfix_operator", "terminal": "string_payload"},
        )

    def test_checked_fixture_projection_pins_interpolated_string_shape(self):
        module = load_script_module()
        checked = json.loads(INTERPOLATED_STRING_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/interpolated_string/source.cs",
        )
        self.assertEqual(len(projection), 13)
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        return_fields = {field["name"]: field for field in return_rule["fields"]}
        self.assertEqual(
            return_fields["Expression"]["alternatives"][0],
            {
                "node_kind": "InterpolatedStringExpressionSyntax",
                "terminal": "nonterminal",
            },
        )
        interpolated_rule = next(
            rule
            for rule in projection
            if rule["name"] == "InterpolatedStringExpressionSyntax"
        )
        interpolated_fields = {
            field["name"]: field for field in interpolated_rule["fields"]
        }
        self.assertEqual(
            [
                alternative["node_kind"]
                for alternative in interpolated_fields["Contents"]["alternatives"]
            ],
            ["InterpolatedStringTextSyntax", "InterpolationSyntax"],
        )
        text_rule = next(
            rule for rule in projection if rule["name"] == "InterpolatedStringTextSyntax"
        )
        text_fields = {field["name"]: field for field in text_rule["fields"]}
        self.assertEqual(
            text_fields["TextToken"]["alternatives"][0],
            {"node_kind": "interpolated_string_text", "terminal": "string_payload"},
        )
        interpolation_rule = next(
            rule for rule in projection if rule["name"] == "InterpolationSyntax"
        )
        interpolation_fields = {
            field["name"]: field for field in interpolation_rule["fields"]
        }
        self.assertEqual(
            interpolation_fields["Expression"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_member_access_expression_shape(self):
        module = load_script_module()
        checked = json.loads(MEMBER_ACCESS_EXPRESSION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs",
        )
        self.assertEqual(len(projection), 9)
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        return_fields = {field["name"]: field for field in return_rule["fields"]}
        self.assertEqual(
            return_fields["Expression"]["alternatives"][0],
            {
                "node_kind": "MemberAccessExpressionSyntax",
                "terminal": "nonterminal",
            },
        )
        member_access_rule = next(
            rule
            for rule in projection
            if rule["name"] == "MemberAccessExpressionSyntax"
        )
        member_access_fields = {
            field["name"]: field for field in member_access_rule["fields"]
        }
        self.assertEqual(
            member_access_fields["Expression"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            member_access_fields["Name"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_field_declaration_shape(self):
        module = load_script_module()
        checked = json.loads(FIELD_DECLARATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/field_declaration/source.cs",
        )
        self.assertEqual(len(projection), 7)
        class_rule = next(
            rule for rule in projection if rule["name"] == "ClassDeclarationSyntax"
        )
        class_fields = {field["name"]: field for field in class_rule["fields"]}
        self.assertEqual(
            class_fields["Members"]["alternatives"][0],
            {"node_kind": "FieldDeclarationSyntax", "terminal": "nonterminal"},
        )
        field_rule = next(
            rule for rule in projection if rule["name"] == "FieldDeclarationSyntax"
        )
        self.assertEqual(
            field_rule["fields"][0]["alternatives"][0],
            {"node_kind": "VariableDeclarationSyntax", "terminal": "nonterminal"},
        )
        declaration_rule = next(
            rule for rule in projection if rule["name"] == "VariableDeclarationSyntax"
        )
        declaration_fields = {
            field["name"]: field for field in declaration_rule["fields"]
        }
        self.assertEqual(
            declaration_fields["Type"]["alternatives"][0],
            {"node_kind": "PredefinedTypeSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            declaration_fields["Variables"]["alternatives"][0],
            {"node_kind": "VariableDeclaratorSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_base_list_shape(self):
        module = load_script_module()
        checked = json.loads(BASE_LIST_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/base_list/source.cs",
        )
        self.assertEqual(len(projection), 6)
        class_rule = next(
            rule for rule in projection if rule["name"] == "ClassDeclarationSyntax"
        )
        class_fields = {field["name"]: field for field in class_rule["fields"]}
        self.assertEqual(
            class_fields["BaseList"]["alternatives"][0],
            {"node_kind": "BaseListSyntax", "terminal": "nonterminal"},
        )
        base_list_rule = next(
            rule for rule in projection if rule["name"] == "BaseListSyntax"
        )
        self.assertEqual(
            base_list_rule["fields"][0]["alternatives"][0],
            {"node_kind": "SimpleBaseTypeSyntax", "terminal": "nonterminal"},
        )
        simple_base_type_rule = next(
            rule for rule in projection if rule["name"] == "SimpleBaseTypeSyntax"
        )
        self.assertEqual(
            simple_base_type_rule["fields"][0]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_conditional_access_shape(self):
        module = load_script_module()
        checked = json.loads(CONDITIONAL_ACCESS_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/conditional_access/source.cs",
        )
        self.assertEqual(len(projection), 15)
        expression_statement_rule = next(
            rule for rule in projection if rule["name"] == "ExpressionStatementSyntax"
        )
        self.assertEqual(
            expression_statement_rule["fields"][0]["alternatives"][0],
            {
                "node_kind": "ConditionalAccessExpressionSyntax",
                "terminal": "nonterminal",
            },
        )
        conditional_access_rule = next(
            rule
            for rule in projection
            if rule["name"] == "ConditionalAccessExpressionSyntax"
        )
        conditional_access_fields = {
            field["name"]: field for field in conditional_access_rule["fields"]
        }
        self.assertEqual(
            conditional_access_fields["Expression"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            conditional_access_fields["WhenNotNull"]["alternatives"][0],
            {"node_kind": "InvocationExpressionSyntax", "terminal": "nonterminal"},
        )
        invocation_rule = next(
            rule for rule in projection if rule["name"] == "InvocationExpressionSyntax"
        )
        invocation_fields = {field["name"]: field for field in invocation_rule["fields"]}
        self.assertEqual(
            invocation_fields["Expression"]["alternatives"][0],
            {
                "node_kind": "MemberBindingExpressionSyntax",
                "terminal": "nonterminal",
            },
        )
        member_binding_rule = next(
            rule
            for rule in projection
            if rule["name"] == "MemberBindingExpressionSyntax"
        )
        self.assertEqual(
            member_binding_rule["fields"][0]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_array_rank_object_creation_shape(self):
        module = load_script_module()
        checked = json.loads(ARRAY_RANK_OBJECT_CREATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs",
        )
        self.assertEqual(len(projection), 20)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        block_fields = {field["name"]: field for field in block_rule["fields"]}
        self.assertIn(
            {"node_kind": "LocalDeclarationStatementSyntax", "terminal": "nonterminal"},
            block_fields["Statements"]["alternatives"],
        )
        self.assertIn(
            {"node_kind": "ReturnStatementSyntax", "terminal": "nonterminal"},
            block_fields["Statements"]["alternatives"],
        )
        array_type_rule = next(
            rule for rule in projection if rule["name"] == "ArrayTypeSyntax"
        )
        array_type_fields = {field["name"]: field for field in array_type_rule["fields"]}
        self.assertEqual(
            array_type_fields["RankSpecifiers"]["alternatives"][0],
            {"node_kind": "ArrayRankSpecifierSyntax", "terminal": "nonterminal"},
        )
        array_rank_rule = next(
            rule for rule in projection if rule["name"] == "ArrayRankSpecifierSyntax"
        )
        array_rank_fields = {
            field["name"]: field for field in array_rank_rule["fields"]
        }
        self.assertIn(
            {
                "node_kind": "OmittedArraySizeExpressionSyntax",
                "terminal": "nonterminal",
            },
            array_rank_fields["Sizes"]["alternatives"],
        )
        self.assertIn(
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
            array_rank_fields["Sizes"]["alternatives"],
        )
        array_creation_rule = next(
            rule
            for rule in projection
            if rule["name"] == "ArrayCreationExpressionSyntax"
        )
        array_creation_fields = {
            field["name"]: field for field in array_creation_rule["fields"]
        }
        self.assertFalse(array_creation_fields["Initializer"]["required"])
        object_creation_rule = next(
            rule
            for rule in projection
            if rule["name"] == "ObjectCreationExpressionSyntax"
        )
        object_creation_fields = {
            field["name"]: field for field in object_creation_rule["fields"]
        }
        self.assertEqual(
            object_creation_fields["Type"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_this_expression_shape(self):
        module = load_script_module()
        checked = json.loads(THIS_EXPRESSION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/this_expression/source.cs",
        )
        self.assertEqual(len(projection), 16)
        argument_rule = next(rule for rule in projection if rule["name"] == "ArgumentSyntax")
        argument_fields = {field["name"]: field for field in argument_rule["fields"]}
        self.assertEqual(
            argument_fields["Expression"]["alternatives"][0],
            {"node_kind": "ThisExpressionSyntax", "terminal": "nonterminal"},
        )
        this_rule = next(rule for rule in projection if rule["name"] == "ThisExpressionSyntax")
        self.assertEqual(this_rule["fields"], [])
        conditional_rule = next(
            rule for rule in projection if rule["name"] == "ConditionalAccessExpressionSyntax"
        )
        conditional_fields = {
            field["name"]: field for field in conditional_rule["fields"]
        }
        self.assertEqual(
            conditional_fields["WhenNotNull"]["alternatives"][0],
            {"node_kind": "InvocationExpressionSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_pattern_expression_shape(self):
        module = load_script_module()
        checked = json.loads(PATTERN_EXPRESSION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs",
        )
        self.assertEqual(len(projection), 14)
        pattern_rule = next(
            rule for rule in projection if rule["name"] == "IsPatternExpressionSyntax"
        )
        pattern_fields = {field["name"]: field for field in pattern_rule["fields"]}
        self.assertEqual(
            pattern_fields["Expression"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            pattern_fields["Pattern"]["alternatives"][0],
            {"node_kind": "ConstantPatternSyntax", "terminal": "nonterminal"},
        )
        constant_rule = next(
            rule for rule in projection if rule["name"] == "ConstantPatternSyntax"
        )
        constant_fields = {field["name"]: field for field in constant_rule["fields"]}
        self.assertEqual(
            constant_fields["Expression"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_arrow_expression_clause_shape(self):
        module = load_script_module()
        checked = json.loads(ARROW_EXPRESSION_CLAUSE_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs",
        )
        self.assertEqual(len(projection), 7)
        property_rule = next(
            rule for rule in projection if rule["name"] == "PropertyDeclarationSyntax"
        )
        property_fields = {field["name"]: field for field in property_rule["fields"]}
        self.assertEqual(
            property_fields["ExpressionBody"]["alternatives"][0],
            {"node_kind": "ArrowExpressionClauseSyntax", "terminal": "nonterminal"},
        )
        arrow_rule = next(
            rule for rule in projection if rule["name"] == "ArrowExpressionClauseSyntax"
        )
        arrow_fields = {field["name"]: field for field in arrow_rule["fields"]}
        self.assertEqual(
            arrow_fields["Expression"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_struct_declaration_shape(self):
        module = load_script_module()
        checked = json.loads(STRUCT_DECLARATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/struct_declaration/source.cs",
        )
        self.assertEqual(len(projection), 5)
        namespace_rule = next(
            rule for rule in projection if rule["name"] == "NamespaceDeclarationSyntax"
        )
        namespace_fields = {field["name"]: field for field in namespace_rule["fields"]}
        self.assertEqual(
            namespace_fields["Members"]["alternatives"][0],
            {"node_kind": "StructDeclarationSyntax", "terminal": "nonterminal"},
        )
        struct_rule = next(
            rule for rule in projection if rule["name"] == "StructDeclarationSyntax"
        )
        struct_fields = {field["name"]: field for field in struct_rule["fields"]}
        self.assertEqual(
            struct_fields["Identifier"]["alternatives"][0],
            {"node_kind": "identifier", "terminal": "identifier_payload"},
        )
        self.assertEqual(
            struct_fields["Members"]["alternatives"][0],
            {"node_kind": "PropertyDeclarationSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_enum_declaration_shape(self):
        module = load_script_module()
        checked = json.loads(ENUM_DECLARATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/enum_declaration/source.cs",
        )
        self.assertEqual(len(projection), 4)
        namespace_rule = next(
            rule for rule in projection if rule["name"] == "NamespaceDeclarationSyntax"
        )
        namespace_fields = {field["name"]: field for field in namespace_rule["fields"]}
        self.assertEqual(
            namespace_fields["Members"]["alternatives"][0],
            {"node_kind": "EnumDeclarationSyntax", "terminal": "nonterminal"},
        )
        enum_rule = next(
            rule for rule in projection if rule["name"] == "EnumDeclarationSyntax"
        )
        enum_fields = {field["name"]: field for field in enum_rule["fields"]}
        self.assertEqual(
            enum_fields["Identifier"]["alternatives"][0],
            {"node_kind": "identifier", "terminal": "identifier_payload"},
        )
        self.assertEqual(
            enum_fields["Members"]["alternatives"][0],
            {"node_kind": "EnumMemberDeclarationSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_event_declaration_shape(self):
        module = load_script_module()
        checked = json.loads(EVENT_DECLARATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/event_declaration/source.cs",
        )
        self.assertEqual(len(projection), 7)
        class_rule = next(
            rule for rule in projection if rule["name"] == "ClassDeclarationSyntax"
        )
        class_fields = {field["name"]: field for field in class_rule["fields"]}
        self.assertEqual(
            class_fields["Members"]["alternatives"][0],
            {"node_kind": "EventFieldDeclarationSyntax", "terminal": "nonterminal"},
        )
        event_rule = next(
            rule
            for rule in projection
            if rule["name"] == "EventFieldDeclarationSyntax"
        )
        self.assertEqual(
            event_rule["fields"][0]["alternatives"][0],
            {"node_kind": "VariableDeclarationSyntax", "terminal": "nonterminal"},
        )
        declaration_rule = next(
            rule
            for rule in projection
            if rule["name"] == "VariableDeclarationSyntax"
        )
        declaration_fields = {
            field["name"]: field for field in declaration_rule["fields"]
        }
        self.assertEqual(
            declaration_fields["Type"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            declaration_fields["Variables"]["alternatives"][0],
            {"node_kind": "VariableDeclaratorSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_yield_statement_shape(self):
        module = load_script_module()
        checked = json.loads(YIELD_STATEMENT_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/yield_statement/source.cs",
        )
        self.assertEqual(len(projection), 8)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        self.assertEqual(
            block_rule["fields"][0]["alternatives"][0],
            {"node_kind": "YieldStatementSyntax", "terminal": "nonterminal"},
        )
        yield_rule = next(
            rule for rule in projection if rule["name"] == "YieldStatementSyntax"
        )
        yield_fields = {field["name"]: field for field in yield_rule["fields"]}
        self.assertEqual(
            yield_fields["ReturnOrBreakKeyword"]["alternatives"][0],
            {"node_kind": "yield_return_keyword", "terminal": "identifier_payload"},
        )
        self.assertEqual(
            yield_fields["Expression"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        self.assertEqual(
            literal_rule["fields"][0]["alternatives"][0],
            {"node_kind": "string_literal", "terminal": "string_payload"},
        )

    def test_checked_fixture_projection_pins_attribute_syntax_shape(self):
        module = load_script_module()
        checked = json.loads(ATTRIBUTE_SYNTAX_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/attribute_syntax/source.cs",
        )
        self.assertEqual(len(projection), 8)
        class_rule = next(
            rule for rule in projection if rule["name"] == "ClassDeclarationSyntax"
        )
        class_fields = {field["name"]: field for field in class_rule["fields"]}
        self.assertEqual(
            class_fields["AttributeLists"]["alternatives"][0],
            {"node_kind": "AttributeListSyntax", "terminal": "nonterminal"},
        )
        attribute_list_rule = next(
            rule for rule in projection if rule["name"] == "AttributeListSyntax"
        )
        self.assertEqual(
            attribute_list_rule["fields"][0]["alternatives"][0],
            {"node_kind": "AttributeSyntax", "terminal": "nonterminal"},
        )
        attribute_rule = next(
            rule for rule in projection if rule["name"] == "AttributeSyntax"
        )
        self.assertEqual(
            attribute_rule["fields"][0]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        identifier_rule = next(
            rule for rule in projection if rule["name"] == "IdentifierNameSyntax"
        )
        self.assertEqual(
            identifier_rule["fields"][0]["alternatives"][0],
            {"node_kind": "identifier", "terminal": "identifier_payload"},
        )

    def test_checked_fixture_projection_pins_generic_name_shape(self):
        module = load_script_module()
        checked = json.loads(GENERIC_NAME_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/generic_name/source.cs",
        )
        self.assertEqual(len(projection), 9)
        property_rule = next(
            rule for rule in projection if rule["name"] == "PropertyDeclarationSyntax"
        )
        property_fields = {field["name"]: field for field in property_rule["fields"]}
        self.assertEqual(
            property_fields["Type"]["alternatives"][0],
            {"node_kind": "GenericNameSyntax", "terminal": "nonterminal"},
        )
        generic_rule = next(
            rule for rule in projection if rule["name"] == "GenericNameSyntax"
        )
        generic_fields = {field["name"]: field for field in generic_rule["fields"]}
        self.assertEqual(
            generic_fields["Identifier"]["alternatives"][0],
            {"node_kind": "identifier", "terminal": "identifier_payload"},
        )
        self.assertEqual(
            generic_fields["TypeArgumentList"]["alternatives"][0],
            {"node_kind": "TypeArgumentListSyntax", "terminal": "nonterminal"},
        )
        argument_list_rule = next(
            rule for rule in projection if rule["name"] == "TypeArgumentListSyntax"
        )
        self.assertEqual(
            argument_list_rule["fields"][0]["alternatives"][0],
            {"node_kind": "PredefinedTypeSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_local_string_return_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_STRING_RETURN_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(len(projection), 8)
        method_rule = next(
            rule for rule in projection if rule["name"] == "MethodDeclarationSyntax"
        )
        method_fields = {field["name"]: field for field in method_rule["fields"]}
        self.assertEqual(
            method_fields["ReturnType"]["alternatives"][0],
            {"node_kind": "PredefinedTypeSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            method_fields["Body"]["alternatives"][0],
            {"node_kind": "BlockSyntax", "terminal": "nonterminal"},
        )
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        return_fields = {field["name"]: field for field in return_rule["fields"]}
        self.assertEqual(
            return_fields["Expression"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        literal_fields = {field["name"]: field for field in literal_rule["fields"]}
        self.assertEqual(
            literal_fields["Token"]["alternatives"][0],
            {"node_kind": "string_literal", "terminal": "string_payload"},
        )

    def test_checked_fixture_projection_pins_local_integer_return_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_INTEGER_RETURN_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/local_integer_return_literal/source.cs",
        )
        self.assertEqual(len(projection), 8)
        type_rule = next(
            rule for rule in projection if rule["name"] == "PredefinedTypeSyntax"
        )
        type_fields = {field["name"]: field for field in type_rule["fields"]}
        self.assertEqual(
            type_fields["Keyword"]["alternatives"][0],
            {"node_kind": "int_keyword", "terminal": "identifier_payload"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        literal_fields = {field["name"]: field for field in literal_rule["fields"]}
        self.assertEqual(
            literal_fields["Token"]["alternatives"][0],
            {"node_kind": "integer_literal", "terminal": "integer_payload"},
        )

    def test_checked_fixture_projection_pins_local_typed_declaration_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_TYPED_DECLARATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(len(projection), 13)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        statements = {
            alternative["node_kind"]
            for alternative in block_rule["fields"][0]["alternatives"]
        }
        self.assertEqual(
            statements,
            {"LocalDeclarationStatementSyntax", "ReturnStatementSyntax"},
        )
        variable_rule = next(
            rule for rule in projection if rule["name"] == "VariableDeclaratorSyntax"
        )
        variable_fields = {field["name"]: field for field in variable_rule["fields"]}
        self.assertEqual(
            variable_fields["Identifier"]["alternatives"][0],
            {"node_kind": "identifier", "terminal": "identifier_payload"},
        )
        self.assertEqual(
            variable_fields["Initializer"]["alternatives"][0],
            {"node_kind": "EqualsValueClauseSyntax", "terminal": "nonterminal"},
        )
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        self.assertEqual(
            return_rule["fields"][0]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_local_implicit_declaration_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_IMPLICIT_DECLARATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/local_implicit_declaration/source.cs",
        )
        self.assertEqual(len(projection), 13)
        declaration_rule = next(
            rule for rule in projection if rule["name"] == "VariableDeclarationSyntax"
        )
        declaration_fields = {
            field["name"]: field for field in declaration_rule["fields"]
        }
        self.assertEqual(
            declaration_fields["Type"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        statements = [
            alternative["node_kind"]
            for alternative in block_rule["fields"][0]["alternatives"]
        ]
        self.assertEqual(
            statements,
            ["LocalDeclarationStatementSyntax", "ReturnStatementSyntax"],
        )

    def test_checked_fixture_projection_pins_local_integer_implicit_declaration_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_INTEGER_IMPLICIT_DECLARATION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/local_integer_implicit_declaration/source.cs",
        )
        self.assertEqual(len(projection), 13)
        type_rule = next(
            rule for rule in projection if rule["name"] == "PredefinedTypeSyntax"
        )
        type_fields = {field["name"]: field for field in type_rule["fields"]}
        self.assertEqual(
            type_fields["Keyword"]["alternatives"][0],
            {"node_kind": "int_keyword", "terminal": "identifier_payload"},
        )
        declaration_rule = next(
            rule for rule in projection if rule["name"] == "VariableDeclarationSyntax"
        )
        declaration_fields = {
            field["name"]: field for field in declaration_rule["fields"]
        }
        self.assertEqual(
            declaration_fields["Type"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        literal_fields = {field["name"]: field for field in literal_rule["fields"]}
        self.assertEqual(
            literal_fields["Token"]["alternatives"][0],
            {"node_kind": "integer_literal", "terminal": "integer_payload"},
        )

    def test_checked_fixture_projection_pins_local_assignment_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_ASSIGNMENT_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(len(projection), 15)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        statements = [
            alternative["node_kind"]
            for alternative in block_rule["fields"][0]["alternatives"]
        ]
        self.assertEqual(
            statements,
            [
                "LocalDeclarationStatementSyntax",
                "ExpressionStatementSyntax",
                "ReturnStatementSyntax",
            ],
        )
        expression_rule = next(
            rule for rule in projection if rule["name"] == "ExpressionStatementSyntax"
        )
        self.assertEqual(
            expression_rule["fields"][0]["alternatives"][0],
            {"node_kind": "AssignmentExpressionSyntax", "terminal": "nonterminal"},
        )
        assignment_rule = next(
            rule for rule in projection if rule["name"] == "AssignmentExpressionSyntax"
        )
        assignment_fields = {
            field["name"]: field for field in assignment_rule["fields"]
        }
        self.assertEqual(
            assignment_fields["Left"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            assignment_fields["Right"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_local_escaped_assignment_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_ESCAPED_ASSIGNMENT_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/local_escaped_assignment/source.cs",
        )
        self.assertEqual(len(projection), 15)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        statements = [
            alternative["node_kind"]
            for alternative in block_rule["fields"][0]["alternatives"]
        ]
        self.assertEqual(
            statements,
            [
                "LocalDeclarationStatementSyntax",
                "ExpressionStatementSyntax",
                "ReturnStatementSyntax",
            ],
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        literal_fields = {field["name"]: field for field in literal_rule["fields"]}
        self.assertEqual(
            literal_fields["Token"]["alternatives"][0],
            {"node_kind": "string_literal", "terminal": "string_payload"},
        )

    def test_checked_fixture_projection_pins_local_if_else_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_IF_ELSE_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/local_if_else/source.cs",
        )
        self.assertEqual(len(projection), 18)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        statements = [
            alternative["node_kind"]
            for alternative in block_rule["fields"][0]["alternatives"]
        ]
        self.assertEqual(
            statements,
            [
                "LocalDeclarationStatementSyntax",
                "IfStatementSyntax",
                "ExpressionStatementSyntax",
                "ReturnStatementSyntax",
            ],
        )
        if_rule = next(
            rule for rule in projection if rule["name"] == "IfStatementSyntax"
        )
        if_fields = {field["name"]: field for field in if_rule["fields"]}
        self.assertEqual(
            if_fields["Condition"]["alternatives"][0],
            {"node_kind": "BinaryExpressionSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            if_fields["Else"]["alternatives"][0],
            {"node_kind": "ElseClauseSyntax", "terminal": "nonterminal"},
        )
        binary_rule = next(
            rule for rule in projection if rule["name"] == "BinaryExpressionSyntax"
        )
        binary_fields = {field["name"]: field for field in binary_rule["fields"]}
        self.assertEqual(
            binary_fields["OperatorToken"]["alternatives"][0],
            {"node_kind": "greater_than_token", "terminal": "identifier_payload"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        literal_fields = {field["name"]: field for field in literal_rule["fields"]}
        self.assertEqual(
            literal_fields["Token"]["alternatives"][0],
            {"node_kind": "integer_literal", "terminal": "integer_payload"},
        )

    def test_checked_fixture_projection_pins_local_for_condition_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_FOR_CONDITION_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/local_for_condition/source.cs",
        )
        self.assertEqual(len(projection), 17)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        statements = [
            alternative["node_kind"]
            for alternative in block_rule["fields"][0]["alternatives"]
        ]
        self.assertEqual(
            statements,
            [
                "LocalDeclarationStatementSyntax",
                "ForStatementSyntax",
                "ExpressionStatementSyntax",
                "ReturnStatementSyntax",
            ],
        )
        for_rule = next(
            rule for rule in projection if rule["name"] == "ForStatementSyntax"
        )
        for_fields = {field["name"]: field for field in for_rule["fields"]}
        self.assertEqual(
            for_fields["Condition"]["alternatives"][0],
            {"node_kind": "BinaryExpressionSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            for_fields["Statement"]["alternatives"][0],
            {"node_kind": "BlockSyntax", "terminal": "nonterminal"},
        )
        binary_rule = next(
            rule for rule in projection if rule["name"] == "BinaryExpressionSyntax"
        )
        binary_fields = {field["name"]: field for field in binary_rule["fields"]}
        self.assertEqual(
            binary_fields["OperatorToken"]["alternatives"][0],
            {"node_kind": "less_than_token", "terminal": "identifier_payload"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        literal_fields = {field["name"]: field for field in literal_rule["fields"]}
        self.assertEqual(
            literal_fields["Token"]["alternatives"][0],
            {"node_kind": "integer_literal", "terminal": "integer_payload"},
        )

    def test_checked_fixture_projection_pins_local_switch_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_SWITCH_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/local_switch/source.cs",
        )
        self.assertEqual(len(projection), 19)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        statements = [
            alternative["node_kind"]
            for alternative in block_rule["fields"][0]["alternatives"]
        ]
        self.assertEqual(
            statements,
            [
                "LocalDeclarationStatementSyntax",
                "SwitchStatementSyntax",
                "ExpressionStatementSyntax",
                "BreakStatementSyntax",
                "ReturnStatementSyntax",
            ],
        )
        switch_rule = next(
            rule for rule in projection if rule["name"] == "SwitchStatementSyntax"
        )
        switch_fields = {field["name"]: field for field in switch_rule["fields"]}
        self.assertEqual(
            switch_fields["Expression"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            switch_fields["Sections"]["alternatives"][0],
            {"node_kind": "SwitchSectionSyntax", "terminal": "nonterminal"},
        )
        section_rule = next(
            rule for rule in projection if rule["name"] == "SwitchSectionSyntax"
        )
        section_fields = {field["name"]: field for field in section_rule["fields"]}
        self.assertEqual(
            section_fields["Labels"]["alternatives"][0],
            {"node_kind": "CaseSwitchLabelSyntax", "terminal": "nonterminal"},
        )
        self.assertIn(
            {"node_kind": "BreakStatementSyntax", "terminal": "nonterminal"},
            section_fields["Statements"]["alternatives"],
        )
        case_rule = next(
            rule for rule in projection if rule["name"] == "CaseSwitchLabelSyntax"
        )
        case_fields = {field["name"]: field for field in case_rule["fields"]}
        self.assertEqual(
            case_fields["Value"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )
        break_rule = next(
            rule for rule in projection if rule["name"] == "BreakStatementSyntax"
        )
        self.assertEqual(break_rule["fields"], [])
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        literal_fields = {field["name"]: field for field in literal_rule["fields"]}
        self.assertEqual(
            literal_fields["Token"]["alternatives"][0],
            {"node_kind": "integer_literal", "terminal": "integer_payload"},
        )

    def test_checked_fixture_projection_pins_local_foreach_string_shape(self):
        module = load_script_module()
        checked = json.loads(LOCAL_FOREACH_STRING_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/local_foreach_string/source.cs",
        )
        self.assertEqual(len(projection), 16)
        block_rule = next(rule for rule in projection if rule["name"] == "BlockSyntax")
        statements = [
            alternative["node_kind"]
            for alternative in block_rule["fields"][0]["alternatives"]
        ]
        self.assertEqual(
            statements,
            [
                "LocalDeclarationStatementSyntax",
                "ForEachStatementSyntax",
                "ExpressionStatementSyntax",
                "ReturnStatementSyntax",
            ],
        )
        type_rule = next(
            rule for rule in projection if rule["name"] == "PredefinedTypeSyntax"
        )
        type_fields = {field["name"]: field for field in type_rule["fields"]}
        self.assertEqual(
            type_fields["Keyword"]["alternatives"],
            [
                {"node_kind": "int_keyword", "terminal": "identifier_payload"},
                {"node_kind": "char_keyword", "terminal": "identifier_payload"},
            ],
        )
        foreach_rule = next(
            rule for rule in projection if rule["name"] == "ForEachStatementSyntax"
        )
        foreach_fields = {field["name"]: field for field in foreach_rule["fields"]}
        self.assertEqual(
            foreach_fields["Type"]["alternatives"][0],
            {"node_kind": "PredefinedTypeSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            foreach_fields["Identifier"]["alternatives"][0],
            {"node_kind": "identifier", "terminal": "identifier_payload"},
        )
        self.assertEqual(
            foreach_fields["Expression"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        literal_fields = {field["name"]: field for field in literal_rule["fields"]}
        self.assertEqual(
            literal_fields["Token"]["alternatives"],
            [
                {"node_kind": "integer_literal", "terminal": "integer_payload"},
                {"node_kind": "string_literal", "terminal": "string_payload"},
            ],
        )

    def test_checked_fixture_projection_pins_two_parameter_method_shape(self):
        module = load_script_module()
        checked = json.loads(TWO_PARAMETER_METHOD_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/two_parameter_method/source.cs",
        )
        self.assertEqual(len(projection), 10)
        method_rule = next(
            rule for rule in projection if rule["name"] == "MethodDeclarationSyntax"
        )
        method_fields = {field["name"]: field for field in method_rule["fields"]}
        self.assertEqual(
            method_fields["ParameterList"]["alternatives"][0],
            {"node_kind": "ParameterListSyntax", "terminal": "nonterminal"},
        )
        parameter_list_rule = next(
            rule for rule in projection if rule["name"] == "ParameterListSyntax"
        )
        parameter_list_fields = {
            field["name"]: field for field in parameter_list_rule["fields"]
        }
        self.assertTrue(parameter_list_fields["Parameters"]["multiple"])
        self.assertEqual(
            parameter_list_fields["Parameters"]["alternatives"][0],
            {"node_kind": "ParameterSyntax", "terminal": "nonterminal"},
        )
        parameter_rule = next(
            rule for rule in projection if rule["name"] == "ParameterSyntax"
        )
        parameter_fields = {field["name"]: field for field in parameter_rule["fields"]}
        self.assertEqual(
            parameter_fields["Type"]["alternatives"][0],
            {"node_kind": "PredefinedTypeSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            parameter_fields["Identifier"]["alternatives"][0],
            {"node_kind": "identifier", "terminal": "identifier_payload"},
        )
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        self.assertEqual(
            return_rule["fields"][0]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_property_plus_method_shape(self):
        module = load_script_module()
        checked = json.loads(PROPERTY_PLUS_METHOD_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/property_plus_method/source.cs",
        )
        self.assertEqual(len(projection), 9)
        class_rule = next(
            rule for rule in projection if rule["name"] == "ClassDeclarationSyntax"
        )
        class_fields = {field["name"]: field for field in class_rule["fields"]}
        self.assertEqual(
            class_fields["Members"]["alternatives"],
            [
                {
                    "node_kind": "PropertyDeclarationSyntax",
                    "terminal": "nonterminal",
                },
                {"node_kind": "MethodDeclarationSyntax", "terminal": "nonterminal"},
            ],
        )
        property_rule = next(
            rule for rule in projection if rule["name"] == "PropertyDeclarationSyntax"
        )
        property_fields = {field["name"]: field for field in property_rule["fields"]}
        self.assertEqual(
            property_fields["Initializer"]["alternatives"][0],
            {"node_kind": "StringLiteralExpressionSyntax", "terminal": "string_payload"},
        )
        method_rule = next(
            rule for rule in projection if rule["name"] == "MethodDeclarationSyntax"
        )
        method_fields = {field["name"]: field for field in method_rule["fields"]}
        self.assertEqual(
            method_fields["Body"]["alternatives"][0],
            {"node_kind": "BlockSyntax", "terminal": "nonterminal"},
        )
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        self.assertEqual(
            return_rule["fields"][0]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_method_invocation_return_shape(self):
        module = load_script_module()
        checked = json.loads(METHOD_INVOCATION_RETURN_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/method_invocation_return/source.cs",
        )
        self.assertEqual(len(projection), 11)
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        return_fields = {field["name"]: field for field in return_rule["fields"]}
        self.assertEqual(
            return_fields["Expression"]["alternatives"],
            [
                {
                    "node_kind": "InvocationExpressionSyntax",
                    "terminal": "nonterminal",
                },
                {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
            ],
        )
        invocation_rule = next(
            rule for rule in projection if rule["name"] == "InvocationExpressionSyntax"
        )
        invocation_fields = {field["name"]: field for field in invocation_rule["fields"]}
        self.assertEqual(
            invocation_fields["Expression"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            invocation_fields["ArgumentList"]["alternatives"][0],
            {"node_kind": "ArgumentListSyntax", "terminal": "nonterminal"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        self.assertEqual(
            literal_rule["fields"][0]["alternatives"][0],
            {"node_kind": "string_literal", "terminal": "string_payload"},
        )

    def test_checked_fixture_projection_pins_method_invocation_argument_return_shape(self):
        module = load_script_module()
        checked = json.loads(METHOD_INVOCATION_ARGUMENT_RETURN_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/method_invocation_argument_return/source.cs",
        )
        self.assertEqual(len(projection), 14)
        method_rule = next(
            rule for rule in projection if rule["name"] == "MethodDeclarationSyntax"
        )
        method_fields = {field["name"]: field for field in method_rule["fields"]}
        self.assertEqual(
            method_fields["ParameterList"]["alternatives"][0],
            {"node_kind": "ParameterListSyntax", "terminal": "nonterminal"},
        )
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        return_fields = {field["name"]: field for field in return_rule["fields"]}
        self.assertEqual(
            return_fields["Expression"]["alternatives"],
            [
                {
                    "node_kind": "InvocationExpressionSyntax",
                    "terminal": "nonterminal",
                },
                {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
            ],
        )
        argument_list_rule = next(
            rule for rule in projection if rule["name"] == "ArgumentListSyntax"
        )
        argument_list_fields = {
            field["name"]: field for field in argument_list_rule["fields"]
        }
        self.assertEqual(
            argument_list_fields["Arguments"]["alternatives"][0],
            {"node_kind": "ArgumentSyntax", "terminal": "nonterminal"},
        )
        argument_rule = next(
            rule for rule in projection if rule["name"] == "ArgumentSyntax"
        )
        self.assertEqual(
            argument_rule["fields"][0]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_collection_initializer_shape(self):
        module = load_script_module()
        checked = json.loads(COLLECTION_INITIALIZER_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/collection_initializer/source.cs",
        )
        self.assertEqual(len(projection), 11)
        method_rule = next(
            rule for rule in projection if rule["name"] == "MethodDeclarationSyntax"
        )
        method_fields = {field["name"]: field for field in method_rule["fields"]}
        self.assertEqual(
            method_fields["ReturnType"]["alternatives"][0],
            {"node_kind": "ArrayTypeSyntax", "terminal": "nonterminal"},
        )
        array_creation_rule = next(
            rule
            for rule in projection
            if rule["name"] == "ArrayCreationExpressionSyntax"
        )
        array_creation_fields = {
            field["name"]: field for field in array_creation_rule["fields"]
        }
        self.assertEqual(
            array_creation_fields["Type"]["alternatives"][0],
            {"node_kind": "ArrayTypeSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            array_creation_fields["Initializer"]["alternatives"][0],
            {"node_kind": "InitializerExpressionSyntax", "terminal": "nonterminal"},
        )
        initializer_rule = next(
            rule for rule in projection if rule["name"] == "InitializerExpressionSyntax"
        )
        initializer_fields = {
            field["name"]: field for field in initializer_rule["fields"]
        }
        self.assertTrue(initializer_fields["Expressions"]["multiple"])
        self.assertEqual(
            initializer_fields["Expressions"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )
        literal_rule = next(
            rule for rule in projection if rule["name"] == "LiteralExpressionSyntax"
        )
        self.assertEqual(
            literal_rule["fields"][0]["alternatives"][0],
            {"node_kind": "integer_literal", "terminal": "integer_payload"},
        )

    def test_checked_fixture_projection_pins_element_access_shape(self):
        module = load_script_module()
        checked = json.loads(ELEMENT_ACCESS_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/element_access/source.cs",
        )
        self.assertEqual(len(projection), 15)
        method_rule = next(
            rule for rule in projection if rule["name"] == "MethodDeclarationSyntax"
        )
        method_fields = {field["name"]: field for field in method_rule["fields"]}
        self.assertEqual(
            method_fields["ParameterList"]["alternatives"][0],
            {"node_kind": "ParameterListSyntax", "terminal": "nonterminal"},
        )
        parameter_rule = next(
            rule for rule in projection if rule["name"] == "ParameterSyntax"
        )
        parameter_fields = {field["name"]: field for field in parameter_rule["fields"]}
        self.assertEqual(
            parameter_fields["Type"]["alternatives"][0],
            {"node_kind": "ArrayTypeSyntax", "terminal": "nonterminal"},
        )
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        self.assertEqual(
            return_rule["fields"][0]["alternatives"][0],
            {"node_kind": "ElementAccessExpressionSyntax", "terminal": "nonterminal"},
        )
        element_access_rule = next(
            rule
            for rule in projection
            if rule["name"] == "ElementAccessExpressionSyntax"
        )
        element_access_fields = {
            field["name"]: field for field in element_access_rule["fields"]
        }
        self.assertEqual(
            element_access_fields["Expression"]["alternatives"][0],
            {"node_kind": "IdentifierNameSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            element_access_fields["ArgumentList"]["alternatives"][0],
            {
                "node_kind": "BracketedArgumentListSyntax",
                "terminal": "nonterminal",
            },
        )
        bracketed_argument_list_rule = next(
            rule
            for rule in projection
            if rule["name"] == "BracketedArgumentListSyntax"
        )
        self.assertTrue(bracketed_argument_list_rule["fields"][0]["multiple"])
        self.assertEqual(
            bracketed_argument_list_rule["fields"][0]["alternatives"][0],
            {"node_kind": "ArgumentSyntax", "terminal": "nonterminal"},
        )
        argument_rule = next(
            rule for rule in projection if rule["name"] == "ArgumentSyntax"
        )
        self.assertEqual(
            argument_rule["fields"][0]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )

    def test_checked_fixture_projection_pins_range_element_access_shape(self):
        module = load_script_module()
        checked = json.loads(RANGE_ELEMENT_ACCESS_METADATA.read_text())
        projection = module.grammar_projection(checked)

        self.assertEqual(
            checked["source"]["path"],
            "tests/fixtures/csharp_syntax_sources/range_element_access/source.cs",
        )
        self.assertEqual(len(projection), 16)
        method_rule = next(
            rule for rule in projection if rule["name"] == "MethodDeclarationSyntax"
        )
        method_fields = {field["name"]: field for field in method_rule["fields"]}
        self.assertEqual(
            method_fields["ReturnType"]["alternatives"][0],
            {"node_kind": "ArrayTypeSyntax", "terminal": "nonterminal"},
        )
        return_rule = next(
            rule for rule in projection if rule["name"] == "ReturnStatementSyntax"
        )
        self.assertEqual(
            return_rule["fields"][0]["alternatives"][0],
            {"node_kind": "ElementAccessExpressionSyntax", "terminal": "nonterminal"},
        )
        argument_rule = next(
            rule for rule in projection if rule["name"] == "ArgumentSyntax"
        )
        self.assertEqual(
            argument_rule["fields"][0]["alternatives"][0],
            {"node_kind": "RangeExpressionSyntax", "terminal": "nonterminal"},
        )
        range_rule = next(
            rule for rule in projection if rule["name"] == "RangeExpressionSyntax"
        )
        range_fields = {field["name"]: field for field in range_rule["fields"]}
        self.assertEqual(
            range_fields["LeftOperand"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )
        self.assertEqual(
            range_fields["RightOperand"]["alternatives"][0],
            {"node_kind": "LiteralExpressionSyntax", "terminal": "nonterminal"},
        )

    def test_fixture_argument_selects_local_string_return_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-string-return-literal"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_string_return_literal/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_string_return_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-string-return-literal.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_integer_return_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-integer-return-literal"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_integer_return_literal/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_integer_return_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-integer-return-literal.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_typed_declaration_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-typed-declaration"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_typed_declaration/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_typed_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-typed-declaration.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_implicit_declaration_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-implicit-declaration"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_implicit_declaration/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_implicit_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-implicit-declaration.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_integer_implicit_declaration_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-integer-implicit-declaration"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_integer_implicit_declaration/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_integer_implicit_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-integer-implicit-declaration.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_assignment_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-assignment"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_assignment/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_assignment/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-assignment.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_escaped_assignment_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-escaped-assignment"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_escaped_assignment/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_escaped_assignment/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-escaped-assignment.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_if_else_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-if-else"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_if_else/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_if_else/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-if-else.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_for_condition_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-for-condition"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_for_condition/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_for_condition/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-for-condition.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_switch_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-switch"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_switch/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_switch/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-switch.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_local_foreach_string_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "local-foreach-string"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/local_foreach_string/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/local_foreach_string/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "local-foreach-string.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_two_parameter_method_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "two-parameter-method"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/two_parameter_method/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/two_parameter_method/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "two-parameter-method.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_property_plus_method_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "property-plus-method"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/property_plus_method/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/property_plus_method/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "property-plus-method.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_method_invocation_return_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "method-invocation-return"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/method_invocation_return/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/method_invocation_return/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "method-invocation-return.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_method_invocation_argument_return_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "method-invocation-argument-return"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/method_invocation_argument_return/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/method_invocation_argument_return/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "method-invocation-argument-return.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_member_access_expression_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "member-access-expression"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/member_access_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "member-access-expression.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_field_declaration_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "field-declaration"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/field_declaration/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/field_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "field-declaration.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_base_list_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "base-list"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/base_list/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/base_list/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "base-list.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_conditional_access_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "conditional-access"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/conditional_access/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/conditional_access/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "conditional-access.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_array_rank_object_creation_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "array-rank-object-creation"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/array_rank_object_creation/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "array-rank-object-creation.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_this_expression_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "this-expression"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/this_expression/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/this_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "this-expression.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_pattern_expression_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "pattern-expression"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/pattern_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "pattern-expression.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_arrow_expression_clause_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "arrow-expression-clause"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/arrow_expression_clause/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "arrow-expression-clause.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_collection_initializer_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "collection-initializer"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/collection_initializer/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/collection_initializer/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "collection-initializer.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_element_access_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "element-access"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/element_access/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/element_access/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "element-access.syntax-metadata-v0.json",
        )

    def test_fixture_argument_selects_range_element_access_paths(self):
        module = load_script_module()
        args = module.parse_args(["--fixture", "range-element-access"])
        args.source, args.checked_metadata = module.FIXTURES[args.fixture]

        self.assertEqual(
            args.source.as_posix(),
            "tests/fixtures/csharp_syntax_sources/range_element_access/source.cs",
        )
        self.assertEqual(
            args.checked_metadata.as_posix(),
            "tests/fixtures/csharp_syntax_metadata/range_element_access/syntax-metadata-v0.json",
        )
        self.assertEqual(
            module.generated_output_name(args.source),
            "range-element-access.syntax-metadata-v0.json",
        )

    def test_missing_dotnet_records_precise_blocker_without_failure(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--dotnet-bin",
                    str(tmp / "missing-dotnet"),
                    "--json-output",
                    str(report_path),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "csharp_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "blocked")
        self.assertEqual(report["blocker"]["kind"], "dotnet_unavailable")
        self.assertFalse(report["dotnet"]["available"])
        self.assertFalse(report["comparison"]["ready"])


if __name__ == "__main__":
    unittest.main()
