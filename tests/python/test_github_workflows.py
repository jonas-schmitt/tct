#!/usr/bin/env python3
"""Workflow configuration checks for maintained quality gates."""

import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


class TestGitHubWorkflows(unittest.TestCase):
    def test_hosted_workflows_do_not_run_automatically(self):
        ci = (REPO_ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        extended = (
            REPO_ROOT / ".github/workflows/extended-quality.yml"
        ).read_text(encoding="utf-8")

        self.assertIn("# push:", ci)
        self.assertIn("# pull_request:", ci)
        self.assertNotIn("\n  push:", ci)
        self.assertNotIn("\n  pull_request:", ci)
        self.assertIn("workflow_dispatch:", ci)
        self.assertIn("workflow_dispatch:", extended)
        self.assertNotIn("\n  schedule:", extended)

    def test_extended_quality_exposes_manual_stable_coverage_gate(self):
        workflow = (
            REPO_ROOT / ".github/workflows/extended-quality.yml"
        ).read_text(encoding="utf-8")

        self.assertIn("- coverage-stable", workflow)
        self.assertIn("name: Stable Coverage Report", workflow)
        self.assertIn("./scripts/testing/coverage-check.sh stable", workflow)
        self.assertIn("cargo install cargo-llvm-cov --locked", workflow)
        self.assertIn("actions/upload-artifact@v4", workflow)
        self.assertIn("target/tct-coverage/stable", workflow)

    def test_extended_quality_exposes_bpe_artifact_reproducibility_gate(self):
        workflow = (
            REPO_ROOT / ".github/workflows/extended-quality.yml"
        ).read_text(encoding="utf-8")

        self.assertIn("- bpe-artifact-reproducibility", workflow)
        self.assertIn("name: BPE Artifact Reproducibility", workflow)
        self.assertIn(
            "python3 scripts/testing/test-bpe-artifact-reproducibility.py",
            workflow,
        )
        self.assertIn("target/tct-generated-cargo/bpe-artifact-reproducibility", workflow)


if __name__ == "__main__":
    unittest.main()
