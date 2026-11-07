# TCT Development Guide

This guide is for developers who want to build, test, extend, or measure TCT,
the compiler that turns a JSON Schema into a schema-specific tokenizer. It is
the command reference for the repository: every workflow below names the
script, test, binary, flag, or environment variable that implements it.

Related documents: `README.md` (overview and usage), `ARCHITECTURE.md`
(four-pass design, decode contract, sequence wrappers), `CONTRIBUTING.md`
(pull requests), `scripts/README.md` (helper script index), and
`docs/huggingface-compatible-tokenizer-contract.md` (Hugging Face sidecar
contract). `schemas/README.md`, `tests/data/instances/README.md`,
`tests/python/README.md`, and `tests/benchmarks/README.md` hold local notes.

The crate also carries experimental non-JSON frontends behind the
`experimental` Cargo feature (off by default). Their gates and helpers are
documented separately in `docs/frontends-development.md`; they run only with
`--features experimental` and are not part of the stable checks below.

## Quick Command Index

Use this as a lookup table. The section "Testing And Validation Layers" explains
which of these a given change needs.

```bash
# Build and stable checks
cargo build --release
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
./scripts/testing/quality-check.sh
./scripts/testing/local-ci.sh

# Round trips and target schemas
cargo test --test round_trip_test test_round_trip
./scripts/testing/test-round-trip-bpe-smoke.sh
./scripts/testing/test-target-schemas.sh
./scripts/testing/test-target-schema-depth.sh
cargo test --test synthetic_schema_test -- --ignored

# Generated crates, wheels, and reproducibility
cargo test generated_crate::tests
./scripts/testing/test-generated-reproducibility.sh
./scripts/testing/test-wheel-builds.sh
python3 scripts/testing/test-bpe-artifact-reproducibility.py --dry-run

# Coverage (report-only)
./scripts/testing/coverage-check.sh quick
./scripts/testing/coverage-check.sh stable

# BPE training, measurement, and schema evolution
cargo run --release --bin tct-prepare-data -- --help
cargo run --release --bin tct-train-bpe -- --help
cargo run --release --bin tct-measure-tokens -- --help
cargo run --release --bin tct-token-diff -- --help

# Benchmarks (opt-in)
./scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh --help
```

## Building And Toolchain

TCT is a single Rust library crate named `tct` plus a set of binaries.

```bash
cargo build --release
./target/release/tct --help
```

The Rust toolchain is pinned by `rust-toolchain.toml` (channel plus the
`rustfmt` and `clippy` components). With `rustup` installed, plain `cargo`
selects the pinned compiler automatically; do not pass `+toolchain` overrides
unless you are deliberately testing another compiler.

Three places carry the supported Rust version and must change together in one
commit:

- `rust-toolchain.toml` (`channel`).
- `Cargo.toml` (`rust-version`).
- `src/generated_crate.rs` (`GENERATED_RUST_VERSION`, the `rust-version`
  written into every generated crate and wheel package).

The focused test `cargo test generated_crate::tests` fails when these drift.

### Cargo Features

