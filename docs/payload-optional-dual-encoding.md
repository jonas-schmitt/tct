# Payload / Optional Dual Encoding Contract Note

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Status: **document-only residual** (Campaign JS-3). Does **not** change token
layouts, generated codecs, or canonical stream pins. Collapsing the dual forms
is gated on a deliberate layout / canonical-form revision — do not force one
solely for this note.

Related: `docs/syntax-state-machine-contract.md` (structural tokens),
`src/syntax_state_machine/raw_replay.rs` (admissibility), JSON Schema
`OptionalField` / bit-packed presence in `src/tokenizer/optional_field.rs` and
`src/generator/struct_generator/bitpacked.rs`.

## Scope

This note records how **empty optional fields** can appear under two different
raw-token surface forms on the syntax-level raw state machine, and how that
relates (by analogy only) to JSON Schema optional encoding. It is residual
decode/layout polish documentation, not an implementation task and not a
paper-claim change.

## Syntax path: two admissible empty-optional forms

On the raw syntax state machine, after a node is open and **no field is open**,
admissible tokens include:

1. `field_start` for the next field in order (when any fields remain), and
2. `node_end` when **no remaining field is required** — i.e. every field at or
   after the cursor is optional (or the cursor is past the last field).

That means trailing empty optionals admit **dual encodings**:

### A. Explicit marker (per optional field)

For each empty optional field, in field order:

```text
field_start → field_missing → field_end
```

Then:

```text
node_end
```

`field_missing` is only legal inside an open optional field with zero values
and without a prior `repeated_empty` / value. It must be followed by
`field_end` before the next field or `node_end`.

This form makes absence **local and explicit** in the stream: every optional
field position is visited.

### B. Skip via `node_end` (implicit trailing absence)

After all **required** fields (and any earlier optional fields the encoder
chose to emit) are closed, and with no field open:

```text
node_end
```

is admissible even if later optional fields were never opened. Those unvisited
optionals are treated as absent without emitting `field_missing` for each.

This form is **shorter** when many trailing optionals are empty, but absence of
a particular later field is not marked at a dedicated stream position.

### Why both exist today

- Explicit markers keep field-cursor progress and mask probes local (useful for
  replay, debugging, and partial-prefix tools).
- Skip-via-`node_end` matches a natural “close the node when nothing required
  remains” control decision and avoids zero-entropy `field_missing` chains for
  sparse records.

Both are currently **mask-admissible** and must remain **strict-decode safe**
relative to each other until a canonical-form decision picks one. Random
admissible walks and elided-stream policy may prefer one surface; that is not
by itself a layout-breaking claim.

## JSON Schema path: related but different dual surfaces

JSON Schema TCT does not use `node_end` / `field_missing` tokens. Optional
JSON object fields use different mechanisms that also create more than one
surface form for “no value”:

| Mechanism | Empty / absent surface | Where |
| --- | --- | --- |
| `Option<T>` field | single presence token `0` (None) or `1` + value | non-bitpacked optional members |
| `OptionalField<T>` | state token `0` Absent / `1` Null / `2` + value Present | nullable / Value-preserving members |
| Bit-packed optionals (O3/Os) | presence **mask** group(s) over many fields; absent bits replace per-field `0` tokens | `generate_bitpacked_encode` |

These are **not** the same dual as skip-via-`node_end` vs `field_missing`.
They are payload-layer presence encodings under the JSON Schema backend. Do not
collapse JSON Schema presence forms by changing syntax `node_end` policy, or
vice versa, without separate layout revisions for each backend.

## Canonical-form residual

**Desired long-term outcome (not this tranche):** one documented canonical
empty-optional form per backend, with:

- encode always emitting the canonical form;
- strict decode accepting the canonical form and documenting whether legacy
  dual forms are rejected, recovered, or still accepted;
- elided-stream / mask-forced policy aligned with that choice;
- a `tct-token-diff` / syntax layout-diff class if the choice changes ranges or
  stream identity (`exact_reuse` only if tokens and roles are unchanged).

**Until then:**

- Do **not** force a layout revision solely to collapse the dual forms.
- Treat dual acceptance as intentional residual polish, not a correctness bug,
  unless a retained gate shows one form fails round-trip or mask consistency.
- New features must not invent a **third** empty-optional encoding without
  updating this note and the relevant layout gate.

## Residual pointer

Deferred follow-ups, not scheduled in this repository:

- Deferred: “Close The Remaining Payload-Layer Decode Polish Items
  (layout-gated)” — collapse dual encodings only under an explicit
  layout/canonical-form revision.
- Campaign JS-3 is satisfied by this document-only note.

## Claim boundary

Documentation of existing dual surfaces and residual policy only. No layout
pin change, no generated-codec change, no paper or models claim update.
