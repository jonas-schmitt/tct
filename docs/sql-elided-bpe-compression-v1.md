# SQL elided-stream BPE compression (v1)

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

**Claim boundary:** measurement / engineering evidence only — **not** a paper claim, not a layout pin change.  
**Date:** 2026-08-05  
**Regen note:** Merge tables and streams from the pre-collapse 833-vocab run are
**stale** after the shared payload pool collapse (`93c71a769`: base vocab
833 → **321**, PAD 320). This document reports the rematerialized ladder and
two human-selected power-of-two budgets: **total vocab 512** (**191** merges)
and **total vocab 1024** (**703** merges).

## Why

SQL A5 expanded streams are long (~median 111 decision tokens on Spider train).
Mask-forced elision cuts that to free-choice positions only. BPE over those
elided streams is expected to compress further. Prior extrapolation (13–16
tokens) was a guess; this report measures it. A second power-of-two cell at
703 merges gives another point on the compression curve: more merges means
fewer free decisions per query (the direct lever against decode-time error
compounding).

## Layout / stream class

Per `DEVELOPMENT.md` schema-evolution classes:

| Question | Answer |
| --- | --- |
| Token-layout pin change? | **No for this BPE regen.** Base SQL layout is already the collapsed pin (`SQL_SYNTAX_LAYOUT_VOCAB_SIZE = 321`, PAD = 320; layout id `sql-schema-linking-token-layout-v1`). That collapse was a prior `breaking_change` vs the four-band 833 pin. |
| Stream-contract change? | **No new contract.** Streams are the existing **elided** A5 contract. |
| Layout-diff class (this work) | **Downstream BPE artifact** over the current base layout → **`exact_reuse` of the base SQL layout**. Retrain/migrate embeddings when merges change; does **not** require Go/C#/SQL layout pin refresh. |

## Inputs

| Input | Path / value |
| --- | --- |
| Catalog | `data/spider-official/tables.json` (sha256 `61bb20aa401f03164e2d7f3b16509b7b5f79cc9c943ca7bd159046df1159e2ed`) |
| Corpus | Stage-1 menu-index prompt corpus (Spider `train` questions with gold SQL; kept outside this repository) |
| Split | `train`, non-excluded |
| Encoded count | **6108** / 0 failures |
| Encode mode | `SqlSchemaLinkingRuntime::encode` + `elide_forced_tokens` (canonical elided stream) |
| Base vocab | **321** (`SQL_SYNTAX_LAYOUT_VOCAB_SIZE`) |
| Elided train sequences | `data/sql-elided-bpe/train-elided.jsonl` (sha256 `6e31da0b1374227fee1877919e24545e149013f22c32908b41a415bbe7cb8d29`) |
| BPE trainer | `tct-train-bpe --encoded … --base-vocab 321 --max-merges {191,703,…}` |
| Corpus encoder | `tct-sql-elided-bpe-encode-corpus --merge-count N --verify-round-trip` |
| Encoder for ladder stats | `SequentialBpeEncoder` via `tct-bpe-length-stats` |

Durable artifacts live under `data/sql-elided-bpe/`
(not checked in):

- `train-elided.jsonl` + `train-encode-report.json` + `metadata.json`
- `sql-elided-bpe-m{191,256,512,703,1024}.json` (+ provenance sidecars)
- `train-elided-bpe-m191.jsonl` + report (total-vocab **512** handoff)
- `train-elided-bpe-m703.jsonl` + report (total-vocab **1024** cell)
- `compression-report.json` / `compression-report-m191.json`

Reproduce (comparison ladder):

```bash
cargo build --release \
  --bin tct-sql-elided-encode-corpus \
  --bin tct-train-bpe \
  --bin tct-bpe-length-stats

./target/release/tct-sql-elided-encode-corpus \
  --corpus path/to/stage1_corpus_indexed.json \
  --tables data/spider-official/tables.json \
  --output-jsonl data/sql-elided-bpe/train-elided.jsonl \
  --output-report data/sql-elided-bpe/train-encode-report.json \
  --split train --mode encode_elided

python3 scripts/analysis/sql_elided_bpe_measure.py \
  --encoded data/sql-elided-bpe/train-elided.jsonl \
  --base-vocab 321 --merges 256,512,1024 \
  --work-dir data/sql-elided-bpe \
  --a0-median 17 \
  --output data/sql-elided-bpe/compression-report.json
```

Reproduce (power-of-two **512** = **191** merges — do not round to 256):

