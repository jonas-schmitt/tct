# SQL shared payload pool (layout collapse)

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.


## Change

SQL opts out of the cross-language disjoint payload bands. The four-band layout
(vocab **833**, PAD **832**) collapses to:

| role | start | width |
| --- | --- | --- |
| `structural_decisions` | 0 | 64 |
| shared payload pool (`utf8_byte_payload` / `u8_integer_payload` / `menu_index_payload`, exact-coincident) | 64 | 256 |
| PAD | 320 | 1 |

`VOCAB_SIZE` = **321**. Layout id `sql-schema-linking-token-layout-v1`
(pin fixture path unchanged: `tests/fixtures/syntax_structural_model/sql-token-layout-pin-v0.json`).

Encode/decode still resolve the three payload **roles**; absolute ids share one
pool. The admissible mask selects the active role at each free decision. Strict
decode still rejects a pool id that is inadmissible for the active role.

## Soundness census (pre-collapse layout)

Tool: `tct-sql-band-mixing-census` (must be run on the **pre-collapse** four-band
layout; after collapse, payload roles are coincident so band classification is
no longer informative).

Checked-in report: the `sql-band-mixing-census-full-train-v0` research report (not included)
(Spider official train, full file; layout vocab 833 at measurement time):

| metric | value |
| --- | --- |
| encodeable streams | 3220 / 7000 rows |
| free decisions | 76960 |
| single-band | 73685 (95.7%) |
| multi-band | 3275 (4.3%) |
| pure | structural 61392, menu 10514, integer 1779 |
| mixed | **only** `(structural_decisions, utf8_byte_payload)` 3275 |
| unsound shared-pool patterns | **none** |
| `sound_for_shared_payload_pool` | **true** |

`menu_index` and `u8_integer` never co-occur with any other band in a free mask.
Structural stays disjoint from the pool because of the measured struct+utf8
string-terminate choice.

## Layout-diff class

Four-band → shared pool is **`breaking_change`** (vocab/pad change; integer and
menu ranges moved onto the shared pool). Downstream raw-token corpora and BPE
artifacts **must be rematerialized** — never reused against the old pin.

Go and C# layout pins were **not** edited (byte-identical to `origin/main`).
SQL deliberately gives up cross-language payload token compatibility.

## Acceptance gate

Tool: `tct-sql-shared-pool-corpus-gate`

```bash
cargo build --release --features experimental --bin tct-sql-shared-pool-corpus-gate
./target/release/tct-sql-shared-pool-corpus-gate \
  --tables /path/to/spider/tables.json \
  --examples /path/to/spider/train_spider.json \
  --output reports/sql-shared-payload-pool-corpus-gate-v0.json
```

Checked-in result: the `sql-shared-payload-pool-corpus-gate-v0` research report (not included)

| metric | value |
| --- | --- |
| encodeable | 3220 |
| `decode`/`encode` fixpoint ok | 3220 |
| `expand(elide(s))==s` ok | 3220 |
| failures | 0 |
| `all_encodeable_passed` | **true** |

Original surface SQL need not equal the rendered form; the gate is a complete
expanded-stream fixpoint plus the elided-stream contract.

## Implementation notes

- `GrammarStructuralTokenLayout::validate` allows **exact-coincident** multi-role
  ranges (intentional shared pool) and still rejects partial overlaps.
- `count_menu_tokens` (range-only) returns `None` when the menu range coincides
  with utf8 or integer; gold stats use `count_menu_tokens_by_role_replay`.
- Menu band width remains 256 (**P9**); multi-digit menu capacity unchanged.

## Not claim-bearing

This is a compiler layout change. It does not train models, rematerialize BPE,
or update paper claims. Models-side rematerialize remains a separate step.
