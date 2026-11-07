#!/usr/bin/env python3
"""Tests for the Go current-plus generated-codec batch-intake helper."""

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/go_syntax_current_plus_batch_intake.py"


def load_helper():
    spec = importlib.util.spec_from_file_location("go_syntax_current_plus_batch_intake", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class TestGoSyntaxCurrentPlusBatchIntake(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.helper = load_helper()

    def test_derives_source_paths_from_manifest_path(self):
        helper = self.helper

        self.assertEqual(helper.underscore_slug("debug/pe/symbol.go"), "debug_pe_symbol")
        self.assertEqual(helper.dash_slug("encoding/gob/decoder.go"), "encoding-gob-decoder")
        # M1 unified artifact (fe5cd5379 / 6bc47665c): default mint is the
        # inventory grammar id, not a retired per-file go_*_source_file_syntax_v0 id.
        self.assertEqual(
            helper.grammar_id_for_manifest_path("encoding/gob/decoder.go"),
            helper.GO_UNIFIED_GRAMMAR_ID,
        )
        self.assertEqual(
            helper.GO_UNIFIED_GRAMMAR_ID,
            "go_ast_schema_inventory_go1_24_3",
        )
        # Inventory-as-metadata: projector consumes the shared inventory fixture.
        self.assertEqual(
            helper.metadata_path_for_manifest_path(
                "encoding/gob/decoder.go", Path("tests/fixtures/go_syntax_metadata")
            ).as_posix(),
            helper.GO_AST_SCHEMA_INVENTORY_FIXTURE,
        )
        self.assertEqual(
            helper.checked_report_path_for_manifest_path(
                "encoding/gob/decoder.go", Path("tests/fixtures/go_syntax_corpus")
            ).as_posix(),
            "tests/fixtures/go_syntax_corpus/source-instance-projection-report-encoding-gob-decoder-v1.json",
        )

    def test_batch_plan_skips_ingested_records_and_keeps_selection_order(self):
        helper = self.helper
        selection = {
            "selected_candidates": [
                {
                    "manifest_path": "already/in.go",
                    "source_path": SCRIPT.as_posix(),
                    "selected": True,
                    "selection_order": 1,
                    "file": {"sha256": "a" * 64, "bytes": 10},
                    "package": "already",
                },
                {
                    "manifest_path": "encoding/gob/decoder.go",
                    "source_path": SCRIPT.as_posix(),
                    "selected": True,
                    "selection_order": 2,
                    "file": {"sha256": "b" * 64, "bytes": 20},
                    "package": "gob",
                },
                {
                    "manifest_path": "later/file.go",
                    "source_path": SCRIPT.as_posix(),
                    "selected": True,
                    "selection_order": 3,
                    "file": {"sha256": "c" * 64, "bytes": 30},
                    "package": "later",
                },
            ]
        }
        checker = {
            "status": "ready",
            "unsupported_constructs": [],
            "unsupported_grammar_gaps": [],
            "next_missing_construct": None,
            "next_missing_grammar_gap": None,
            "files": [
                {"sha256": "a" * 64, "unsupported_constructs": [], "unsupported_grammar_gaps": []},
                {"sha256": "b" * 64, "unsupported_constructs": [], "unsupported_grammar_gaps": []},
                {"sha256": "c" * 64, "unsupported_constructs": [], "unsupported_grammar_gaps": []},
            ],
        }
        manifest = {"records": [{"manifest_path": "already/in.go", "source_sha256": "a" * 64}]}

        plans = helper.build_batch_plan(
            selection,
            checker,
            manifest,
            max_new_records=1,
            go_bin="go",
            source_fixture_root=Path("tests/fixtures/go_syntax_corpus/src"),
            metadata_root=Path("tests/fixtures/go_syntax_metadata"),
            checked_report_root=Path("tests/fixtures/go_syntax_corpus"),
            projection_output_root=Path("target/tct-analysis/go-syntax-source-instance"),
            tokenizer_output_root=Path("target/tct-syntax-real-source-demo"),
        )

        self.assertEqual([plan.manifest_path for plan in plans], ["encoding/gob/decoder.go"])
        self.assertEqual(
            plans[0].candidate_grammar_id,
            helper.GO_UNIFIED_GRAMMAR_ID,
        )

    def test_manifest_record_comes_from_generated_reports(self):
        helper = self.helper
        plan = helper.CandidatePlan(
            manifest_path="encoding/gob/decoder.go",
            source_path="/goroot/src/encoding/gob/decoder.go",
            source_sha256="b" * 64,
            source_bytes=20,
            package_name="gob",
            selection_order=2,
            source_fixture_path="tests/fixtures/go_syntax_corpus/src/encoding/gob/decoder.go",
            metadata_path=helper.GO_AST_SCHEMA_INVENTORY_FIXTURE,
            checked_projection_report_path="tests/fixtures/go_syntax_corpus/source-instance-projection-report-encoding-gob-decoder-v1.json",
            projection_output_path="target/tct-analysis/go-syntax-source-instance/encoding-gob-decoder-source-instance-projection-v1.json",
            tokenizer_report_path="target/tct-syntax-real-source-demo/go-encoding-gob-decoder-tokenizer-generator-report-v1.json",
            candidate_grammar_id=helper.GO_UNIFIED_GRAMMAR_ID,
        )
        # GoTokenizerGeneratorReport (src/syntax_go_tokenizer_generator.rs)
        # emits both expanded raw_* fields and elided canonical_* fields.
        # Intake reads the canonical (model-facing) stream contract.
        tokenizer_report = {
            "raw_token_count": 200,
            "raw_token_ids_sha256": "d" * 64,
            "max_raw_token_id": 440,
            "canonical_token_count": 123,
            "canonical_tokens_sha256": "c" * 64,
            "canonical_max_token_id": 440,
            "token_layout_vocab_size": 641,
            "token_layout_pad_token": 640,
            "pad_token_absent": True,
        }
        generated_report = {
            "artifact_kind": "tct_go_syntax_source_instance_generated_codec_report_v1",
            "route": "go_source_instance_projection_contextual_raw_tokens",
            "report_path": "target/tct-syntax-real-source-demo/go-encoding-gob-decoder-source-instance-generated-codec-report-v1.json",
            "source": {
                "path": "tests/fixtures/go_syntax_corpus/src/encoding/gob/decoder.go",
                "sha256": "b" * 64,
                "bytes": 20,
            },
            "parser": {"runtime_go_version": "go1.24.3"},
            "canonical_render": {"canonical_byte_count": 20, "canonical_sha256": "b" * 64},
            "claim_boundary": "syntax-level report evidence only; not arena-IR evidence and not stable-backend promotion.",
        }

        record = helper.manifest_record_from_reports(plan, tokenizer_report, generated_report)

        self.assertEqual(record["role"], "current_plus_generated_codec_round_trip")
        self.assertEqual(record["manifest_path"], "encoding/gob/decoder.go")
        self.assertEqual(record["package"], "gob")
        self.assertEqual(record["expected_tct_token_count"], 123)
        self.assertEqual(record["expected_tct_tokens_sha256"], "c" * 64)
        self.assertEqual(record["expected_max_raw_token_id"], 440)
        self.assertEqual(record["expected_vocab_size"], 641)
        self.assertFalse(record["source_program_validity_claim_allowed"])
        self.assertFalse(record["stable_promotion_allowed"])

    def test_manifest_record_rejects_pre_elision_tokenizer_report_keys(self):
        helper = self.helper
        plan = helper.CandidatePlan(
            manifest_path="encoding/gob/decoder.go",
            source_path="/goroot/src/encoding/gob/decoder.go",
            source_sha256="b" * 64,
            source_bytes=20,
            package_name="gob",
            selection_order=2,
            source_fixture_path="tests/fixtures/go_syntax_corpus/src/encoding/gob/decoder.go",
            metadata_path=helper.GO_AST_SCHEMA_INVENTORY_FIXTURE,
            checked_projection_report_path="tests/fixtures/go_syntax_corpus/source-instance-projection-report-encoding-gob-decoder-v1.json",
            projection_output_path="target/tct-analysis/go-syntax-source-instance/encoding-gob-decoder-source-instance-projection-v1.json",
            tokenizer_report_path="target/tct-syntax-real-source-demo/go-encoding-gob-decoder-tokenizer-generator-report-v1.json",
            candidate_grammar_id=helper.GO_UNIFIED_GRAMMAR_ID,
        )
        # Pre-elision shape only (raw_* without canonical_*): must not be
        # silently accepted as the model-facing token count.
        tokenizer_report = {
            "raw_token_count": 123,
            "raw_token_ids_sha256": "c" * 64,
            "max_raw_token_id": 440,
            "token_layout_vocab_size": 641,
            "token_layout_pad_token": 640,
            "pad_token_absent": True,
        }
        generated_report = {
            "artifact_kind": "tct_go_syntax_source_instance_generated_codec_report_v1",
            "route": "go_source_instance_projection_contextual_raw_tokens",
            "report_path": "target/tct-syntax-real-source-demo/go-encoding-gob-decoder-source-instance-generated-codec-report-v1.json",
            "source": {
                "path": "tests/fixtures/go_syntax_corpus/src/encoding/gob/decoder.go",
                "sha256": "b" * 64,
                "bytes": 20,
            },
            "parser": {"runtime_go_version": "go1.24.3"},
            "canonical_render": {"canonical_byte_count": 20, "canonical_sha256": "b" * 64},
            "claim_boundary": "syntax-level report evidence only; not arena-IR evidence and not stable-backend promotion.",
        }

        with self.assertRaises(ValueError) as ctx:
            helper.manifest_record_from_reports(plan, tokenizer_report, generated_report)
        message = str(ctx.exception)
        self.assertIn("canonical_token_count", message)
        self.assertIn("tokenizer_report", message)
        self.assertIn("raw_token_count", message)

    def test_refresh_existing_conflicts_with_missing_record_intake_flags(self):
        helper = self.helper

        with self.assertRaises(SystemExit):
            helper.parse_args(["--refresh-existing", "--max-new-records", "4"])
        with self.assertRaises(SystemExit):
            helper.parse_args(
                ["--refresh-existing", "--selection-report", "selection.json"]
            )
        with self.assertRaises(SystemExit):
            helper.parse_args(["--refresh-existing", "--checker-report", "checker.json"])

    def test_refresh_existing_plan_mode_reports_targets_without_modifying_manifest(self):
        helper = self.helper
        with tempfile.TemporaryDirectory() as tempdir:
            manifest_path = Path(tempdir) / "training-report-ingestion-current-plus-v1.json"
            manifest_path.write_text(
                json.dumps(
                    {
                        "records": [
                            {
                                "manifest_path": "already/in.go",
                                "source_path": "tests/fixtures/go_syntax_corpus/src/already/in.go",
                                "source_sha256": "a" * 64,
                                "source_bytes": 10,
                                "package": "already",
                                "raw_tree_render_attempt": True,
                            },
                            {
                                "manifest_path": "encoding/gob/decoder.go",
                                "source_path": "tests/fixtures/go_syntax_corpus/src/encoding/gob/decoder.go",
                                "source_sha256": "b" * 64,
                                "source_bytes": 20,
                                "package": "gob",
                            },
                        ]
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            manifest_bytes_before = manifest_path.read_bytes()
            output_path = Path(tempdir) / "refresh-plan-report.json"

            exit_code = helper.main(
                [
                    "--refresh-existing",
                    "--report-ingestion-manifest",
                    str(manifest_path),
                    "--output",
                    str(output_path),
                ]
            )

            self.assertEqual(exit_code, 0)
            self.assertEqual(manifest_path.read_bytes(), manifest_bytes_before)
            report = json.loads(output_path.read_text(encoding="utf-8"))
            self.assertEqual(report["mode"], "refresh_existing")
            self.assertEqual(report["status"], "ready")
            self.assertFalse(report["apply"])
            self.assertIsNone(report["first_blocker"])
            self.assertEqual(report["planned_record_count"], 2)
            self.assertEqual(report["refreshed_record_count"], 0)
            self.assertEqual(
                [record["manifest_path"] for record in report["planned_records"]],
                ["already/in.go", "encoding/gob/decoder.go"],
            )
            self.assertEqual(
                report["rewrite_targets"]["report_ingestion_manifest_path"],
                manifest_path.as_posix(),
            )
            self.assertEqual(
                report["rewrite_targets"]["checked_projection_report_paths"],
                [
                    "tests/fixtures/go_syntax_corpus/source-instance-projection-report-already-in-v1.json",
                    "tests/fixtures/go_syntax_corpus/source-instance-projection-report-encoding-gob-decoder-v1.json",
                ],
            )


if __name__ == "__main__":
    unittest.main()
