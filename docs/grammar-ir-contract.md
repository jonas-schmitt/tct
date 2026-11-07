# GrammarIR And Decision Graph Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

`GrammarIR` is the proposed normalized grammar input for future non-JSON TCT
frontends. It is the required syntax-level route for multi-language source work;
the checked toy grammar and Tree-sitter metadata probes are the first evidence
that grammar-derived structure is worth sharing.

`GrammarIR` is a working name. It may be renamed when grammar-ingestion
evidence reveals a clearer contract name, but the intended layer remains
upstream of the shared structural backend boundary.

`GrammarIR` is not the stable tokenizer backend. It is the mandatory upstream
grammar-ingestion and decision-graph layer for multi-language source work. Its
job is to normalize grammar facts and then lower them into `StructuralModel` or
another explicit structural boundary before token layout and generated codec
execution.

## Purpose

`GrammarIR` should describe parseable syntax and rendering policy in a way that
can lower into `StructuralModel`. It is not a replacement for frontend-specific
parsers, type checkers, or validators.

```text
Tree-sitter, ANTLR, hand-written toy grammar, or target parser metadata
  -> GrammarIR
  -> StructuralModel
  -> token layout and generated codec
```

The first implementation should support a tiny language before Go,
TypeScript, JavaScript, Tree-sitter, or ANTLR adapters depend on it.

## Adapter Choice

Adapters are bridges into `GrammarIR`, not stable backend boundaries. The
current preference is:

- Use language-specific parser metadata first for Go and C# because `go/parser`
  / `go/ast` and Roslyn expose mature syntax facts with source-tool
  provenance.
- For Go, treat the pinned `go/ast` parser output shape as the whole-language
  metadata target before attempting textual spec-EBNF ingestion. `go/parser`
  acceptance, AST node structs, tokenizable exported fields, interface
  alternatives, finite token domains, precedence/render policy, and
  comment/trivia policy must be normalized before they lower into shared
  `GrammarIR`; the Go spec grammar is explanatory context until a
  parser-aligned adapter proves equivalence.
- Keep Tree-sitter as the reusable metadata path for targets where native
  parser APIs are too heavy, unavailable, or less useful than a grammar-derived
  node/type inventory.
- Defer ANTLR until a concrete target needs it and a spike proves that grammar
  actions, predicates, runtime assumptions, precedence, payload classification,
  and render policy can be normalized without target-language leakage.

Every adapter must lower into the same `GrammarIR` and decision-graph contract,
then into `StructuralModel` or another explicit structural boundary before
token layout or generated-code execution. Target-specific arena IRs may stay as
control and migration evidence, but they are not adapter substitutes for the
syntax-level route.

## Required Concepts

The initial IR needs these concepts:

- Terminals: structural literals, keywords, identifiers, string/byte literals,
  numeric literals, comments, and whitespace policy.
- Nonterminals: named grammar symbols with frontend provenance and optional
  renderer/type metadata.
- Choices: finite alternatives that can become TCT variants or decision tokens.
- Optional forms: absent/present state with explicit default and rendering
  policy.
- Repetition forms: zero-or-more, one-or-more, bounded repetition, separator
  lists, and trailing-separator policy.
- Precedence and associativity: enough metadata to render or reject ambiguous
  expression trees.
- Payload terminals: leaves whose value is encoded by string, byte, numeric,
  identifier, regex, or target-specific payload codecs.
- Semantic hooks: external validators, type-checking sidecars, or renderer
  constraints that are not part of generated tokenizer hot paths.

## Decision Graph

The decision graph is the tokenization-facing view of `GrammarIR`. It should be
explicit about where a model chooses structure versus emits payload:

- Decision nodes select among grammar alternatives, optional states,
  repetitions, or finite domains.
- Payload nodes encode identifiers, literals, comments, or raw byte/string
  leaves using payload codecs.
- Render nodes preserve required structural tokens or canonical spelling.
- Validation nodes record sidecar checks or static constraints that must be
  satisfied before evidence is trusted.

