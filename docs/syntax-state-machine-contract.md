# Syntax State Machine Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This contract defines the raw syntax-token state machine for future
syntax-level source generation. It is a planning and implementation contract,
not a stable-backend promotion. The boundary remains:

```text
parser-backed metadata or grammar metadata
  -> GrammarIR and decision graph
  -> StructuralModel
  -> token layout
  -> generated codec
```

Go source-instance reports are the first replay inputs. C# must be able to use
the same state, transition, token-layout, payload, and error contract before
the state machine becomes target agnostic. Arena IR routes are controls and
migration evidence only; they are not replay authority for source validity.

## Authority

The state machine is derived from structured syntax metadata:

- `GrammarIR` supplies grammar rules, fields, alternatives, terminal payload
  kinds, parser provenance, and source or metadata identity.
- `StructuralModel` supplies records, ordered fields, required/optional and
  repeated-field policy, references, payload leaves, renderer metadata,
  validation hooks, token ranges, and decision-graph edges.
- Token layout supplies `VOCAB_SIZE`, `PAD_TOKEN`, raw structural ranges,
  payload ranges, optional BPE ranges, and non-collision facts.
- Parser, formatter, type-checker, and renderer sidecars validate evidence,
  but they do not define the generated-codec transition semantics.

## Renderer Policy Requirements

Renderer metadata for syntax-level routes must keep source-language formatting
facts explicit and outside generated hot paths. Before a route claims source
round trips, maintained evidence must name these sidecar-owned policies:

- Canonical declaration separators, blank-line policy, block indentation, and
  grouped versus single-spec declarations.
- Expression grouping, precedence, associativity, selector/index/call shape,
  slice bounds, and type-parameter grouping where the target supports them.
- Literal spelling policy for raw strings, rune/character literals, numeric
  lexemes, tags, import paths, interpolation forms, and any target-specific
  canonicalization.
- Comments, whitespace, and trivia policy, including whether comments and
  arbitrary source trivia are preserved or intentionally normalized away.
- External validation policy for formatter, parser reparse, type checker, or
  equivalent target tools. These checks are evidence sidecars and must not run
  inside generated tokenizer state transitions.

Checked source-instance reports may carry segment annotations such as
owner rule, owner field, node kind, payload kind, and raw-token offsets. Those
annotations are useful for the first report-only replay gate. They are not a
substitute for generated-codec state: after the replay gate, strict decode,
recovery decode, prefix inspection, and admissible-token masks must be
answerable from the raw token prefix plus compiled `StructuralModel` and token
layout.

If a valid source-instance raw token stream needs sidecar segment metadata to
identify a grammar choice that should be visible to generated code, the replay
gate must record the missing token-layout or structural-model fact as a
blocker instead of adding a Go-only lookup table.

## State Representation

The target-neutral replay state has these fields:

```text
SyntaxReplayState {
  route_id
  grammar_id
  structural_model_id
  token_layout_id
  mode: strict | recovery | prefix
  stack: [Frame]
  payload: none | PayloadState
  consumed_raw_tokens
  status: active | complete | partial | error
}

Frame {
  rule
  node_kind
  expected_fields
  field_index
  open_field
  repeated_count
  source_path
}

PayloadState {
  owner_rule
  owner_field
  node_kind
  payload_kind
  range_role
  byte_buffer
  byte_count
  finite_value
}
```

`route_id`, `grammar_id`, `structural_model_id`, and `token_layout_id` are
provenance, not dispatch hacks. They let reports tie replay decisions to the
checked `GrammarIR -> StructuralModel` input and token layout that generated
the state machine.

The active stack is the source of structural validity. It records which rule is
open, which field is expected, whether a field is currently open, and how many
values a repeated field has consumed. Generated code may specialize this into
compact enums and counters, but the observable report/API behavior must match
this contract.

## Structural Tokens

Structural token IDs are interpreted through token-layout roles and state, not
through generated Rust source strings. The current Go source-instance profile
uses these raw structural event classes:

- `node_start`
- `node_end`
- `field_start`
- `field_end`
- `field_missing`
- `field_repeated_empty`
- `terminal`
- `payload_end`
- `boolean_false`
- `boolean_true`

