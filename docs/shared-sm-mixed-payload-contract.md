# Shared-SM Mixed-Payload Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Status: **implemented** (maintainer-owned shared raw SM). Subset codecs still
homogenize; this document is the owner of the shared mixed-field mechanism.

This document is the design and implementation contract for **I5** (shared
decision-token / cross-grammar structural alphabet work). It specifies how a
single raw syntax state-machine field choice may mix `Nonterminal` wrappers
with immediate payload terminals, what that implies for token layout, decode
determinism, and consumed-token accounting, and what it would break in existing
pins and subset codecs.

It does **not** change subset codecs, golden absolute token vectors, production
routes, or promote I5.

## Surfaces and claim boundary

| Surface | Role |
| --- | --- |
| Full-pin fact | `docs/tree-sitter-json-mixed-payload.md` — tree-sitter-json `_value.subtype` already mixes Nonterminal / BooleanPayload / IntegerPayload in GrammarIR |
| StructuralModel | `src/grammar_ir_structural_model.rs` — already accepts mixed alternatives; records payload leaves and choice edges separately |
| Codec policy today | Subset codecs **homogenize** to all-Nonterminal field choices (JSON wraps `number`; C inventory records mixed-payload risks) |
| This contract | Shared raw SM admits mixed fields without per-language wrappers (synthetic witness + compile-time alternative table) |
| Out of scope | New subset codecs, Go/C# layout pin changes, tree-sitter-go inventory swap, corpus/entropy claims |

Claim id for this document:

```text
shared_sm_mixed_payload_contract_v1
```

## Problem

A GrammarIR field alternative is a pair `(node_kind, GrammarTerminalKind)`.
When every alternative on a choice field is `Nonterminal`, the raw SM can use a
uniform control flow:

1. emit structural choice token for alternative *k*;
2. descend into rule *k* (which may later open payload substates on its own
   fields).

When some alternatives are payload terminals (`IdentifierPayload`,
`StringPayload`, `IntegerPayload`, `BooleanPayload`) and others are
`Nonterminal`, step 2 is not uniform: choosing a payload alternative must
**enter a payload substate immediately** without a child rule, while choosing a
Nonterminal alternative must **descend**. That is the mixed-payload problem.

Concrete witness (full pin, not a codec claim): tree-sitter-json
`_value.subtype` mixes Nonterminal (`array`, `object`, `string`, `null`),
BooleanPayload (`true`, `false`), and IntegerPayload (`number`). See
`docs/tree-sitter-json-mixed-payload.md`.

## Current layers

| Layer | Mixed field OK? | Notes |
| --- | --- | --- |
| Adapter → GrammarIR | Yes | Classification is per alternative; default map + optional `terminal_classes.json` (CPU-5 T3) |
| GrammarIR → StructuralModel | Yes | `collect_references` only for Nonterminal; payload leaves collected separately; decision edges may set `crosses_payload_boundary` |
| Raw SM | **Yes (shared)** | Compile-time discriminated alternatives; mixed fields use alternative-specific structural choice tokens only |
| Subset codecs | **No (policy)** | Codecs still force homogeneous Nonterminal fields; deliberate migration later |
| Token-ID golden pins | N/A to mixed SM | Absolute vectors pin *current* homogeneous codecs; any layout change is `breaking_change` |

## Contract goals

A shared-SM mixed-payload design must satisfy:

1. **Decode determinism** — from the choice token alone (plus known field
   signature), the decoder knows whether to enter a payload substate or descend
   into a child rule, without look-ahead into payload bytes.
2. **Consumed-token accounting** — encode and decode agree on how many tokens a
   mixed field consumes for each alternative class; strict paths reject
   truncated payload sequences; recovery paths document partial results.
3. **Mask correctness** — at a mixed choice state, the mask admits only the
   structural choice tokens for legal alternatives (not raw payload bytes of
   sibling alternatives).
4. **Layout stability roles** — payload ranges stay attached to stable roles
   (`utf8_byte_payload`, `u8_integer_payload` / `integer_lexeme_payload`,
   structural decisions); mixing does not invent ad hoc per-field payload
   bands.
5. **No silent layout drift** — enabling mixed fields on a pin that today uses
   wrappers is a layout-affecting change for any codec that already emitted
   absolute token IDs; treat as `breaking_change` unless a proven
   `exact_reuse` report says otherwise.

## Implemented contract

### A. Discriminated choice, then class-specific body

Keep the **structural choice** as a single decision over alternatives. Each
alternative carries an explicit **body kind** derived from
`GrammarTerminalKind` at `CompiledSyntaxModel` construction
(`ModelFacts::alternatives_by_field`):

| Alternative terminal | After choice token | Body |
| --- | --- | --- |
| `Nonterminal` | descend | enter child rule’s first field / empty record end |
| `BooleanPayload` | no payload bytes | choice alone completes; zero-width body |
| `IntegerPayload` | open integer payload substate | consume integer encoding (e.g. u8 band) until complete |
| `StringPayload` / `IdentifierPayload` | open utf8 payload substate | consume utf8 payload until terminator / length contract |