Every graph edge should carry enough context for decode errors and generation
masks: owner rule, source grammar path, expected token range, strict versus
recovery mode, and whether the edge crosses a payload boundary.

The current checked lowering emits this graph in the report-only
`GrammarIR -> StructuralModel` sidecar. Decision nodes cover grammar rules,
fields, reference alternatives, and payload alternatives; edges preserve owner
rule/field context and mark payload-boundary crossings; token ranges link syntax
decisions to structural, UTF-8 byte, and integer payload ranges.

## Adapter Requirements

Tree-sitter and ANTLR adapters should not be chosen before a spike records the
tradeoffs. Any adapter must provide:

- Grammar source and version provenance.
- Node/type metadata or normalized rule metadata.
- Ambiguity and precedence representation.
- Payload terminal classification.
- Render policy for separators, grouping, whitespace, comments, and
  punctuation.
- Licensing and generated-parser footprint notes.
- A fixture proving parse -> IR -> render -> parse stability for the supported
  subset.

Target-specific adapters may reject unsupported constructs early. Unsupported
syntax should be reported with rule/path context rather than silently lowered
to a lossy payload.

## Interaction With StructuralModel

`GrammarIR` lowers into `StructuralModel` only after grammar choices are
resolved into records, variants, lists, optionals, finite domains, bounded
integers, payloads, references, and renderer metadata. The lowering step owns
the semantic mapping; generated Rust code should not infer grammar meaning from
source strings or parser node names.

The first useful tests should assert:

- Round-trip parse/render stability for a toy grammar fixture.
- Token encode/decode round-trip for the lowered structural model.
- Vocabulary and token-range accounting for structural decisions and payloads.
- Rejection of unsupported constructs with actionable rule/path errors.
- Separation between raw structural tokens, optional BPE, and sequence wrapper
  BOS/EOS/schema tokens.

## First Toy Grammar Probe

The checked-in `toy_assignment_v1` probe lives in
`src/toy_grammar_frontend.rs` and is covered by
`tests/toy_grammar_frontend_test.rs`. It is intentionally hand-written and
toolchain-free. The supported language is a tiny sequence of assignments:

```text
let name = 7;
let enabled = true;
let message = "hello";
```

The probe proves a narrow grammar-tokenizer boundary before any Tree-sitter,
ANTLR, Go, TypeScript, or JavaScript adapter depends on it:

- source parse with rule/position errors for unsupported constructs;
- schema-local token encode/decode with explicit structural, UTF-8 byte, u8
  integer, PAD, and vocabulary ranges;
- strict decode rejection for malformed token streams and trailing PAD;
- canonical render followed by parse stability.

It is not a production frontend and does not touch `TctCompiler` or generated
JSON Schema tokenizers. Future `GrammarIR` work should treat it as a boundary
fixture for deciding which pieces are worth generalizing.

`ToyGrammarSpec::to_grammar_ir` now exposes the hand-written toy grammar as
the minimal checked `GrammarIR` input. The additive lowering in
`src/grammar_ir_structural_model.rs` maps that input into StructuralModel-style
records, variants, lists, payload leaves, token ranges, renderer metadata, and
validation hooks, plus the explicit decision graph described above, while
keeping the JSON Schema compiler and production grammar compiler paths
unchanged.

