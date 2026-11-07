# TCT Helper Scripts

Scripts are grouped by purpose. Core work (schema compilation, tokenization,
vocabulary analysis, BPE training, round-trip validation, and measurement)
belongs in the Rust library and the binaries under `src/bin/`; the scripts here
wrap those tools into repeatable gates or provide external utilities for data
collection, format conversion, and comparison. `DEVELOPMENT.md` documents the
workflows and report contracts in detail.

Run scripts from the repository root. Shell scripts need bash; Python scripts
need Python 3.11 or newer. The Python scripts, `local-ci.sh`,
`coverage-check.sh`, and `run-reproducible-tokenizer-benchmark.sh` print their
options with `--help`; the other shell gates take no arguments and run their
full gate.

## Testing (`scripts/testing/`)

Gates:

- `scripts/testing/quality-check.sh` - Stable local gate (disk preflight,
  clippy with zero warnings, rustfmt, `cargo test`, Python maintenance tests,
  release build) plus advisory reports; `TCT_STRICT_QUALITY=1` makes advisory
  findings fail.
- `scripts/testing/local-ci.sh` - Local mirror of the ordinary CI job; runs
  `quality-check.sh` (which includes the default round-trip smoke).
- `scripts/testing/test-round-trip-bpe-smoke.sh` - Default round-trip smoke plus
  `github-workflow` and `package`, each without BPE and with the
  `legacy-tct-500` preset.
- `scripts/testing/test-target-schemas.sh` - Fast O3 round trip for every schema
  in `schemas/target-schemas.txt`.
- `scripts/testing/test-target-schema-depth.sh` - Full-scenario release-build
  round trips for `schemas/target-schema-depth.txt`; writes
  `target/tct-target-schema-depth-report.json`.
- `scripts/testing/test-generated-reproducibility.sh` - Generated Cargo template
  drift check, one generated binary crate build, and one generated wheel build.
- `scripts/testing/test-wheel-builds.sh` - Records the packaging tool envelope,
  then runs the ignored streaming-wheel and bundle-wrapper build tests.
- `scripts/testing/test-bpe-artifact-reproducibility.py` - Retrains
  provenance-backed merge tables in `schemas/bpe/` and compares hashes;
  `--public-artifacts` is the release gate for all six shipped recipes.
  For development, `--dry-run` only validates inputs and `--artifact` selects
  tables outside the default power-of-two, at-most-2048 set.

Reports and helpers:

- `scripts/testing/coverage-check.sh` - Report-only `cargo llvm-cov` coverage
  (`quick`, `stable`, `synthetic-ignored`, `round-trip-full`) under
  `target/tct-coverage/`.
- `scripts/testing/coverage-focus.py` - Summarize coverage for chosen paths or
  files changed since a Git ref from a `summary.json`.
- `scripts/testing/module-size-report.py` - Advisory oversized-module report,
  classified by `scripts/testing/module-size-policy.toml`.
- `scripts/testing/documentation-coverage-report.py` - Advisory Rust module
  documentation coverage report.
- `scripts/testing/generated-source-inventory.py` - Advisory inventory of
  production code and tests that inspect generated source by substring.
- `scripts/testing/unwrap-expect-report.py` - Advisory `unwrap()`/`expect()`
  report, classified by `scripts/testing/unwrap-expect-policy.toml`.
- `scripts/testing/wheel-tool-envelope.py` - Record Python packaging tool
  versions and wheel build inputs; optionally fail when required tools are
  missing.

```bash
./scripts/testing/quality-check.sh
env TCT_STRICT_QUALITY=1 ./scripts/testing/quality-check.sh
./scripts/testing/local-ci.sh --dry-run
./scripts/testing/test-round-trip-bpe-smoke.sh
./scripts/testing/test-target-schemas.sh
env TARGET_SCHEMA_DEPTH_DRY_RUN=1 ./scripts/testing/test-target-schema-depth.sh
./scripts/testing/test-generated-reproducibility.sh
./scripts/testing/test-wheel-builds.sh
python3 scripts/testing/test-bpe-artifact-reproducibility.py --dry-run
./scripts/testing/coverage-check.sh quick
env TCT_COVERAGE_FOCUS=src/generator ./scripts/testing/coverage-check.sh quick
python3 scripts/testing/coverage-focus.py target/tct-coverage/stable/summary.json src/generator
python3 scripts/testing/module-size-report.py --json-output target/tct-quality/module-size-report.json --exit-zero
python3 scripts/testing/documentation-coverage-report.py --json-output target/tct-quality/documentation-coverage-report.json --exit-zero
python3 scripts/testing/generated-source-inventory.py --mode all --json-format artifact --json-output target/tct-quality/generated-source-inventory.json
python3 scripts/testing/unwrap-expect-report.py --root src --json-output target/tct-quality/unwrap-expect-report.json
python3 scripts/testing/wheel-tool-envelope.py --output target/tct-wheel-tool-envelope.json --require-wheel-build-tools
```

Ignored synthetic generated-crate round trips have no wrapper; run them with
cargo directly:

```bash
cargo test --test synthetic_schema_test -- --ignored
cargo test --test synthetic_schema_test test_simple_duplicate_round_trip -- --ignored --exact
```

## Analysis (`scripts/analysis/`)

