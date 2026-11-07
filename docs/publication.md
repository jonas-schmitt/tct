# Preparing a public TCT source release

The public project and distribution name is `tct`. TCT implementation is
Apache-2.0; imported material retains its recorded terms. Consult
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) and the per-component
inventories before choosing a publication scope.

## Keep the private archive separate

The existing private release and research histories are recovery archives, not
implicit public inputs. Preserve them. Removing a file from the current tree
does not remove it from earlier commits. A public source export must contain
only the reviewed files and a new, independent Git history; it must not carry
private objects, refs, alternates, hooks, or unreviewed data.

The public repository has one initial release commit containing the reviewed
source and demonstration artifacts. The complete earlier history and preparation
receipts remain in the private `tct-research` archive. They are historical
records; public verification does not need that repository or any old TCT commit.

## Validate source and notices

```bash
python3 third_party/clearance/instances/verify.py --require-exclusions
python3 third_party/clearance/vendored/verify_inventory.py
python3 third_party/clearance/schemas/verify.py --public-scope
python3 scripts/analysis/public_bpe_source_manifest.py
```

These checks require retained source/notice hashes and reject uncleared inputs.
Original source reconstruction and optional upstream-cache checks are described
in the component README files.

## Reproduce the demonstration BPE tables

`publication/bpe-training-spec.json` fixes all six recipes, twelve independently
authored fixtures, requested vocabulary budgets, and output names. The version 2
manifest `third_party/clearance/bpe-artifacts.json` binds the current source by
content: sorted file paths, executable modes, byte lengths, and SHA-256 hashes.
It covers the complete filesystem source tree without consulting `.gitignore`.
Only the manifest itself, its twelve separately hashed BPE table/sidecar files,
root `.git`, `target`, `.venv`, `venv`, `.pytest_cache`, `.mypy_cache`, and
`.ruff_cache`, and Python `__pycache__`/`.pytest_cache` directories are excluded.
Source symlinks are rejected. Added, missing, modified, or mode-changed source
files fail verification, even when Git would ignore them.

The original sidecars are retained byte-for-byte. Their `repo_commit` records
the original private training snapshot. The manifest labels this commit, its
tree, training date, and external receipt digest as `original_training` with
`role: archival_training_record`. These identifiers are not public checkout
requirements. A fresh run binds the current source-content digest and verifies
that it still reproduces every table and its complete token-corpus evidence.

```bash
# Populate the dependency cache once; reproduction itself runs offline.
cargo fetch --locked
python3 scripts/testing/test-bpe-artifact-reproducibility.py --public-artifacts
```

Public mode always selects all six recipes, rejects dry runs and partial
selection, clears inherited source/compiler/build overrides, uses the current
checkout as `TCT_SOURCE_DIR`, and builds the workspace with `--locked`. Generated
encoder builds inherit offline mode and the workspace dependency lockfile.
The run uses a fresh work subdirectory and records tool versions, admitted
build settings, per-table raw hashes and token-corpus comparisons, and the
complete source digest before and after execution. Both comparisons must pass
for every table. Regenerated files never overwrite the original tables or
sidecars. `--cargo-home` and `--cargo-target-dir` explicitly select caches;
inherited Cargo overrides are not used. Cache paths within the checkout must
be under `target/`, and generated temporary crates stay in the fresh run
directory. See `DEVELOPMENT.md` for output paths.

After reviewed source changes, refresh the source snapshot and rerun all six
recipes before qualifying a release:

```bash
python3 scripts/analysis/public_bpe_source_manifest.py --write
python3 scripts/testing/test-bpe-artifact-reproducibility.py --public-artifacts
```

Refreshing a manifest alone is not a reproduction result. These small authored
examples demonstrate the pipeline; they do not establish model quality or
historical performance. The manifest records actual attained vocabulary sizes,
which can be smaller than the requested budget.

## Secret and artifact gates

Use `scripts/release/scan-public-source.py --help` for the pinned scanner and
receipt arguments. Scan the final public commit and refs, all historical blobs,
Git metadata, and encoded JSON payloads. Findings are redacted. Any accepted
non-secret match must be independently reviewed and bound to its exact blob,
rule, and location; blanket fixture or rule suppressions do not qualify it.
Rerun after changes because a clean receipt applies only to its recorded inputs.

Run the stable gate, generated-artifact reproducibility checks, the README
quickstart, and the applicable wheel checks on the final public candidate:

```bash
./scripts/testing/quality-check.sh
./scripts/testing/test-generated-reproducibility.sh
```

The mandatory BPE release check is `--public-artifacts` above. The generic
power-of-two queue and individual `--artifact` checks cannot qualify this set.
The manual BPE workflow uses the same all-six mode.

Inspect the actual Cargo source package and wheel ZIPs. Component-bearing
artifacts must include the exact source form and notices described in
`third_party/clearance/schemas/ESLINT_SOURCE_NOTICE.md`. The root compiler wheel
embeds that component; generated schema payload selection conservatively follows recognized input
references and can retain unused referenced source.
The archive tests in `tests/python/test_distribution_license_payloads.py`
require explicit paths to built root, component-bearing, and unrelated wheels;
configuration-only passes or skipped archive tests do not qualify a binary.

Record commit IDs, tool versions, commands, outcomes, hashes, and any incomplete
checks under `target/`. A source clearance does not qualify arbitrary linked
binaries, new input datasets, or a later Git history. Repository visibility,
remote replacement, tags, and registry uploads are separate publication actions.
