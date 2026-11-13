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
- Members whose `type` is an array of JSON types (`["string", "null"]`,
  `["object", "null"]`, `["integer", "null"]`, `["string", "boolean"]`, ...)
  compile to a generated enum with one variant per listed type, the
  representation a definition with a `type` array already had. This applies
  to properties, array items, and map values. Before, such a member became an
  untyped `serde_json::Value`: its declared properties, item schemas, enum
  values, formats, and integer bounds were ignored, its content was encoded
  generically (object keys and enum values spelled as string bytes), and
  values of the wrong type were accepted. Each variant applies the keywords
  of its type, and `null` is a unit variant, so a required
  `["integer", "null"]` member still accepts null and an optional one keeps
  the absent, null, and present states. A non-empty `enum` keeps, per
  variant, the values of that variant's type (`["string", "null"]` with
  `"enum": ["a", null]` is a string enum of `a` plus a null variant). This
  also applies to definitions with a `type` array and an `enum`: before, a
  mixed enum was ignored (the string variant accepted any string) and an
  all-string enum was copied into every variant. A one-element `type` array
  is the plain type. Instances whose value in such a member has a type the
  array does not list, a string outside its `enum`, or an object that breaks
  the declared properties are now rejected when deserializing. Integers with
  both `minimum` and `maximum` use the bounded-integer encoding; as for other
  bounded integers, an out-of-range value is not rejected when deserializing.
  A `type` array with an unknown type name or without any type name, and one
  whose non-empty `enum` holds no value of a listed type, is a compile error
  with a schema path; non-string entries are skipped as before. The stream of
  every instance that uses such a member changes, and streams from earlier
  builds are not valid streams for those members. Among the shipped target
  schemas the generated tokenizer changes for tsconfig, package,
  github-workflow, jest, babelrc, cypress, codecov, and appsettings, with
  unchanged token ranges and `VOCAB_SIZE`; treat each as a `breaking_change`
  for downstream artifacts. (`tct-token-diff` compiles both sides with one
  compiler. Applying its rules to the layouts of the previous and the current
  compiler gives `breaking_change` for tsconfig, jest, babelrc, and cypress,
  and `layout_only_unchanged` for package, github-workflow, codecov, and
  appsettings, whose changed members are outside the root-level structural
  token map: nested, or at an opaque root.) The other eight target schemas
  generate identical code. The merge tables
  `schemas/bpe/package-tct-bpe-500.json`,
  `schemas/bpe/github-workflow-tct-bpe-500.json`,
  `schemas/bpe/tsconfig-tct-bpe-500.json`, and
  `schemas/bpe/tsconfig-tct-bpe-1k.json` were trained on the old streams and
  must be retrained.
- An `allOf` subschema whose `type` is `["object", "null"]` merges like an
  object subschema; before, it was rejected as an unsupported `allOf` shape.
  The object case is merged exactly, and whether the member accepts null is
  decided by the containing schema, as for other `allOf` members. tsconfig
  needs this once its `ts-node` options are typed: `ts-node.compilerOptions`
  reuses the nullable `compilerOptions` object through `allOf`.

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
- Documentation corrections; no behaviour, token, or stream changes. The
  README has a "Number Encoding" section (with a glossary entry) that states
  the default: a `number` is one type-marker token plus 8 little-endian byte
  tokens, an `integer` without both bounds is one signed/unsigned marker plus
  8 bytes, bounded integers use the offset encoding at `-O2`, `-O3`, `-Os`,
  and `-OVocab`, and `--decimal-numbers` replaces the 8 bytes with decimal
  text plus `END_STRING_MARKER`. The Hugging Face contract, `ARCHITECTURE.md`,
  `DEVELOPMENT.md`, and the rustdoc of `SequenceGenerationMask` state that the
  bundle and direct-profile generation mask is stateless and admits EOS at
  every step, so a decoder that needs complete outputs must admit EOS only
  once `strict_prefix_roles` reports the payload complete. The github-workflow
  example no longer says that field names are never strings (keys of
  free-form maps are spelled as string bytes), and its tiktoken comparison
  reports fewer or more tokens instead of "better" or "worse". The SQL
  frontend contract describes the menu-index stream as shorter, makes no
  model-accuracy claim, and lists `UNION`, `INTERSECT`, and `EXCEPT` as out
  of scope for v0.
