"""Tests for the compiler bindings of the root `tct` extension module.

`tct.compile` turns a JSON Schema into generated Rust source and `tct.analyze`
reports the vocabulary analysis, the two modes of the `tct` command line tool.

Skipped when the extension is not importable (ordinary pytest without a
maturin install) or when an unrelated `tct` package without the compiler
bindings is installed. Build the extension with `maturin develop` to run them.
Module-level skip uses `unittest.SkipTest` so both
`python3 -m pytest tests/python` and
`python3 -m unittest discover tests/python` treat a missing extension as a
skip rather than a collection error. Focused Rust coverage lives in
`src/python/compile_api.rs`.
"""

from __future__ import annotations

import json
import unittest
import warnings

try:
    import tct
except ImportError as exc:  # pragma: no cover - exercised without extension
    raise unittest.SkipTest(
        "tct extension not importable (build it with `maturin develop`)"
    ) from exc

if not hasattr(tct, "compile"):  # pragma: no cover - other package named tct
    raise unittest.SkipTest("installed `tct` module has no compiler bindings")


TASK_SCHEMA = {
    "title": "task item",
    "type": "object",
    "properties": {
        "status": {"enum": ["active", "pending", "completed"]},
        "priority": {"type": "integer", "minimum": 1, "maximum": 5},
    },
    "required": ["status"],
    "additionalProperties": False,
}


class CompileTests(unittest.TestCase):
    def test_compile_returns_generated_rust_source(self):
        code = tct.compile(TASK_SCHEMA, "Task")
        self.assertIsInstance(code, str)
        self.assertIn("pub mod tokenizer", code)
        self.assertIn("pub struct Task", code)
        self.assertIn("pub const VOCAB_SIZE", code)

    def test_schema_may_be_text_bytes_or_a_python_object(self):
        text = json.dumps(TASK_SCHEMA)
        from_object = tct.compile(TASK_SCHEMA, "Task")
        self.assertEqual(tct.compile(text, "Task"), from_object)
        self.assertEqual(tct.compile(text.encode("utf-8"), "Task"), from_object)

    def test_root_type_name_defaults_to_the_schema_title_then_schema_root(self):
        self.assertIn("pub struct TaskItem", tct.compile(TASK_SCHEMA))
        untitled = {k: v for k, v in TASK_SCHEMA.items() if k != "title"}
        self.assertIn("pub struct SchemaRoot", tct.compile(untitled))
        # An explicit name wins and is converted to UpperCamelCase like the CLI.
        self.assertIn("pub struct MyTask", tct.compile(TASK_SCHEMA, "my_task"))

    def test_output_is_deterministic(self):
        self.assertEqual(
            tct.compile(TASK_SCHEMA, "Task"), tct.compile(TASK_SCHEMA, "Task")
        )

    def test_default_optimization_level_is_o3(self):
        self.assertEqual(
            tct.compile(TASK_SCHEMA, "Task"),
            tct.compile(TASK_SCHEMA, "Task", optimization="O3"),
        )

    def test_optimization_levels_accept_the_cli_spellings(self):
        self.assertEqual(tct.OPTIMIZATION_LEVELS, ("O0", "O2", "O3", "Os", "OVocab"))
        reference = tct.compile(TASK_SCHEMA, "Task", optimization="O2")
        for spelling in ("2", "O2", "o2", "-O2"):
            with self.subTest(spelling=spelling):
                self.assertEqual(
                    tct.compile(TASK_SCHEMA, "Task", optimization=spelling), reference
                )
        # Integer ranges only get their compact encoding once optimizations run.
        self.assertNotEqual(
            tct.compile(TASK_SCHEMA, "Task", optimization="O0"), reference
        )

    def test_invalid_optimization_level_is_a_value_error(self):
        with self.assertRaises(ValueError) as caught:
            tct.compile(TASK_SCHEMA, "Task", optimization="O9")
        self.assertIn("invalid optimization level", str(caught.exception))

    def test_streaming_adds_prefix_decoding(self):
        plain = tct.compile(TASK_SCHEMA, "Task")
        streaming = tct.compile(TASK_SCHEMA, "Task", streaming=True)
        self.assertNotIn("fn decode_prefix", plain)
        self.assertIn("fn decode_prefix", streaming)

    def test_invalid_schema_raises_compile_error_which_is_a_value_error(self):
        self.assertTrue(issubclass(tct.CompileError, ValueError))
        dangling_ref = {
            "type": "object",
            "properties": {"x": {"$ref": "#/definitions/missing"}},
        }
        with self.assertRaises(tct.CompileError):
            tct.compile(dangling_ref, "Broken")

    def test_malformed_json_text_is_a_value_error(self):
        with self.assertRaises(ValueError):
            tct.compile("{not json", "Broken")

    def test_non_json_python_object_is_rejected(self):
        with self.assertRaises(TypeError):
            tct.compile({"type": object()}, "Broken")

    def test_missing_bpe_merge_table_is_reported(self):
        with self.assertRaises(tct.CompileError):
            tct.compile(TASK_SCHEMA, "Task", bpe_merges="/nonexistent/merges.json")

    def test_max_vocabulary_below_the_byte_alphabet_is_rejected(self):
        with self.assertRaises(tct.CompileError):
            tct.compile(TASK_SCHEMA, "Task", max_vocabulary=100)

    def test_options_after_the_name_are_keyword_only(self):
        with self.assertRaises(TypeError):
            tct.compile(TASK_SCHEMA, "Task", "O3")  # type: ignore[misc]

    def test_compile_warnings_are_python_warnings(self):
        # An object that allows arbitrary extra properties is flagged by the
        # schema quality analysis; the finding must surface as a UserWarning.
        loose = {
            "type": "object",
            "properties": {"a": {"type": "string"}},
            "additionalProperties": True,
        }
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            tct.compile(loose, "Loose")
        messages = [str(w.message) for w in caught if issubclass(w.category, UserWarning)]
        self.assertEqual(len(messages), 1, messages)
        self.assertIn("explicit constraints", messages[0])

    def test_clean_schemas_raise_no_warnings(self):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            tct.compile(TASK_SCHEMA, "Task")
        self.assertEqual([w for w in caught if issubclass(w.category, UserWarning)], [])

    def test_warnings_can_be_escalated_to_errors(self):
        loose = {"type": "object", "additionalProperties": True}
        with warnings.catch_warnings():
            warnings.simplefilter("error")
            with self.assertRaises(UserWarning):
                tct.compile(loose, "Loose")


