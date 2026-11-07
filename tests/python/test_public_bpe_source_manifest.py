"""Content closure and authored-input regressions for the public BPE manifest."""
import copy
import importlib.util
import json
import shutil
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("public_bpe_source_manifest", ROOT / "scripts/analysis/public_bpe_source_manifest.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def public_fixture(root):
    manifest = MODULE.read_json(ROOT / MODULE.MANIFEST_PATH)
    spec = MODULE.read_json(ROOT / MODULE.SPEC_PATH)
    paths = [MODULE.SPEC_PATH, "Cargo.lock"] + [row["path"] for row in manifest["files"]]
    for dataset in spec["datasets"].values():
        paths += [dataset["schema"]] + [row["path"] for row in dataset["fixtures"]]
    for relative in set(paths):
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / relative, path)
    (root / MODULE.MANIFEST_PATH).parent.mkdir(parents=True, exist_ok=True)
    (root / MODULE.MANIFEST_PATH).write_text(json.dumps(manifest))
    MODULE.write_snapshot(root)
    return manifest


class SourceSnapshotTests(unittest.TestCase):
    def test_closure_without_git_and_build_exclusions(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "source.rs").write_text("source\n")
            snapshot = MODULE.source_snapshot(root)
            for relative in ["target/build/file", ".git/config", ".venv/bin/python", "nested/__pycache__/cache.pyc", ".pytest_cache/state"]:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("cache")
            self.assertEqual(snapshot, MODULE.source_snapshot(root))
            MODULE.verify_snapshot(root, snapshot)

    def test_source_edit_add_delete_and_mode_change(self):
        for change in ("edit", "add", "delete", "mode"):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                path = root / "source.rs"
                path.write_text("source\n")
                snapshot = MODULE.source_snapshot(root)
                if change == "edit":
                    path.write_text("other\n")
                elif change == "add":
                    (root / "new.rs").write_text("new")
                elif change == "delete":
                    path.unlink()
                else:
                    path.chmod(0o755)
                with self.assertRaisesRegex(ValueError, "snapshot"):
                    MODULE.verify_snapshot(root, snapshot)

    def test_reject_unsafe_duplicate_paths_and_symlinks(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "a").write_text("a")
            snapshot = MODULE.source_snapshot(root)
            for relative in ("../a", "/a", "x/../a", "x\\a"):
                bad = copy.deepcopy(snapshot)
                bad["files"][0]["path"] = relative
                with self.assertRaises(ValueError):
                    MODULE.verify_snapshot(root, bad)
            bad = copy.deepcopy(snapshot)
            bad["files"] *= 2
            with self.assertRaises(ValueError):
                MODULE.verify_snapshot(root, bad)
            (root / "link").symlink_to("a")
            with self.assertRaisesRegex(ValueError, "symlink"):
                MODULE.source_snapshot(root)

    def test_artifact_and_input_contracts(self):
        for change in ("none", "partial", "sidecar", "extra_fixture", "changed_fixture", "extra_bpe"):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                public_fixture(root)
                if change == "partial":
                    (root / "schemas/bpe/package-tct-bpe-500.json").unlink()
                elif change == "sidecar":
                    path = root / "schemas/bpe/package-tct-bpe-500.provenance.json"
                    path.write_text(path.read_text() + "\n")
                elif change in ("extra_fixture", "changed_fixture"):
                    name = "public_extra.json" if change == "extra_fixture" else "public_cli.json"
                    (root / "tests/data/instances/package/synthetic" / name).write_text("{}")
                elif change == "extra_bpe":
                    (root / "schemas/bpe/extra.json").write_text("{}")
                if change == "none":
                    self.assertEqual(len(MODULE.validate_manifest(root)["artifacts"]), 6)
                else:
                    with self.assertRaises(ValueError):
                        MODULE.validate_manifest(root)

    def test_partial_or_duplicate_spec_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            public_fixture(root)
            spec = MODULE.read_json(root / MODULE.SPEC_PATH)
            spec["artifacts"].pop()
            (root / MODULE.SPEC_PATH).write_text(json.dumps(spec))
            with self.assertRaisesRegex(ValueError, "six"):
                MODULE.load_spec(root)
