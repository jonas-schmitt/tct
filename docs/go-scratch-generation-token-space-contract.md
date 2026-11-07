# Go Scratch-Generation Token-Space Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This contract defines the generation boundary for a TCT-trained model that emits
new Go programs from scratch in token space. The contract is syntax-level: it
builds on parser-backed Go metadata, `GrammarIR`, `StructuralModel`, token
layout, and the raw syntax state machine. It does not use GoIR, ArenaIR,
route-handoff provenance, source-prefix ladders, or downstream BPE artifacts as
the source-validity authority.

## Boundary

Training and evidence start from source files. A checked Go source file is
parsed with `go/parser`, lowered into canonical syntax metadata, projected into
the structural syntax model, encoded as raw TCT syntax tokens, and replayed
through the raw syntax state machine. `gofmt`, render->reparse, and parser
round-trip checks are sidecar validation for that evidence path.

Inference starts from a precompiled Go syntax tokenizer artifact, not from a
source file and not from per-output parser metadata. The initial state is the
empty `GoSourceFile` prefix for that artifact. At each decoding step the runtime
asks the compiled state machine for admissible raw token IDs, masks model logits
to that finite set, applies the selected token transition, and repeats until EOF
is admitted and selected. The completed raw token stream decodes into a syntax
tree, renders to Go source, and may then be checked by `go/parser` and `gofmt`
as sidecar validation.

The generated token stream carries only syntax choices and payload tokens that
the precompiled artifact knows how to decode. Parser metadata is a training-time
and compiler-time input; it is not generated alongside each new program.

## Payload Policy

Identifiers, literals, comments, import paths, and other payload-bearing leaves
must be represented by the tokenizer artifact as token-space choices. Current
Go work may keep payload-byte or direct raw-token policies while the source
corpus is fixture-scale, but each policy must be explicit in the artifact
contract before it is used for scratch generation. BPE may wrap or compress an
already valid raw syntax-token stream, but BPE is not the syntax-validity
authority and must not bypass the raw admissible-token state.

## Required Evidence

Before claiming scratch-generation readiness for Go, maintained evidence must
show:

- artifact identity and version for the compiled Go syntax tokenizer,
- empty-prefix initial-state construction for `GoSourceFile`,
- finite admissible-token reporting for representative syntax and payload
  states,
- strict rejection for invalid raw tokens, PAD, malformed payloads, and
  incomplete prefixes,
- token-only decode into a reusable raw syntax tree,
- render->reparse sidecar validation for generated or replayed streams,
- stable or versioned token ID policy across artifact refreshes,
- focused validation commands for the changed syntax surface.

## Non-Goals

This contract does not claim type correctness, semantic usefulness, model
quality, whole-Go coverage before the gap runner proves it, paper-facing
evaluation results, or JSON Schema backend changes. C# remains the syntax-level
cross-target check for the shared state, transition, error, decode, and
admissible-token contracts.
