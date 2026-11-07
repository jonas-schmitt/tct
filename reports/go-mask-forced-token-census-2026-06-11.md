# Mask-Forced Token Census Over The Current-Plus Go Raw Streams

Date: 2026-06-11  
Commit: `53af6a378232901c15f9c1223a41176f0446622f`

## Claim boundary

Preliminary mask-forced token census over the 142-record current-plus Go
stdlib corpus (996,722 source bytes, 2,134,145 raw tokens, pinned 641
layout). Every record stream was walked token-by-token on the incremental
`RawSyntaxStream` handle against the same per-file candidate
grammar/sidecar/layout the record was emitted with (pre-M1-unification);
the streams pre-date the M1 unified-artifact switch and the elision
contract, so the absolute numbers will shift when the corpus is re-emitted
against the unified grammar. Census evidence only: it counts mask-singleton
positions, changes no stream contract, runs no BPE retraining or model
evaluation, and is not stdlib-scale (M2) or paper-grade evidence. The
BPE-level impact of elision requires retraining at matched vocab and is
deferred to the elision tranche (the research roadmap, "Elide Mask-Forced Tokens From
The Canonical Go Stream").

## Headline

**34.90% of all raw tokens are mask-forced** (744,778 of 2,134,145
positions have a singleton admissible mask, so the consumed token carries
zero information). Every forced token is a fixed structural role; 82.14%
of fixed structural-role tokens are forced at their position. Zero
positions had an empty mask or an inadmissible consumed token, and all 142
streams replayed to a complete state.

## Class table

| Token class | Tokens | Share of stream | Forced | Forced within class | Share of all forced |
|---|---|---|---|---|---|
| fixed structural role (ids 0..10, excl. payload_end) | 906,727 | 42.49% | 744,778 | 82.14% | 100.00% |
| payload_end (id 7) | 111,217 | 5.21% | 0 | 0.00% | 0.00% |
| alternative choice (ids 10..128) | 93,421 | 4.38% | 0 | 0.00% | 0.00% |
| utf8 payload byte (ids 128..384) | 1,011,328 | 47.39% | 0 | 0.00% | 0.00% |
| integer payload byte (ids 384..640) | 11,452 | 0.54% | 0 | 0.00% | 0.00% |
| **total** | **2,134,145** | **100.00%** | **744,778** | **34.90%** | **100.00%** |

Notes:

- `payload_end` is never forced: inside a payload the admissible mask
  always offers the payload byte range alongside the terminator (and only
  valid-UTF-8 byte prefixes admit the terminator at all).
- Alternative-choice positions are never forced: only multi-alternative
  nonterminal fields emit alternative node_start tokens (single-alternative
  slots emit the plain `node_start` fixed role), so every emitted
  alternative position had at least two admissible alternatives.
- Payload byte positions are never forced: the byte ranges are 256 wide
  and boolean payloads admit both boolean tokens.

## Implied elided stream

| Quantity | Raw | Elided (implied) |
|---|---|---|
| tokens | 2,134,145 | 1,389,367 |
| tokens/byte (B = 996,722) | 2.1412 | 1.3939 |

- Eliding forced tokens removes 34.90% of the raw stream; the elided
  stream is 65.10% of raw (raw-compression headroom factor 1.536x before
  any BPE).
- Per-record forced share ranges from 22.33% (`io/fs/fs.go`) to 48.50%
  (`internal/profile/proto.go`), median 35.65%; the full per-record table
  is in `go-mask-forced-token-census-2026-06-11.json`.

## Projected impact

The raw TCT stream drops from 2.141 to a projected 1.394 tokens/byte: a
third of the raw expansion that drives the measured BPE-parity gap
(G(4096) = 1.458, the Go BPE compression-parity probe report (not included))
is mask-forced structure. The post-elision BPE-level G(V) cannot be
projected from this census, because BPE merges already compress repeated
forced structure; the parity probe must be re-run on elided streams at
matched vocab after retraining, in the elision tranche that also carries
the breaking stream-contract handling (layout-diff gate, corpus
regeneration, `breaking_change` handoff, BPE retraining).

## Setup

- Census generator:
  `cargo test --test syntax_state_machine_replay_test go_mask_forced_token_share_census_over_current_plus_records -- --ignored --exact`
  (writes `target/forced-share-census.json`; the checked JSON beside this
  report is that output).
- Per-record grammars: each record's checked projection report pins
  `candidate_grammar_id` and the syntax-metadata fixture; the census
  lowers that per-file grammar through `GrammarIR -> StructuralModel` and
  walks the stream with `CompiledSyntaxModel::begin_stream`/`advance`.
- Stream integrity: every regenerated stream's length and `json_sha256`
  matched `expected_tct_token_count`/`expected_tct_tokens_sha256` in
  `tests/fixtures/go_syntax_corpus/training-report-ingestion-current-plus-v1.json`
  (manifest order, 142 records).
- A position counts as forced when the admissible-mask cardinality
  (finite token ids plus payload byte-range widths) is exactly 1 before
  consuming the canonical token.

## Files

- `reports/go-mask-forced-token-census-2026-06-11.json` (full per-record table and provenance)
- `target/forced-share-census.json` (regenerated census output)
