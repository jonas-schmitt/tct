#!/usr/bin/env python3
"""Scope checks for tokenizer benchmark helper scripts."""

import subprocess
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def run_help(*command: str) -> str:
    completed = subprocess.run(
        list(command),
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout


class TestBenchmarkingScriptScope(unittest.TestCase):
    def test_reproducible_wrapper_help_exposes_generated_metric_flags(self):
        help_text = run_help(
            "bash",
            "scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh",
            "--help",
        )

        self.assertIn("--generated-bpe-metrics", help_text)
        self.assertIn("--generated-encode-hot-path-metrics", help_text)
        self.assertIn("--generated-encode-validation-phase-metrics", help_text)
        self.assertIn("--generated-decode-prefix-metrics", help_text)
        self.assertIn("--cleanup-generated-helper", help_text)
        self.assertIn("generated-crate-report.json", help_text)

    def test_scripts_readme_classifies_benchmarking_scope(self):
        readme = (REPO_ROOT / "scripts/README.md").read_text(encoding="utf-8")

        self.assertIn(
            "Use `run-reproducible-tokenizer-benchmark.sh` for maintained TCT runtime",
            readme,
        )
        self.assertIn("Failed generated helper builds preserve the temporary project path", readme)
        self.assertIn("Pass `--cleanup-generated-helper`", readme)



if __name__ == "__main__":
    unittest.main()