| Feature | Purpose |
|---|---|
| (default) | Library, compiler, and the core binaries. No Python dependency and no experimental frontend code. |
| `experimental` | Compiles the experimental non-JSON frontends (SQL schema linking, the syntax layer with Go and C# targets, tree-sitter subset codecs, the exact-mask contract), their binaries and integration tests, and, together with `python`, the SQL schema-linking Python bindings. No stability guarantees; see `docs/frontends-development.md`. |
| `python` | Compiles the root PyO3 bindings in `src/python.rs` so their unit tests run under `cargo test --features python`. |
| `python-extension` | `python` plus `pyo3/extension-module`. Selected by `pyproject.toml` for maturin builds of the root `tct` Python module. Do not use it for `cargo test`. |
| `extended-benchmarks` | Enables the report-only extended schema compilation benchmark in `tests/schema_compilation_test.rs`. |
| `round-trip-tests`, `comprehensive-tests` | Gate additional opt-in suites inside `tests/schema_compilation_test.rs`. |

If the active Python points at a shared library that is not on the runtime
linker path, set PyO3's standard `PYO3_PYTHON` variable to an interpreter whose
shared library is loadable before running feature-gated Rust tests.

### Binaries

All binaries live in `src/bin/` except `tct`, whose entry point is
`src/main.rs`. Run any of them with `cargo run --release --bin <name> -- --help`.

| Binary | Purpose |
|---|---|
| `tct` | Compile a schema to a Rust tokenizer module or a Python wheel; analyze vocabulary requirements. |
| `tct-prepare-data` | Encode an instance corpus once and write train/validate/all token JSONL with provenance. |
| `tct-train-bpe` | Train a BPE merge table on TCT token sequences; writes a provenance sidecar. |
| `tct-measure-tokens` | Measure token counts, context-window fit, runtime, and allocations for a schema and corpus. |
| `tct-analyze-usage` | Report which token IDs a corpus uses, grouped by numeric range (0-255, 256 and above, `u32::MAX`). |
| `tct-bpe-lookup-benchmark` | Compare BPE pair-lookup strategies over one encoded corpus. |
| `tct-bpe-length-stats` | Report median, mean, and p90 sequence lengths for base or BPE-compressed token JSONL. |
| `tct-token-diff` | Compare token-ID layouts of two schema versions and classify reuse safety. |
| `tct-build-bundle-wrapper` | Write a pure-Python multi-schema bundle wrapper package from a JSON spec. |
| `tct-validate-direct-hf-profile` | Validate a serialized direct Hugging Face compatibility profile (see `docs/huggingface-compatible-tokenizer-contract.md`). |
| `tct-validate-server` | Compile a schema once, then validate instance paths read line by line from stdin. |
| `tct-transliterate-workflows` | Transliterate a directory of JSON files to ASCII (`--input`, `--output`, `--report`). |

### The `tct` Command

```bash
# Generate Rust tokenizer source (default optimization level: O3)
cargo run --release --bin tct -- schemas/popular/package.json -o target/package_tokenizer.rs

# Render without writing, to check that a schema compiles
cargo run --release --bin tct -- schemas/popular/package.json --dry-run -o target/package_tokenizer.rs

# Development build of the generated code (no optimizations)
cargo run --release --bin tct -- schemas/popular/package.json -O 0 -o target/package_debug.rs

# Vocabulary analysis instead of code generation
cargo run --release --bin tct -- schemas/popular/package.json --analyze
cargo run --release --bin tct -- schemas/popular/package.json --analyze --format json --compare 257,512,1024

# Python wheel (requires maturin; see "Python Wheels")
cargo run --release --bin tct -- schemas/popular/github-workflow.json \
  --target python --python-name tct-github-workflow --streaming -o wheels
```

Flags worth knowing:

- `-O, --optimization`: `0`/`O0`, `2`/`O2`, `3`/`O3` (default), `s`/`Os`/`size`,
  or `vocab`/`OVocab`.
- `--enable-pass NAME` (repeatable): select named built-in passes
  (`enum_optimization`, `integer_optimization`, `vocabulary_optimization`)
  instead of a level. `vocabulary_optimization` has no effect on the generated
  encodings. Any named-pass selection runs at level O2, and the vocabulary
  statistics shown by `--metrics` depend only on the effective level not being
  O0. `pattern_optimization` is reserved and rejected.
- `--disable-all-passes`, `--enable-all-passes`, `--enum-optimization`,
  `--bounded-int-optimization`, `--bit-packing`: explicit pass toggles. These
  toggles and `--enable-pass` apply to Rust output only; `--target python` uses
  the `-O` level as is.
- `--max-vocab N`: cap the structural vocabulary of enums, union
  discriminators, and bounded integers: any whose variant count or range
  exceeds `N` is encoded with a lossless multi-token codec in base `N` (not
  necessarily a power of two). `N` must be at least 257 so the 256 byte tokens
  plus the end-of-value marker stay representable. The cap is applied by the
  enum and bounded-integer passes only: at `-O0`, or with an `--enable-pass`
  list that omits `enum_optimization` or `integer_optimization`, the matching
  values are not capped (enums and union discriminators stay one-token
  indices), and nothing afterwards checks that `BASE_VOCAB_SIZE` is at most `N`.
- `--bpe-merges FILE`: embed a trained merge table.
- `--streaming`: also generate `decode_prefix()` (truncation-tolerant decoding).
  The token format is identical to non-streaming output.
- `--decimal-numbers`: encode unbounded integers and floats as digit-aligned
  decimal text. The same setting must be used for BPE training, measurement,
  and any wheel that consumes those merges.
- `--metrics`, `--benchmark`, `-q/--quiet`. Without `-o`, the
  generated Rust goes to stdout and informational lines go to stderr, so
  `tct schema.json > tokenizer.rs` writes a valid file.
- `-n/--name`: root type name. Without it, `--target rust` uses the schema
  `title`, then the file stem; `--target python` ignores `title` and derives
  the name from the package name (`--python-name`, default derived from the
  file stem with a `tct-` prefix). The root type name is the generated
  `SEQUENCE_SCHEMA_ID`, so the same schema gets a different schema id under the
  two targets unless `--name` is given.
- `--target python` with `--python-name`, `--python-version`, `--description`.

## Repository Layout

```
src/
  compiler/              TctCompiler builder API and schema preprocessing
  generator/             four-pass pipeline: schema walker, type analyzer,
                         optimization phase, code generator
  optimization_pass/     optimization profiles (O0/O2/O3/Os/OVocab) and pass names
  vocabulary_allocator/  dense structural token allocation
  vocabulary_analyzer/   per-position vocabulary requirement analysis
  vocabulary_encoder/    vocabulary-constrained multi-token encodings
  schema_analyzer/       schema feature and complexity analysis
  grammar.rs, grammar/   compiled Grammar and generated-code assembly
  tokenizer.rs, tokenizer/  JsonTokenEncoder trait and shared runtime support
  decoder/               decode helpers shared by generated code
  bpe/                   BPE trainer and encoder
  instance_encoder.rs, instance_encoder/  generated helper crates used to
                         encode corpora from the host process
  schema_evolution.rs, schema_evolution/  token-ID layout diffs (tct-token-diff)
  sequence_bundle.rs     BOS/EOS/schema-token wrappers, bundles, prompt boundary
  wheel_builder.rs, wheel_builder/  Python wheel generation
  generated_crate.rs, generated_crate/  single source for generated Cargo.toml
                         templates and child cargo configuration
  python.rs              optional root PyO3 bindings (feature `python`)
  provenance.rs          SHA-256 and file provenance helpers
  main.rs, bin/          command-line tools
tests/                   integration tests (one crate per file)
tests/data/instances/    real-world and synthetic JSON instances per schema
tests/fixtures/          fixtures for the experimental frontends
tests/benchmarks/        compilation benchmark baseline and regression script
tests/python/            maintenance tests for the helper scripts
schemas/                 schema corpus, BPE merge tables, target-schema manifests
scripts/                 testing, benchmarking, analysis, data, conversion helpers
examples/github-workflow/  end-to-end Python usage example with a type stub
docs/                    interface contracts (frontend contracts included)
corpora/                 small source corpora for the experimental frontends
reports/                 checked evidence fixtures read by frontend tests
.github/workflows/       CI definitions
```

## Core Concepts

### Schema-Local Token Streams

A generated tokenizer maps one JSON instance of one schema to a stream of `u32`
token IDs. Tokens are untyped: the generated code knows, at each position, what
the schema allows there, so the same numeric ID can mean a byte, an enum index,
or an array length depending on position. This is what lets TCT drop keys,
punctuation, and type tags that the schema already determines.

At string and value positions the base layer uses byte tokens `0..=255` plus
`END_STRING_MARKER` (256). Structural tokens (enum indices, union
discriminators, presence flags, lengths) are allocated densely from 0 in the
same untyped ID space, because their position gives them meaning. The base
layer is therefore as large as the largest of these needs. Optional BPE merges
are appended after the base layer, and `PAD_TOKEN` is the last raw ID.

### JsonTokenEncoder

Generated code carries its own copy of the encoder trait,
`tokenizer::JsonTokenEncoder`, which mirrors the library's
`tct::JsonTokenEncoder` (`src/tokenizer.rs`); every generated type implements
it:

- `encode(&self, tokens: &mut Vec<u32>)` appends the canonical token sequence.
- `decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError>`
  reconstructs one value.
- `decode_full`, `decode_strict`, `decode_strict_full`: see the decode contract.
- `token_count(&self) -> usize` returns the encoded length.

```rust
// Inside the generated crate; from outside, import
// `<generated_crate>::tokenizer::JsonTokenEncoder`.
use crate::tokenizer::JsonTokenEncoder;

// `MyStruct` is a type emitted by the generator for your schema.
let mut tokens = Vec::new();
value.encode(&mut tokens);
let decoded = MyStruct::decode_full(tokens.iter().copied())?;
assert_eq!(value, decoded);
```

### Decode Contract

TCT separates three decode behaviors. Keep them separate when adding generated
paths; `ARCHITECTURE.md` ("Token Encoding and Decode Contract") has the full
rules.

- **Canonical encode.** `encode()` emits one canonical sequence and never
  normalizes. Every emitted ID is below `VOCAB_SIZE`.
- **Recovery decode.** `decode()` decodes one value and is recovery-oriented for
  bounded finite choices: booleans, enum and union discriminators,
  optional-field states, and BPE IDs are normalized with modulo arithmetic so a
  model-generated stream still decodes. It still errors on EOF, invalid string
  byte tokens, invalid UTF-8, schema validation failures, and decoded collection
  lengths above the configured cap. `decode_full()` applies the same semantics
  and then rejects unconsumed trailing tokens.
- **Strict decode.** `decode_strict()` and `decode_strict_full()` reject
  out-of-range finite choices instead of normalizing them. They are additive;
  the recovery APIs keep their behavior for generation workflows.
- **Prefix decode.** `decode_prefix()` (generated only with streaming enabled,
  `--streaming` on the CLI or `TctCompiler::with_streaming_encode()`) accepts a
  left prefix of a canonical stream and returns a `PrefixDecodeResult` with the
  partially decoded value, field progress, and completeness. It is
  truncation-tolerant, may default missing fields, and is not a strict
  validator.

Prefix completion and gap infill are different contracts. `encode_partial()`
and `GapInfo` describe default-filled fields in a complete token sequence; they
are not a suffix-constrained infill state. Do not treat them as evidence for an
infill API. The requirements for a future infill API are in `ARCHITECTURE.md`
("Prefix And Gap Completion Contracts").

Focused tests for these contracts:

```bash
cargo test --test decode_contract_test
cargo test --test prefix_completion_contract_test
cargo test --test streaming_decode_integration_test
cargo test --test decode_state_equivalence_test
```

### Vocabulary Accounting

The generated token layout is a compile-time contract, not a runtime heuristic.
Generated code exports `BASE_VOCAB_SIZE`, `VOCAB_SIZE`, and `PAD_TOKEN` (plus BPE
range constants when merges are embedded). `BASE_VOCAB_SIZE` is the maximum of
the string-token capacity (257 for UTF-8, 129 for ASCII), the structural token
requirement, and the encoding base (at least 256). It is therefore at least 257
with the default UTF-8 string encoding (256 with ASCII), and `VOCAB_SIZE` is
`BASE_VOCAB_SIZE` plus the BPE merges plus one for `PAD_TOKEN`. Invariants:

- Every emitted token ID is strictly below `VOCAB_SIZE`.
- The base range `[0, BASE_VOCAB_SIZE)`, which byte, end-marker, and
  structural tokens share, the BPE merge range, and `PAD_TOKEN` never overlap.
  Payload tokens never equal `PAD_TOKEN`.
- `BASE_VOCAB_SIZE`, `PAD_TOKEN`, and `VOCAB_SIZE` agree with what the generated
  encoder actually emits.

Changes to enum allocation, bounded-integer codecs, BPE ranges, or
`MAX_BASE_VOCAB` must come with focused layout tests. The relevant suites:

```bash
cargo test --test vocabulary_constants_test
cargo test --test generated_token_count_contract_test
cargo test --test max_base_vocab_integration_test
cargo test --test vocabulary_optimization_test
cargo test --test robust_vocabulary_optimization_test
cargo test --test sequence_boundary_contract_test
```

`MAX_BASE_VOCAB` is the round-trip harness name for
`TctCompiler::with_max_base_vocab()` (values above 256). It sets the encoding
base: the largest power of two not exceeding the value, never below 256. The
generated fixed-width multi-token integer encodings (for example collection
lengths and bounded-integer offsets) use that base, and it is a lower bound on
`BASE_VOCAB_SIZE`. A value such as 512 therefore raises the base layer to at
least 512 and never caps the structural part, and every value from 257 to 511
gives the default base 256. The multi-token codecs for enums and bounded
integers are selected by `--max-vocab`, not by this setting, and use that cap
itself as their base. When changing configurable base-vocabulary behavior, run
the round trip at no fewer than two values that widen the base, for example:

```bash
env SCHEMA_NAME=package SCHEMA_FOLDER=popular MAX_BASE_VOCAB=512 cargo test --test round_trip_test test_round_trip
env SCHEMA_NAME=package SCHEMA_FOLDER=popular MAX_BASE_VOCAB=1024 cargo test --test round_trip_test test_round_trip
```

The round-trip harness checks the generated constants against the compiled
grammar for every schema it builds.

### Optimization Levels

- **O0**: development mode. No optimization passes run and presence flags are
  not bit-packed. Enum values and union discriminators are still encoded as
  one-token indices, as at every level.
- **O2**: balanced. Enum and bounded-integer optimization. The enum pass does
  not introduce the one-token index encoding; it adds zero-token constants for
  single-variant enums and, only when `--max-vocab` is smaller than the variant
  count, a multi-token codec.
- **O3** (default): O2 plus format-aware string encodings and bit-packed
  optional-field presence.
- **Os** and **OVocab**: currently the same profile under two names. Both run
  the passes O3 runs and bit-pack optional presence, but not the O3-only
  format-aware string encodings, so they produce identical encodings. The
  separate names keep distinct levels for future size-specific and
  vocabulary-specific decisions.

Every level must round-trip. `test_named_optimization_profiles_generated_round_trip`
in `tests/round_trip_test.rs` builds a small schema at Os, O3, and OVocab, and
`ROUND_TRIP_SCENARIOS=all` covers every level on real schemas.

Direct finite-regex encoding is not implemented. `patternProperties` validation
is schema-driven and does not depend on the reserved `pattern_optimization`
pass name, which the CLI rejects.

### additionalProperties

- `additionalProperties: false` generates a closed struct. Undeclared keys fail
  deserialization (reported as `unknown_field` by `tct-prepare-data`).
- `additionalProperties: true` flattens extra keys into a map.
- `additionalProperties: {schema}` validates extra values against the subschema.

Validation-only `allOf` constraints are enforced by generated JSON Schema
validators that are built once on first use. Instances are validated before
encoding, and generated decode revalidates decoded values before returning.
Focused tests: `tests/additional_properties_test.rs`,
`tests/smart_additional_properties_test.rs`, `tests/allof_schema_test.rs`.

## Architecture

The compiler runs four passes with strict responsibilities:

1. **Discovery** (`SchemaWalker`): walks the schema and records structure. No
   code generation.
2. **Analysis** (`TypeAnalyzer`): dependency graph, cycles, and trait
   derivability (`src/generator/type_analyzer/trait_derivability.rs`; keep it
   conservative for cycles, missing references, maps, floats, and
   `serde_json::Value`).
3. **Optimization** (`OptimizationPhase`): produces an `EncodingStrategy`, pure
   encoding decisions and generation metadata.
4. **Generation** (`CodeGenerator`): renders Rust from the outputs of the
   earlier passes.

Do not mix generation into discovery or analysis. New optimizer behavior flows
through structured metadata and `EncodingStrategy`, not through passes that
rewrite rendered token streams. `tests/architecture_boundary_test.rs` enforces
part of this boundary. `ARCHITECTURE.md` documents the data structures, the
structural codec design, the decode contract, and the sequence wrappers.

Generated hot paths should stay easy for LLVM to optimize: decisions resolved at
compile time, constants propagated, predictable control flow, no runtime schema
introspection, and no avoidable allocation. A performance regression in
tokenization or generated code needs a correctness or measurable algorithmic
justification and a note on how the impact was measured (see "Benchmarks").

## Testing And Validation Layers

Choose the smallest layer that covers the behavior you changed, and escalate
when a change crosses compiler, generated-code, artifact, or packaging
boundaries.

| Layer | When | Commands |
|---|---|---|
| Focused | While iterating; before and after every fix | the single unit test, integration test, synthetic fixture, or schema-specific round trip that exposes the behavior |
| Stable | Default pre-commit gate for code changes | `cargo fmt --all -- --check`, `cargo clippy --all-targets`, `cargo test`, `./scripts/testing/quality-check.sh` |
| Extended | Changes to generated crates, BPE artifacts, wheels, target-schema manifests, or scripts | round-trip variants, target-schema scripts, reproducibility, wheel smoke, focused coverage |
| Full | Broad contract changes and release candidates | full round-trip matrix, depth gate, ignored synthetic tests, full coverage modes |

### Focused Iteration And Test-Driven Fixes

For correctness bugs, schema semantics, token accounting, and generated-code
behavior:

1. Identify the narrowest failing behavior.
2. Add or update a unit test, synthetic schema fixture, generated-crate contract
   test, or target-schema regression that demonstrates it.
3. Run it and confirm it fails for the expected reason.
4. Make the smallest change that fixes it, in the responsible module
   (`src/generator/` for most schema features).
5. Re-run the focused test, the relevant round-trip or generated-crate gate, and
   the stable checks.

When a real schema fails in a complex way, isolate the behavior first with a
minimal schema under `schemas/synthetic/` and instances under
`tests/data/instances/<name>/`, add a focused ignored test to
`tests/synthetic_schema_test.rs`, fix the root cause there, then return to the
real schema.

### Stable Gate

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
./scripts/testing/quality-check.sh
```

`cargo test` runs every non-ignored test, including the default round-trip
smoke (`round_trip_test::test_round_trip`), which builds small generated crates.
The first run compiles those crates' dependencies into
`target/tct-generated-cargo`; later runs reuse that cache.

### Extended Gate

Run only the commands relevant to the change:

```bash
cargo test --test round_trip_test test_round_trip            # default smoke
./scripts/testing/test-round-trip-bpe-smoke.sh               # BPE and no-BPE
./scripts/testing/test-target-schemas.sh                     # target schema set
./scripts/testing/test-generated-reproducibility.sh          # Cargo templates, toolchain
./scripts/testing/test-wheel-builds.sh                       # wheel generation, PyO3
python3 scripts/testing/test-bpe-artifact-reproducibility.py --public-artifacts # all shipped tables
cargo audit --deny warnings                                  # advisories in Cargo.lock
env TCT_COVERAGE_FOCUS=src/generator ./scripts/testing/coverage-check.sh quick
```

Typical pairings: wheel or PyO3 changes need the wheel smoke; Cargo template,
dependency pin, or toolchain changes need the reproducibility gate; BPE runtime
or artifact changes need the BPE smoke and possibly artifact reproducibility;
compiler or tokenizer refactors should pair semantic tests with a focused
coverage report. Dependency changes should also pass `cargo audit`
(`cargo install cargo-audit --locked` installs it). PyO3 is pinned twice, in
`Cargo.toml` and in `PYO3_VERSION` in `src/generated_crate.rs` for generated
wheels; move both together.

### Full Matrix

```bash
env ROUND_TRIP_FULL=1 ROUND_TRIP_SCENARIOS=all ROUND_TRIP_BUILD_PROFILE=release \
  cargo test --test round_trip_test test_round_trip
./scripts/testing/test-target-schema-depth.sh
cargo test --test synthetic_schema_test -- --ignored --nocapture
./scripts/testing/coverage-check.sh stable
./scripts/testing/coverage-check.sh synthetic-ignored
./scripts/testing/coverage-check.sh round-trip-full
```

These are slow. Use them for broad generated-code contract changes and release
candidates, not routine commits.

### Test Organization

Integration tests are one crate per file under `tests/`. Main groups:

| Area | Files |
|---|---|
| Round trips | `round_trip_test.rs`, `round_trip_helpers.rs`, `round_trip_fixture_loader_test.rs`, `synthetic_schema_test.rs`, `simple_correctness_test.rs`, `correctness_validation_test.rs` |
| Decode contracts | `decode_contract_test.rs`, `prefix_completion_contract_test.rs`, `streaming_decode_integration_test.rs`, `enum_streaming_decode_test.rs`, `decode_state_equivalence_test.rs`, `terminal_key_cursor_test.rs` |
| Vocabulary and layout | `vocabulary_constants_test.rs`, `generated_token_count_contract_test.rs`, `max_base_vocab_integration_test.rs`, `vocabulary_optimization_test.rs`, `robust_vocabulary_optimization_test.rs`, `token_count_test.rs`, `sequence_boundary_contract_test.rs` |
| Schema features | `additional_properties_test.rs`, `allof_schema_test.rs`, `anyof_union_test.rs`, `const_schema_test.rs`, `not_schema_test.rs`, `recursive_schema_test.rs`, `null_type_support.rs`, `default_values_test.rs`, `regex_conversion_test.rs`, `utf8_string_encoding_test.rs`, `bounded_integer_test.rs`, `unbounded_integer_test.rs` |
| Schema corpus | `schema_compilation_test.rs`, `schema_compatibility_test.rs`, `schema_compatibility_report.rs`, `schema_draft_compatibility.rs`, `real_world_schemas.rs` |
| Robustness | `schema_robustness_test.rs`, `input_validation_test.rs`, `schema_warnings_test.rs`, `property_based_correctness.rs`, `stress_test.rs`, `performance_stack_safety.rs`, `performance_memory_limits.rs` |
| BPE | `bpe_integration_test.rs`, `bpe_trainer_reference_test.rs` |
| Wheels and bundles (mostly ignored) | `streaming_wheel_build_test.rs`, `bundle_wrapper_build_test.rs`, `streaming_wheel_test.rs`, `rebuild_github_workflow_wheel_test.rs`, `terminal_key_cursor_wheel_test.rs`, `build_kubernetes_wheel_test.rs` |
| CLI and API | `cli_test.rs`, `cli_vocabulary_analysis_test.rs`, `tct_compiler_api_test.rs`, `readme_api_examples_test.rs`, `architecture_boundary_test.rs` |
| Performance (report-only) | `benchmark.rs`, `optimization_level_benchmarks.rs`, `performance_analysis_benchmarks.rs`, `performance_reality_check.rs`, `profiling_*` |

Slow and report-only tests are marked `#[ignore]` and run explicitly with
`-- --ignored`.

### Python Maintenance Tests

`tests/python/` covers the helper scripts (quality gate behavior, coverage and
policy reports, BPE manifests, wheel tool envelope, benchmark script scope,
workflow definitions). They need only the Python 3 standard library under
`unittest`; pytest also works. Without pytest installed, `uv` runs it in a
throwaway environment from `tests/python/requirements.txt`.

```bash
python3 -m unittest discover tests/python
python3 -m pytest tests/python -q
python3 -m pytest tests/python/test_quality_check_script.py -q
uv run --no-project --with-requirements tests/python/requirements.txt \
  python -m pytest tests/python -q
```

`quality-check.sh` runs the suite with `unittest discover` and sets
`TCT_PYTHON_SUITE_ACTIVE=1` for it. Some tests spawn `quality-check.sh`, so the
helper refuses to launch the suite again when that variable is already set.
This prevents unbounded recursion; run the helper from a top-level shell.

## Round-Trip Testing

A round trip checks the core contract end to end: compile the schema, generate
and build a Rust crate, then for each instance run `JSON -> tokens -> JSON` and
compare semantically. The harness in `tests/round_trip_test.rs` compiles each
schema once and pushes all of its instances through one generated binary in
batch mode.

```bash
# Default smoke gate (part of ordinary cargo test)
cargo test --test round_trip_test test_round_trip

# One schema
env SCHEMA_NAME=tsconfig SCHEMA_FOLDER=popular cargo test --test round_trip_test test_round_trip

# Full default matrix: all instances, all optimization scenarios
env ROUND_TRIP_FULL=1 cargo test --test round_trip_test test_round_trip
```

### Default Smoke Shape

Without overrides, the smoke gate covers `kubernetes`, `eslintrc`, and
`tsconfig` from `schemas/popular/`, at O3, with debug generated builds and an
explicit fixture manifest in `tests/round_trip_test.rs` (so results do not
depend on directory listing order). Schemas with a focused smoke manifest use
those fixtures when selected with `SCHEMA_NAME`; others fall back to the first
three discovered instances. `test_default_smoke_fixture_manifest_is_explicit_and_existing`
and `test_focused_smoke_fixture_manifests_are_explicit_and_existing` keep the
manifests valid.

### Environment Variables

| Variable | Effect |
|---|---|
| `SCHEMA_NAME` | Run one schema instead of the default set. |
| `SCHEMA_FOLDER` | Folder under `schemas/` for `SCHEMA_NAME` (default `popular`). Ignored without `SCHEMA_NAME`. |
| `ROUND_TRIP_FULL=1` | All discovered instances and all scenarios. |
| `ROUND_TRIP_SCENARIOS` | Comma list of `o0`, `o2`, `o3`, `os`, `ovocab`, or `all`. |
| `ROUND_TRIP_BUILD_PROFILE` | `debug` (default) or `release` for generated crates. |
| `MAX_INSTANCES` | Use the first N discovered instances instead of the smoke manifest. |
| `MAX_BASE_VOCAB` | Value for `TctCompiler::with_max_base_vocab()` (must exceed 256). The encoding base is the largest power of two not exceeding it (minimum 256) and lower-bounds `BASE_VOCAB_SIZE`, so 257-511 reproduce the default layout. |
| `BPE_MERGES` | Path to a merge table to embed. |
| `ROUND_TRIP_BPE_PRESET` | `legacy-tct-500`: use `schemas/bpe/<schema>-tct-bpe-500.json`. Mutually exclusive with `BPE_MERGES`. |
| `ROUND_TRIP_DEBUG` | Print per-instance debug output from the generated binary. |
| `TCT_GENERATED_CARGO_TARGET_DIR` | Shared Cargo target directory for generated crates. |

### BPE Round Trips

```bash
# Default schemas with the legacy 500-vocabulary merge tables
env ROUND_TRIP_BPE_PRESET=legacy-tct-500 cargo test --test round_trip_test test_round_trip

# One schema with an explicit merge table
env SCHEMA_NAME=tsconfig SCHEMA_FOLDER=popular \
  BPE_MERGES=schemas/bpe/tsconfig-tct-bpe-1k.json \
  cargo test --test round_trip_test test_round_trip

# Maintained BPE/no-BPE bundle: default set plus github-workflow and package
./scripts/testing/test-round-trip-bpe-smoke.sh
```

The `legacy-tct-500` preset name marks regression coverage for existing
checked-in merge tables; it is not a recommendation for new training
vocabulary sizes. `test-round-trip-bpe-smoke.sh` accepts
`ROUND_TRIP_BPE_SMOKE_TIMEOUT_SECONDS` (default 600) and
`ROUND_TRIP_BPE_SMOKE_PRESET`.

### Round-Trip Report

Every run writes `target/tct-round-trip-report.json` with the mode (smoke or
full), fixture selection, scenarios, BPE inputs, `MAX_BASE_VOCAB`, generated-code
hashes, and per-schema results. The generated binary validates each source
instance against the schema before encoding and validates the decoded JSON
after decoding, so a failure names the schema, fixture, phase, and JSON context
instead of surfacing as an opaque parse or equality error. Vendored external
schemas under `schemas/external/` are registered for `$ref` resolution when
available.

To turn a report into a per-schema validity manifest (schema hashes, features
present, generated-code hashes, scenario summaries, reproduction commands,
consumed-token and PAD/BOS/EOS policy, BPE fingerprints, and explicit gaps):

```bash
python3 scripts/analysis/round_trip_validity_manifest.py \
  --round-trip-report target/tct-round-trip-report.json \
  --output target/tct-round-trip-validity-manifest.json
```

### Synthetic Generated-Crate Tests

`tests/synthetic_schema_test.rs` holds slow, ignored round trips over minimal
schemas in `schemas/synthetic/` with instances under `tests/data/instances/`:

```bash
cargo test --test synthetic_schema_test -- --ignored
cargo test --test synthetic_schema_test test_simple_duplicate_round_trip -- --ignored --exact
```

### Schema Compilation Coverage

The compilation suites check that real-world schemas compile, without building
generated crates:

```bash
cargo test --test schema_compilation_test test_basic_real_world_schemas
cargo test --test schema_compilation_test test_configuration_schemas
cargo test --test schema_compilation_test test_api_schemas
```

Detailed per-schema compatibility reports are ignored by default:

```bash
env SCHEMA_NAME=commitlintrc cargo test --test schema_compatibility_report test_individual -- --ignored --nocapture
cargo test --test schema_compatibility_report test_generate_comprehensive_compatibility_report -- --ignored --nocapture
```

## Target Schema Gates

The maintained regression set is `schemas/target-schemas.txt`
(`schema-name:schema-folder` per line). Run it with:

```bash
./scripts/testing/test-target-schemas.sh
```

The helper pins a fast shape for each schema: `ROUND_TRIP_FULL=0`,
`ROUND_TRIP_SCENARIOS=o3`, debug generated builds, and no inherited
`BPE_MERGES`, `ROUND_TRIP_BPE_PRESET`, `MAX_INSTANCES`, or `MAX_BASE_VOCAB`.
It reuses generated dependencies through
`target/tct-target-schema-generated-cargo`. Failures are logged to
`target/tct-target-schema-<schema>.log`, and the script prints the exact
single-schema command to reproduce. Overrides: `TARGET_SCHEMA_MANIFEST`,
`TARGET_SCHEMA_TIMEOUT_SECONDS` (default 240), and
`TCT_GENERATED_CARGO_TARGET_DIR`.

Selected release-depth coverage lives in `schemas/target-schema-depth.txt`
(`schema-name:schema-folder[:bpe-preset]`). The only supported preset is
`legacy-tct-500`, which also requires the merge table's `.provenance.json`
sidecar. Run it with:

```bash
./scripts/testing/test-target-schema-depth.sh
env TARGET_SCHEMA_DEPTH_DRY_RUN=1 ./scripts/testing/test-target-schema-depth.sh
```

Each entry runs with `ROUND_TRIP_FULL=1`, `ROUND_TRIP_SCENARIOS=all`, and
release generated builds, once without BPE and, when a preset is listed, once
with it. The helper clears inherited `MAX_INSTANCES`, `MAX_BASE_VOCAB`, and BPE
variables so the manifest controls the shape. It writes
`target/tct-target-schema-depth-report.json`, copies each run's
`target/tct-round-trip-report.json` beside it with schema and mode in the file
name, and logs failures to `target/tct-target-schema-depth-<schema>-<mode>.log`.
Overrides: `TARGET_SCHEMA_DEPTH_MANIFEST`, `TARGET_SCHEMA_DEPTH_TIMEOUT_SECONDS`
(default 900), `TARGET_SCHEMA_DEPTH_DRY_RUN`, `TARGET_SCHEMA_DEPTH_REPORT`, and
`TCT_GENERATED_CARGO_TARGET_DIR`.

Update the manifests rather than duplicating schema lists in scripts or docs.

## Generated Crates And Reproducibility

Many tests and tools build temporary Cargo projects from generated source: the
round-trip harness, `InstanceEncoder` helper crates used by `tct-prepare-data`,
`tct-train-bpe`, and `tct-measure-tokens`, and Python wheel packages.

### Single Source Policy

`src/generated_crate.rs` is the only place that defines generated Cargo
manifests and child `cargo` configuration:

- `GeneratedCargoToml` renders every generated `Cargo.toml` with pinned
  dependency versions, `GENERATED_EDITION`, and `GENERATED_RUST_VERSION`. Do
  not write ad hoc dependency blocks in tests or helpers.
- `configure_generated_cargo_command` (in `src/generated_crate/env.rs`)
  configures child `cargo` and maturin processes: it appends the generated lint
  cap (`GENERATED_CARGO_RUSTFLAGS`) to inherited `RUSTFLAGS` or
  `CARGO_ENCODED_RUSTFLAGS` (so `cargo llvm-cov` instrumentation survives),
  passes `RUSTUP_TOOLCHAIN` through or falls back to the channel in
  `rust-toolchain.toml`, and selects the generated target directory.
- `copy_workspace_cargo_lock` copies the workspace `Cargo.lock` into generated
  projects so path-dependency builds reuse pinned resolution, including in
  restricted-network environments.

The generated target directory defaults to `target/tct-generated-cargo`. An
inherited `CARGO_TARGET_DIR` is preserved; set `TCT_GENERATED_CARGO_TARGET_DIR`
only when you need a specific shared cache. The helper scripts each choose
their own cache directory so concurrent gates do not share build state.

When changing dependency pins or command policy, run the drift check:

```bash
cargo test generated_crate::tests
```

### Helper Crates And Failure Context

`InstanceEncoder` helper crates get unique package and binary names per
generated project, so helpers for different schemas never overwrite each other
in a shared target directory. Use `InstanceEncoder::encoder_binary()` rather than
assuming a fixed binary path.

Helper crates depend on the `tct` crate by path and reuse its `Cargo.lock` and
`rust-toolchain.toml`, so the tools that encode instances through one
(`tct-prepare-data`, `tct-train-bpe`, `tct-measure-tokens`, `tct-analyze-usage`,
`tct-validate-server`) need to know where the source tree is.
`generated_crate::tct_source_dir()` takes the first of: `TCT_SOURCE_DIR`; the
current directory when its `Cargo.toml` declares package `tct`; the directory the
binary was built from. The tools therefore work from any working directory on the
machine that built them. Set `TCT_SOURCE_DIR` when the source tree moved after the
build or when the binary was copied to another machine; with none of the three
available they fail with an error that names the variable.

If a generated helper fails to compile, the temporary project is preserved and
the error message includes its path ("Generated helper project preserved for
inspection at ..."), so you can inspect the generated `Cargo.toml`, source, and
Cargo output. Do not add cleanup that deletes failed build context. Successful
runs of `tct-measure-tokens` and the benchmark wrapper may pass
`--cleanup-generated-helper` to remove the helper crate after measurement.

### Reproducibility Gate

```bash
./scripts/testing/test-generated-reproducibility.sh
```

This runs the template drift check, builds one generated binary crate
(`test_simple_duplicate_round_trip`), and builds one generated wheel through
`test-wheel-builds.sh`. It needs the same Python and maturin environment as the
wheel smoke; set `TCT_GENERATED_REPRODUCIBILITY_SKIP_WHEEL=1` to skip the wheel
step when that tooling is unavailable. Run it when you change generated Cargo
templates, dependency pins, the pinned toolchain, or wheel package metadata.

### Deterministic Generated Source

The compiler emits byte-identical source for the same schema, options, and compiler
build. Types are listed in dependency order with ties broken by name, and the fields
of a compound discriminator key are joined in alphabetical order (`apiVersion|kind`).
`tests/generation_determinism_test.rs` generates real schemas repeatedly in one
process and compares the source, and two unit tests pin the generation order and the
discriminator detection. When you add code that iterates a `HashMap` or `HashSet`
while generating, sort first or use a `BTreeMap`/`BTreeSet`: the `std` hash
collections draw a random seed per instance, so an unsorted iteration reorders the
generated file from one run to the next.

## Python Wheels

`tct --target python` generates a schema-specific PyO3 package and builds it
with maturin. Wheel builds need external tooling (Python 3, `pip`, `maturin`,
`setuptools`, `wheel`), so they are not part of ordinary `cargo test`.

```bash
python3 -m pip install maturin setuptools wheel
cargo run --release --bin tct -- schemas/popular/github-workflow.json \
  --target python \
  --python-name tct-github-workflow \
  --optimization O3 \
  --streaming \
  -o wheels
```

The wheel honors the same `-O` profiles, `--max-vocab`, `--bpe-merges`, and
`--decimal-numbers` settings as Rust generation. Set `TCT_WHEEL_PYTHON` to pass
an explicit interpreter to maturin (`maturin build -i`) when the default
interpreter would produce a wheel with the wrong tag.

### Wheel API

Every generated wheel exposes raw schema-local functions:

- `encode(json_str) -> list[int]` and `decode(tokens) -> (json_str,
  tokens_consumed, total_tokens)`. For BPE wheels the counts are in expanded
  raw-token space, not the compressed input length.
- `vocab_size()` and `pad_token()`.
- `token_roles(tokens)`: one role per expanded raw token (0 structure, 1 value
  byte, 2 end-of-value marker, 3 discriminator).
- `strict_prefix_roles(tokens) -> (roles, complete)`: strictly replays a complete
  or incomplete raw prefix without recovery normalization; recovery-only,
  surplus, and malformed streams raise `RuntimeError`.

Raw APIs are content-only: they never emit BOS, EOS, schema, or padding tokens.
Model-facing single-schema sequence helpers add those:

- `encode_sequence(json_str)` returns
  `[BOS, SCHEMA, payload + offset ..., EOS]`, where the payload is exactly what
  `encode()` returns (BPE included) shifted by `sequence_payload_offset()`.
- `decode_sequence(tokens)` validates the wrapper (schema token, EOS, no padding
  before EOS, only padding after EOS, payload inside the offset range), then
  runs the raw decoder. Trailing sequence padding is accepted but not counted
  as consumed: `tokens_consumed` is the payload length (wheel-space) plus three
  (BOS, schema token, EOS), and `total_tokens` is the whole input length,
  padding included.
- `bos_token()`, `eos_token()`, `schema_token()`, `sequence_pad_token()`,
  `sequence_payload_offset()`, `sequence_boundary_version()`,
  `sequence_vocab_size()`.
- `sequence_metadata_json()`: artifact kind and version, boundary version,
  schema id and token, raw `BASE_VOCAB_SIZE`/`VOCAB_SIZE`/`PAD_TOKEN`, base and
  BPE ranges, payload range and offset, sequence vocabulary size, special-token
  reservations, whether `decode_prefix` exists, and BPE metadata.

Wheels built with `--streaming` also expose:

- `decode_prefix(tokens) -> (json_str, fields_decoded, complete)`. The Rust
  `PrefixDecodeResult` also tracks the partial field; the Python wrapper does
  not expose it.
- `begin_decode_state()` and `begin_decode_state_restricted(root_tokens)`,
  which return a cloneable `DecodeState` for host-side candidate probing.

```python
from tct_github_workflow import (
    begin_decode_state, decode, decode_prefix, decode_sequence,
    encode, encode_sequence, sequence_metadata_json, strict_prefix_roles,
)

tokens = encode(workflow_json)
decoded_json, consumed, total = decode(tokens)

sequence = encode_sequence(workflow_json)
decoded_json, consumed, total = decode_sequence(sequence)
roles, complete = strict_prefix_roles(tokens)

partial_json, fields_decoded, complete = decode_prefix(tokens[:8])

state = begin_decode_state()
for token in tokens:
    status = state.try_append(token)   # "prefix_ok", "complete", or "dead"
json_out, fields_decoded, complete = state.render_prefix()
```

`DecodeState` contract:

- Status strings are `"complete"`, `"prefix_ok"`, and `"dead"`. Empty prefixes
  are `"prefix_ok"`.
- Classification replays the stored prefix through generated decode (BPE wheels
  expand first). It is replay-based; constant-time transitions are not claimed.
- `begin_decode_state_restricted(ids)` marks a prefix dead when its first
  wheel-space token (the same space `encode()` returns) is outside `ids`.
- `clone_state()` deep-copies tokens and the restriction for speculative
  appends; `status()`, `tokens()`, `len()`, and `allowed_root_tokens()` inspect
  the state.
- `current_terminal_key()` and `current_terminal_path()` return the active
  object-property key and dotted path when the prefix sits at a string or
  decimal-number value (the positions marked with value roles), and `None`
  otherwise, including for complete and dead prefixes. They never guess.

`encode`, `decode`, `encode_sequence`, `decode_sequence`, `token_roles`,
`strict_prefix_roles`, and `decode_prefix` on BPE wheels release the GIL
(`py.detach`) around Rust parsing, validation, encode/decode, and JSON
serialization; those that take a JSON string copy it first. `decode_prefix` on
wheels without BPE and the `DecodeState` methods run while holding the GIL.
Keep the detach pattern for new bindings.

Wheels do not ship type stubs. `examples/github-workflow/` shows the calling
shape and a hand-written stub (`examples/github-workflow/tct_github_workflow.pyi`)
that can be copied next to the installed module.

### Wheel Smoke

```bash
./scripts/testing/test-wheel-builds.sh
```

The smoke first records `target/tct-wheel-tool-envelope.json` (Python, pip,
maturin, setuptools, wheel versions and generated pyproject build-input
markers; override the path with `TCT_WHEEL_TOOL_ENVELOPE_OUTPUT`) and fails
early if a required tool is missing, leaving the envelope for diagnosis. It then
runs two ignored tests:

- `test_streaming_wheel_build` (`tests/streaming_wheel_build_test.rs`): builds a
  Kubernetes streaming wheel with the 500-vocabulary merge table, installs it in
  a temporary virtual environment, and checks raw and sequence encode/decode,
  token bounds against both vocabularies, metadata, and `decode_prefix`.
- `test_bundle_wrapper_wheel_build` (`tests/bundle_wrapper_build_test.rs`):
  builds and imports a fixture bundle wrapper with
  `pip wheel --no-deps --no-build-isolation`.

Run it when you change wheel generation, PyO3 bindings, streaming APIs, bundle
wrappers, or generated package metadata. Focused library tests for wheel code
generation run without Python:

```bash
cargo test --lib generate_combined_lib
cargo test --test decode_state_equivalence_test
cargo test --features python --lib python
```

### The Root `tct` Python Module

The root crate also builds the Python module `tct` (`pyproject.toml`, feature
`python-extension`). It exposes the compiler (`tct.compile`, `tct.analyze`,
`tct.CompileError`, in `src/python/compile_api.rs`) and the BPE helper classes
(`src/python.rs`). `tct.compile` returns the same source as
`tct schema.json -o out.rs` for the matching options. The Rust-side tests above
cover argument handling and the compilation helper; the Python tests need the
extension installed in a virtualenv:

```bash
maturin develop
python3 -m pytest tests/python/test_compiler_bindings.py -q
```

Without the extension those tests are skipped.

### Multi-Schema Bundles

`tct::sequence_bundle` defines the manifest for combining several generated
tokenizers into one model vocabulary:

- `SequenceBundleManifest` records the manifest version, sequence boundary
  version, bundle vocabulary size, shared PAD/BOS/EOS policy, masking policy,
  and one `SequenceBundleSchema` per tokenizer (schema id, schema token,
  optional sentinel text and artifact path, raw `VOCAB_SIZE`/`PAD_TOKEN`, raw
  and global payload ranges, BPE flag).
- Global payload ranges may be disjoint per schema or share one block; the
  schema token disambiguates shared IDs.
- `validate()` rejects unsupported versions, special-token collisions,
  duplicate schema ids or tokens, raw PAD as payload, payload ranges outside the
  bundle vocabulary, and unsupported masking policies.
- `wrap_payload_for_schema_id()` / `wrap_payload_for_schema_token()` produce
  `[BOS, schema, global payload..., EOS]`; `unwrap_payload()` validates BOS,
  schema token, EOS, trailing PAD-only padding, and payload ranges, then returns
  local IDs.
- `generation_mask_for_schema_token()` returns the selected schema's payload
  range plus EOS. Prompt, BOS, schema, and PAD positions are label-masked; EOS is
  a training target.

`tct-build-bundle-wrapper` writes the source of a pure-Python package that
imports already built schema wheels and applies this contract. It does not
build or publish the wheel; run Python packaging from the printed directory.

```bash
cargo run --release --bin tct-build-bundle-wrapper -- --spec bundle-spec.json --output-dir wheels
```

A spec names the package and lists the schemas:

```json
{
  "package_name": "tct-config-bundle",
  "version": "0.1.0",
  "schemas": [
    {"schema_id": "tsconfig", "module_name": "tct_tsconfig", "schema_token": 3,
     "raw_vocab_size": 1024, "raw_pad_token": 1023, "global_payload_start": 8,
     "bpe_enabled": true}
  ]
}
```

Optional fields: `description`, `output_dir`, `vocab_size`, and per-schema
`schema_sentinel` and `generated_artifact`. The Rust entry point is
`tct::wheel_builder::write_bundle_wrapper_package()`; the generated package
exposes `encode_sequence`, `decode_sequence`, `sequence_vocab_size`, and
`sequence_bundle_metadata_json`.

### Prompt And Response Boundary

`PromptResponseBoundaryManifest` (also in `tct::sequence_bundle`) describes a
pretrained prompt tokenizer followed by a TCT response span:

- Prompt tokens keep their pretrained IDs. Bundle IDs map to
  `hf_added_token_block.start + bundle_token_id`; the added block starts at or
  after the prompt tokenizer's first free ID and has exactly the bundle
  vocabulary length.
- Numeric mode-entry and mode-exit sentinels must lie outside the prompt ID
  space and the TCT block, and differ from each other.
- `wrap_response_payload_for_schema_id()` / `unwrap_response_tokens()` apply the
  bundle contract in downstream IDs; `wrap_handoff_response_for_schema_id()` /
  `unwrap_handoff_response_tokens()` add and validate the mode sentinels.
- `hf_generation_mask_for_schema_token()` returns the selected payload range
  plus EOS in downstream IDs.

TCT's guarantees cover the structured response span after mode entry. The
Hugging Face sidecar contract is in
`docs/huggingface-compatible-tokenizer-contract.md`. Focused tests:
`cargo test --lib sequence_bundle` and
`cargo test --test sequence_boundary_contract_test`.

## BPE Training And Measurement

TCT can compress its token streams with byte pair encoding (BPE) trained on TCT
token sequences, not on JSON text. Merge IDs are appended after the base layer.
Legacy merge files with a nonzero `dict_size` field are rejected; retrain them on
plain TCT token sequences.

The library trainer and `tct-train-bpe` share one implementation:
`tct::bpe::trainer::train_bpe_incremental` for the default entry point and
`train_bpe_incremental_with_options` for target-average stopping and progress
events. Training sorts deduplicated sequences before building state and breaks
equal-frequency ties lexicographically, so results do not depend on hash order
or thread count. Sampled training (`--sample-size`) records its seed and sample
in provenance but is not by itself an exact corpus reproduction; prefer a fixed
encoded corpus (`--encoded`) or a token cache (`--cache`) when an exact merge
table must be reproducible.

### Workflow

```
instances --tct-prepare-data--> all.jsonl / train.jsonl / validate.jsonl
all.jsonl --tct-train-bpe-----> merges.json + merges.provenance.json
merges.json --tct-measure-tokens / tct-analyze-usage--> JSON reports
```

```bash
# 1. Encode once and split
cargo run --release --bin tct-prepare-data -- \
  --schema schemas/popular/github-workflow.json \
  --sources "tests/data/instances/github-workflow/real/*.json" \
  --output target/tct-examples/github-workflow-encoded \
  --train-ratio 0.9 \
  --seed 42

# 2. Train on the encoded corpus
cargo run --release --bin tct-train-bpe -- \
  --schema schemas/popular/github-workflow.json \
  --encoded target/tct-examples/github-workflow-encoded/all.jsonl \
  --target-vocab 2048 \
  --output target/tct-examples/github-workflow-bpe2048.json

# 3. Measure
cargo run --release --bin tct-measure-tokens -- \
  --schema schemas/popular/github-workflow.json \
  --instances "tests/data/instances/github-workflow/real/*.json" \
  --bpe-merges target/tct-examples/github-workflow-bpe2048.json
```

Keep generated datasets and merge tables with their sidecars (`metadata.json`,
`sources.jsonl`, `stats.json`, `*.provenance.json`) when moving them, and keep
the JSON reports from `tct-measure-tokens` and `tct-analyze-usage` when you use
them as evidence.

### tct-prepare-data

Encodes a corpus once and writes splits for both BPE training and model
training.

Options:

- `--mode`: `tct` (schema-aware, default) or `utf8` (raw bytes).
- `--schema`: schema file (required in `tct` mode).
- `--sources`: glob pattern; repeat for several.
- `--encoded`: an existing token JSONL to reshuffle or BPE-compress instead of
  encoding sources.
- `--output`: output directory.
- `--train-ratio` (default 0.9), `--validate-ratio` (default 0.1), `--seed`
  (default 42), `--shuffle` (default keeps source order).
- `-O, --optimization`, `--max-vocab`, `--decimal-numbers`: compiler settings;
  they must match what later training and measurement use. `tct-measure-tokens`
  accepts all three. `tct-train-bpe` accepts `-O` and `--decimal-numbers` but
  has no `--max-vocab`, and `tct-analyze-usage` accepts none of them.
- `--bpe-merges`: write BPE-compressed sequences.
- `--export-roles`: also write role files aligned 1:1 with the base token
  sequences for structure-only BPE (requires `tct` mode from sources, without
  `--bpe-merges`). Role bytes: 0 structure, 1 value, 2 value end,
  3 discriminator.
- `--quiet`.

Output:

```
output/
  metadata.json            command, schema, sources, split info, fingerprints
  train.jsonl              training sequences
  validate.jsonl           validation sequences
  all.jsonl                all sequences (use for BPE training)
  train.roles.jsonl        role files (with --export-roles)
  validate.roles.jsonl
  all.roles.jsonl
  sources.jsonl            per-instance source path, split, token count, SHA-256
  stats.json               token counts, lengths, vocabulary usage
  failed.txt               instances that failed to encode (when any)
  unencodable-report.json  classified failures (when any)
```

Each JSONL line is a JSON array of `u32` token IDs for one instance.

#### Schema-valid does not mean encodable

An instance that passes a JSON Schema validator can still fail to encode. With
`additionalProperties: false`, TCT objects are closed: undeclared keys fail
deserialization. `unencodable-report.json` (`schema_version`
`tct-unencodable-report-v1`) counts failures by class (`unknown_field`,
`missing_field`, `type_mismatch`, `unknown_variant`, `io_or_parse`,
`empty_tokens`, `other`), with capped samples and per-path details. TCT does not
silently strip unknown keys, because that would drop data. The programmatic API
is `InstanceEncoder::encode_batch_efficient_with_failures` together with
`tct::instance_encoder::{EncodeFailure, UnencodableReport, classify_encode_failure_message}`.

### tct-train-bpe

```bash
# From instances (encodes during training)
cargo run --release --bin tct-train-bpe -- \
  --schema schemas/popular/github-workflow.json \
  --instances "tests/data/instances/github-workflow/real/*.json" \
  --target-vocab 2048 \
  --output target/tct-examples/github-workflow-bpe2048.json

# From tct-prepare-data output (recommended)
cargo run --release --bin tct-train-bpe -- \
  --schema schemas/popular/github-workflow.json \
  --encoded target/tct-examples/github-workflow-encoded/all.jsonl \
  --target-vocab 2048 \
  --output target/tct-examples/github-workflow-bpe2048.json

# One training run, several vocabulary sizes
cargo run --release --bin tct-train-bpe -- \
  --schema schemas/popular/github-workflow.json \
  --encoded target/tct-examples/github-workflow-encoded/all.jsonl \
  --checkpoints "512,1024,2048" \
  --output target/tct-examples/github-workflow-checkpoints.json
# writes github-workflow-checkpoints-512.json, -1024.json, -2048.json
```

Options:

- `--schema`, `--instances` (glob), or `--encoded` (token JSONL). With
  `--schema`, the base vocabulary comes from compiling the schema at `-O`
  without any `--max-vocab` cap (the trainer has no such option). With
  `--encoded` and no schema, it is `--base-vocab`, else the `base_vocab_size` in
  the `metadata.json` next to the corpus, else 257.
- `--target-vocab N`: the vocabulary budget. Training stops after
  `N - (base + 3 + |schema-ids|)` merges: the budget counts the base layer, the
  merges, `PAD`, the always-reserved BOS and EOS slots, and one slot per id in
  `--schema-ids`. To get `M` merges on a 257-token base, use at least
  `257 + M + 3`. The generated raw `VOCAB_SIZE` is `base + merges + 1`, so for
  single-schema training it is `N - 2`. The generated single-schema sequence
  wrapper adds one schema token beyond this budget: its `SEQUENCE_VOCAB_SIZE`
  (`sequence_vocab_size()` in a wheel) is `PAD_TOKEN + 4`, which is `N + 1` for
  single-schema training without `--schema-ids`.
- `--checkpoints`: comma-separated list of vocabulary budgets; training runs
  once to the largest and writes one table per size.
- `--schema-ids`: comma-separated schema identifiers for multi-schema training;
  one reserved slot each.
- `--target-avg-length`: stop when the average sequence length reaches this
  value. `--max-merges` is deprecated in favor of `--target-vocab`.
- `--sample-size N`, `--cache FILE`: sample instances; cache encoded sequences
  for reruns.
- `-O, --optimization`, `--decimal-numbers`, `--name`, `--verbose`.
- `--structure-only-merges`: forbid merges that span value payload tokens
  (string bytes, decimal digits, the end-of-value marker). Requires role files
  from `tct-prepare-data --export-roles`; by default it reads `all.roles.jsonl`
  next to `--encoded`, or pass `--roles PATH`.

Every run writes a provenance sidecar next to the output (`merges.json` gets
`merges.provenance.json`; checkpoint runs write one sidecar for the set). It
records the command, schema or encoded input, adjacent `tct-prepare-data`
metadata when present, base-vocabulary source, target vocabulary or
checkpoints, output files, package version, the git commit and dirty state of
the source tree before outputs were written, SHA-256 fingerprints, and a
`token_corpus` block (sequence count, sequence fingerprint, sample size and
seed, cache status, and accepted and rejected instance manifests). Timings
separate merge selection (`timings.training_wall_time_ns`) from provenance
hashing (`timings.provenance_hashing_ns`).

Set `TCT_BPE_TRAINING_DATE=YYYY-MM-DD` to fix the training date recorded in the
merge file when regenerating an artifact for byte-identical comparison.

### Vocabulary Size Policy

Train new merge tables at power-of-two total vocabulary sizes. The default
ceiling for a single-schema table is 2048; 4096 needs an explicit
schema-specific or shared-corpus reason. The total is the `--target-vocab`
budget described above; the generated single-schema sequence wrapper's
vocabulary is one token larger. `schemas/bpe/` ships only the small
provenance-backed tables that tests and examples use; train larger tables for
your own corpus rather than checking them in.

### tct-measure-tokens

```bash
# Schema-backed: builds a generated helper crate and measures real encode/decode
cargo run --release --bin tct-measure-tokens -- \
  --schema schemas/popular/github-workflow.json \
  --instances "tests/data/instances/github-workflow/real/*.json" \
  [--bpe-merges FILE] [--warmup-iterations N] [--track-allocations] \
  [--generated-bpe-metrics] [--generated-encode-phase-metrics] \
  [--generated-encode-hot-path-metrics] \
  [--generated-encode-validation-phase-metrics] \
  [--generated-decode-prefix-metrics] [--cleanup-generated-helper]

# Encoded mode: same-process BPE over pre-encoded sequences
cargo run --release --bin tct-measure-tokens -- \
  --encoded target/tct-examples/github-workflow-encoded/all.jsonl \
  --bpe-merges target/tct-examples/github-workflow-bpe2048.json \
  --warmup-iterations 1
```

Other options: `-O, --optimization`, `--max-vocab`, `--decimal-numbers`,
`--output-files FILE` (list of successfully measured files), and `--quiet`
(CSV `tokens,instances,avg_tokens` for shell pipelines).

The JSON report (abbreviated):

```json
{
  "tokens": 12345, "instances": 42, "characters": 67890, "has_bpe": true,
  "warmup_iterations": 1, "base_vocab_size": 257, "generated_vocab_size": 2046,
  "pad_token": 2045, "bpe_base_vocab_size": 257, "bpe_merges_count": 1788,
  "token_length_stats": {
    "average_tokens": 293.93, "p50_tokens": 240, "p95_tokens": 770, "p99_tokens": 890,
    "context_window_tokens": 2048, "context_window_pass_rate": 1.0,
    "percentile_method": "nearest_rank"
  },
  "timings": {"generated_encode_wall_time_ms": 25, "total_wall_time_ms": 934, "note": "..."},
  "allocation_metrics": {"available": true},
  "generated_batch_runtime_metrics": {"available": true},
  "same_process_bpe_metrics": {"available": false},
  "provenance": {
    "command": ["tct-measure-tokens", "..."],
    "schema_file": {"path": "...", "bytes": 123, "sha256": "..."},
    "input_files": [{"path": "...", "bytes": 123, "sha256": "..."}],
    "base_vocab_source": "grammar", "optimization_level": "O3"
  }
}
```

How to read it:

- `base_vocab_size` is the generated base layer before BPE and PAD;
  `generated_vocab_size` is `VOCAB_SIZE`; `pad_token` is `PAD_TOKEN`.
- `token_length_stats` is computed over the measured output sequences (after
  BPE when enabled), with nearest-rank percentiles and the inclusive 2048-token
  context-window pass rate.
- Top-level generated encode/decode wall times are parent-observed envelopes
  around the generated helper subprocess, including process transport and JSONL
  handling. The `timings.note` field says so and is part of the contract. Use
  the scoped blocks below before citing hot-path numbers.
- `generated_batch_runtime_metrics` separates child-side batch time from
  parent-side stdin write, stdout read and parse, and wait time. Parent fields
  may overlap child execution; the transport-delta fields are saturating
  parent-minus-child diagnostics, not additive phases. Generated decode timing
  covers the full public decode contract, including post-decode schema
  validation such as `allOf` checks.
- Allocation fields are `null` unless `--track-allocations` is set. When set,
  they are real allocator counters from the helper's measured batch windows
  (JSON parsing, encode/decode, optional BPE, and serialization), not just the
  innermost tokenizer calls. Encoded mode does not support allocation tracking.

Optional sidecar blocks, each from an extra verified pass:

- `--generated-bpe-metrics` (`generated_bpe_metrics`): the embedded generated
  `bpe::encode`/`bpe::decode` loops alone, after helper JSONL parsing and before
  serialization, with round-trip verification and optional allocation counters.
- `--generated-encode-phase-metrics` (`generated_encode_phase_metrics`):
  per-phase nanoseconds (JSON read and parse, base encode, BPE encode,
  serialization, stdout write), verified against the primary pass. Phase fields
  are summed across worker threads and can exceed wall time.
- `--generated-encode-hot-path-metrics` (`generated_encode_hot_path_metrics`):
  encodes the same files but emits only an aggregate token count and checksum,
  removing serialization and transport from the measurement. With
  `--track-allocations` it reports helper allocations for that window.
- `--generated-encode-validation-phase-metrics`
  (`generated_encode_validation_phase_metrics`): diagnostic split of file read,
  `serde_json::Value` parse, and typed `serde_json::from_value` validation,
  verified against the primary `from_str` path. Use for attribution only.
- `--generated-decode-prefix-metrics` (`generated_decode_prefix_metrics`): runs
  `decode_prefix` over empty, midpoint, and full prefix cuts of each measured
  sequence and reports completeness, consumed tokens, field progress, and
  timing. BPE sequences are expanded to raw tokens before cutting, so this does
  not measure BPE-prefix streaming.

The sidecar flags are not supported with `--encoded` or `--quiet`. Encoded mode
instead fills `same_process_bpe_metrics`, which times `SequentialBpeEncoder` and
`BpeDecoder` in process and verifies that decoding reproduces the input.

### tct-analyze-usage

```bash
cargo run --release --bin tct-analyze-usage -- \
  --schema schemas/popular/github-workflow.json \
  --instances "tests/data/instances/github-workflow/real/*.json" \
  --format json
```

Reports the base vocabulary size, per-token frequencies, unique and unused
token counts, a category breakdown, rare tokens (fewer than 10 uses), and the 20
most frequent tokens, with JSON provenance (command, schema and input
fingerprints, `base_vocab_source: "grammar"`, BPE fields `null`). Token IDs are
untyped (see "Schema-Local Token Streams"), so the categories are numeric
ranges, not token types: IDs 0-255 count as `byte`, `u32::MAX` as `special`, and
everything else as `structural`. An enum index below 256 is therefore counted as
a byte token, and the end-of-value marker (256) as structural. The schema is
always compiled at O3 with the root type name `TestType`; `--name` only sets the
label used in the report (default: the schema file stem).

### BPE Lookup And Length Tools

```bash
# Compare pair-lookup strategies before changing the generated BPE lookup
cargo run --release --bin tct-bpe-lookup-benchmark -- \
  --bpe-merges target/tct-examples/github-workflow-bpe2048.json \
  --encoded target/tct-examples/github-workflow-encoded/all.jsonl \
  --iterations 5 \
  --warmup-iterations 1

# Sequence length stats for base or BPE-compressed JSONL
cargo run --release --bin tct-bpe-length-stats -- \
  --encoded target/tct-examples/github-workflow-encoded/all.jsonl \
  --bpe-merges target/tct-examples/github-workflow-bpe2048.json
```

The lookup benchmark compares the sorted binary-search table used by generated
code, a hash map, and a bucketed table under the same merges and vocabulary cap
(`--max-vocab-size`, normally `VOCAB_SIZE`). Each strategy must first reproduce
`SequentialBpeEncoder` output exactly. It is a microbenchmark, not a substitute
for end-to-end measurement.

### Vocabulary Scaling Summaries

To compare several vocabulary sizes for one corpus, train with `--checkpoints`,
measure each table with `tct-measure-tokens`, and summarize:

```bash
python3 scripts/analysis/bpe_vocab_scaling.py \
  --entry package=base=target/tct-analysis/package-base-measure.json \
  --entry package=512=target/tct-analysis/package-512-measure.json \
  --entry package=1024=target/tct-analysis/package-1024-measure.json \
  --entry package=2048=target/tct-analysis/package-2048-measure.json \
  --json-output target/tct-analysis/bpe-vocab-scaling.json \
  --markdown-output target/tct-analysis/bpe-vocab-scaling.md \
  --require-ready
```

The helper reads adjacent provenance sidecars (including checkpoint-set
sidecars) and records average, p95, p99, context-window pass rate, marginal
average-token reduction, merge count, artifact size, corpus fingerprint, and
trainer wall time.

### BPE Artifact Manifest And Reproducibility

Checked-in merge tables live in `schemas/bpe/`; each carries a
`.provenance.json` sidecar. To inventory them:

```bash
python3 scripts/analysis/bpe_artifact_manifest.py --output target/tct-bpe-artifact-manifest.json
```

For each table the manifest records path, SHA-256, schema and encoding family
from the file name, base vocabulary, merge count, generated vocabulary size
with PAD, legacy `dict_size`, and a sidecar summary. `provenance_status` is
`provenance_ready`, `incomplete_sidecar`, or `missing_sidecar`; a table is ready
only when its sidecar has input corpus fingerprints, matching output-file
hashes, and token-corpus evidence where sampling or caching was used. Each
table is also classified against the vocabulary size policy above, and the
top-level `gap_counts` field groups tables by missing evidence.

The release gate reproduces every shipped table from the current source:

```bash
cargo fetch --locked
python3 scripts/analysis/public_bpe_source_manifest.py
python3 scripts/testing/test-bpe-artifact-reproducibility.py --public-artifacts
```

The version 2 clearance manifest binds the complete published source by path,
mode, byte length, and SHA-256, independently of Git history. It also pins all
six tables and original sidecars. The sidecars' original `repo_commit` remains
archival training provenance; no historical checkout is required. The all-six
gate verifies the exact authored fixtures and source snapshot before and after
retraining. It compares raw table file hashes and the full token-corpus block,
and fails on any mismatch, missing result, partial selection, or dry run.

Public mode clears inherited `TCT_*`, `CARGO_*`, Rust compiler overrides and
corpus overrides, sets `TCT_SOURCE_DIR` to the current checkout, and uses locked
workspace Cargo commands. Generated helpers copy the workspace lockfile; all
Cargo builds run offline with two build jobs and two Rayon threads. Populate
the dependency cache with `cargo fetch --locked` first. Use `--cargo-home` or
`--cargo-target-dir` to explicitly admit a different dependency/build cache.
Cache paths inside the source checkout must be under `target/`; files and
symlink aliases back into ordinary source directories are rejected before any
build. Generated temporary crates use the fresh run directory via `TMPDIR`.

The report goes to `target/tct-bpe-artifact-reproducibility-report.json` and
includes tool versions, build settings, all six comparison outcomes, and source
digests. Regenerated files and logs go into a fresh `public-*` directory under
`target/tct-bpe-artifact-reproducibility/`. Original tables and sidecars are
never overwritten. Other options are `--output`, `--work-dir`,
`--cargo-profile dev|release`, and `--timeout-seconds`; output inside the source
checkout must stay under `target/`.

After reviewed source edits, run
`python3 scripts/analysis/public_bpe_source_manifest.py --write` before the
all-six gate. Snapshot refresh alone does not qualify a release. The precise
source exclusions and archival provenance roles are in
[`docs/publication.md`](docs/publication.md).

For development, individual `--artifact PATH` checks and `--dry-run` remain
available without `--public-artifacts`. Default discovery selects only
provenance-backed, power-of-two targets at or below 2048. The shipped 500/1000
recipes are outside that queue; only `--public-artifacts` qualifies the release.

## Schema Evolution Reports

`tct-token-diff` compiles two versions of a schema and compares their token-ID
layouts, so you can tell whether downstream artifacts (encoded corpora, BPE
tables, model embeddings) can be reused.

```bash
cargo run --release --bin tct-token-diff -- \
  --old-schema path/to/old-schema.json \
  --new-schema path/to/new-schema.json \
  --name Example \
  --optimization O3 \
  --string-encoding utf8 \
  --max-base-vocab 512 \
  --old-bpe-merges path/to/old-merges.json \
  --new-bpe-merges path/to/new-merges.json \
  --output target/tct-analysis/token-diff.json
```

`--optimization` applies to both sides, and merge tables are given per side.
The other settings it models have a shared flag plus `--old-*` / `--new-*`
overrides: `--name`, `--enum-optimization`, `--bounded-int-optimization`,
`--bit-packing`, `--string-encoding` (`utf8` or `ascii`), and
`--max-base-vocab`. Use the overrides when the transition itself changes a
setting. (A hidden `--pattern-optimization` flag exists only for compatibility;
pattern optimization is reserved, so the flag always fails.)

Not every setting that changes the token layout is modeled. `tct-token-diff` has
no `--decimal-numbers` or `--max-vocab` option, and `TokenIdLayoutCompileOptions`
carries neither setting; it always compiles with the default number encoding and
no `--max-vocab` cap. Reports therefore cannot model artifacts built with
`--decimal-numbers` or `--max-vocab`.

The report contains schema file provenance, generated-code hashes and sizes,
compile timings, base vocabulary, string encoding, `max_base_vocab`, pass
overrides, BPE range, padding token, vocabulary size, and a structural token map
derived from pass metadata (not from generated source text). Structural entries
carry generated order, encoding decision, and per-member emission metadata
(selectors, presence flags, bit-packed presence bits, value width, catch-all
maps, nested objects, strict stream shape), so a review can see when an
existing entry moved. With merge tables it adds file and canonical merge
fingerprints, base-vocabulary match evidence, and whether the new table
extends the old one as an exact prefix (`prefix_extended`).

If BPE artifact validation prevents a complete comparison, the command exits
nonzero but still writes a `partial_error` report with compile errors, no-BPE
layout evidence, and parsed BPE provenance. Every report has an
`experiment_gate` block stating that it is compiler-artifact evidence only and
listing the downstream provenance (corpus, seeds, training and evaluation
commands, checkpoints, result files) needed before a model-level reuse claim.

### Decision Classes

Read the stable-vocabulary decision conservatively:

- `exact_reuse`: generated code and layout are identical. Existing tokenizer,
  BPE, and embedding artifacts may be reused for the same compiler
  configuration. This is the only class that permits reuse without review.
- `layout_only_unchanged`: top-level ranges match but generated code changed.
  Regenerate the tokenizer and review the structural token map before reusing
  BPE tables or embeddings. Most additive schema edits land here today.
- `additive_compatible`: existing IDs and stream shapes are proven stable. The
  proof is deliberately narrow: append-only root enum variants, and appended
  required members that the generated tokenizer emits as zero tokens (consts),
  when compiler configuration and BPE artifacts are unchanged or absent. A
  schema `const` alone is not enough; for example a required string `const` at
  O0 still emits an enum token.
- `breaking_change`: top-level ranges moved, or the ranges are unchanged but the
  structural token map shows changed token semantics. Retrain or explicitly
  migrate BPE tables, embeddings, and any schema-local token assumptions.

Also retrain or migrate whenever string encoding, `max_base_vocab`, the BPE base
vocabulary, or any top-level token range changes. Matching ranges alone do not
prove semantic compatibility. Ordinary object-property additions usually shift
meaning (optional members add presence or state tokens, required members add
value tokens, and members before a catch-all map can move later members), so
they are reported as `layout_only_unchanged` or `breaking_change`. Promoting
them would need an explicit, versioned migration contract covering member
order and emission, old-stream acceptance, default and presence tokens,
bit-packed slots, variable-width encodings, and catch-all maps.

Complete reports also carry a `vocabulary_extension_handoff` block (artifact
kind `tct_vocabulary_extension_handoff`): old and new schema hashes, ranges and
deltas, reusable old ranges when evidence supports them, appended token ranges,
changed structural paths, BPE prefix-extension evidence and fingerprints, and
the downstream metadata a migration must supply (for example the embedding
initialization policy). A `candidate_extension_artifact` status means the block
can seed a migration or fine-tuning review; it does not prove that existing
encoded sequences, embeddings, or model behavior are unchanged, and it never
overrides the decision class. Treat `prefix_extended` BPE evidence the same
way: old merge IDs are preserved, but new merges can still change encoded
sequences.

Focused tests: `cargo test --lib schema_evolution`.

## Quality Gates

### quality-check.sh

`./scripts/testing/quality-check.sh` is the local quality summary and the job
CI runs. It has two gate classes.

Stable gates (fail the script):

1. Disk space preflight: at least `TCT_QUALITY_MIN_FREE_KIB` (default 10 GiB)
   free on the repository and temp filesystems.
2. `cargo clippy --all-targets` completes, and reports zero warnings.
3. `cargo fmt --all -- --check`.
4. `cargo test` (non-ignored tests, including the default round-trip smoke).
5. Python maintenance tests (`python3 -m unittest discover tests/python`).
6. `cargo build --release` (skipped, and counted as failed, when the disk
   preflight failed).

Advisory gates (reported; they fail the script only with
`TCT_STRICT_QUALITY=1`):

- Module size report (`scripts/testing/module-size-report.py` with
  `scripts/testing/module-size-policy.toml`; limit `TCT_MODULE_LINE_LIMIT`,
  default 600 lines). The policy file is a review queue, not a hard cap.
- Documentation coverage (`scripts/testing/documentation-coverage-report.py`;
  threshold `TCT_DOC_COVERAGE_THRESHOLD`, default 90).
- Generated-source substring inventory
  (`scripts/testing/generated-source-inventory.py`): flags production code that
  classifies generated source by substring and tests that assert on generated
  source text.
- `unwrap`/`expect` policy report (`scripts/testing/unwrap-expect-report.py`
  with `scripts/testing/unwrap-expect-policy.toml`). The policy file lists only
  narrow documented invariants; every other production site is a cleanup
  candidate.

Other controls:

- Advisory JSON sidecars are written to `target/tct-quality/` (`TCT_QUALITY_REPORT_DIR`).
  They carry an artifact kind and version.
- Generated crates built by the helper use
  `target/tct-quality-generated-cargo` unless `TCT_GENERATED_CARGO_TARGET_DIR`
  is already set.
- Concurrent runs are serialized with `flock` on `target/tct-quality.lock`
  (`TCT_QUALITY_LOCK_FILE`). A second run fails immediately and prints the
  holder; set `TCT_QUALITY_LOCK_WAIT_SECONDS=N` to wait instead, or
  `TCT_QUALITY_USE_LOCK=0` if something else already serializes runs.
- When the disk preflight fails, prefer `cargo clean` over lowering
  `TCT_QUALITY_MIN_FREE_KIB`; the other stable and advisory checks still run so
  their results are visible.

You can run the advisory reports directly:

```bash
python3 scripts/testing/module-size-report.py --json-output target/tct-quality/module-size-report.json --exit-zero
python3 scripts/testing/documentation-coverage-report.py --json-output target/tct-quality/documentation-coverage-report.json --exit-zero
python3 scripts/testing/generated-source-inventory.py --mode all --json-format artifact --json-output target/tct-quality/generated-source-inventory.json
python3 scripts/testing/unwrap-expect-report.py --root src --json-output target/tct-quality/unwrap-expect-report.json
```

Do not describe advisory checks as blocking unless a cleanup policy makes them
so.

### local-ci.sh and GitHub Actions

`./scripts/testing/local-ci.sh` mirrors the ordinary CI job locally: it sets a
local-ci-owned `TCT_GENERATED_CARGO_TARGET_DIR` under
`target/tct-generated-cargo/` and runs `quality-check.sh`, whose `cargo test`
already includes the default round-trip smoke. `--dry-run` prints the commands.

The GitHub-hosted workflows are dispatch-only; nothing runs automatically on
pushes or pull requests. `.github/workflows/ci.yml` installs the toolchain from
`rust-toolchain.toml` and runs `./scripts/testing/quality-check.sh`; uncomment
its `push` and `pull_request` triggers to enable hosted runs.
`.github/workflows/extended-quality.yml` offers these gates: `target-schemas`, `round-trip-bpe-smoke`,
`generated-reproducibility`, `wheel-smoke`, `bpe-artifact-reproducibility`,
`coverage-stable` (uploads `target/tct-coverage/stable`), and
`synthetic-ignored-coverage`, or `all`.

Local results are authoritative. Run the relevant local commands before
submitting; the hosted workflows mirror them rather than replacing them.

### Coverage Reports

Coverage is report-only: there is no global threshold. The tool is
`cargo llvm-cov` (install with `cargo install cargo-llvm-cov`), wrapped by
`scripts/testing/coverage-check.sh`, which writes under `target/tct-coverage/`
(`TCT_COVERAGE_DIR`).

```bash
./scripts/testing/coverage-check.sh quick              # library only, fast
./scripts/testing/coverage-check.sh stable             # non-ignored workspace tests
./scripts/testing/coverage-check.sh synthetic-ignored  # ignored synthetic round trips
./scripts/testing/coverage-check.sh round-trip-full    # ROUND_TRIP_FULL=1 round trips
```

Each mode writes `summary.txt`, `summary.json`, `lcov.info`, and `html/`. The
generated-crate modes also write `generated-crates-summary.json` and
`generated-crates-lcov.info`, because generated helper binaries are external
packages that the workspace report does not include.

Focus a report on the files you touched:

```bash
env TCT_COVERAGE_FOCUS='src/generator src/bpe/encoder.rs' ./scripts/testing/coverage-check.sh quick
env TCT_COVERAGE_CHANGED_FROM=origin/main ./scripts/testing/coverage-check.sh stable
python3 scripts/testing/coverage-focus.py target/tct-coverage/stable/summary.json src/generator
```

Through the wrapper the focused summary lands in `focused-summary.txt`. Treat
it as a signal for test design next to targeted semantic tests.

Other variables: `TCT_COVERAGE_BRANCH=1` (branch coverage; requires nightly
Rust), `TCT_COVERAGE_NO_CLEAN=1` (reuse profile data),
`TCT_SYNTHETIC_IGNORED_SCOPE=passing|all`, and `TCT_SYNTHETIC_IGNORED_TEST=NAME`
(one ignored synthetic test, for debugging the wrapper):

```bash
env TCT_SYNTHETIC_IGNORED_TEST=test_simple_duplicate_round_trip ./scripts/testing/coverage-check.sh synthetic-ignored
```

Do not commit coverage artifacts or copy coverage percentages into maintained
docs.

## Benchmarks

### Schema Compilation Benchmarks

```bash
cargo test --features extended-benchmarks --test schema_compilation_test test_extended_real_world_schemas -- --ignored --exact --nocapture
./tests/benchmarks/regression_check.sh
```

Output goes to `target/tct-benchmarks/` (`TCT_BENCHMARK_OUTPUT_DIR`).
`tests/benchmarks/baseline.json` is the curated comparison baseline; update it
only deliberately.

### Generated Tokenizer Runtime

For runtime evidence about generated tokenizers, use the reproducible wrapper.
It runs `tct-measure-tokens`, preserves the JSON report, stderr log, and the list
of successfully measured files, and can measure an importable wheel over the
same files:

```bash
./scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh \
  --schema schemas/popular/github-workflow.json \
  --instances "tests/data/instances/github-workflow/real/*.json" \
  --output-dir target/tct-benchmarks/generated-runtime/github-workflow \
  --optimization O3 \
  --warmup-iterations 1 \
  --track-allocations \
  --generated-encode-hot-path-metrics
```

Outputs: `generated-crate-report.json` and `generated-crate.log`. The wrapper
passes through `--bpe-merges` and every generated metric flag of
`tct-measure-tokens`. Add `--cleanup-generated-helper` to remove the helper
crate after a successful run; failed helper builds keep their project path.

To measure an already built and importable wheel over the same instances:

```bash
./scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh \
  --schema schemas/popular/github-workflow.json \
  --instances "tests/data/instances/github-workflow/real/*.json" \
  --output-dir target/tct-benchmarks/generated-runtime/github-workflow-wheel \
  --wheel-module tct_github_workflow \
  --wheel-pythonpath /path/to/importable/package \
  --wheel-iterations 5 \
  --wheel-warmup-iterations 1
```

This adds `wheel-encode-report.json` and `wheel-encode.log` (via
`scripts/benchmarking/measure_wheel_encode.py`; `--wheel-decode` also times
decode). Wheel reports are wall-clock Python API measurements with module,
input, Python version, token count, timing, and failure metadata, without
allocation counters.

`measure_tiktoken.py` counts tokens with an external tokenizer for baselines.

When reporting a number, keep enough to reproduce it: schema, corpus, compiler
settings, BPE artifact, seed, command, and output report.

## Troubleshooting

**A generated crate fails to build.** The error names the preserved temporary
project; open its `Cargo.toml` and source there. Check that you did not add a
dependency outside `GeneratedCargoToml`, and that `rust-toolchain.toml`,
`Cargo.toml`, and `GENERATED_RUST_VERSION` agree (`cargo test
generated_crate::tests`).

**A round trip fails.** Re-run the single schema with details:

```bash
env SCHEMA_NAME=<schema> SCHEMA_FOLDER=<folder> ROUND_TRIP_DEBUG=1 \
  cargo test --test round_trip_test test_round_trip -- --nocapture
```

Look at `target/tct-round-trip-report.json` and the test output for the
failing phase and fixture. If the schema is large, reduce it
to a synthetic schema first (see "Focused Iteration And Test-Driven Fixes").

**An instance validates but will not encode.** Check `unencodable-report.json`
from `tct-prepare-data`; `unknown_field` means a closed object
(`additionalProperties: false`) received an undeclared key.

**quality-check.sh fails on disk space.** Run `cargo clean`, or remove stale
generated crates under the temp directory; lower `TCT_QUALITY_MIN_FREE_KIB` only
on constrained machines.

**quality-check.sh says another helper holds the lock.** Wait for it, or set
`TCT_QUALITY_LOCK_WAIT_SECONDS`.

**The Python maintenance gate refuses to run.** `TCT_PYTHON_SUITE_ACTIVE` is set
in your environment; run the helper from a fresh shell.

**Wheel smoke fails before building.** Read
`target/tct-wheel-tool-envelope.json`; install the missing tool. If the built
wheel has the wrong interpreter tag, set `TCT_WHEEL_PYTHON`. For
`--features python` link errors, set `PYO3_PYTHON`.

**Compilation is slow or the vocabulary is large.** Use `-O 0` while developing,
check `--analyze` output, prefer specific enums over open strings in the schema,
and consider BPE or `--max-vocab` (which has no effect at `-O 0`).

**Debug output.** `RUST_LOG=debug` (or `trace`), `TCT_DEBUG=1`, and
`TCT_VERBOSE=1` enable library logging:

```bash
RUST_LOG=debug cargo test --test cli_test -- --nocapture
```

`KEEP_SCHEMASTORE=1` keeps the temporary schema directory created by the
schema compilation tests.

## Documentation Maintenance

- Update `README.md`, `ARCHITECTURE.md`, `DEVELOPMENT.md`, and
  `scripts/README.md` in the same change as the commands, flags, scripts, tests,
  or contracts they describe.
- Every path, script, test name, flag, and environment variable in these docs
  must exist. Check references with `rg` (or `grep -r`) before committing.
- Keep volatile numbers (test counts, benchmark results, coverage percentages,
  dated status notes) out of maintained docs. They belong in generated reports
  under `target/`.
- `scripts/README.md` must keep the phrases asserted by
  `tests/python/test_benchmarking_script_scope.py`.

## Public source preparation

Publication tooling and its safety tests require Python 3.11 or newer. The
runtime Python binding retains the compatibility specified in `pyproject.toml`.
See `docs/publication.md` for the single-commit release, content-addressed
source checks, archival training provenance, all-six BPE reproduction, and
distribution artifact qualification. Preserve the private preparation history
separately in `tct-research`.
