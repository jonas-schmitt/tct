# Source Program Completion Semantics

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This document defines the source-program prefix and gapped-completion boundary
for non-JSON frontend work. It is a contract and roadmap decision, not an
implementation of a completion API. Historical GoIR examples are provenance
only; residual GoIR fixtures and operational helpers were retired. Forward
source-language work is syntax/grammar-level (Go and C# `minimal_class` on the
shared raw syntax-state-machine path).

## Decision

TCT does not currently claim source-program completion for any non-JSON
frontend. Complete-document encode/decode is the supported unit:

```text
complete source -> parser-backed syntax metadata -> StructuralModel ->
token layout -> raw syntax tokens -> decode/render -> canonical source
```

In particular:

- Existing generated `decode_prefix` behavior is left-prefix truncation over
  raw schema-local token streams after a complete instance has been encoded.
- That left-prefix behavior is not source text parser recovery, not source
  prefix parsing, and not suffix-constrained gap infill.
- Raw generated tokenizers do not allocate cursor, hole, suffix, BOS, EOS, or
  schema-decision tokens in their schema-local `VOCAB_SIZE`.
- `StructuralModel` sidecars and token-layout records may note completion
  boundaries as metadata, but they do not define completion behavior by
  themselves.
- Forward syntax/grammar work uses `GrammarIR -> StructuralModel`; completion
  claims still require a concrete completion API with parser-state evidence.

The retired GoIR control route used complete GoIR JSON documents as the
tokenized unit and likewise made no source-program completion claim.

## Completion API Boundary

True source-program completion belongs in a future versioned completion API or
sequence wrapper outside raw generated tokenizers. That API must own:

- The source prefix and optional source suffix.
- The cursor or gap span.
- Whether the request is left-prefix only or suffix-constrained.
- The target frontend parser or recovery state.
- The rendered candidate validation policy.
- Consumed-token and partial-result accounting.
- Any model-facing wrapper tokens.

Generated syntax tokenizers should continue to encode and decode complete
admissible streams for supported programs. They should not shell out to
`go/parser`, infer parser state from a truncated token stream, or accept
placeholder gap tokens as if they were normal payload.

## Current Semantics

| Scenario | Current status | Meaning |
| --- | --- | --- |
| Complete supported source through syntax codec | Supported by syntax gates | Parser-backed metadata, generated-codec round trip, render, and optional Go/C# tool checks stay outside generated hot paths where applicable. |
| Complete raw syntax token stream | Supported by generated codec evidence | Strict decode must consume the complete stream with documented consumed-token accounting. |
| Raw token left-prefix over a complete encoded instance | Existing generated streaming mode only | Truncation-tolerant raw token decoding; not a source-program completion API. |
| Incomplete source prefix | Not a TCT input claim | The target frontend must classify parser recovery before TCT can claim completion behavior. |
| Source suffix or cursor gap | Not supported | Requires a suffix-constrained completion envelope and fixtures before any claim. |
| BPE prefix streaming | Not implied | BPE must record separate prefix-extension or streaming evidence. |

This means the maintained source routes have no source-program completion
claim and no gapped infill claim. They can still prepare those claims by
preserving full-source route evidence, token-layout metadata, renderer
metadata, and explicit failure examples.

## Required Evidence Before Claims

Before TCT claims source-program completion beyond raw left-prefix truncation,
add focused evidence for:

- A complete minimal source fixture that round trips through source, syntax
  metadata, generated TCT tokens, rendered source, and parser or formatter
  validation.
- A parser-error fixture that records how target frontend failures are reported
  before token encoding.
- An incomplete source prefix fixture that proves whether recovery state is
  available or explicitly unavailable.
- A suffix-constrained gap fixture with prefix, suffix, cursor span, and the
  expected ownership of parser state.
- A raw token prefix fixture that stays clearly separate from source prefix
  behavior.
- BPE/no-BPE evidence when a completion claim crosses BPE boundaries.

Until those fixtures exist, use the phrase `no source-program completion claim`
for syntax-route reports and downstream handoffs.

## Next Task

The next local implementation task is not a completion API. Prefer syntax-level
generator-core gaps, M6 cross-target contract closure, or audit cleanup of
remaining obsolete surfaces. Completion work should stay report-only until a
TODO item names a concrete prefix or gapped-completion fixture and API owner.
