#!/usr/bin/env python3
"""Tests for the BPE vocabulary-scaling summary helper."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/bpe_vocab_scaling.py"


def write_training_sidecar(
    path: Path,
    target_vocab: int | None,
    merges: int,
    checkpoints: list[int] | None = None,
) -> None:
    path.write_text(
        json.dumps(
            {
                "target_vocab": target_vocab,
                "checkpoints": checkpoints,
                "generated_merges": merges,
                "trained_on_instances": 4,
                "input_corpus_fingerprint": "c" * 64,
                "token_corpus": {"sequence_count": 4},
                "timings": {"training_wall_time_ns": 123_456_789},
            }
        ),
        encoding="utf-8",
    )


def write_measurement(
    path: Path,
    *,
    grid: str,
    average_tokens: float,
    p95_tokens: int,
    p99_tokens: int,
    context_pass_rate: float,
    checkpoint_sidecar: bool = False,
) -> None:
    has_bpe = grid != "base"
    report = {
        "has_bpe": has_bpe,
        "instances": 4,
        "tokens": int(average_tokens * 4),
        "generated_vocab_size": 258 if grid == "base" else int(grid),
        "bpe_merges_count": None if grid == "base" else int(grid) - 258,
        "token_length_stats": {
            "records": 4,
            "total_tokens": int(average_tokens * 4),
            "average_tokens": average_tokens,
            "p95_tokens": p95_tokens,
            "p99_tokens": p99_tokens,
            "context_window_tokens": 2048,
            "context_window_pass_rate": context_pass_rate,
        },
        "provenance": {
            "input_files": [
                {"path": "fixtures.jsonl", "bytes": 10, "sha256": "a" * 64}
            ],
        },
    }
    if has_bpe:
        merge_path = (
            path.with_name(f"checkpoint-{grid}.json")
            if checkpoint_sidecar
            else path.with_suffix(".merges.json")
        )
        merge_path.write_text('{"merges":[]}\n', encoding="utf-8")
        if checkpoint_sidecar:
            write_training_sidecar(
                merge_path.with_name("checkpoint.provenance.json"),
                None,
                4096 - 258,
                [512, 1024, 2048, 4096],
            )
        else:
            write_training_sidecar(
                merge_path.with_suffix(".provenance.json"),
                int(grid),
                int(grid) - 258,
            )
        report["provenance"]["bpe_merges_file"] = {
            "path": str(merge_path),
            "bytes": merge_path.stat().st_size,
            "sha256": "b" * 64,
        }
    path.write_text(json.dumps(report), encoding="utf-8")


class TestBpeVocabScaling(unittest.TestCase):
    def test_summary_applies_power_of_two_reasonableness_policy(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reports = {
                "base": root / "base.json",
                "512": root / "512.json",
                "1024": root / "1024.json",
                "2048": root / "2048.json",
                "4096": root / "4096.json",
            }
            write_measurement(
                reports["base"],
                grid="base",
                average_tokens=100.0,
                p95_tokens=150,
                p99_tokens=180,
                context_pass_rate=0.80,
            )
            write_measurement(
                reports["512"],
                grid="512",
                average_tokens=80.0,
                p95_tokens=130,
                p99_tokens=170,
                context_pass_rate=0.90,
            )
            write_measurement(
                reports["1024"],
                grid="1024",
                average_tokens=76.0,
                p95_tokens=125,
                p99_tokens=165,
                context_pass_rate=0.95,
            )
            write_measurement(
                reports["2048"],
                grid="2048",
                average_tokens=65.0,
                p95_tokens=100,
                p99_tokens=140,
                context_pass_rate=0.99,
            )
            write_measurement(
                reports["4096"],
                grid="4096",
                average_tokens=64.0,
                p95_tokens=98,
                p99_tokens=139,
                context_pass_rate=0.99,
            )
            output = root / "summary.json"
            markdown = root / "summary.md"

            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--entry",
                    f"package=base={reports['base']}",
                    "--entry",
                    f"package=512={reports['512']}",
                    "--entry",
                    f"package=1024={reports['1024']}",
                    "--entry",
                    f"package=2048={reports['2048']}",
                    "--entry",
                    f"package=4096={reports['4096']}",
                    "--json-output",
                    str(output),
                    "--markdown-output",
                    str(markdown),
                    "--require-ready",
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )

            summary = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(summary["artifact_kind"], "tct_bpe_vocab_scaling_summary")
            self.assertEqual(summary["manifest_version"], 1)
            self.assertEqual(summary["tool"], "bpe_vocab_scaling")
            self.assertTrue(summary["summary"]["all_ready"])
            schema = summary["schemas"][0]
            self.assertEqual(schema["default_ceiling_without_4096_review"], 2048)
            self.assertEqual(schema["recommended_ceiling_after_grid"], 2048)
            decisions = {
                entry["grid"]: entry["decision"] for entry in schema["entries"]
            }
            self.assertEqual(decisions["512"], "reasonable_average_reduction")
            self.assertEqual(decisions["1024"], "not_reasonable")
            self.assertEqual(decisions["2048"], "reasonable_average_reduction")
            self.assertEqual(decisions["4096"], "not_reasonable")
            entry_2048 = next(
                entry for entry in schema["entries"] if entry["grid"] == "2048"
            )
            self.assertAlmostEqual(
                entry_2048["marginal_average_token_reduction"],
                (76.0 - 65.0) / 76.0,
            )
            self.assertEqual(entry_2048["trainer_wall_time_ns"], 123_456_789)
            self.assertIn("package", markdown.read_text(encoding="utf-8"))

    def test_checkpoint_training_sidecar_covers_grid_entry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = root / "512.json"
            output = root / "summary.json"
            write_measurement(
                report,
                grid="512",
                average_tokens=80.0,
                p95_tokens=130,
                p99_tokens=170,
                context_pass_rate=0.90,
                checkpoint_sidecar=True,
            )

            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--entry",
                    f"package=512={report}",
                    "--json-output",
                    str(output),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )

            entry = json.loads(output.read_text(encoding="utf-8"))["schemas"][0][
                "entries"
            ][0]
            self.assertEqual(entry["training_provenance_mode"], "checkpoint_set")
            self.assertEqual(
                entry["source_training_provenance"]["path"],
                str(report.with_name("checkpoint.provenance.json")),
            )
            self.assertEqual(entry["gaps"], [])

    def test_4096_requires_explicit_review_when_it_meets_policy(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reports = {grid: root / f"{grid}.json" for grid in ("base", "2048", "4096")}
            write_measurement(
                reports["base"],
                grid="base",
                average_tokens=100.0,
                p95_tokens=150,
                p99_tokens=180,
                context_pass_rate=0.80,
            )
            write_measurement(
                reports["2048"],
                grid="2048",
                average_tokens=80.0,
                p95_tokens=120,
                p99_tokens=160,
                context_pass_rate=0.90,
            )
            write_measurement(
                reports["4096"],
                grid="4096",
                average_tokens=60.0,
                p95_tokens=90,
                p99_tokens=120,
                context_pass_rate=0.99,
            )
            output = root / "summary.json"

            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--entry",
                    f"package=base={reports['base']}",
                    "--entry",
                    f"package=2048={reports['2048']}",
                    "--entry",
                    f"package=4096={reports['4096']}",
                    "--json-output",
                    str(output),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )

            schema = json.loads(output.read_text(encoding="utf-8"))["schemas"][0]
            self.assertEqual(schema["recommended_ceiling_after_grid"], 4096)
            self.assertIn("review", schema["recommendation_note"])

    def test_require_ready_rejects_missing_measurement_stats(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = root / "broken.json"
            report.write_text(
                json.dumps({"has_bpe": False, "provenance": {}}),
                encoding="utf-8",
            )
            output = root / "summary.json"

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--entry",
                    f"package=base={report}",
                    "--json-output",
                    str(output),
                    "--require-ready",
                ],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.returncode, 1)
            self.assertIn("missing token_length_stats", completed.stderr)
            summary = json.loads(output.read_text(encoding="utf-8"))
            self.assertFalse(summary["summary"]["all_ready"])


if __name__ == "__main__":
    unittest.main()