```bash
./target/release/tct-train-bpe \
  --encoded data/sql-elided-bpe/train-elided.jsonl \
  --base-vocab 321 --max-merges 191 \
  --name sql-schema-linking-elided \
  --output data/sql-elided-bpe/sql-elided-bpe-m191.json

./target/release/tct-sql-elided-bpe-encode-corpus \
  --corpus path/to/stage1_corpus_indexed.json \
  --tables data/spider-official/tables.json \
  --bpe-merges data/sql-elided-bpe/sql-elided-bpe-m191.json \
  --merge-count 191 --split train --verify-round-trip \
  --output-jsonl data/sql-elided-bpe/train-elided-bpe-m191.jsonl \
  --output-report data/sql-elided-bpe/train-elided-bpe-m191-report.json
```

Reproduce (power-of-two **1024** = **703** merges):

```bash
./target/release/tct-train-bpe \
  --encoded data/sql-elided-bpe/train-elided.jsonl \
  --base-vocab 321 --max-merges 703 \
  --name sql-schema-linking-elided \
  --output data/sql-elided-bpe/sql-elided-bpe-m703.json

./target/release/tct-sql-elided-bpe-encode-corpus \
  --corpus path/to/stage1_corpus_indexed.json \
  --tables data/spider-official/tables.json \
  --bpe-merges data/sql-elided-bpe/sql-elided-bpe-m703.json \
  --merge-count 703 --split train --verify-round-trip \
  --output-jsonl data/sql-elided-bpe/train-elided-bpe-m703.jsonl \
  --output-report data/sql-elided-bpe/train-elided-bpe-m703-report.json
```

## Measured baselines (full Spider train, n=6108)

| Stream | median | mean | p90 |
| --- | ---: | ---: | ---: |
| Expanded A5 | 111 | 125.73 | 206 |
| Elided A5 (this run) | **29** | 31.48 | 50 |

Elided length is **unchanged** by the pool collapse (same free-choice count;
only absolute payload IDs remapped into the shared pool). A0 surface-SQL BPE
baseline remains **median 17** (given).

## BPE compression table (post-collapse base 321)

### Power-of-two cells (citable only when losslessness is green)

`DEVELOPMENT.md` admits power-of-two **total** vocabularies on the training queue.
With base **321**:

- total **512** requires exactly **191** merges (`321 + 191 = 512`)
- total **1024** requires exactly **703** merges (`321 + 703 = 1024`)

Rounding 191→256 or 703→512/1024 ladder rungs **misses** the power-of-two
target and is not acceptable for those handoffs.

| Merges | Total vocab | median | mean | p90 | vs elided 29 | vs A0 17 | New embedding rows | Power-of-two total? | Round-trip |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| **191** | **512** | **8.0** | **10.06** | 19.0 | 0.276× (72% shorter) | 0.471× | **191** | **yes (512)** | **6108 / 0** |
| **703** | **1024** | **6.0** | **6.98** | 14.0 | 0.207× (79% shorter) | 0.353× | **703** | **yes (1024)** | **6108 / 0** |

m703 vs m191 on the same 6108 sequences: mean **−3.08**, median **−2**, p90
**−5** tokens. Elided baseline at both cells is unchanged (mean 31.48 / median
29 / p90 50). m703 merge table sha256
`40c03cf08f10514a196e252477a78c5d4fd4d83927a39c656613eb6fcdb3565d`; corpus
jsonl sha256
`d19c7d77b856cf8a145f30580275e5495b236c754d628fe6b84db92814e955e2`
(measured at `tct_commit` `61ab5603b`).

### Comparison ladder (not a power-of-two production target)

| Merges | Total vocab | median | mean | p90 | vs elided 29 | vs A0 17 | New embedding rows | Power-of-two total target |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 256 | **577** | **8.0** | 9.25 | 18.0 | 0.276× (72% shorter) | 0.471× | 256 | under **1024** (room for 703 merges at base 321) |
| 512 | **833** | **6.0** | 7.62 | 15.0 | 0.207× (79% shorter) | 0.353× | 512 | under **1024** |
| 1024 | **1345** | **5.0** | 6.26 | 12.0 | 0.172× (83% shorter) | 0.294× | 1024 | under **2048** (room for 1727 merges at base 321) |

