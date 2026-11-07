#!/usr/bin/env python3
"""Tests for deterministic route source manifest generation."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/route_source_manifest.py"


class TestRouteSourceManifest(unittest.TestCase):
    def test_self_test(self):
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "--self-test"],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn("self-test passed", completed.stdout)

    def test_writes_sorted_csharp_manifest_and_report(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            source_root = tmp / "repo"
            nested = source_root / "nested"
            generated = source_root / "generated"
            ignored = source_root / "obj"
            nested.mkdir(parents=True)
            generated.mkdir()
            ignored.mkdir()
            (source_root / "z.cs").write_text("class Z {}\n", encoding="utf-8")
            (nested / "a.cs").write_text("class A {}\n", encoding="utf-8")
            (generated / "ignored.cs").write_text(
                "class Generated {}\n",
                encoding="utf-8",
            )
            (ignored / "ignored.cs").write_text("class Bad {}\n", encoding="utf-8")
            output = tmp / "sources.txt"
            report_path = tmp / "report.json"
            markdown_path = tmp / "report.md"

            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-root",
                    str(source_root),
                    "--output",
                    str(output),
                    "--json-output",
                    str(report_path),
                    "--markdown-output",
                    str(markdown_path),
                    "--exclude-glob",
                    "generated/*",
                    "--min-files",
                    "2",
                    "--corpus-id",
                    "test-csharp-corpus",
                    "--corpus-source",
                    "temporary test repository",
                    "--corpus-license",
                    "test-only",
                    "--selection-note",
                    "exclude generated sources",
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(report_path.read_text(encoding="utf-8"))
            manifest_lines = output.read_text(encoding="utf-8").splitlines()
            markdown = markdown_path.read_text(encoding="utf-8")

        self.assertEqual(
            manifest_lines,
            [
                "# tct-csharp-route-corpus-v1",
                "nested/a.cs",
                "z.cs",
            ],
        )
        self.assertEqual(report["tool"], "route_source_manifest")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["target"], "csharp")
        self.assertEqual(report["entry_count"], 2)
        self.assertEqual(report["minimum_entry_count"], 2)
        self.assertEqual(report["exclude_globs"], ["generated/*"])
        self.assertEqual(
            report["corpus_provenance"],
            {
                "provided": True,
                "corpus_id": "test-csharp-corpus",
                "source": "temporary test repository",
                "license": "test-only",
                "selection_note": "exclude generated sources",
            },
        )
        self.assertFalse(report["fixture_scope"])
        self.assertEqual(report["evidence_role"], "route_intended_corpus")
        self.assertEqual(
            report["corpus_fingerprint"]["fingerprint_kind"],
            "path_sha256_list_v1",
        )
        self.assertEqual(report["corpus_fingerprint"]["entry_count"], 2)
        self.assertEqual(len(report["corpus_fingerprint"]["sha256"]), 64)
        self.assertEqual(
            [entry["manifest_path"] for entry in report["entries"]],
            ["nested/a.cs", "z.cs"],
        )
        self.assertIn("# C# Route Source Manifest", markdown)
        self.assertIn("- Entry count: 2", markdown)
        self.assertIn("- Corpus ID: `test-csharp-corpus`", markdown)
        self.assertIn("- Corpus license: `test-only`", markdown)
        self.assertIn("`generated/*`", markdown)
        self.assertIn("`nested/a.cs`", markdown)

    def test_limit_is_applied_after_sorting(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            source_root = tmp / "repo"
            source_root.mkdir()
            (source_root / "b.go").write_text("package b\n", encoding="utf-8")
            (source_root / "a.go").write_text("package a\n", encoding="utf-8")
            output = tmp / "sources.txt"

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "go-syntax",
                    "--source-root",
                    str(source_root),
                    "--output",
                    str(output),
                    "--limit",
                    "1",
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(completed.stdout)
            manifest_lines = output.read_text(encoding="utf-8").splitlines()

        self.assertEqual(manifest_lines[1:], ["a.go"])
        self.assertEqual(report["entry_count"], 1)
        self.assertEqual(report["corpus_fingerprint"]["entry_count"], 1)

    def test_min_files_fails_after_limit(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            source_root = tmp / "repo"
            source_root.mkdir()
            (source_root / "a.cs").write_text("class A {}\n", encoding="utf-8")
            (source_root / "b.cs").write_text("class B {}\n", encoding="utf-8")
            output = tmp / "sources.txt"

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-root",
                    str(source_root),
                    "--output",
                    str(output),
                    "--limit",
                    "1",
                    "--min-files",
                    "2",
                ],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
            )

        self.assertEqual(completed.returncode, 1)
        self.assertIn("minimum is 2", completed.stderr)
        self.assertFalse(output.exists())

    def test_missing_sources_fail_with_context(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            source_root = tmp / "repo"
            source_root.mkdir()
            output = tmp / "sources.txt"

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-root",
                    str(source_root),
                    "--output",
                    str(output),
                ],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
            )

        self.assertEqual(completed.returncode, 1)
        self.assertIn("no .cs files found", completed.stderr)
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