- `scripts/analysis/bpe_artifact_manifest.py` - Deterministic inventory of
  checked-in merge tables with hashes, vocabulary sizes, and provenance status.
- `scripts/analysis/bpe_vocab_scaling.py` - Summarize `tct-measure-tokens`
  reports across several BPE vocabulary sizes (JSON and Markdown).
- `scripts/analysis/round_trip_validity_manifest.py` - Turn
  `target/tct-round-trip-report.json` into a per-schema validity manifest.

```bash
python3 scripts/analysis/bpe_artifact_manifest.py --output target/tct-bpe-artifact-manifest.json
python3 scripts/analysis/bpe_vocab_scaling.py \
  --entry package=base=target/tct-analysis/package-base-measure.json \
  --entry package=1024=target/tct-analysis/package-1024-measure.json \
  --json-output target/tct-analysis/bpe-vocab-scaling.json \
  --markdown-output target/tct-analysis/bpe-vocab-scaling.md
python3 scripts/analysis/round_trip_validity_manifest.py \
  --round-trip-report target/tct-round-trip-report.json \
  --output target/tct-round-trip-validity-manifest.json
```

## Benchmarking And Comparison

- `scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh` - Maintained
  generated-tokenizer measurement: runs `tct-measure-tokens`, preserves the JSON
  report, log, and measured file list, and optionally measures an importable
  wheel over the same files.
- `scripts/benchmarking/measure_wheel_encode.py` - Time `encode()` (and
  optionally `decode()`) of an already importable generated wheel module over a
  fixed instance set.
- `scripts/benchmarking/measure_tiktoken.py` - Count tiktoken tokens over a
  validation directory.

Use `run-reproducible-tokenizer-benchmark.sh` for maintained TCT runtime
evidence. It preserves the generated JSON report, stderr log, and measured file
manifest. `measure_tiktoken.py` gives an external-tokenizer baseline only.
Failed generated helper builds preserve the temporary project path for
inspection. Pass `--cleanup-generated-helper` when a successful benchmark run
should remove its temporary generated helper crate before exit; cleanup remains
explicit so failed build context stays available.

```bash
./scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh --help
./scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh \
  --schema schemas/popular/github-workflow.json \
  --instances "tests/data/instances/github-workflow/real/*.json" \
  --output-dir target/tct-benchmarks/generated-runtime/github-workflow \
  --warmup-iterations 1 \
  --cleanup-generated-helper
python3 scripts/benchmarking/measure_wheel_encode.py --help
python3 scripts/benchmarking/measure_tiktoken.py --help
```

## Data Collection (`scripts/data/`)

These collect and clean real-world instance corpora. The GitHub collectors need
a `GITHUB_TOKEN` environment variable and the `requests`, `pyyaml`, and
`jsonschema` packages.

- `scripts/data/collect_workflows.py` - Collect GitHub Actions workflow files
  from public repositories.
- `scripts/data/collect_workflows_adaptive.py` - Multi-pass workflow collection
  that adapts its star-range sampling to reach a target count.
- `scripts/data/clean_corpus.py` - Drop non-encodable and duplicate workflows
  and keep a target number of clean instances.
- `scripts/data/download_k8s_manifests.py` - Download Kubernetes manifests from
  a public dataset, skipping Secrets. Check the license of whatever you collect
  before committing it; see `tests/data/instances/PROVENANCE.md`.
- `scripts/data/recover_file_mapping.py` - Recover which source file produced
  each line of a prepared JSONL dataset.

```bash
python3 scripts/data/collect_workflows_adaptive.py --help
python3 scripts/data/clean_corpus.py --help
python3 scripts/data/download_k8s_manifests.py --help
python3 scripts/data/recover_file_mapping.py --help
```

## Conversion (`scripts/conversion/`)

- `scripts/conversion/yaml_to_json.py` - Convert a YAML file (or URL with
  `--url`) to JSON.
- `scripts/conversion/toml_to_json.py` - Convert a TOML file to JSON.

```bash
python3 scripts/conversion/yaml_to_json.py --help
python3 scripts/conversion/toml_to_json.py input.toml output.json
```

## Rust Binaries For Core Workflows

Prefer these over new Python scripts for core tokenizer, BPE, and measurement
work:

```bash
cargo run --release --bin tct-prepare-data -- --help
cargo run --release --bin tct-train-bpe -- --help
cargo run --release --bin tct-measure-tokens -- --help
cargo run --release --bin tct-analyze-usage -- --help
cargo run --release --bin tct-token-diff -- --help
cargo run --release --bin tct-build-bundle-wrapper -- --help
```

`tests/python/` holds maintenance tests for these scripts
(`python3 -m unittest discover tests/python`).

## Release verification

- `analysis/public_bpe_source_manifest.py` checks the complete content-addressed
  source snapshot, six public recipes, authored inputs, and original artifacts;
  `--write` refreshes the snapshot after reviewed source changes.
- `testing/test-bpe-artifact-reproducibility.py --public-artifacts` reproduces all
  six shipped tables and compares raw hashes and complete token-corpus evidence.
- `release/scan-public-source.py` runs the pinned maintained secret scanner
  against committed files, history, metadata, and encoded JSON payloads.

See `../docs/publication.md` for the source and artifact qualification gates.
