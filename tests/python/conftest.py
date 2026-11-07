"""Session fixtures for TCT Python tests."""

from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

# Mark this suite run (and every descendant process) so scripts that would
# re-run the suite refuse instead of recursing: quality-check.sh's Python
# maintenance gate checks this sentinel so a suite -> quality-check.sh ->
# suite chain cannot fork without bound.
# `unittest discover` does not load conftest.py; that path is marked by the
# gate command in quality-check.sh and by test_quality_check_script.py itself.
os.environ.setdefault("TCT_PYTHON_SUITE_ACTIVE", "1")

# Ensure tests/python is importable when pytest is invoked from the repo root
# without an explicit PYTHONPATH.
_TESTS_PYTHON = Path(__file__).resolve().parent
if str(_TESTS_PYTHON) not in sys.path:
    sys.path.insert(0, str(_TESTS_PYTHON))

from tct_go_toolchain import GoToolchainError, export_into_os_environ  # noqa: E402


@pytest.fixture(scope="session", autouse=True)
def _export_pinned_go_toolchain() -> None:
    """Ensure the inventory-pinned Go is on PATH / TCT_GO_BIN for the suite."""
    try:
        export_into_os_environ()
    except GoToolchainError as exc:
        import os

        require = os.environ.get("TCT_REQUIRE_GO_SIDECARS", "").lower() in {
            "1",
            "true",
            "yes",
        }
        if require:
            raise
        print(f"conftest: pinned Go export skipped: {exc}")
