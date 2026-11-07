# TCT: Type-Constrained Tokenization

TCT compiles a JSON Schema into a schema-specific tokenizer. The generated
tokenizer turns JSON instances of that schema into compact token sequences and
back, with token semantics that depend on the position in the schema: each
position is read as a field, a choice among the schema's finite alternatives
(enum values, union branches, whether an optional field is present), or a value
of the declared type. A successfully decoded value therefore always has the
schema's structure, and vocabularies are small: a few hundred to a few thousand
tokens (never fewer than the 256 byte tokens plus markers) instead of the 50K+
of a general-purpose tokenizer. TCT guarantees structure, not value
constraints such as string patterns or length bounds; see
[Decode Contract](#decode-contract).

**Example**: for `{"role": "admin"}` with a closed object
(`"additionalProperties": false`) that requires `role: enum["admin", "user", "guest"]`

- Illustrative text pieces: `{`, `"`, `role`, `"`, `:`, `"`, `admin`, `"`, `}` (9 pieces, not a measured tokenizer baseline)
- TCT: `0` (1 token, the context-aware enum index)

An object that allows additional properties, which is the JSON Schema default,
also encodes them, so the same instance costs two more tokens for the (empty) set
of additional properties when the schema does not say `"additionalProperties": false`.

The compiler is written in Rust and emits Rust code. It can also package the
generated tokenizer as a Python wheel, train BPE merge tables on top of the
schema-local token streams, and report token-layout compatibility between
schema versions.

Experimental frontends for SQL (schema linking) and source code (Go and C#
syntax, tree-sitter subsets) are available behind the `experimental` Cargo
feature; see [Experimental Frontends](#experimental-frontends).

## Installation

```bash
git clone https://github.com/jonas-schmitt/tct.git
cd tct
cargo build --release
```

`rust-toolchain.toml` pins the Rust toolchain; `rustup` installs it on first
use. Python wheel generation additionally needs `maturin` (see
`DEVELOPMENT.md`, "Python Wheels").

## Usage

### Command Line

```bash
# Compile a schema to a Rust tokenizer module (default: -O3)
./target/release/tct schema.json -o tokenizer.rs

# Preflight the compilation without writing generated code
./target/release/tct schema.json --dry-run -o tokenizer.rs

# Generate a Python wheel, optionally with BPE compression and prefix decoding
./target/release/tct schema.json --target python \
    --bpe-merges bpe.json --streaming --optimization O3

# Development mode (preserves schema structure, no optimizations)
./target/release/tct schema.json -O0 -o tokenizer_dev.rs

# Size and vocabulary profiles
./target/release/tct schema.json -Os -o tokenizer_small.rs
./target/release/tct schema.json -OVocab -o tokenizer_small_vocab.rs

# Analyze vocabulary requirements instead of generating code
./target/release/tct schema.json --analyze
```

Run `./target/release/tct --help` for the full flag list.

### Rust API

```rust
use tct::{TctCompiler, OptimizationLevel};

// `schema` is a `serde_json::Value`.
let grammar = TctCompiler::new(schema, "MyType")?
    .with_optimization_level(OptimizationLevel::O3)
    .compile()?;
let code = grammar.generate_code()?;
```

The root type of the generated code implements `encode`, `decode`,
`decode_full`, `decode_strict`, `decode_strict_full`, and (with streaming
enabled) `decode_prefix`; the file also defines `VOCAB_SIZE`, `PAD_TOKEN`, and
the token-layout constants.

### Using the Generated Rust Code

The generated file is self-contained and does not depend on the `tct` crate,
but it refers to its own items through `crate::` paths, so it must be the root
(`src/lib.rs`) of its own crate. That crate needs the dependencies the
compiler's wheel builder pins in `src/generated_crate.rs`:

```toml
[dependencies]
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
rustc-hash = "2.1.2"
regex = "1.12.3"
lazy_static = "1.5.0"
jsonschema = { version = "0.46.2", default-features = false }
chrono = { version = "0.4.44", features = ["serde"] }
base64 = "0.22.1"
```

`jsonschema` is used to pick the matching branch of `anyOf`/`oneOf` values when
JSON is deserialized. Generated code never fetches schemas: external `$ref`s
resolve only against the schemas vendored into the compiler, so the default
features (an HTTP/TLS stack for remote references) are not needed. Add
`features = ["preserve_order"]` to `serde_json` if untyped values must keep
their object key order.

```bash
./target/release/tct schemas/popular/github-workflow.json -o my_tokenizer/src/lib.rs
```

```rust
use my_tokenizer::GithubWorkflow; // root type: --name, else the schema title, else the file name
use my_tokenizer::tokenizer::JsonTokenEncoder;

let value: GithubWorkflow = serde_json::from_str(&json)?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let decoded = GithubWorkflow::decode_full(tokens.iter().copied())?;
```

Pass `--name` to choose the root type name.

### Python Module

The root crate also builds a Python extension module, `tct` (distribution name
`tct`), that exposes the compiler itself, so schemas can be compiled
without the command line:

```bash
python3 -m pip install maturin
maturin develop --release   # builds and installs the module into the active virtualenv
```

```python
import tct

schema = {
    "title": "Task",
    "type": "object",
    "properties": {"status": {"enum": ["active", "done"]}},
    "required": ["status"],
}
rust_source = tct.compile(schema, streaming=True)  # what `tct schema.json --streaming` writes
report = tct.analyze(schema)  # `tct schema.json --analyze --format json`, with `schema_file` "<schema>"
```

`schema` may be JSON text, `bytes`, or any JSON-serializable object, and the
root type name defaults to the schema `title`. The keyword options mirror the
command line: `optimization` (`"O0"`, `"O2"`, `"O3"`, `"Os"`, `"OVocab"`),
`streaming`, `bit_packing`, `decimal_numbers`, `bpe_merges` (path of a trained
merge table), and `max_vocabulary`. Invalid schemas and options raise
`tct.CompileError` (a `ValueError`); schema quality findings are raised as
`UserWarning`s, so `warnings.simplefilter("error")` turns them into failures.
The module also exports the BPE helpers `Utf8BpeEncoder`, `Utf8BpeDecoder`, and
`Utf8BpeTokenizer`. Wheels with `encode`/`decode` for one specific schema are
generated with `tct --target python`, described next.

### Generated Python Wheels

Wheels generated with `--target python` export:

```python
from tct_schema import (
    decode,
    decode_sequence,
    encode,
    encode_sequence,
    sequence_metadata_json,
    sequence_vocab_size,
    vocab_size,
)

tokens = encode(json_string)           # JSON -> tokens
json_out, consumed, total = decode(tokens)

sequence = encode_sequence(json_string)  # [BOS, schema, offset payload..., EOS]
json_out, consumed, total = decode_sequence(sequence)
metadata_json = sequence_metadata_json()
```

Wheels generated with `--streaming` also export
`decode_prefix(partial_tokens)`, returning `(json_out, fields_decoded, complete)`.

## Optimization Levels

| Level | Description | Use Case |
|-------|-------------|----------|
| `-O0` | No optimization passes: enums are still one index token each, but single-variant enums cost a token and integers use the general multi-token encoding | Debugging |
| `-O2` | Adds bounded-integer offset encodings and zero-token single-variant enums | Balanced |
| `-O3` | `-O2` plus format-aware string encodings (RFC 3339 date-times) and bit-packed optional-field presence for structs with at least four optional fields (default) | Maximum compression |
| `-Os`, `-OVocab` | Currently identical: `-O3` without the format-aware string encodings (bit-packing stays on); the two names are kept for future divergence | Context-constrained decoding |

**Optimization features**:

- `enum_optimization`: single-variant enums take no tokens, and `--max-vocab`
  switches enums larger than the cap to a multi-token codec (every other enum
  value is one index token at every level)
- `integer_optimization`: bounded integers become compact offset encodings
- `vocabulary_optimization`: only affects the vocabulary statistics the
  compiler reports, not the generated encodings
- `bit_packing`: optional-field presence becomes bit vectors; enabled by O3, Os,
  and OVocab and exposed separately with `--bit-packing`

## String Encoding

Strings use end-marker encoding: a byte sequence terminated by
`END_STRING_MARKER` (token 256 for the default UTF-8 base vocabulary). Full
`decode()` rejects invalid string byte tokens and invalid UTF-8; streaming
`decode_prefix()` can return partial text from truncated strings.

## Decode Contract

`decode()` is a complete recovery decode API. It reconstructs one value and
returns errors for EOF, invalid strings, `allOf` constraints that the generated
code re-validates after decoding, and decoded collection lengths above the
configured cap. Some bounded choices, such as booleans, enum discriminators,
optional-state flags, and BPE recovery, are normalized with modulo arithmetic.
Bounded integers are not: recovery decode adds the minimum to whatever offset
the tokens encode, so a stream can decode to an integer outside
`minimum`/`maximum` (for the range 1 to 10, the tokens `[0, 200]` decode to
`201`). `decode_full()` applies those same recovery semantics for one value and
then rejects unconsumed trailing tokens.

`decode_strict()` and `decode_strict_full()` reject out-of-range finite choices
and out-of-range bounded-integer offsets instead of normalizing or accepting
them.

### What the Tokenizer Does Not Enforce

The token layout encodes types, structure, and finite choices; it is not a
validator. These keywords are accepted in a schema but not enforced by
`encode`, `decode`, or `decode_strict`: `pattern` on string values,
`minLength`, `maxLength`, `minItems`, `maxItems`, `uniqueItems`, `multipleOf`,
and numeric bounds on floats and on integers that have only one limit. An
instance that violates them encodes and decodes unchanged. Validate decoded
values against the schema when they matter.

`decode_prefix()` is explicitly truncation-tolerant and may return partial data
with defaults. It accepts a left prefix of a canonical token stream; it is not a
suffix-constrained gapped infill API.

## Streaming Decode

`decode_prefix()` returns partial results from truncated token sequences:

- Strings: bytes collected before the end marker
- Arrays: elements decoded before the length is exhausted
- Structs: fields decoded in schema property order (the order is part of the
  token layout)

Generated `GapInfo`/`PartialEncoding` metadata describes default-filled fields
in a complete token sequence. It does not provide parser state for generating a
middle span that must satisfy a right suffix.

## Sequence Wrappers

Raw generated token streams stay schema-local and content-only. Model-facing
wheel helpers use a versioned wrapper instead: explicit sequence PAD, BOS, EOS,
and schema-decision tokens, plus an offset payload range so wrapper specials do
not collide with raw content tokens. BPE compression remains inside the payload:
the wrapper is applied after local BPE encode and removed before local BPE
decode. `sequence_metadata_json()` reports the artifact kind and version,
wrapper policy, schema token, raw vocabulary size, base and BPE payload ranges,
wrapper special-token reservations, BPE merge metadata when present, and whether
the optional prefix decode API was generated.

`tct::wheel_builder::write_bundle_wrapper_package()` and the
`tct-build-bundle-wrapper --spec bundle-spec.json` binary assemble a pure-Python
package that imports several generated schema wheels and applies one shared
bundle sequence contract around their raw `encode`/`decode` APIs.

Hybrid prompt/response metadata is represented in `tct::sequence_bundle`.
Prompt tokens keep the pretrained tokenizer's native IDs; TCT response tokens
map into a contiguous added-token block as `block_start + bundle_token_id`, and
numeric mode-entry/mode-exit sentinels are validated outside the prompt
tokenizer's occupied IDs and outside the TCT block. Validity-by-construction
applies to the TCT response span after the mode switch, not to the
natural-language prompt. The Hugging Face-loadable sidecar artifact contract is
documented in `docs/huggingface-compatible-tokenizer-contract.md`.

## Schema Support

**Supported**: object, string, integer, number, boolean, array, null,
enum/const, oneOf, anyOf, allOf object intersections, local and vendored refs,
definitions, recursive schemas, additionalProperties, and patternProperties.

**Limitations**: allOf support is exact for the documented object-merge and
validation-only subset; unsupported composition shapes fail at compile time.
Mixed-type enums have limited support. Value-level validation keywords are
accepted but not enforced; see
[What the Tokenizer Does Not Enforce](#what-the-tokenizer-does-not-enforce).

**Generated names**: a definition named like a Rust prelude item (`string`,
`default`, `result`, `option`, ...) would shadow that item in the generated crate,
so its type is generated with a `Type` suffix (`StringType`, `DefaultType`). A root
type name that collides is rejected with a suggestion; choose another with `--name`.

The regression set of real-world schemas is maintained in
`schemas/target-schemas.txt` and checked with
`./scripts/testing/test-target-schemas.sh`.

## BPE Compression

TCT supports byte-pair encoding on top of the schema-local token sequences:

```bash
# Encode a corpus once and split it for training
./target/release/tct-prepare-data --mode tct --schema schema.json \
  --sources "instances/*.json" --output prepared/
# Train merges on the prepared TCT token sequences
./target/release/tct-train-bpe --schema schema.json \
  --encoded prepared/all.jsonl --target-vocab 2048 -o bpe.json
# Generate a wheel with compression
./target/release/tct schema.json --target python --bpe-merges bpe.json
```

Use power-of-two total target vocabulary sizes; 2048 is a sensible default
ceiling for a single schema. `tct-measure-tokens` and `tct-analyze-usage`
report compression and token usage with provenance. Legacy
dictionary-compression artifacts are not supported.

## Schema Evolution

`tct-token-diff` compares the token layouts of two schema versions and reports
one of `exact_reuse`, `layout_only_unchanged`, `additive_compatible`, or
`breaking_change`. Read the report conservatively: unchanged ranges alone do
not prove that downstream BPE tables, embeddings, or encoded corpora can be
reused. See `DEVELOPMENT.md` ("Schema Evolution Reports").

## Experimental Frontends

The crate also contains experimental non-JSON frontends: a
syntax/grammar-level route (parser-backed metadata, `GrammarIR`,
`StructuralModel`, token layout, raw syntax state machine, generated codec)
exercised with Go and C# syntax fixtures, a SQL schema-linking frontend,
tree-sitter subset codecs, and an exact-mask contract. They are off by default
and compiled only with the `experimental` Cargo feature:

```bash
cargo build --features experimental
cargo test --features experimental
```

These surfaces are experimental: they carry no stability guarantees, and
their APIs, token layouts, binaries, and gates may change or be removed
without notice. Their command reference is `docs/frontends-development.md`
and their contracts live under `docs/`. Some of their tests and helpers need
the pinned Go toolchain and a .NET SDK (see
`scripts/testing/resolve-pinned-go.sh` and
`scripts/analysis/csharp_toolchain_smoke.py`).

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md): the four-pass compilation pipeline,
  codec generation, and the encode/decode contracts
- [DEVELOPMENT.md](DEVELOPMENT.md): building, testing, validation layers,
  generated crates, wheels, BPE training, and quality gates
- [CONTRIBUTING.md](CONTRIBUTING.md): how to report problems and submit changes
- [GLOSSARY.md](GLOSSARY.md): the project's vocabulary, one term per concept
- [docs/huggingface-compatible-tokenizer-contract.md](docs/huggingface-compatible-tokenizer-contract.md):
  sidecar artifact contract for Hugging Face workflows
- [docs/frontends-development.md](docs/frontends-development.md): command
  reference for the experimental frontends
- [examples/github-workflow](examples/github-workflow): a generated GitHub
  Actions workflow tokenizer used from Python

## Testing

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test                                              # All non-ignored core tests
cargo test --features experimental                      # Plus the experimental frontends
./scripts/testing/quality-check.sh                      # Stable gates plus advisory reports
env SCHEMA_NAME=kubernetes SCHEMA_FOLDER=popular cargo test --test round_trip_test test_round_trip
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) and
[NOTICE](NOTICE). Source revisions and retained notices are indexed in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Third-party schemas and test instances keep their own
licenses; [tests/data/instances/PROVENANCE.md](tests/data/instances/PROVENANCE.md)
lists the sources of the test instances.

## Citation

For research use, cite this software with [CITATION.cff](CITATION.cff) and record
the source commit, schema, compiler configuration, and any BPE artifact used.
