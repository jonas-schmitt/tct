# StructuralModel Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

`StructuralModel` is the proposed target-neutral bridge IR for future TCT
frontends. It is not implemented as a compiler path yet. This contract defines
what must be true before JSON Schema, grammar-derived frontends (Go, C#), or a
later justified additional frontend share compiler or runtime code.

`StructuralModel` is a working name. It may be renamed when route evidence
reveals a clearer contract name, but the intended layer is the shared
structural boundary below target-specific frontends and above token layout.

`StructuralModel` is the candidate shared structural backend boundary. It is
not a grammar representation and should not contain parser-rule machinery. A
grammar-derived route may lower through `GrammarIR`, but the shared tokenizer
backend should consume structural facts after grammar choices have been mapped
into records, variants, lists, optionals, finite domains, payloads, references,
renderer metadata, and validation hooks.

For source-language frontends, this syntax-level route is mandatory for the
multi-language backend. Target-specific arena IRs (legacy GoIR / C# arena) were
cleanup candidates and are not the stable backend boundary: they do not expose
grammar decisions directly enough for TCT-owned token availability.

## Purpose

The model should describe tokenizable structure without exposing frontend parse
trees or generated Rust source. It sits after target-specific frontend
validation and before token-layout planning:

```text
target frontend IR -> StructuralModel -> token layout -> generated codec
GrammarIR -> StructuralModel -> token layout -> generated codec
```

For JSON Schema, the existing discovery, analysis, and optimization passes are
still the source of truth. `StructuralModel` should be introduced only when it
can be populated from pass-owned metadata without changing supported JSON
Schema behavior.

## Required Node Kinds

The first useful model should cover these structural forms:

- Record: ordered named fields, required/optional policy, catch-all policy,
  validation hooks, and field-level renderer metadata.
- Variant: finite tagged choices with discriminator policy, payload shape, and
  strict versus recovery decode behavior.
- List: ordered repeated items with length encoding, min/max constraints, and
  optional separator/render policy.
- Map: key and value structure, key payload codec, deterministic ordering
  policy when required, and catch-all validation.
- Optional: presence state, default policy, and bit-packed grouping metadata
  when the token layout packs multiple optionals together.
- Finite domain: enums, booleans, literals, and const-like zero-token or
  discriminator-token decisions.
- Bounded integer: min/max, signedness, width/base codec, and strict digit
  validation policy.
- Payload: strings, bytes, numbers, comments, identifiers, raw literals, or
  frontend-specific leaf payloads with payload codec metadata.
- Reference: stable links to other model nodes, cycle/boxing requirements, and
  recursive decode constraints.

The model should not contain `proc_macro2::TokenStream`, generated Rust type
names as semantic identity, or generated-source substrings.

## Metadata Contract

Every node needs durable metadata:

- Stable model id and frontend-local path.
- Source provenance or schema path when available.
- Token-layout-affecting compiler inputs such as optimization profile,
  string/byte encoding, base vocabulary, BPE artifact fingerprints, and feature
  flags.
- Validation hooks that run outside generated hot paths when they are
  domain-specific.
- Renderer metadata for non-JSON targets: canonical spelling, separators,
  precedence, grouping, comments/whitespace policy, and toolchain check hooks.
- Compatibility metadata for schema evolution: generated order, token ranges,
  payload ranges, and whether changes are exact, additive, layout-only, or
  breaking.

Metadata must be structured and pass-owned. If a compatibility or wrapper
feature needs a fact, the frontend or compiler pass should emit that fact
directly instead of reconstructing it from generated source.

## Token Layout Contract

`StructuralModel` must preserve current vocabulary invariants:

- Every emitted raw token ID is below `VOCAB_SIZE`.
- `PAD_TOKEN`, base tokens, BPE merge IDs, structural token ranges, and wrapper
  tokens do not collide.
- Raw generated tokenizers stay content-only; sequence PAD/BOS/EOS and
  schema-decision tokens are wrapper-level.
- BPE is optional and payload-local. Merge-table compatibility and
  prefix-extension evidence are recorded separately from structural
  compatibility.
- Strict whole-stream decoding has a path to reject trailing tokens and invalid
  finite choices, even when recovery decode remains available.

## Frontend Responsibilities

Target-specific frontends remain responsible for:

- Parsing source or source-adjacent artifacts.
- Producing and validating a frontend IR.
- Rendering decoded IR back to canonical target output.
- Running domain tools such as `go/parser`, `go/types`, TypeScript compiler
  checks, ESLint, or Kubernetes validators.
- Recording functional evidence without moving those tools into generated hot
  paths.

`StructuralModel` should describe what TCT can tokenize and decode, not what a
language toolchain can prove.

## Introduction Gate

Do not implement a broad `StructuralModel` refactor until at least one of these
is true:

- A second frontend repeats the same runtime and token-layout needs as the
  first syntax-level target (today: Go plus C# `minimal_class`).
- A toy grammar-to-tokenizer path demonstrates encode/decode/render with
  token-range accounting.
- Schema evolution or bundle-wrapper work needs structured facts that the
  current JSON Schema pass metadata cannot provide cleanly.

The first code increment should be additive: create model data structures and
conversion/reporting tests without changing generated JSON Schema token output.

## Retired GoIR Residual Sidecars

The former GoIR `structural-model-v0` sidecar fixtures, GoIR rejection probes,
token-layout manifest under `tests/fixtures/go_syntax_sources/`, and helpers
`structural_model_goir_mapping.py` / `source_frontend_token_layout_manifest.py`
were retired with the GoIR residual investigation family. They are not
regeneration targets. Layout invariants live in
`docs/source-frontend-token-layout-manifest.md`. Live structural evidence is the
grammar-derived `StructuralModel` path below.

## GrammarIR Toy Lowering Evidence

The first grammar-derived lowering increment lives in
`src/grammar_ir_structural_model.rs`. It lowers the checked toy assignment
`GrammarIR` shape into `StructuralModel` v0 categories without changing JSON
Schema compilation, generated JSON Schema output, or production grammar
compiler paths.

The focused tests are:

- `tests/toy_grammar_frontend_test.rs`, which lowers the hand-written
  `toy_assignment_v1` grammar spec and checks records, variants, payload
  leaves, references, token ranges, renderer metadata, validation hooks, and
  boundary flags.
- `tests/tree_sitter_metadata_adapter_test.rs`, which lowers the checked
  Tree-sitter-style toy `node-types.json` fixture and the pinned
  tree-sitter-json **text**, tree-sitter-c, and tree-sitter-javascript grammar
  exports under `tests/fixtures/tree_sitter_metadata/{json,c,javascript}/`
  (metadata pin + adapter lower; C may exercise StructuralModel; JavaScript is
  GrammarIR-focused until anonymous-operator policy is designed), without a
  Tree-sitter runtime.
- `tests/tree_sitter_json_subset_codec_test.rs`, which projects a fixed
  null/bool/string/number construct subset of that pin through StructuralModel
  and the raw syntax state machine for syntax-level encode/decode +
  render/reparse smoke (`src/tree_sitter_json_subset_codec.rs`; claim boundary
  `syntax_level_tree_sitter_json_subset_only`).
- `tests/tree_sitter_c_subset_codec_test.rs`, which projects the minimal
  free-function construct subset of the tree-sitter-c pin through the same
  path (`src/tree_sitter_c_subset_codec.rs`; claim boundary
  `syntax_level_tree_sitter_c_subset_only`).
- `tests/csharp_syntax_metadata_adapter_test.rs`, which lowers the checked
  minimal C# syntax metadata fixture through the same path with source hash and
  `.NET`/Roslyn provenance while keeping C# arena fixtures as controls; the
  optional `scripts/analysis/csharp_syntax_metadata_smoke.py --self-test`
  reproduces that metadata projection from the C# source through Roslyn.
- `tests/go_syntax_metadata_adapter_test.rs`, which lowers the checked Go
  syntax metadata fixtures for the minimal function, local declarations and
  assignments, short variable declarations, typed/zero/integer declarations,
  scalar return literals and identifiers, and parenthesized/unary/binary/call
  return expressions through the same path with source hash and `go/parser` /
  `go/ast` provenance. The maintained `go_syntax_metadata_smoke.go` self-test
  and fixture-specific smoke commands in `DEVELOPMENT.md` and
  `scripts/README.md` reproduce those metadata projections from Go source
  through the Go parser.
- `tests/syntax_decision_comparison_test.rs`, which compares the Tree-sitter
  metadata, C# syntax metadata, and Go syntax metadata sidecars for shared
  rules, fields, repetitions, references, payload leaves, payload-boundary
  edges, token-range roles, provenance, and boundary flags.

This is backend-boundary selection input only. It proves grammar-derived
structural decisions and their decision graph can be represented explicitly, but
it does not promote `StructuralModel` into the core compiler, start production
`GrammarIR`, Tree-sitter, Roslyn, Go parser, or ANTLR paths.
The promotion strategy for this syntax-level route is maintained in
`docs/syntax-backend-gate-strategy.md`; its first concrete gate-wiring slice is
audit inventory before any production compiler routing.
The current boundary-selection decision is maintained in
`docs/structural-model-boundary-selection.md`, which keeps `StructuralModel` as
the candidate target-agnostic backend boundary, points to
`tests/syntax_structural_model_conformance_test.rs` as the maintained
conformance gate, records the versioned boundary manifest under
`tests/fixtures/syntax_structural_model/boundary-manifest-v1.json`, points to
`docs/syntax-structural-model-stable-gate-plan.md` as the manifest-backed
stable-gate plan, records that generated-code metadata consumes that manifest,
records the sidecar stable-gate helper, points to
`reports/m6-cross-target-contract-closure-2026-07-21.md`, records the
production-boundary contract fixture under
`tests/fixtures/syntax_backend/production-boundary-contract-v1.json`, records
the production generator API boundary in `src/syntax_backend_boundary.rs` with
focused coverage in `tests/syntax_backend_boundary_test.rs`, records the
generated artifact envelope fixture under
`tests/fixtures/syntax_backend/generated-artifact-envelope-v1.json` with
validation support in `src/syntax_artifact_envelope.rs`, records that
generated-code metadata carries that envelope identity for toy, C#, and Go,
records the validation-hook contract fixture under
`tests/fixtures/syntax_backend/validation-hook-contract-v1.json` with
validation support in `src/syntax_validation_hooks.rs`, records that
generated-code metadata carries the validation-hook contract handoff, records
the input-validation diagnostics contract fixture under
`tests/fixtures/syntax_backend/input-validation-diagnostics-v1.json` with
validation support in `src/syntax_input_diagnostics.rs`, records that
generated-code metadata carries the input diagnostics contract handoff, and
records the input-validation negative fixture gate under
`tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json`. The
generated-code gate now hands off that negative fixture identity through
volatile metadata, including direct parser-backed C# and Go negative fixture
coverage. It also records the validation-hook sidecar evidence fixture under
`tests/fixtures/syntax_backend/validation-hook-sidecar-evidence-v1.json` and
hands that identity through toy, C#, and Go generated-code metadata. It also
records `tests/fixtures/syntax_backend/generated-artifact-replay-v1.json` as
the syntax generated artifact replay fixture with validation support in
`src/syntax_reproducibility_replay.rs`. The refreshed production routing review
clears the next additive slice: move the generated syntax codec renderer and
artifact builder behind a library-owned sidecar API before any core compiler
integration.