The first metadata adapters are similarly narrow:
`src/tree_sitter_metadata.rs` loads checked-in Tree-sitter-style
`node-types.json` metadata and lowers named nodes, field maps, field-less
`children` blocks, and `subtypes` into the minimal `src/grammar_ir.rs` shape.
The toy fixture under `tests/fixtures/tree_sitter_metadata/toy_assignment/`
proves field requiredness, multiplicity, payload terminal classification,
provenance hashes, and error context for unsupported empty alternatives. The
pinned real-grammar fixtures under `tests/fixtures/tree_sitter_metadata/json/`
(tree-sitter-json), `tests/fixtures/tree_sitter_metadata/c/` (tree-sitter-c),
and `tests/fixtures/tree_sitter_metadata/javascript/` (tree-sitter-javascript)
record package/version/license/sha256 and exercise supertype, children, and
named-field lowering across three grammars. The C pin also proves referenced
anonymous operator leaves materialize so StructuralModel nonterminal refs
resolve; the JavaScript pin is GrammarIR-lower evidence (full-pin StructuralModel
may remain limited on anonymous operator kinds). These paths are metadata-only;
they do not embed or run a Tree-sitter parser and do not claim C/JS codecs.
The fixed null/bool/string/number subset codec in
`src/tree_sitter_json_subset_codec.rs` projects the JSON pin into a construct
subset for raw SM encode/decode + render/reparse smoke
(`tests/tree_sitter_json_subset_codec_test.rs`; syntax-level claim only).
The minimal free-function C subset codec in
`src/tree_sitter_c_subset_codec.rs` is the first non-JSON instantiation
(`tree_sitter_c_minimal_function_subset_v0`;
`tests/tree_sitter_c_subset_codec_test.rs`; claim
`syntax_level_tree_sitter_c_subset_only` — not full C). Shared pin→SM→
multi-alt / report / inventory helpers live in
`src/tree_sitter_subset_framework.rs` (CPU-2); language projection stays local.

`src/csharp_syntax_metadata.rs` loads the checked C# syntax metadata fixtures
under `tests/fixtures/csharp_syntax_metadata/` and lowers Roslyn-style syntax
rules into the same `GrammarIR` shape. The fixtures record source file hashes
plus `.NET`/Roslyn metadata provenance, then prove class, namespace, property,
method, block, local declaration, expression statement, assignment expression,
predefined-type, identifier, and string-literal decisions can enter the
decision-graph sidecar without changing generated JSON Schema output or the
production grammar compiler path. They are metadata-only in Rust tests, while
`scripts/analysis/csharp_syntax_metadata_smoke.py --self-test` uses a temporary
Roslyn project to reproduce the checked metadata projection from source with
`Microsoft.CodeAnalysis.CSharp` package provenance.

`src/go_syntax_metadata.rs` loads the checked Go syntax metadata fixtures under
`tests/fixtures/go_syntax_metadata/` for the minimal function, local
declarations and assignments, short variable declarations, typed/zero/integer
declarations, scalar return literals and identifiers, and
parenthesized/unary/binary/call return expressions, then lowers `go/parser` /
`go/ast` syntax rules into the same `GrammarIR` shape. The fixtures record
source file hashes plus Go parser provenance, then prove package, function,
field-list, block, return, local declaration, typed and zero-value typed value
specs, assignment, short variable declaration, identifier, string-literal
payload shapes, and integer-literal payload shapes can enter the same
decision-graph sidecar without changing generated JSON Schema output or the
production compiler path. They are metadata-only in Rust tests, while the
maintained `go_syntax_metadata_smoke.go` self-test and fixture-specific smoke
commands in `DEVELOPMENT.md` and `scripts/README.md` reproduce the checked
metadata projections from source with `go/parser` provenance.

`tests/syntax_decision_comparison_test.rs` compares the checked Tree-sitter-style
toy metadata, checked C# syntax metadata, and checked Go syntax metadata after
they lower through `GrammarIR -> StructuralModel`. It asserts the shared
target-agnostic categories needed by a multi-language backend: rules, fields,
repetitions, references, payload leaves, payload-boundary edges, token-range
roles, input provenance, and unchanged JSON Schema/compiler boundary flags.

`docs/syntax-backend-gate-strategy.md` defines the JSON-backend-grade gate
categories that must exist before this syntax-level route can become a stable
backend. The first gate-wiring slice is audit inventory, not production
compiler routing.

## Non-Goals

Do not use `GrammarIR` to claim paper-facing non-JSON results by itself. It is
a compiler interface. Evidence still needs fixtures, parser/toolchain checks,
tokenizer reports, BPE provenance, and downstream evaluation artifacts when
model behavior or functional usefulness is being claimed.
