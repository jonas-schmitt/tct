#!/usr/bin/env python3
"""Smoke tests for the Go syntax metadata helper."""

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/go_syntax_metadata_smoke.go"


def go_binary():
    from tct_go_toolchain import go_binary as _go_binary

    return _go_binary()


class TestGoSyntaxMetadataSmoke(unittest.TestCase):
    def setUp(self):
        self.go = go_binary()
        if not self.go:
            self.skipTest("Go toolchain unavailable; set TCT_GO_BIN to run this smoke")

    def run_smoke(self, *args):
        return subprocess.run(
            [self.go, "run", str(SCRIPT), *args],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )

    def test_self_test_reproduces_checked_projection(self):
        completed = self.run_smoke("--self-test")
        self.assertIn("go_syntax_metadata_smoke self-test passed", completed.stdout)

    def test_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertIsNone(report["blocker"])
        self.assertEqual(report["artifact_kind"], "tct_go_syntax_metadata_v0")
        self.assertEqual(report["source"]["path"], "tests/fixtures/go_syntax_sources/minimal/main.go")
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["ast_package"], "go/ast")
        self.assertEqual(report["go"]["generated_rules"], 12)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 12)
        self.assertEqual(report["comparison"]["field_count"], 20)

    def test_write_checked_metadata_seeds_fixture_from_parser_projection(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            checked_path = tmp / "checked" / "syntax-metadata-v0.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--json-output",
                str(report_path),
                "--checked-metadata",
                str(checked_path),
                "--generated-output",
                str(generated_path),
                "--write-checked-metadata",
            )
            report = json.loads(report_path.read_text())
            checked = json.loads(checked_path.read_text())
            generated = json.loads(generated_path.read_text())

        self.assertEqual(report["status"], "ready")
        self.assertIsNone(report["blocker"])
        self.assertTrue(report["comparison"]["ready"])
        self.assertEqual(checked, generated)

    def test_local_declaration_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 12)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 12)
        self.assertEqual(report["comparison"]["field_count"], 18)

    def test_local_assignment_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-assignment",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-assignment")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_assignment/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_assignment/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 13)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 13)
        self.assertEqual(report["comparison"]["field_count"], 21)

    def test_local_escaped_assignment_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-escaped-assignment",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-escaped-assignment")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_escaped_assignment/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_escaped_assignment/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 13)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 13)
        self.assertEqual(report["comparison"]["field_count"], 21)

    def test_local_short_var_declaration_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-short-var-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-short-var-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_short_var_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_short_var_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 10)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 10)
        self.assertEqual(report["comparison"]["field_count"], 16)

    def test_local_integer_short_var_declaration_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-integer-short-var-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-integer-short-var-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_integer_short_var_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_integer_short_var_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 10)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 10)
        self.assertEqual(report["comparison"]["field_count"], 16)

    def test_local_integer_return_literal_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-integer-return-literal",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-integer-return-literal")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_integer_return_literal/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_integer_return_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 9)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 9)
        self.assertEqual(report["comparison"]["field_count"], 13)

    def test_local_parenthesized_integer_return_expression_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-parenthesized-integer-return-expression",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(
            report["fixture"], "local-parenthesized-integer-return-expression"
        )
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_parenthesized_integer_return_expression/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_parenthesized_integer_return_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 10)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 10)
        self.assertEqual(report["comparison"]["field_count"], 14)

    def test_local_unary_integer_return_expression_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-unary-integer-return-expression",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-unary-integer-return-expression")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_unary_integer_return_expression/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_unary_integer_return_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 10)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 10)
        self.assertEqual(report["comparison"]["field_count"], 15)

    def test_local_binary_integer_return_expression_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-binary-integer-return-expression",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-binary-integer-return-expression")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_binary_integer_return_expression/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_binary_integer_return_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 10)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 10)
        self.assertEqual(report["comparison"]["field_count"], 16)

    def test_local_call_return_expression_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-call-return-expression",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-call-return-expression")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_call_return_expression/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_call_return_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 9)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 9)
        self.assertEqual(report["comparison"]["field_count"], 13)

    def test_local_argument_call_return_expression_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-argument-call-return-expression",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-argument-call-return-expression")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_argument_call_return_expression/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_argument_call_return_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 10)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 10)
        self.assertEqual(report["comparison"]["field_count"], 15)

    def test_local_string_return_literal_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-string-return-literal",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-string-return-literal")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_string_return_literal/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_string_return_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 9)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 9)
        self.assertEqual(report["comparison"]["field_count"], 13)

    def test_local_float_return_literal_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-float-return-literal",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-float-return-literal")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_float_return_literal/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_float_return_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 9)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 9)
        self.assertEqual(report["comparison"]["field_count"], 13)

    def test_local_boolean_return_identifier_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-boolean-return-identifier",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-boolean-return-identifier")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_boolean_return_identifier/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_boolean_return_identifier/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 8)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 8)
        self.assertEqual(report["comparison"]["field_count"], 12)

    def test_local_nil_return_identifier_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-nil-return-identifier",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-nil-return-identifier")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_nil_return_identifier/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_nil_return_identifier/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 8)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 8)
        self.assertEqual(report["comparison"]["field_count"], 12)

    def test_local_typed_declaration_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-typed-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-typed-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_typed_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_typed_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 12)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 12)
        self.assertEqual(report["comparison"]["field_count"], 19)

    def test_local_zero_value_typed_declaration_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-zero-value-typed-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-zero-value-typed-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_zero_value_typed_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_zero_value_typed_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 11)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 11)
        self.assertEqual(report["comparison"]["field_count"], 17)

    def test_local_integer_typed_declaration_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-integer-typed-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-integer-typed-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_integer_typed_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_integer_typed_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 12)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 12)
        self.assertEqual(report["comparison"]["field_count"], 19)

    def test_local_untyped_integer_declaration_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-untyped-integer-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-untyped-integer-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_untyped_integer_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_untyped_integer_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 12)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 12)
        self.assertEqual(report["comparison"]["field_count"], 18)

    def test_local_integer_assignment_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-integer-assignment",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-integer-assignment")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_integer_assignment/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_integer_assignment/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 13)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 13)
        self.assertEqual(report["comparison"]["field_count"], 21)

    def test_local_compound_assignment_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-compound-assignment",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-compound-assignment")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_compound_assignment/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_compound_assignment/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 13)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 13)
        self.assertEqual(report["comparison"]["field_count"], 21)

    def test_local_increment_decrement_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-increment-decrement",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-increment-decrement")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_increment_decrement/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_increment_decrement/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 13)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 13)
        self.assertEqual(report["comparison"]["field_count"], 20)

    def test_local_branch_statement_json_report_records_ready_parser_backed_fixture(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "local-branch-statement",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "local-branch-statement")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/local_branch_statement/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/local_branch_statement/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 11)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 11)
        self.assertEqual(report["comparison"]["field_count"], 15)

    def test_control_flow_json_reports_record_ready_parser_backed_fixtures(self):
        cases = (
            (
                "local-if-else",
                "tests/fixtures/go_syntax_metadata/local_if_else/main.go",
                "tests/fixtures/go_syntax_metadata/local_if_else/syntax-metadata-v0.json",
                15,
                27,
            ),
            (
                "local-for-condition",
                "tests/fixtures/go_syntax_metadata/local_for_condition/main.go",
                "tests/fixtures/go_syntax_metadata/local_for_condition/syntax-metadata-v0.json",
                15,
                26,
            ),
            (
                "local-switch",
                "tests/fixtures/go_syntax_metadata/local_switch/main.go",
                "tests/fixtures/go_syntax_metadata/local_switch/syntax-metadata-v0.json",
                15,
                25,
            ),
            (
                "local-switch-init",
                "tests/fixtures/go_syntax_corpus/src/local_switch_init/main.go",
                "tests/fixtures/go_syntax_metadata/local_switch_init/syntax-metadata-v0.json",
                17,
                29,
            ),
            (
                "local-type-switch",
                "tests/fixtures/go_syntax_corpus/src/local_type_switch/main.go",
                "tests/fixtures/go_syntax_metadata/local_type_switch/syntax-metadata-v0.json",
                19,
                31,
            ),
            (
                "local-range-string",
                "tests/fixtures/go_syntax_metadata/local_range_string/main.go",
                "tests/fixtures/go_syntax_metadata/local_range_string/syntax-metadata-v0.json",
                14,
                23,
            ),
        )
        for fixture, source_path, metadata_path, rule_count, field_count in cases:
            with self.subTest(fixture=fixture):
                with tempfile.TemporaryDirectory() as tmpdir:
                    tmp = Path(tmpdir)
                    report_path = tmp / "report.json"
                    generated_path = tmp / "generated.json"
                    self.run_smoke(
                        "--fixture",
                        fixture,
                        "--json-output",
                        str(report_path),
                        "--generated-output",
                        str(generated_path),
                    )
                    report = json.loads(report_path.read_text())

                self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
                self.assertEqual(report["version"], 1)
                self.assertEqual(report["status"], "ready")
                self.assertEqual(report["fixture"], fixture)
                self.assertIsNone(report["blocker"])
                self.assertEqual(report["source"]["path"], source_path)
                self.assertEqual(report["checked_metadata"]["path"], metadata_path)
                self.assertEqual(report["go"]["parser_package"], "go/parser")
                self.assertEqual(report["go"]["generated_rules"], rule_count)
                self.assertTrue(report["comparison"]["ready"])
                self.assertTrue(report["comparison"]["grammar_projection_matches"])
                self.assertTrue(report["comparison"]["parser_matches"])
                self.assertTrue(report["comparison"]["source_matches"])
                self.assertEqual(report["comparison"]["rule_count"], rule_count)
                self.assertEqual(report["comparison"]["field_count"], field_count)

    def test_package_type_declaration_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "package-type-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "package-type-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/package_type_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/package_type_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 11)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 11)
        self.assertEqual(report["comparison"]["field_count"], 17)

    def test_method_declaration_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "method-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "method-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/method_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/method_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 11)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 11)
        self.assertEqual(report["comparison"]["field_count"], 19)

    def test_struct_type_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "struct-type",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "struct-type")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/struct_type/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/struct_type/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 12)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 12)
        self.assertEqual(report["comparison"]["field_count"], 19)

    def test_composite_literal_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "composite-literal",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "composite-literal")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/composite_literal/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/composite_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 13)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 13)
        self.assertEqual(report["comparison"]["field_count"], 21)

    def test_key_value_composite_literal_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "key-value-composite-literal",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "key-value-composite-literal")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/key_value_composite_literal/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/key_value_composite_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 14)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 14)
        self.assertEqual(report["comparison"]["field_count"], 23)

    def test_array_composite_literal_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "array-composite-literal",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "array-composite-literal")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/array_composite_literal/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/array_composite_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 11)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 11)
        self.assertEqual(report["comparison"]["field_count"], 16)

    def test_index_expression_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "index-expression",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "index-expression")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/index_expression/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/index_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 11)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 11)
        self.assertEqual(report["comparison"]["field_count"], 17)

    def test_slice_expression_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "slice-expression",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "slice-expression")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/slice_expression/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/slice_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 11)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 11)
        self.assertEqual(report["comparison"]["field_count"], 17)

    def test_star_expression_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "star-expression",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "star-expression")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/star_expression/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/star_expression/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 9)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 9)
        self.assertEqual(report["comparison"]["field_count"], 14)

    def test_package_var_declaration_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "package-var-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "package-var-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/package_var_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/package_var_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 11)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 11)
        self.assertEqual(report["comparison"]["field_count"], 18)

    def test_package_const_declaration_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "package-const-declaration",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "package-const-declaration")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/package_const_declaration/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/package_const_declaration/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 10)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 10)
        self.assertEqual(report["comparison"]["field_count"], 16)

    def test_function_literal_json_report_records_ready_parser_backed_fixture(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            report_path = tmp / "report.json"
            generated_path = tmp / "generated.json"
            self.run_smoke(
                "--fixture",
                "function-literal",
                "--json-output",
                str(report_path),
                "--generated-output",
                str(generated_path),
            )
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "go_syntax_metadata_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["fixture"], "function-literal")
        self.assertIsNone(report["blocker"])
        self.assertEqual(
            report["source"]["path"],
            "tests/fixtures/go_syntax_metadata/function_literal/main.go",
        )
        self.assertEqual(
            report["checked_metadata"]["path"],
            "tests/fixtures/go_syntax_metadata/function_literal/syntax-metadata-v0.json",
        )
        self.assertEqual(report["go"]["parser_package"], "go/parser")
        self.assertEqual(report["go"]["generated_rules"], 10)
        self.assertTrue(report["comparison"]["ready"])
        self.assertTrue(report["comparison"]["grammar_projection_matches"])
        self.assertTrue(report["comparison"]["parser_matches"])
        self.assertTrue(report["comparison"]["source_matches"])
        self.assertEqual(report["comparison"]["rule_count"], 10)
        self.assertEqual(report["comparison"]["field_count"], 15)


if __name__ == "__main__":
    unittest.main()