Those class names are target-neutral. Their numeric IDs are profile-specific
and must be below `VOCAB_SIZE`, outside `PAD_TOKEN`, and inside a raw
structural range. A target may add more finite structural decision tokens when
the grammar requires a model-visible choice.

For generated-codec validity, every non-deterministic grammar choice must be
represented by a raw token or by a deterministic state transition. If a field
allows multiple child rules and the current token layout only emits a generic
`node_start`, report-only replay may use checked segment annotations to
validate existing evidence, but generated-codec promotion is blocked until the
choice is tokenized or made deterministic by `StructuralModel` metadata.
When a multi-alternative nonterminal field tokenizes the choice, the token is
still interpreted as a `node_start` event. Its ID names the selected child
node kind under the current field state; it is not a record-specific prefix
table or an arena-IR route.

## Stack Transitions

Replay starts with an empty stack and expects the root rule named by the
checked syntax route.

`node_start` is valid only when the state expects a record node. It pushes a
frame for the selected rule and initializes the ordered field cursor. If the
expected context admits multiple rule alternatives, the preceding or current
raw token must identify the selected alternative unless the
`StructuralModel` proves the alternative is deterministic.

`field_start` is valid only for the next field of the top frame. It opens that
field and records its required, optional, and repeated policy.

`field_missing` is valid only inside an optional field with no values consumed.
It closes the field value position but still requires the matching
`field_end`.

`field_repeated_empty` is valid only inside a repeated field with zero values.
It records an empty list and still requires the matching `field_end`.

A child `node_start` inside an open field is valid only when the field
references that child rule or node kind. A terminal transition inside an open
field is valid only when the field has a matching payload leaf.

`field_end` is valid only for the current open field after that field has
satisfied requiredness and cardinality. It advances the parent frame's field
cursor.

`node_end` is valid only after all required fields in the top frame have been
satisfied and no field is open. It pops the frame. If that closes the root
frame, the stream is complete only when no more raw content tokens remain.

When the field cursor still has **optional-only** fields remaining, both
`field_start` (then explicit `field_missing` / values) and `node_end` (skip
trailing empty optionals) can be admissible. That dual empty-optional surface
is documented in `docs/payload-optional-dual-encoding.md`; collapsing it is a
layout/canonical-form revision, not a silent encode preference change.

## Terminal Payload Substates

`terminal` enters a payload substate owned by the open field. The payload kind
comes from the `GrammarIR -> StructuralModel` payload leaf:

- UTF-8 string, identifier, comment, and raw-literal payloads consume byte
  tokens from a payload range and end with `payload_end`.
- Integer lexeme payloads (`integer_lexeme_payload` range) consume byte tokens
  from that range and end with `payload_end` once the accumulated bytes are
  valid UTF-8 (the lexeme string).
- Bounded u8 integer payloads (`u8_integer_payload` range) consume exactly one
  value token from that range and end with `payload_end`. The value token is a
  raw integer in `0..=255`, not a UTF-8 code unit: `payload_end` is admissible
  only after that single token, further value tokens are not admissible, and
  materialization is the decimal string of the byte (so values `>= 128` round
  trip without UTF-8 coupling).
- Boolean or other finite payloads consume a finite value token such as
  `boolean_false` or `boolean_true` and then leave the payload state.

Payload bytes must map back to valid payload data for the declared payload
kind. Invalid UTF-8 (for UTF-8 and integer-lexeme payloads), incomplete or
over-wide u8 integer payloads, payload bytes outside the active range, missing
`payload_end` for variable-length or bounded-integer payloads, and value tokens
for the wrong payload kind are hard errors in strict mode and replay errors in
prefix/report modes.

Decoded-tree events must carry enough target-neutral payload data to rebuild
terminal leaves from raw token IDs and token-layout roles: payload-byte events
record the materialized byte, `payload_end` records the completed UTF-8 string,
integer lexeme, or decimal u8 value, and finite payload events record the
selected value such as `true` or `false`.

## EOF And Trailing Tokens

Strict whole-stream decode accepts EOF only when the stack is empty, no field
is open, no payload is active, and the root frame has closed. It rejects
premature EOF, trailing content tokens, trailing `PAD_TOKEN`, and raw token IDs
that are greater than or equal to `VOCAB_SIZE`.