The choice token itself remains in the **structural_decisions** band (at or
above `RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET`). Payload bytes never share the
choice mask. Homogeneous multi-alt Nonterminal fields keep the existing
`node_start_alternative_tokens` assignment; mixed fields also receive choice
tokens for payload node kinds via `ensure_mixed_field_choice_tokens`.

This matches the decision-graph edge flag `crosses_payload_boundary` already
emitted by StructuralModel for payload alternatives: the edge is structural in
the graph, but the **next runtime step** opens a payload substate.

### B. BooleanPayload and field-less rule identity

Two legitimate encodings of boolean-like leaves already exist in TCT:

1. **Rule identity** — field-less Nonterminal rule (`true` / `false` as
   wrappers; meaning is the rule name). Preferred by current JSON subset.
2. **BooleanPayload** — choice alternative is BooleanPayload; the choice token
   alone selects true vs false without a child rule.

A shared SM must pick one per field and document it. Mixing both on one field
(BooleanPayload `true` *and* Nonterminal wrapper `true`) is forbidden: it is
ambiguous for decode and double-counts vocabulary. Construction rejects
duplicate or ambiguous `(node_kind, terminal class)` identities with a
diagnostic that names owner rule, field, node kind, and terminal class.

**Migration note (duplicate alternatives on a pin):**
`ModelFacts::from_sidecar` hard-fails when one field repeats the same
`(node_kind, terminal class)` pair. The error field is
`syntax_state_machine.mixed_payload.ambiguous_alternative` and the message
looks like:

```text
owner_rule=<rule> field=<field> node_kind=<kind> terminal_class=<Class>: \
  duplicate alternative identity on the same field
```

Neither `src/tree_sitter_metadata.rs` (`lower_field` over `field.types`) nor
`src/go_syntax_metadata.rs` (field-alternative map) currently deduplicates
`types` / alternatives before lowering. A **future** pin that carries a
repeated alternative therefore fails at model construction rather than
silently collapsing. The fix is to dedup at the adapter (or correct the pin
export) so each `(node_kind, terminal class)` appears once per field — **not**
to relax the `ModelFacts` uniqueness check. Do not reintroduce silent
last-wins or first-wins collapsing in the shared SM.

**Default for new shared-SM work:** prefer rule identity (homogeneous
Nonterminal) unless a pin’s full surface requires BooleanPayload for finite
domain density. Do not flip existing golden-pinned codecs without a layout
migration report.

Zero-width BooleanPayload materializes only from **explicit identities**:
mixed-field alternatives must use node kind exactly `"true"` or `"false"`.
Construction rejects any other BooleanPayload node kind on a mixed field
(diagnostic names owner rule, field, node kind, and terminal class). Decode
maps `"true"` → `"true"` and `"false"` → `"false"`; it never defaults
non-`false` names to true.

### C. Integer / string payloads without wrappers

Bare `number: IntegerPayload` on a mixed field is allowed **only if**:

- the integer value range is declared on the token layout
  (`GrammarIntegerValueRange`);
- the choice → payload transition is recorded as `crosses_payload_boundary`;
- encode emits exactly: `[choice_token] ++ integer_payload_tokens`;
- decode, after reading `choice_token` for that alternative, consumes the
  payload with the same bounds and fails closed on short streams.

Same for string/identifier: `[choice_token] ++ utf8_payload_tokens` under the
utf8 band.

### D. Homogenization remains a valid specialization

Subset codecs may continue to wrap payload leaves as Nonterminals (synthetic
`number.value`) to keep fields homogeneous. That is a **local specialization**,
not a contradiction of this contract. Shared-SM mixed payload is opt-in for
paths that need full-pin fidelity or shared alphabets (I5).

**Legacy homogeneous encodings preserved:**

- generic `terminal` remains the start token for homogeneous payload fields;
- tree-sitter JSON and C subset homogenization stays unchanged;
- Go/C# layouts and absolute JSON/C subset golden token vectors must not move.

### E. Synthetic witness

Focused GrammarIR witness (unit tests under
`src/syntax_state_machine/{model_facts,raw_replay}.rs`):

```text
wrapper.value : child:Nonterminal
              | number:IntegerPayload
              | true:BooleanPayload
              | false:BooleanPayload
child         : (field-less record)
```

Claim boundary: shared SM mechanism only — not a production codec, not a
layout pin, not I5 transfer evidence.

## Token-layout implications

1. **Structural band size** — choice tokens still count one structural ID per
   alternative (or a shared encoding of the domain). Mixed fields do not remove
   structural IDs; they may remove *wrapper rule* structural overhead when
   leaves no longer need `node_start`/`node_end` pairs.
2. **Payload bands** — remain global to the tokenizer layout, not per field.
   Multiple mixed fields share `utf8_byte_payload` / integer bands.