class AnalyzeTests(unittest.TestCase):
    def test_analyze_returns_the_cli_json_report(self):
        report = tct.analyze(TASK_SCHEMA)
        self.assertIsInstance(report, dict)
        self.assertEqual(set(report), {"schema_file", "analysis", "recommendations"})
        analysis = report["analysis"]
        self.assertGreater(analysis["total_positions"], 0)
        self.assertGreaterEqual(analysis["max_vocabulary_size"], 257)
        self.assertIn("complexity_metrics", analysis)
        sizes = [row["vocab_size"] for row in report["recommendations"]["efficiency_analysis"]]
        self.assertEqual(sizes, [64, 128, 256, 512])

    def test_compare_sizes_selects_the_vocabulary_budgets(self):
        report = tct.analyze(TASK_SCHEMA, compare_sizes=[300, 1000])
        sizes = [row["vocab_size"] for row in report["recommendations"]["efficiency_analysis"]]
        self.assertEqual(sizes, [300, 1000])

    def test_analyze_accepts_schema_text(self):
        self.assertEqual(
            tct.analyze(json.dumps(TASK_SCHEMA)), tct.analyze(TASK_SCHEMA)
        )

    def test_analyze_rejects_malformed_schema_text(self):
        with self.assertRaises(ValueError):
            tct.analyze("{not json")


class ModuleMetadataTests(unittest.TestCase):
    def test_version_matches_the_crate(self):
        self.assertRegex(tct.__version__, r"^\d+\.\d+\.\d+")

    def test_bpe_helpers_are_still_exported(self):
        for name in ("Utf8BpeEncoder", "Utf8BpeDecoder", "Utf8BpeTokenizer"):
            self.assertTrue(hasattr(tct, name), name)


if __name__ == "__main__":
    unittest.main()
