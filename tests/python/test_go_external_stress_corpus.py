#!/usr/bin/env python3
"""Smoke tests for the Go external stress corpus pin/fetch/enumerate helper."""

import os
import shutil
import subprocess
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/go_external_stress_corpus.go"
PINS = REPO_ROOT / "tests/fixtures/go_external_stress/pins-v1.json"


def go_binary():
    from tct_go_toolchain import go_binary as _go_binary

    return _go_binary()


class TestGoExternalStressCorpus(unittest.TestCase):
    def setUp(self):
        self.go = go_binary()
        if not self.go:
            self.skipTest("Go toolchain unavailable; set TCT_GO_BIN to run this smoke")

    def test_pins_fixture_exists(self):
        self.assertTrue(PINS.is_file(), f"missing pins fixture: {PINS}")

    def test_self_test_passes(self):
        completed = subprocess.run(
            [self.go, "run", str(SCRIPT), "--self-test"],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn("go_external_stress_corpus self-test passed", completed.stdout)


if __name__ == "__main__":
    unittest.main()
