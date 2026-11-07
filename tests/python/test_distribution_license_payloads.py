"""Verify component source and notices in configuration and actual wheel archives.

The archive checks require explicit TCT_ROOT_WHEEL, TCT_SCHEMA_COMPONENT_WHEEL,
or TCT_UNRELATED_SCHEMA_WHEEL paths. Missing artifacts are reported as skipped,
so a configuration-only run cannot be mistaken for wheel qualification.
"""

from email.parser import BytesParser
import json
import os
from pathlib import Path
import tomllib
import zipfile

import pytest


ROOT = Path(__file__).resolve().parents[2]
COMPONENT_SOURCE = "schemas/external/partial-eslint-plugins.json"
COMPONENT_RECEIPTS = [
    "third_party/clearance/schemas/ESLINT_SOURCE_NOTICE.md",
    "third_party/clearance/schemas/embedded-schemas.json",
    "third_party/clearance/schemas/eslint-plugin-inputs.json",
    "third_party/clearance/schemas/eslint-schema-changes.json",
]


def component_payload_paths():
    ledger = json.loads((ROOT / "third_party/clearance/schemas/embedded-schemas.json").read_text())
    component = next(entry for entry in ledger["files"] if entry["local_path"] == COMPONENT_SOURCE)
    return [COMPONENT_SOURCE, *COMPONENT_RECEIPTS, *component["retained_notices"]]


def wheel_from_environment(variable):
    value = os.environ.get(variable)
    if not value:
        pytest.skip(f"set {variable} to an actual built wheel; archive not qualified")
    path = Path(value)
    assert path.is_file(), f"{variable} does not name a wheel file: {path}"
    return path


def test_root_maturin_includes_exact_component_payload():
    project = tomllib.loads((ROOT / "pyproject.toml").read_text())
    include = project["tool"]["maturin"].get("include", [])
    for path in ["LICENSE", "NOTICE", *component_payload_paths()]:
        assert path in include, f"root wheel must explicitly include {path}"


@pytest.mark.parametrize("variable", ["TCT_ROOT_WHEEL", "TCT_SCHEMA_COMPONENT_WHEEL"])
def test_component_wheel_contains_exact_source_and_notices(variable):
    wheel_path = wheel_from_environment(variable)
    with zipfile.ZipFile(wheel_path) as wheel:
        members = wheel.namelist()
        for path in ["LICENSE", "NOTICE", *component_payload_paths()]:
            assert members.count(path) == 1, f"missing or duplicate payload path: {path}"
            assert wheel.read(path) == (ROOT / path).read_bytes(), f"payload differs: {path}"
        metadata_paths = [path for path in members if path.endswith(".dist-info/METADATA")]
        assert len(metadata_paths) == 1
        metadata = BytesParser().parsebytes(wheel.read(metadata_paths[0]))
        assert metadata["License"] == "Apache-2.0", "TCT project license must remain Apache-2.0"


def test_unrelated_schema_wheel_omits_plugin_component():
    wheel_path = wheel_from_environment("TCT_UNRELATED_SCHEMA_WHEEL")
    with zipfile.ZipFile(wheel_path) as wheel:
        members = wheel.namelist()
        assert not any(path.endswith(COMPONENT_SOURCE) for path in members)
        assert not any("third_party/licenses/jsonschema-extractor/" in path for path in members)
        assert not any("third_party/licenses/schema-eslint-inputs/" in path for path in members)
