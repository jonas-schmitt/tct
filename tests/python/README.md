# TCT Python Test Suite

Maintenance tests for the repository's Python and shell helper scripts:
quality gates, coverage and policy reports, BPE artifact manifests, wheel
tooling, and benchmarking scope checks. None of these tests require a built
schema wheel; wheel validation lives in
`tests/rebuild_github_workflow_wheel_test.rs` and
`scripts/testing/test-wheel-builds.sh`.

The tests that import the root `tct` extension module
(`test_compiler_bindings.py`, and the SQL runtime tests under the
`experimental` feature) are skipped unless it is installed. Build and install
it into a virtualenv with `maturin develop`, then run them:

```bash
maturin develop
python3 -m pytest tests/python/test_compiler_bindings.py -q
```

## Running Tests

The stable quality helper runs this suite as the Python maintenance gate:

```bash
python3 -m unittest discover tests/python
```

pytest works as well:

```bash
python3 -m pytest tests/python/ -q
```

Run one module directly when iterating:

```bash
python3 -m pytest tests/python/test_quality_check_script.py -q
```

## Setup

The tests use the Python standard library plus `pytest` when invoked through
pytest:

```bash
cd tests/python
pip install -r requirements.txt
```

## Test Structure

```
tests/python/
├── __init__.py          # Package initialization
├── conftest.py          # Marks suite runs so gates never recurse into themselves
├── pytest.ini           # Pytest settings
├── requirements.txt     # Optional pytest dependencies
├── test_*.py            # Helper-script maintenance tests
└── README.md            # This file
```

Each `test_*.py` module covers one maintained helper under `scripts/` (for
example, `test_quality_check_script.py` covers
`scripts/testing/quality-check.sh`); `test_github_workflows.py` checks the
workflow definitions under `.github/workflows/`. Tests execute the helpers as
subprocesses or import them by path, so they validate the same entry points
the documented workflows use.

## Contributing

When adding new tests:

1. Follow the existing `unittest.TestCase` structure so
   `python3 -m unittest discover tests/python` finds them.
2. Use descriptive test names (`test_<what>_<condition>`).
3. Add docstrings explaining test purpose.
4. Keep tests hermetic: no network access, no dependence on uncommitted
   artifacts, and write scratch output under `target/`.

## Related Documentation

- **DEVELOPMENT.md**: Command reference and validation layers
- **scripts/README.md**: Helper script index