3. **PAD and VOCAB_SIZE** — unchanged invariants: every emitted ID `< VOCAB_SIZE`;
   PAD last; no collisions with base / BPE ranges.
4. **Breaking change class** — moving a leaf from wrapped Nonterminal to bare
   payload on a field changes absolute token sequences even if ranges are
   unchanged in size. Downstream BPE / embedding artifacts must retrain or
   migrate (`tct-token-diff` → `breaking_change` unless proven otherwise).
5. **Golden pins** — `tree_sitter_{json,c}_subset_absolute_token_id_golden_pins`
   lock today’s homogeneous codecs. Implementing this contract must not update
   those pins to “make tests green”; either keep subset homogenization or treat
   pin updates as an explicit migration commit with report evidence.

## Decode determinism

Decoder state at a mixed field:

```text
state = Choice(field)
  on structural token t:
    alt = alternative_for(t)   // fail if t not in field domain
    if alt.terminal == Nonterminal:
      push Frame(rule=alt.node_kind, field_index=0)
    else:
      enter PayloadSubstate(kind=alt.terminal, owner=field, node_kind=alt.node_kind)
      // BooleanPayload may complete immediately with zero payload tokens
```

No alternative may require peeking at the next payload byte to decide whether
the choice was structural vs payload. The choice token is always structural.
Generic `terminal` is rejected on mixed fields (mask and apply path).

## Consumed-token accounting

For alternative *a* of mixed field *f*:

| Class | Tokens consumed |
| --- | --- |
| Nonterminal | `1` (choice) + tokens of the full child rule encoding |
| BooleanPayload | `1` (choice only) |
| IntegerPayload | `1` (choice) + integer payload length under the layout’s integer contract |
| String/IdentifierPayload | `1` (choice) + utf8 payload length under the string encoding contract |

Strict decode: if the stream ends mid-payload, error with field path, expected
class (`RawSyntaxError.expected_payload_class`, from the open payload body's
StructuralModel `payload_kind`), and tokens consumed so far. Construction of a
mixed field that includes `IntegerPayload` also requires a declared
`GrammarIntegerValueRange` on the token layout (see §C); missing declaration
fails at `CompiledSyntaxModel` construction with owner rule/field/node kind.
Recovery paths (if any) must document normalization and partial results; they
are not the default for syntax codecs.

Round-trip integrity remains mandatory for any implementation: instance →
tokens → instance preserves semantic data for supported constructs.

## What this would break in existing pins / codecs

| Artifact | Risk if mixed payload enabled without migration |
| --- | --- |
| JSON subset codec | Absolute token IDs change if `number` unwraps; golden pin fails |
| C subset codec | Any leaf reclassified from Nonterminal to payload moves streams |
| Full-pin JS/C/Rust SM lower | No break today — SM already accepts mixed shapes |
| Go / C# syntax layouts | Unrelated unless a shared alphabet (I5) reuses roles; do not couple |
| BPE merge tables trained on subset streams | Invalid if token sequences change |
| Forced-share / entropy reports on subset streams | Historical; do not edit curated reports to “match” a new contract |

Existing subset policy remains correct until an explicit layout migration.

## Relationship to I5

I5 (shared decision-token alphabet / transfer across grammars) needs a
**stable meaning** for structural choice tokens that may appear next to payload
entries in some grammars and as pure structure in others. Without this mixed
payload contract, I5 would either:

- force every grammar through homogenizing wrappers (losing full-pin fidelity),
  or
- invent ad hoc per-language SM branches (defeating shared alphabet goals).

This contract is landed. Promoting I5 still requires a separate Ready trigger
(implementation plan, layout pins, and cross-grammar evidence).

## Implementation checklist (landed)

1. Focused synthetic GrammarIR fixture with one mixed field (Nonterminal +
   IntegerPayload + BooleanPayload) and raw-SM tests (TDD).
2. Raw SM choice handling per § “Decode determinism” without language keys.
3. Subset codecs remain on homogenization.
4. Do not touch Go/C# layout pins or production routes in the same tranche.
5. `docs/tree-sitter-json-mixed-payload.md` points here as the shared contract
   owner.

## Non-goals

- No new subset codec, no full C/JS/Rust codec.
- No terminal_classes entries for held-out pins “to make mixed payload work.”
- No corpus, entropy, or forced-share measurements.
- No production-route registration.
- No golden-pin updates.

## Validation

```bash
cargo test --features experimental --lib syntax_state_machine
cargo test --features experimental --test syntax_backend_boundary_test
cargo test --features experimental --test syntax_generated_codec_gate_test
cargo test --features experimental --test tree_sitter_metadata_adapter_test
cargo test --features experimental --test tree_sitter_json_subset_codec_test
cargo test --features experimental --test tree_sitter_c_subset_codec_test
eval "$(scripts/testing/resolve-pinned-go.sh --export --repo-root "$PWD")" && ./scripts/testing/test-syntax-backend-gates.sh
```
