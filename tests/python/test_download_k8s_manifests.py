#!/usr/bin/env python3
"""Tests for the Kubernetes manifest downloader's safety filters."""

import importlib.util
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/data/download_k8s_manifests.py"


def load_script():
    # The script imports `datasets` and `yaml` only inside main(), so loading
    # the module needs nothing beyond the standard library.
    spec = importlib.util.spec_from_file_location("download_k8s_manifests", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CarriesCredentialsTest(unittest.TestCase):
    def setUp(self):
        self.script = load_script()

    def test_secret_manifests_are_credential_bearing(self):
        self.assertTrue(self.script.carries_credentials({"kind": "Secret", "data": {"k": "dg=="}}))
        self.assertTrue(self.script.carries_credentials({"kind": "SecretList", "items": []}))

    def test_a_list_wrapping_a_secret_is_credential_bearing(self):
        wrapped = {
            "kind": "List",
            "items": [{"kind": "ConfigMap"}, {"kind": "Secret", "stringData": {"k": "v"}}],
        }
        self.assertTrue(self.script.carries_credentials(wrapped))
        nested = {"kind": "List", "items": [{"kind": "List", "items": [{"kind": "Secret"}]}]}
        self.assertTrue(self.script.carries_credentials(nested))

    def test_ordinary_manifests_are_kept(self):
        for manifest in (
            {"kind": "Deployment", "spec": {"replicas": 1}},
            {"kind": "ConfigMap", "data": {"kind": "Secret"}},
            {"kind": "List", "items": [{"kind": "Service"}]},
            {"kind": "List", "items": "not-a-list"},
            {},
        ):
            self.assertFalse(self.script.carries_credentials(manifest), manifest)

    def test_non_mapping_input_is_not_credential_bearing(self):
        for value in (None, "Secret", ["Secret"], 7):
            self.assertFalse(self.script.carries_credentials(value), value)


class ManifestFilenameTest(unittest.TestCase):
    def setUp(self):
        self.script = load_script()

    def test_plain_kinds_are_lowercased_and_indexed(self):
        self.assertEqual(self.script.manifest_filename("Deployment", 7), "deployment_000007.json")
        self.assertEqual(self.script.manifest_filename("ClusterRole", 12), "clusterrole_000012.json")

    def test_hostile_kinds_cannot_leave_the_output_directory(self):
        output_dir = Path("corpus")
        for kind in (
            "../../etc/cron.d/job",
            "a/b",
            "..\\..\\windows",
            "/absolute",
            "..",
            ".",
            "",
            "k\x00ind",
            "kind\nname",
        ):
            name = self.script.manifest_filename(kind, 3)
            self.assertRegex(name, r"^[a-z0-9_-]+_000003\.json$", repr(kind))
            self.assertEqual((output_dir / name).parent, output_dir, repr(kind))

    def test_a_kind_without_usable_characters_gets_a_placeholder(self):
        self.assertEqual(self.script.manifest_filename("///", 1), "unknown_000001.json")


if __name__ == "__main__":
    unittest.main()
