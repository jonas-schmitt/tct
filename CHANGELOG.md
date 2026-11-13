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

### Changed

- Map entries are encoded in one canonical order: ascending key order (UTF-8
  byte order). This covers typed `additionalProperties` maps,
  `patternProperties` maps, catch-all maps, and objects inside untyped
  `serde_json::Value` members, in generated tokenizers and in the library
  runtime. Before, entries followed hash order, so the same value could encode
  to different streams depending on the key order of the input. Streams change
  for maps and untyped objects with two or more entries whose previous order
  was not already ascending. Token IDs, token ranges, and `VOCAB_SIZE` do not
  change, and decode still accepts entries in any order, so existing streams
  still decode to the same values. Merge tables or models trained on streams
  from earlier builds saw the old entry order. This includes the BPE merge
  tables shipped under `schemas/bpe`: they were trained on streams in the old
  order, all except `package-tct-bpe-500` no longer reproduce from current
  source, and they must be retrained (and
  `third_party/clearance/bpe-artifacts.json` refreshed) before a release. The
  library impl `JsonTokenEncoder for FxHashMap<K, V>` now requires `K: Ord`
  instead of `K: Eq`; generated tokenizers use `String` keys and are not
  affected.
- With `--decimal-numbers` (Python: `decimal_numbers=True`), floats with
  `|x| >= 1e21` or `0 < |x| < 1e-6` are written in exponent form (`1e21`,
  `1e-7`, `5e-324`, `1.7976931348623157e308`) instead of plain decimal text.
  Before, such values were written as long runs of zeros (326 characters,
  327 tokens with the end marker, for `5e-324`), and `token_count()` matched
  that length. Floats that are zero or in `1e-6 <= |x| < 1e21` keep their
  previous text, and integers are unchanged. Streams change only for floats
  outside that range; decode accepts both forms, so streams from earlier
  builds still decode to the same values. Merge tables or models trained on
  decimal-mode streams from earlier builds saw the old form of those values.
  `tct-token-diff` does not model `--decimal-numbers` and does not report
  this change.

### Fixed

- JSON floats are parsed correctly rounded. Generated crates and wheels, and
  the `tct` crate itself, now enable serde_json's `float_roundtrip` feature.
  Before, serde_json's default float parser could round a decimal to a
  neighbouring double (for example `1.23e25` was parsed as
  `1.2299999999999998e25`), so JSON -> tokens -> JSON changed the value even
  though the encoder is lossless. Decimal inputs that were parsed wrongly now
  encode to a different, correct value, in the default 8-byte encoding and in
  `--decimal-numbers` text; all other values and all token IDs are unchanged,
  and existing streams decode as before. Crates that compile generated code
  themselves should add the feature to their `serde_json` dependency (see the
  README).
- Strict decode rejects map content that encode never emits. A key repeated
  in one map (typed `additionalProperties` maps, `patternProperties` maps,
  catch-all maps, and objects inside untyped `serde_json::Value` members) now
  fails `decode_strict()` with `InvalidData` naming the key and entry, for
  example `duplicate map key "k" at entry 2 of 2`. Before, the later entry
  silently replaced the earlier one, so the decoded map had fewer entries than
  the stream declared. Recovery decode still keeps the last entry. A catch-all
  key equal to a declared property name (for example `id` when `id` is a
  declared property) is now rejected by both `decode()` and `decode_strict()`;
  before, the value decoded and then serialized with that property twice.
  Encoded streams do not change, and every stream that encode produces still
  decodes. The library impl `JsonTokenEncoder for FxHashMap<K, V>` now also
  requires `K: Debug`; generated tokenizers use `String` keys and are not
  affected.
