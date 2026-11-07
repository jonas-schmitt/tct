# SQL elided-stream BPE admissible mask (v1)

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

**Artifact:** collapsed base vocab 321; **191-merge / total-512 handoff**  
**Claim boundary:** `compiler_sql_schema_linking_only` — engineering validation only;
**not** a paper claim.  
**Module:** `src/sql_schema_linking/elided_bpe_mask.rs`  
**Binaries:** `tct-sql-elided-bpe-validate`, `tct-sql-elided-bpe-encode-corpus`  
**Regen note:** Merge tables trained against the pre-collapse 833 layout must
not be reused. This document reports validation against rematerialized BPE
tables trained on collapsed-layout elided streams, including the
human-selected **191 merges → total vocab 512** production budget.

## Why

Constrained decoding over BPE'd elided SQL streams needs an **exact** admissible
mask: a BPE token is legal iff every constituent base free-choice token is legal
from the current elided walker state, with mask-forced runs auto-drained between
elements (`SqlElidedTctDecodeState::apply`, bounded by `ELIDED_FORCED_RUN_LIMIT`).

Without this, the measured compression (median 6 tokens at 512 merges) cannot be
used for A5 constrained generation.

## Layout / stream class

| Question | Answer |
| --- | --- |
| Token-layout pin change? | **No for this BPE regen.** Base layout is already collapsed: `SQL_SYNTAX_LAYOUT_VOCAB_SIZE = 321`, PAD = 320 (`sql-schema-linking-token-layout-v1`). |
| Layout-diff class | **Downstream BPE artifact** over the existing **elided** stream contract. Treat like other BPE merge tables: retrain/migrate embeddings when merges change. **Does not** require Go/C#/SQL layout pin refresh (`exact_reuse` of the base SQL layout). |
| Stream contract | Canonical **elided** A5. |

## Inputs

| Input | Path / value |
| --- | --- |
| Catalog | `data/spider-official/tables.json` (sha256 `61bb20aa…e2ed`) |
| Corpus | Stage-1 menu-index prompt corpus (Spider `train` questions with gold SQL; kept outside this repository) |
| Split | `train`, non-excluded → **6108** examples |
| Merge table (production) | `target/sql-elided-bpe/sql-elided-bpe-m191.json` (191 merges, total vocab **512**) |
| Merge table (ladder) | `target/sql-elided-bpe/sql-elided-bpe-m1024.json` (truncated for 256/512) |
| Base vocab | **321** |

## 1. Corpus-wide round trip

Check: `decode(expand(unbpe(bpe(elide(encode(sql))))))` re-encodes to the same
elided stream (semantic fixpoint via `strict_decode_complete_elided`).

| Metric | Value |
| --- | ---: |
| Checked | **6108** |
| Passed | **6108** |
| Failed | **0** |
| Failing example ids | *(none)* |

**Production budget:** **191 merges** (total / BPE vocab **512** = 321 + 191).
Report: `target/sql-elided-bpe/train-validate-report-m191.json`.

Comparison ladder (256/512/1024 merges) remains in
`target/sql-elided-bpe/train-validate-report.json` (round-trip also 6108/0 at
those budgets).

## 2. Mask exactness

- Raw free-choice positions validated via target-neutral
  `src/exact_mask_contract.rs` (`ExactMaskContractReport::validate`).
- BPE lift: gold next BPE token admitted at every prefix; expansions applied
  through `try_apply_bpe_token` / `bpe_token_is_admissible`.
- Sample of 32 train queries: mask reports written under
  `target/sql-elided-bpe/mask-report-sample.*.json` — all gold admitted,
  raw status `exact_mask_contract_checked`.
- Full-vocab scan (every non-PAD BPE id) on the 32-query mask sample:

  **At 191 merges (production; denom = 512 − 1 = 511):**
  - Aggregate over **222** gold prefixes: mean **~74 admitted / ~437 rejected**
    per prefix (admit ratio **~14.5%**; median admit 18).
  - Ladder first-db sample (126 prefixes): mean **~82 / ~429** of 511.
  - Mask still **rejects the large majority** (~85.5%) of BPE ids — genuinely
    constraining, not accept-all.

  **At 512 merges (comparison ladder; denom = 833 − 1 = 832):**
  - Aggregate over **151** gold prefixes: mean **~116 admitted / ~716 rejected**
    per prefix (admit ratio **~13.9%**; median admit 43).
  - Ladder first-db timing sample (90 prefixes): mean **~128 / ~704**.
  - Pre-collapse (base 833, denom ~1344) was ~47 admitted / ~1297 rejected
    (~3.5% admit). Absolute admit count rose under the shared pool.

  Artifact: mask reports under `target/sql-elided-bpe/mask-report-sample*.json`;
  ladder rows in `train-validate-report*.json`.

Focused unit tests (fixture gold queries):

```text
cargo test --features experimental --lib sql_schema_linking::elided_bpe_mask
# 5 passed: round-trip+contract, gold path, inadmissible reject,
#           forced_token_in_elided_stream, full-vocab scan
```

## 3. Negative cases

