# SQL integer multi-digit design freeze (v1)

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

**Status:** locked for step-4 implementation tranche.  
**Band:** SharedAlphabet / SQL `u8_integer_payload` width **256** — **do not widen**.  
**VOCAB:** unchanged (SQL pin vocab 833).

## Encoding (canonical + prefix-free)

On `u8_integer_payload` only (not menu band):

| Value | Digits |
|---|---|
| `0..=253` | 1 digit = value (bit-identical to pre-change streams for this range) |
| `254..=65_535` | marker `254` + 2 big-endian base-256 digits |
| `65_536..=16_777_215` | marker `255` + 3 big-endian base-256 digits |
| `> 16_777_215` | fail closed → residual `integer_capacity` |

Values 254 and 255 as *payloads* never use a single digit (markers occupy those codes). That is a **stream-contract change** for those two values (breaking for corpora/BPE reuse).

## Canonical mask (non-negotiable)

Without mask discipline the scheme is not unique: value `5` admits marker `254`+`(0,5)`. Non-canonical streams break decode→re-encode **token fixpoint**.

Continuation constraints:

- After marker `254`: reconstructed value ≥ 254. If high digit `d1 == 0`, low digit `d0 ∈ {254, 255}` only.
- After marker `255`: high digit `d2 ≥ 1` (value ≥ 65_536).

Encode always emits the shortest legal form. Decode and mask reject non-canonical continuations.

## Residual targets

Compiler capability: 2-digit and 3-digit paths live; over-capacity → `integer_capacity`.

Corpus rematerialize (C2) reports the historical 544 `tct_integer_u8_band` ids by:

- `recovered`
- `still_excluded:<new_reason>` (multi-issue taxonomy; not an integer failure)
- `integer_path_failed` (**only this gates rematerialize**)

491 / 542 are upper-bound expectations for pure-integer-only blockers, not hard fail-closed counts.

## Layout-diff

Absolute ranges unchanged. `integer_value_range.valid_max` records value domain `16_777_215`. Stream-contract change with unchanged ranges → treat as **breaking** for raw-token corpora / BPE (`non-exact_reuse`). Shared-role regression: `syntax_backend_boundary_test`, Go/C# layout pins, focused SQL tests.

## Systems note

Variable width keeps common SQL integers (`LIMIT 1`, small predicates) at **1** value token. Fixed W=3 would tax every integer with 3 tokens — rejected. Token delta vs fixed W=3 should be measured on residual samples when rematerializing.

## Implementation module

`src/syntax_state_machine/integer_codec.rs` — pure encode/decode/mask helpers; wired from `encode.rs`, `raw_replay.rs`, SQL parser/AST/tree.
