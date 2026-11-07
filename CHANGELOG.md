# Changelog

All notable changes to TCT are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

Initial source release candidate of the TCT compiler core, licensed under
Apache-2.0.

### Added

- JSON Schema to tokenizer compiler with a four-pass architecture (discovery,
  analysis, optimization, generation) and optimization profiles `O0`, `O2`,
  `O3`, `Os`, and `OVocab`.
- Generated Rust tokenizers with recovery, strict, full, and prefix decode
  APIs, bounded-integer and enum encodings, end-marker string encoding, and
  optional bit-packed optional-field presence.
- Python wheel generation (`--target python`) with `encode`, `decode`,
  streaming `decode_prefix`, versioned sequence wrappers, and multi-schema
  bundle wrappers.
- Python module `tct` (distribution `tct`) that exposes the compiler:
  `tct.compile` returns generated Rust source with the options of the command
  line, `tct.analyze` returns the vocabulary analysis, and `tct.CompileError`
  reports rejected schemas and options. The BPE helper classes remain.
- BPE training and measurement binaries (`tct-prepare-data`, `tct-train-bpe`,
  `tct-measure-tokens`, `tct-analyze-usage`, `tct-bpe-lookup-benchmark`,
  `tct-bpe-length-stats`); `tct-train-bpe` writes provenance sidecars for the
  merge tables it produces.
- Schema evolution reports (`tct-token-diff`) with conservative compatibility
  classes.
- Round-trip, target-schema, generated-crate reproducibility, wheel smoke, and
  quality gates under `scripts/testing/`.
- Experimental (behind the `experimental` feature): non-JSON frontends with
  no stability guarantees, off by default. They comprise a SQL schema-linking
  frontend with Python bindings (with `python`), a syntax/grammar layer
  (`GrammarIR`, `StructuralModel`, token layout, raw syntax state machine,
  generated codec) with Go and C# syntax targets, tree-sitter subset codecs,
  and an exact-mask contract, plus their CLI binaries and tests. See
  `docs/frontends-development.md`.
