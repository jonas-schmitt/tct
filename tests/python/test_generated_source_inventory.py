#!/usr/bin/env python3
"""Tests for the generated-source substring inventory helper."""

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
HELPER = REPO_ROOT / "scripts/testing/generated-source-inventory.py"


def load_generated_source_inventory_module():
    spec = importlib.util.spec_from_file_location(
        "generated_source_inventory_script", HELPER
    )
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class TestGeneratedSourceInventory(unittest.TestCase):
    def test_collect_hits_distinguishes_tests_from_production(self):
        inventory = load_generated_source_inventory_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "src").mkdir()
            (root / "tests").mkdir()
            (root / "src/lib.rs").write_text(
                "\n".join(
                    [
                        "pub fn generated_shape(source: &str) -> bool {",
                        "    source.contains(\"pub struct RuntimeShape\")",
                        "}",
                        "",
                        "#[cfg(test)]",
                        "mod tests {",
                        "    #[test]",
                        "    fn checks_generated_enum_shape() {",
                        "        let code = \"pub enum TestShape {}\";",
                        "        assert!(code.contains(\"pub enum TestShape\"));",
                        "    }",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )
            (root / "tests/generated_shape_test.rs").write_text(
                "\n".join(
                    [
                        "#[test]",
                        "fn checks_generated_struct_shape() {",
                        "    let code = \"pub struct FixtureShape;\";",
                        "    assert!(code.contains(\"pub struct FixtureShape\"));",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )

            hits = inventory.collect_hits(root, ["src", "tests"])
            production = [hit for hit in hits if hit.kind == "production-classifier"]
            test_assertions = [hit for hit in hits if hit.kind == "test-source-shape"]

            self.assertEqual(len(production), 1)
            self.assertEqual(production[0].path, "src/lib.rs")
            self.assertIn("RuntimeShape", production[0].text)
            self.assertEqual(len(test_assertions), 2)
            self.assertEqual(
                {hit.path for hit in test_assertions},
                {"src/lib.rs", "tests/generated_shape_test.rs"},
            )

    def test_top_level_rust_tests_are_not_production_classifiers(self):
        inventory = load_generated_source_inventory_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(
                "\n".join(
                    [
                        "#[test]",
                        "fn checks_generated_struct_shape() {",
                        "    let code = \"pub struct TestShape;\";",
                        "    assert!(code.contains(\"pub struct TestShape\"));",
                        "}",
                        "",
                        "pub fn generated_shape(source: &str) -> bool {",
                        "    source.contains(\"pub enum RuntimeShape\")",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )

            hits = inventory.collect_hits(root, ["src"])
            production = [hit for hit in hits if hit.kind == "production-classifier"]
            test_assertions = [hit for hit in hits if hit.kind == "test-source-shape"]

            self.assertEqual(len(production), 1)
            self.assertIn("RuntimeShape", production[0].text)
            self.assertEqual(len(test_assertions), 1)
            self.assertIn("TestShape", test_assertions[0].text)

    def test_cfg_test_helpers_are_not_production_classifiers(self):
        inventory = load_generated_source_inventory_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(
                "\n".join(
                    [
                        "#[cfg(test)]",
                        "fn test_only_generated_shape(source: &str) -> bool {",
                        "    source.contains(\"pub struct TestOnlyShape\")",
                        "}",
                        "",
                        "pub fn generated_shape(source: &str) -> bool {",
                        "    source.contains(\"pub enum RuntimeShape\")",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )

            hits = inventory.collect_hits(root, ["src"])
            production = [hit for hit in hits if hit.kind == "production-classifier"]
            test_assertions = [hit for hit in hits if hit.kind == "test-source-shape"]

            self.assertEqual(len(production), 1)
            self.assertIn("RuntimeShape", production[0].text)
            self.assertEqual(len(test_assertions), 1)
            self.assertIn("TestOnlyShape", test_assertions[0].text)

    def test_cfg_any_test_helpers_are_not_production_classifiers(self):
        inventory = load_generated_source_inventory_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(
                "\n".join(
                    [
                        "#[cfg(any(test, feature = \"fixture-support\"))]",
                        "fn test_support_generated_shape(source: &str) -> bool {",
                        "    source.contains(\"pub struct CompoundCfgShape\")",
                        "}",
                        "",
                        "pub fn generated_shape(source: &str) -> bool {",
                        "    source.contains(\"pub enum RuntimeShape\")",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )

            hits = inventory.collect_hits(root, ["src"])
            production = [hit for hit in hits if hit.kind == "production-classifier"]
            test_assertions = [hit for hit in hits if hit.kind == "test-source-shape"]

            self.assertEqual(len(production), 1)
            self.assertIn("RuntimeShape", production[0].text)
            self.assertEqual(len(test_assertions), 1)
            self.assertIn("CompoundCfgShape", test_assertions[0].text)

    def test_comments_do_not_create_hits(self):
        inventory = load_generated_source_inventory_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(
                "\n".join(
                    [
                        "pub fn ordinary(source: &str) -> bool {",
                        "    // source.contains(\"pub struct CommentedOut\")",
                        "    source.contains(\"ordinary runtime text\")",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )

            self.assertEqual(inventory.collect_hits(root, ["src"]), [])

    def test_non_string_contains_do_not_create_source_shape_hits(self):
        inventory = load_generated_source_inventory_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "tests").mkdir()
            (root / "tests/token_values_test.rs").write_text(
                "\n".join(
                    [
                        "#[test]",
                        "fn checks_token_values_not_generated_source() {",
                        "    let raw_bpe_tokens = vec![BASE_VOCAB_SIZE];",
                        "    assert!(raw_bpe_tokens.contains(&BASE_VOCAB_SIZE));",
                        "    assert!(raw_bpe_tokens.contains(&0), \"missing enum token\");",
                        "    let generated_code = \"pub const BASE_VOCAB_SIZE: u32 = 257;\";",
                        "    assert!(generated_code.contains(\"pub const BASE_VOCAB_SIZE\"));",
                        "    let helper = \"if line.contains(\\\"pub struct \\\") { true }\";",
                        "    assert!(!helper.is_empty());",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )

            hits = inventory.collect_hits(root, ["tests"])

            self.assertEqual(len(hits), 1)
            self.assertEqual(hits[0].path, "tests/token_values_test.rs")
            self.assertIn("generated_code.contains", hits[0].text)

    def test_diagnostic_prose_with_shape_words_is_not_source_shape(self):
        inventory = load_generated_source_inventory_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "tests").mkdir()
            (root / "tests/error_messages_test.rs").write_text(
                "\n".join(
                    [
                        "#[test]",
                        "fn distinguishes_errors_from_generated_source() {",
                        "    let error_str = \"Invalid enum values\";",
                        "    assert!(error_str.contains(\"Invalid enum values\"));",
                        "    let generated_code = \"enum GeneratedChoice {}\";",
                        "    assert!(generated_code.contains(\"enum GeneratedChoice\"));",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )

            hits = inventory.collect_hits(root, ["tests"])

            self.assertEqual(len(hits), 1)
            self.assertIn("generated_code.contains", hits[0].text)

    def test_unsafe_impl_generated_source_snippet_is_counted(self):
        inventory = load_generated_source_inventory_module()

        self.assertTrue(
            inventory.is_source_shape_assertion(
                'assert!(support.contains("unsafe impl GlobalAlloc"));'
            )
        )

    def test_cargo_dependency_pins_are_not_generated_source_shape(self):
        inventory = load_generated_source_inventory_module()

        self.assertFalse(
            inventory.is_source_shape_assertion(
                'assert!(cargo_toml.contains("'
                'serde = { version = \\"=1.0.228\\", features = [\\"derive\\"] }'
                '"));'
            )
        )
        self.assertFalse(
            inventory.is_source_shape_assertion(
                'assert!(cargo_toml.contains("'
                'chrono = { version = \\"=0.4.44\\", features = [\\"serde\\"] }'
                '"));'
            )
        )
        self.assertTrue(
            inventory.is_source_shape_assertion(
                'assert!(generated_code.contains("serde :: Serialize"));'
            )
        )

    def test_cli_json_and_fail_on_production(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(
                "pub fn generated_shape(source: &str) -> bool { "
                "source.contains(\"pub enum RuntimeShape\") }\n",
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--repo-root",
                    str(root),
                    "--json",
                    "--mode",
                    "production-classifiers",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            hits = json.loads(completed.stdout)
            self.assertEqual(len(hits), 1)
            self.assertEqual(hits[0]["kind"], "production-classifier")
            self.assertEqual(hits[0]["path"], "src/lib.rs")

            json_output = root / "target" / "generated-source-inventory.json"
            text_completed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--repo-root",
                    str(root),
                    "--json-output",
                    str(json_output),
                    "--mode",
                    "production-classifiers",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            written_hits = json.loads(json_output.read_text(encoding="utf-8"))
            self.assertIn("Generated-source substring inventory", text_completed.stdout)
            self.assertEqual(written_hits, hits)

            failed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--repo-root",
                    str(root),
                    "--mode",
                    "production-classifiers",
                    "--fail-on-production",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("production-classifier", failed.stdout)

    def test_cli_fail_on_test_assertions(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "tests").mkdir()
            (root / "tests" / "generated_shape_test.rs").write_text(
                "\n".join(
                    [
                        "#[test]",
                        "fn checks_generated_struct_shape() {",
                        "    let code = \"pub struct FixtureShape;\";",
                        "    assert!(code.contains(\"pub struct FixtureShape\"));",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )

            failed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--repo-root",
                    str(root),
                    "--mode",
                    "all",
                    "--fail-on-test-assertions",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("test-source-shape", failed.stdout)
            self.assertIn("tests/generated_shape_test.rs", failed.stdout)

    def test_cli_artifact_json_includes_provenance(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(
                "pub fn generated_shape(source: &str) -> bool { "
                "source.contains(\"pub enum RuntimeShape\") }\n",
                encoding="utf-8",
            )

            json_output = root / "target" / "generated-source-inventory-artifact.json"
            completed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--repo-root",
                    str(root),
                    "--json",
                    "--json-format",
                    "artifact",
                    "--json-output",
                    str(json_output),
                    "--mode",
                    "production-classifiers",
                    "src",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            artifact = json.loads(completed.stdout)
            written_artifact = json.loads(json_output.read_text(encoding="utf-8"))
            self.assertEqual(written_artifact, artifact)
            self.assertEqual(
                artifact["artifact_type"], "tct-generated-source-inventory"
            )
            self.assertEqual(artifact["version"], 1)
            self.assertEqual(artifact["mode"], "production-classifiers")
            self.assertEqual(artifact["source_paths"], ["src"])
            self.assertEqual(artifact["repo"]["root"], root.as_posix())
            self.assertEqual(artifact["repo"]["head"], None)
            self.assertEqual(artifact["repo"]["dirty"], None)
            self.assertEqual(artifact["counts"]["filtered_hits"], 1)
            self.assertEqual(
                artifact["counts"]["filtered_by_kind"]["production-classifier"], 1
            )
            self.assertEqual(
                artifact["counts"]["path_counts_by_kind"][
                    "production-classifier"
                ],
                {"src/lib.rs": 1},
            )
            self.assertEqual(
                artifact["counts"]["filtered_path_counts_by_kind"][
                    "production-classifier"
                ],
                {"src/lib.rs": 1},
            )
            self.assertEqual(artifact["hits"][0]["path"], "src/lib.rs")


if __name__ == "__main__":
    unittest.main()
