# Non-JSON Runtime Boundary

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This note defines the first shared runtime boundary for non-JSON frontends. It
is a design contract, not permission to refactor the JSON Schema compiler path
before another frontend proves the shared shape.

## Scope

The forward source-language route is syntax/grammar-level: parser-backed
metadata lowers through `GrammarIR -> StructuralModel -> token layout ->
generated codec`. That route owns validity-by-construction evidence for Go,
C#, and later source-language targets.

The legacy GoIR bridge lived outside the JSON Schema compiler (Go source to
arena-indexed GoIR JSON, TCT encode/decode, render, optional Go tooling) as
control and migration evidence only. Residual GoIR fixtures and operational
helpers were retired; do not expand or regenerate that route. Future frontends
should keep parser, renderer, formatter, and validator work separated from
generated hot paths and use the syntax-level structural route already justified
for Go and C# `minimal_class`.

The first shared runtime boundary is below frontend parsing and above
model-facing sequence wrappers:

```text
frontend source or artifact
  -> target-specific IR and validator
  -> structural token codec over schema-local u32 tokens
  -> optional BPE over schema-local payload tokens
  -> optional sequence or bundle wrapper for model-facing BOS/EOS/schema tokens
```

Raw generated tokenizers stay content-only. BOS, EOS, schema-decision tokens,
padding, prompt label masking, and generation masks remain wrapper concerns.

## Shared Runtime Pieces

Only these pieces should become target-neutral first:

- Token codec trait: a small encode/decode contract over schema-local `u32`
  streams. It should report consumed tokens and distinguish complete,
  truncated, and invalid streams.
- Decode errors: typed errors with token position, expected range or domain,
  target path, and strict versus recovery mode. Errors must not hide modulo
  normalization or partial consumption.
- Buffered token cursor: a reusable cursor for lookahead, consumed-token
  accounting, EOF context, and future prefix or gap parser state.
- Variable-base helpers: bounded integer, enum codec, collection length, and
  presence-mask helpers that operate on explicit base/vocabulary parameters.
- String and byte payload codecs: UTF-8 byte payload handling, end markers,
  malformed UTF-8 errors, and future byte-oriented language payloads.
- BPE wrapper: optional compression around raw schema-local token streams,
  with merge-table provenance and vocabulary compatibility checks outside the
  structural encoder.
- Token layout metadata: `BASE_VOCAB_SIZE`, `VOCAB_SIZE`, `PAD_TOKEN`, raw
  payload ranges, BPE ranges, wrapper offsets, and collision checks.

The durable source-frontend layout contract is
`docs/source-frontend-token-layout-manifest.md`. GoIR-era checked layout
fixtures were retired; the contract remains report-level guidance for
syntax-level layouts and does not reopen shared frontend runtime code by itself.
Source-program completion semantics are defined in
`docs/source-program-completion-semantics.md`; neither the retired GoIR route
nor the forward syntax path has a source-program completion or gapped infill
claim beyond raw left-prefix token truncation evidence.

These are runtime contracts. Frontend-specific parse trees, render policies,
type-checking hooks, and canonical formatting stay in target-specific modules
until `StructuralModel` has a tested reason to carry shared structural metadata,
or until a `GrammarIR` adapter has a tested reason to lower grammar facts into
`StructuralModel`.

## StructuralModel And GrammarIR

`StructuralModel` and `GrammarIR` are current working names, not final API or
crate names. Future evidence may justify clearer terminology. Rename them when
that improves the contract, but preserve the layer separation described here.

Do not treat `StructuralModel` and `GrammarIR` as interchangeable backend
candidates.

`StructuralModel` is the candidate target-agnostic structural backend boundary.
It should describe tokenizable records, variants, lists, optionals, finite
domains, bounded integers, payloads, references, renderer metadata, and
validation hooks after target-specific parsing and IR validation. If TCT grows
a stable multi-language backend, this is the layer that must carry the shared
tokenization contract before token-layout planning and generated codec
execution.

`GrammarIR` is upstream of that boundary. It describes grammar-facing concepts:
terminals, nonterminals, choices, repetition, precedence, payload terminals,
render policy, and semantic hooks. It is useful when grammar decisions need to
drive token availability or when parser metadata should be normalized before
lowering. A `GrammarIR` route must lower into `StructuralModel` or another
explicit structural boundary before it becomes part of the shared tokenizer
backend.

Adapter selection into `GrammarIR` is owned by
[`docs/grammar-ir-contract.md#adapter-choice`](grammar-ir-contract.md#adapter-choice):
use language-specific parser metadata first for Go/C#, keep Tree-sitter as the
reusable metadata path, and defer ANTLR until a target-specific spike justifies
it.

For source-language frontends, this syntax-level route is required before a
multi-language backend can be promoted. Target-specific arena IRs are not the
long-term backend path and residual GoIR/C# arena operational surfaces were
retired.

The route from Go is target-specific syntax evidence: parser-backed metadata
lowers through `GrammarIR -> StructuralModel -> token layout -> generated
codec`. Retired GoIR material is not a prerequisite for syntax backend
promotion and is not the stable backend.

## Error And Validation Contract

Encoding is canonical and must reject values the frontend validator cannot
represent. Decoding may have recovery modes, but strict validation paths must
reject invalid finite choices, malformed strings or byte payloads, impossible
variant states, EOF, and trailing tokens when whole-stream validation is
requested.

Every non-JSON frontend must define three validation layers:

- IR validity: the target-specific IR has valid references, spans, edge bounds,
  and frontend invariants.
- Token round-trip validity: IR -> tokens -> IR preserves semantic data and
  consumes the expected token span.
- Functional validity: an external tool accepts or classifies the rendered
  artifact. This remains sidecar evidence, not generated-tokenizer behavior.

## BPE And Sequence Wrappers

BPE remains payload-local. A frontend should first prove raw structural
round-trip behavior, then add BPE evidence with merge-table fingerprints and
base-vocabulary compatibility. BPE prefix-extension evidence is useful for
artifact migration, but it is not proof that model behavior or encoded
sequences are unchanged.

Sequence wrappers remain model-facing. Multi-schema bundles should wrap local
payloads with explicit PAD/BOS/EOS and schema-decision metadata rather than
putting these tokens into raw generated tokenizers. Raw `PAD_TOKEN` is not a
payload token.

## Implementation Order

Use this order for future non-JSON implementation:

1. Keep target-specific parser, IR validator, renderer, and toolchain checks
   outside the JSON Schema compiler.
2. Add fixture and evidence gates for IR validity, generated TCT round trips,
   token-range accounting, rendering, and optional external parser checks.
3. Extract shared runtime helpers only when the second frontend repeats the
   same runtime need.
4. Define `StructuralModel` before broad compiler refactors. It should model
   records, variants, lists, optionals, finite domains, bounded integers,
   payloads, references, invariants, and renderer metadata.
5. Define `GrammarIR` only after `StructuralModel` or a toy grammar proves the
   shared interface for rules, terminals, nonterminals, choices, repetition,
   precedence, payload terminals, render policy, and semantic hooks.

Do not infer structural metadata from generated source strings. Shared
metadata must be owned by frontend lowering, compiler passes, or generated
manifest structures.