**Collapse effect on compression:** medians are **identical** to the
pre-collapse ladder (8 / 6 / 5). Mean lengths differ only in the fourth
decimal of token totals (e.g. 56507 vs 56512 at 256 merges) — no material
compression change. The payoff of the collapse is **embedding-table headroom**:
at total vocab **1024**, base 321 admits **703** merges vs only **191** under
the old base 833; at **2048**, **1727** vs **1215**. At total **512**, base 321
admits **191** merges vs **none** under old base 833 (833 already exceeds 512).

Trainer log (simulation avg length on the training set): starts at 31.48
elided, ends at **10.06** after 191 merges / **6.98** after 703 merges /
**6.26** after 1024 merges — consistent with the SequentialBpeEncoder and
encode-corpus stats above.

Do not auto-escalate toward 2048 merges for the embedding-cost reasons in the
next section.

## Gate verdict

**PASS** (compression + losslessness).

The report gate has two required blocks; overall `verdict` is pass only when
both are green. **Do not cite compression ratios when losslessness is not
green.**

### Compression block

Criterion: median BPE length at 512 merges is at least 10% below the
measured elided median (`median < 0.90 × 29` ⇒ must be `< 26.1`).

Measured: **median 6.0** at 512 merges ⇒ **79% reduction** vs elided, and
**well below A0’s 17** (ratio 0.35×).

### Losslessness block

Criterion: each cited power-of-two cell reports
`n_encoded == 6108`, `n_failed == 0`, and
`n_round_trip_verified == 6108` via
`round_trip_sql_via_elided_bpe` (the encode-corpus `--verify-round-trip`
path).

| Cell | merges | n_encoded | n_failed | n_round_trip_verified |
| --- | ---: | ---: | ---: | ---: |
| m191 (total 512) | 191 | 6108 | 0 | 6108 |
| m703 (total 1024) | 703 | 6108 | 0 | 6108 |

## Embedding-cost note (why not always more merges)

Each merge adds **one new token id** that needs an embedding row learned from
scratch. Occurrences per new embedding row (fine-tune corpus; +literacy
corpus) motivated the dual power-of-two cells:

| total vocab | merges | ~median tokens | occ/row (FT) | occ/row (+literacy) |
| ---: | ---: | ---: | ---: | ---: |
| **512** | **191** | **~9 (meas. 8)** | **110** | **904** |
| **1024** | **703** | **6 (meas. 6)** | **45** | **372** |
| 2048 | 1727 | 5 | 19 | 153 |

The failure this line of work fixes was the old `u8_integer` band starved at
**~25 occurrences per id**. Total **512** gives ~904 with literacy — wide
margin. Total **1024** is the second selected power-of-two point
(still above starvation with literacy). Total **2048** would land at ~19 on
the fine-tune corpus, back in starvation territory.

| Merges | New IDs | Total vocab | Notes |
| ---: | ---: | ---: | --- |
| **191** | **191** | **512** | **Power-of-two cell; higher occ/row** |
| **703** | **703** | **1024** | **Power-of-two cell; fewer free decisions** |
| 256 | 256 | 577 | Ladder rung; not power-of-two total |
| 512 | 512 | 833 | Ladder; median 6 |
| 1024 | 1024 | 1345 | Ladder; diminishing returns / starvation risk |

## Efficiency / error-compounding context

- **Efficiency:** elided+BPE at **191** merges lands at median **8** tokens vs
  A0 surface BPE **17** and raw expanded A5 **~111**. At **703** merges the
  median is **6** (mean **6.98**).
- **Error compounding:** free-decision count drops from ~31 elided decisions
  (mean) to ~10 BPE tokens (mean at 191) or ~7 BPE tokens (mean at 703).
  Exact-match probability scales with the exponent (number of independent
  steps). Net task accuracy remains empirical because each merged token is a
  harder class.

## Phase 2 — exact BPE admissible mask

Gate passed, so Phase 2 is implemented and re-validated under the collapsed
layout, including the **191-merge / total-512** handoff. See:

- Narrative: [`docs/sql-elided-bpe-admissible-mask-v1.md`](sql-elided-bpe-admissible-mask-v1.md)
- Checked-in pointer: the `sql-elided-bpe-admissible-mask-v1` research report (not included)

Summary: full Spider train round-trip **6108/6108** at 191 merges, exact-mask
contract + negative rejection cases, comparison ladder 256/512/1024, production
training corpus at **191** under
`data/sql-elided-bpe/train-elided-bpe-m191.jsonl`,
and the additional **703** cell under
`train-elided-bpe-m703.jsonl`. No model training was launched from this
measurement report.
