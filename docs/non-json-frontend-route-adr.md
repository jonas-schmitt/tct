# Non-JSON Frontend Route ADR

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Status: Accepted as historical/control evidence; superseded for forward
roadmap work by the research strategy notes and the research roadmap.

Supersession note: the current TCT north star is syntax/grammar-level Go
tokenizer generation: parser-backed metadata -> `GrammarIR` ->
`StructuralModel` -> token layout -> raw syntax state machine -> generated
codec/tokenizer. GoIR and ArenaIR routes are historical context and cleanup
candidates unless a current retained test proves direct utility for JSON Schema
TCT, Go syntax TCT, C# syntax TCT, or shared raw syntax-state-machine
infrastructure. Do not use this ADR to prioritize new GoIR implementation work
or to make source-language validity-by-construction claims.

This ADR records the historical accepted route for the first durable non-JSON
frontend evidence path. It is compiler-route provenance only. Residual GoIR
operational surfaces (route preflight/evidence helpers, structural-model
sidecars, token-layout manifests, lexer-baseline packaging, and the
`route_source_manifest` `goir` target) were retired; do not regenerate them.
Forward source-language work is owned by the syntax-level route in
the research strategy notes. This ADR does not reopen other language frontends
or a shared frontend abstraction.

## Context

TCT already has strong JSON Schema compiler, generated-tokenizer, validation,
and BPE contracts. Non-JSON frontend work should reuse those contracts without
turning the JSON Schema compiler into a parser framework before evidence proves
the shared shape.

The historical low-conflict hypothesis was the GoIR bridge: parse Go source
with external Go tooling, lower it to arena-indexed GoIR JSON, validate that
JSON against a GoIR schema, run the existing TCT compiler and generated
tokenizer, decode back to GoIR, render canonical Go, and reparse or format with
Go tools when available. That path proved parser/render/token-range control
evidence but does not satisfy syntax-level validity-by-construction. A later
route decision for the forward path required parser-backed syntax metadata,
`GrammarIR`, token layout, and generated-codec gates instead.

Historical target-selection reports provisionally pointed to GoIR as the
starting control hypothesis because it already had compiler-local TCT evidence.
They also kept C#, Java, and TypeScript/JavaScript in the first-target
comparison. Forward policy now demotes GoIR/ArenaIR and out-of-scope frontends:
C# remains only the cheapest cross-target check (`minimal_class`); Java and
TypeScript stay out of scope until the project explicitly promotes them.

## Layering Options

### JSON-Schema-backed arena IR

Lower frontend parser output into a target-specific, arena-indexed JSON IR and
compile that IR with the existing JSON Schema TCT compiler.

This was the preferred near-term hypothesis when this ADR was accepted. It
preserved the compiler boundary, exercised generated-tokenizer contracts, kept
parser and renderer failures in target-specific evidence tools, and gave GoIR
a concrete path to route-decision-ready corpus evidence. It is now retained as
control evidence rather than as the forward implementation hypothesis.

This route does not prove that JSON Schema is the final frontend abstraction.
It proves whether a target-specific IR can reuse current TCT tokenization,
validation, BPE, provenance, and generated-code contracts without contaminating
core compiler passes with parser concerns.

### StructuralModel

Lower frontend output into a target-neutral structural model for records,
variants, finite domains, references, payloads, invariants, and renderer
metadata, then generate schema/tokenizer artifacts from that model.

`StructuralModel` is the candidate target-agnostic structural backend boundary,
not a grammar-ingestion layer. It is attractive after at least two frontends
repeat the same structural tokenization needs. It should not be the first
implementation route because it risks generalizing around unproven Go-only
details or duplicating JSON Schema analysis before there is a second working
frontend.

### GrammarIR

Lower grammar or parser metadata into a grammar-oriented IR with terminals,
nonterminals, alternatives, precedence, repetition, payload terminals, and
semantic hooks.

`GrammarIR` is upstream of `StructuralModel`. It is necessary only when grammar
decisions themselves must drive token availability or constrained generation.
It needs stronger lexer/parser boundary evidence, unsupported-syntax policy,
and first failing tests for grammar errors before a grammar adapter enters the
compiler path. A `GrammarIR` route must lower into `StructuralModel` or another
explicit structural backend boundary before it can share tokenizer backend
logic.