| Case | Result |
| --- | --- |
| BPE/base identity outside free-choice mask | **Rejected** (`try_apply_bpe_token` → `None`) |
| PAD token | **Rejected** |
| Full BPE vocab at start of fixture query | Rejects **more** tokens than it admits |
| Spelling mask-forced token on elided walker | Typed error phase **`forced_token_in_elided_stream`** |
| Forced identity as BPE token at free-choice state | **Rejected** (forced ids never appear in free mask) |

## 4. Merge-count ladder and production selection

Base vocab **321**. Power-of-two **total** vocab targets:

| Total vocab target | Merges available (base 321) | Status |
| ---: | ---: | --- |
| **512** | **191** | **Selected production handoff** |
| **1024** | **703** | comparison only (starvation risk higher) |
| **2048** | **1727** | not selected (~19 occ/row on FT) |

Comparison ladder 256/512/1024 (total vocab **577 / 833 / 1345**) is retained
for compression curves; it is **not** the production merge count:

| Merges | Total vocab | BPE median | BPE mean | BPE p90 | vs A0 17 | Full-scan mean admit/reject (first-db sample) | Prefixes |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **191** | **512** | **8** | **10.06** | 19 | 0.471× | **~82 / ~429 of 511** | 126 |
| 256 | 577 | **8** | 9.25 | 18 | 0.471× | ~95 / ~481 of 576 | 110 |
| 512 | 833 | **6** | 7.62 | 15 | 0.353× | ~128 / ~704 of 832 | 90 |
| 1024 | 1345 | **5** | 6.26 | 12 | 0.294× | ~193 / ~1151 of 1344 | 76 |

Mask construction with `--full-vocab-scan` **rises** with merge count on a fixed
sample because each prefix scans a larger BPE vocab (~0.31s at 191 / 0.38s /
0.57s / 0.94s on the first-db sample).

## 5. Training corpora (models handoff)

**Primary handoff: 191 merges (total vocab 512).** Comparison emits at 512 and
1024 remain available; do not auto-select larger budgets.

| Artifact | Path |
| --- | --- |
| **191 merges JSONL (production)** | `target/sql-elided-bpe/train-elided-bpe-m191.jsonl` |
| **191 report** | `target/sql-elided-bpe/train-elided-bpe-m191-report.json` |
| **191 merge table + provenance** | `target/sql-elided-bpe/sql-elided-bpe-m191.json` (+ `.provenance.json`) |
| 512 merges JSONL | `target/sql-elided-bpe/train-elided-bpe-m512.jsonl` |
| 1024 merges JSONL | `target/sql-elided-bpe/train-elided-bpe-m1024.jsonl` |

Per-record fields: `bpe_token_ids`, `elided_token_ids`, `merge_table_path`,
`merge_table_sha256`, `base_vocab_size` (**321**), `bpe_vocab_size` /
`total_vocab_size` (**512** at m191), `merge_count` (**191**), `tct_commit`,
`stream_contract=elided`, catalog sha, example/db ids.

| Emit | Encoded / failed / RT verified | BPE median / mean | total_vocab | merge_table_sha256 (prefix) |
| --- | ---: | ---: | ---: | --- |
| **m191** | **6108 / 0 / 6108** | **8.0 / 10.06** | **512** | `3d810f13bca36737…` |
| m512 | **6108 / 0** | 6.0 / 7.62 | 833 | `0ec7d32ba8341e1f…` |
| m1024 | **6108 / 0** | 5.0 / 6.26 | 1345 | `96913c8efcff3312…` |

Production emit used `--verify-round-trip`. Streams are runtime-versioned via
`tct_commit` + `merge_table_sha256`; a corpus without provenance is unusable
downstream.

Reproduce (production 191):

```bash
cargo build --release \
  --bin tct-train-bpe \
  --bin tct-sql-elided-bpe-validate \
  --bin tct-sql-elided-bpe-encode-corpus

./target/release/tct-train-bpe \
  --encoded target/sql-elided-bpe/train-elided.jsonl \
  --base-vocab 321 --max-merges 191 \
  --name sql-schema-linking-elided \
  --output target/sql-elided-bpe/sql-elided-bpe-m191.json

./target/release/tct-sql-elided-bpe-validate \
  --corpus path/to/stage1_corpus_indexed.json \
  --tables data/spider-official/tables.json \
  --bpe-merges target/sql-elided-bpe/sql-elided-bpe-m191.json \
  --merge-count 191 --ladder 191 --mask-sample 32 --full-vocab-scan \
  --output target/sql-elided-bpe/train-validate-report-m191.json \
  --mask-report-output target/sql-elided-bpe/mask-report-sample-m191.json

./target/release/tct-sql-elided-bpe-encode-corpus \
  --corpus … --tables … \
  --bpe-merges target/sql-elided-bpe/sql-elided-bpe-m191.json \
  --merge-count 191 --verify-round-trip \
  --output-jsonl target/sql-elided-bpe/train-elided-bpe-m191.jsonl \
  --output-report target/sql-elided-bpe/train-elided-bpe-m191-report.json
```

## Validation commands

```bash
cargo test --features experimental --lib sql_schema_linking::elided_bpe_mask
cargo test --features experimental --lib sql_schema_linking
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
python3 -m pytest tests/python -q
```
