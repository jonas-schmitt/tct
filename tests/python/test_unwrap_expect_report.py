#!/usr/bin/env python3
"""Tests for the unwrap()/expect() policy report helper."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
HELPER = REPO_ROOT / "scripts/testing/unwrap-expect-report.py"


class TestUnwrapExpectReport(unittest.TestCase):
    def test_self_test_passes(self):
        subprocess.run(
            [sys.executable, str(HELPER), "--self-test"],
            check=True,
            capture_output=True,
            text=True,
        )

    def test_json_reports_policy_classification(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            src = root / "src"
            src.mkdir()
            (src / "lib.rs").write_text(
                "\n".join(
                    [
                        "pub fn unchecked(value: Option<u8>) -> u8 {",
                        "    value.unwrap()",
                        "}",
                        "",
                        "pub fn documented(value: Option<u8>) -> u8 {",
                        "    value.expect(\"schema discovery guarantees a value\")",
                        "}",
                        "",
                        "#[cfg(test)]",
                        "mod tests {",
                        "    fn helper(value: Option<u8>) -> u8 {",
                        "        value.unwrap()",
                        "    }",
                        "}",
                    ]
                ),
                encoding="utf-8",
            )
            policy = root / "policy.toml"
            policy.write_text(
                "\n".join(
                    [
                        "[[allow]]",
                        'path = "src/lib.rs"',
                        'pattern = "schema discovery guarantees a value"',
                        'reason = "documented test policy"',
                        'category = "documented_invariant"',
                    ]
                ),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--root",
                    str(src),
                    "--policy",
                    str(policy),
                    "--json-output",
                    str(root / "unwrap-report.json"),
                    "--json",
                ],
                cwd=root,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.returncode, 1)
            report = json.loads(completed.stdout)
            self.assertEqual(report["artifact_type"], "tct-unwrap-expect-report")
            self.assertEqual(report["version"], 1)
            self.assertEqual(report["tool"], "unwrap-expect-report")
            self.assertEqual(report["total_sites"], 3)
            self.assertEqual(report["non_test_sites"], 2)
            self.assertEqual(report["documented_invariant_sites"], 1)
            self.assertEqual(report["unclassified_non_test_sites"], 1)
            self.assertEqual(report["inline_test_sites"], 1)
            self.assertEqual(report["unclassified"][0]["kind"], "unwrap")
            self.assertEqual(
                report["documented"][0]["policy"]["category"],
                "documented_invariant",
            )
            written = json.loads((root / "unwrap-report.json").read_text(encoding="utf-8"))
            self.assertEqual(written, report)

    def test_json_clean_policy_exits_zero(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            src = root / "src"
            src.mkdir()
            (src / "lib.rs").write_text(
                "pub fn documented(value: Option<u8>) -> u8 { "
                "value.expect(\"checked before construction\") }\n",
                encoding="utf-8",
            )
            policy = root / "policy.toml"
            policy.write_text(
                "\n".join(
                    [
                        "[[allow]]",
                        'path = "src/lib.rs"',
                        'pattern = "checked before construction"',
                        'reason = "documented invariant"',
                    ]
                ),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--root",
                    str(src),
                    "--policy",
                    str(policy),
                    "--json",
                ],
                cwd=root,
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(completed.stdout)
            self.assertEqual(report["unclassified_non_test_sites"], 0)
            self.assertEqual(report["documented_invariant_sites"], 1)


if __name__ == "__main__":
    unittest.main()