### new structural boundary

Introduce a separate structural boundary only if the current JSON-Schema-backed
arena route and `StructuralModel` cannot express the needed cross-target
tokenization contract.

This should be a fallback only. It is justified if first-target evidence plus a
second high-impact target show that JSON-Schema-backed arena IRs cannot express
the needed validation, rendering, provenance, or token-boundary contracts, and
if `StructuralModel` would force unclear or target-specific semantics.

## Accepted Route

Historically, GoIR was accepted as the first durable non-JSON evidence target
because it had strong local evidence: a stable Go parser/toolchain path, a
pinned source-corpus manifest, complete bridge/renderer scan coverage for the
clean corpus, source-BPE and Go lexer baselines, and representative generated
TCT route evidence. That route is now control evidence, not the forward
source-language implementation path.

Treat the JSON-Schema-backed arena GoIR bridge as historical first-target
control evidence only, not as a live implementation or evidence-growth route.
It showed that source parser output can be preserved, validated, rendered, and
compared with TCT tokenization, but it does not expose enough syntax-level
decisions for a multi-language backend and residual operational surfaces are
retired.

Forward second-target work is the wired C# `minimal_class` cross-target check
only; do not drive C# breadth, Java, or TypeScript unless the project
explicitly promotes them. Do not route new arena IR work through the core compiler.
`GrammarIR` / decision graph is the required syntax input layer for
multi-language source work; it must lower into `StructuralModel` before
generated codec execution.

The active route is the parser-backed syntax path. The checked toy grammar,
Tree-sitter metadata, minimal C# syntax metadata, and Go syntax metadata
lowerings emit explicit `GrammarIR -> StructuralModel` decision graphs as
executable syntax-level boundary fixtures. Cross-target comparison and
Roslyn/Go metadata smokes remain retained gate evidence. Gate strategy lives in
`docs/syntax-backend-gate-strategy.md`; do not expand arena controls as a
substitute for syntax backend work.

## Acceptance Evidence

Go is selected as the first evidence target, and the JSON-Schema-backed arena
GoIR bridge is accepted as baseline/control route evidence.

The historical route-decision spike resolved the old external-input framing:
a local Go toolchain, pinned clean standard-library source manifest, source
byte baseline, and Go lexer-token baseline made the arena GoIR bridge
input-ready. The clean standard-library bridge scan found no unsupported AST
files or nonrenderable current-GoIR files, and representative GoIR, source-BPE,
Go lexer, and generated TCT evidence passed over the clean corpus.

Keep historical C#, Java, and TypeScript/JavaScript comparison notes for
provenance, but keep forward implementation focused on the syntax-level path
named in the research strategy notes. Do not update or regenerate GoIR control
surfaces unless a current retained test proves a precise migration need.

## Implementation Entry Conditions

- This ADR is historical route evidence. Do not use the retired readiness
  helper as an entry condition for new work; use the research strategy notes,
  the research roadmap, and current syntax backend gates for active syntax-level
  promotion requirements.
- Keep the first implementation tests identified before code moves beyond the
  evidence prototype: minimal source fixture, parser-error fixture, IR
  validation failure, generated-tokenizer round trip, token-range assertion,
  canonical render/reparse check, and BPE/no-BPE smoke where relevant.
- Keep C#, Java, and TypeScript/JavaScript planning summaries as optional route
  signals until they are backed by pinned source manifests and available
  toolchain checks. C# remains the architecture-first second target when
  feasibility matches Java; Java remains the architecture peer/fallback.

## Boundaries

- This ADR is compiler-route evidence only. It is not paper evidence,
  downstream fine-tuning evidence, or a claim about model quality.
- External parser, formatter, compiler, and validator failures stay in
  target-specific evidence reports. Generated tokenizers should not shell out
  to domain tools.
- BPE remains a wrapper over schema-local payload tokens until a frontend
  proves a stronger token-boundary need.
- Source-program completion semantics are bounded by
  `docs/source-program-completion-semantics.md`: neither the retired GoIR
  route nor the forward syntax path has a source completion or gapped infill
  claim beyond raw left-prefix token truncation evidence.
- Source manifests, source hashes, compiler inputs, generated-code hashes, and
  optional BPE merge fingerprints must travel with route-decision evidence.