Recovery decode may normalize wrapper-level padding only when the route
explicitly marks the stream as wrapper input. Raw content streams do not admit
`PAD_TOKEN`. Recovery reports must record the consumed-token count, the
normalization applied, and whether the result is complete or partial.

Prefix replay may stop at any valid active state. It reports `partial` rather
than `complete`, includes consumed-token accounting, and exposes admissible
next raw tokens or token ranges for that state.

## Error Contract

Replay and generated-codec APIs should report typed errors with enough context
to debug fixtures and model masks:

- `unexpected_structural_token`
- `unexpected_rule_or_node_kind`
- `unexpected_field`
- `missing_required_field`
- `repeated_field_cardinality`
- `invalid_payload_token`
- `invalid_payload_continuation`
- `premature_eof`
- `extra_trailing_token`
- `pad_token_in_raw_stream`
- `token_id_out_of_vocab`
- `bpe_raw_expansion_not_admissible`
- `missing_state_metadata`

Every error records the consumed-token count before the failing token, the
failing token when present, the active rule and field when present, the payload
kind when present, and the source/report identity when replaying checked
artifacts.

## Vocabulary And BPE Invariants

The raw syntax state machine owns raw token validity:

- Every accepted raw token ID is strictly below `VOCAB_SIZE`.
- Raw content streams never include `PAD_TOKEN`.
- Structural ranges, payload ranges, optional BPE ranges, sequence wrapper
  tokens, and `PAD_TOKEN` do not collide.
- BPE is a downstream compression layer. It may ask whether an expansion into
  raw syntax tokens is admissible, but BPE tokens do not define syntax
  validity.
- A BPE prefix API must report how many raw tokens were accepted, whether the
  expansion ended in the middle of a payload, and which raw next tokens or
  ranges are admissible.

## Prefix And Admissible-Token API Boundary

The generated-codec API promoted after report-only replay should expose:

- state after a raw token prefix;
- whether EOF is currently valid;
- whether the prefix is complete, partial, or invalid;
- consumed raw token count;
- the finite admissible next-token set when it is small;
- admissible token ranges for payload-heavy states;
- transition result for one candidate raw token;
- diagnostics for invalid transitions;
- a BPE helper that evaluates raw expansions through the same transition
  semantics.

The API must not expose record-specific prefix-state tables as the framework
boundary. Go-specific payload rules and parser facts may be frontend metadata,
but the state, transition, EOF, consumed-token, and admissible-token semantics
must be target neutral.

## First Report-Only Replay Gate

The first implementation slice should add a report-only replay gate for checked
Go source-instance evidence. It should consume:

- checked Go syntax metadata;
- the lowered `GrammarIR -> StructuralModel` sidecar;
- the Go source-file token layout;
- regenerated source-instance projection artifacts;
- generated source-instance raw token reports under `target/`.

The gate should produce a replay report under `target/` and a focused Rust test
such as `tests/syntax_state_machine_replay_test.rs`. It should accept at least
one complete real-source Go raw token stream and selected valid prefixes.

The first invalid-mutation set should include:

- wrong structural token for the active state;
- missing required field before `node_end`;
- repeated-field cardinality violation;
- invalid payload byte or continuation;
- premature EOF inside an open node, field, or payload;
- extra trailing token after the root closes;
- `PAD_TOKEN` inside raw content;
- token ID greater than or equal to `VOCAB_SIZE`;
- missing tokenized alternative identity when replay cannot derive a grammar
  choice from raw tokens plus `StructuralModel`.

If the existing Go source-instance raw token IDs are not self-sufficient for a
generated-codec state API, the report should still validate current
segment-backed evidence and record the exact missing metadata or token-layout
fact as the blocker for generated-codec promotion.

## Non-Goals

This contract does not:

- promote the syntax backend into `TctCompiler`;
- change generated JSON Schema tokenizer output;
- move external parsers, formatters, or type checkers into generated hot paths;
- use GoIR or C# arena IR as the source-validity boundary;
- make downstream BPE or model reports authoritative for syntax validity;
- claim paper-facing source-language validity before the Go/C# framework gates
  and stable promotion evidence exist.
