# Go elided stream: lossless layout under gofmt canonical form

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

## Why this document exists

The Go syntax route documents its canonical encoded stream as the *elided*
contract (`raw-syntax-stream-elided-v1`) over gofmt-canonical source. The
stdlib coverage gate reported ~100% "render_reparse_equal" for the full
denominator, and that number has been quoted as evidence that the compiled
stream reproduces the canonical bytes.

It does not. `render_reparse_equal` was derived from
`canonical_render.matches_source`, which the Go projection sidecar computes as
`bytes.Equal(format.Source(sourceBytes), sourceBytes)`
(`scripts/analysis/go_syntax_source_instance_project.go`,
`canonicalRenderEvidenceForSource`). That certifies that the **source** is
gofmt-idempotent. It never runs the decoder or the renderer, so it is blind to
what the token stream does or does not carry. In stdlib coverage mode the Rust
renderer was not invoked at all: the render attempt is gated on a
report-ingestion manifest opt-in (`raw_tree_render_attempt_selection`) that the
coverage path does not pass, so
`raw_decoded_tree_render_blocker.raw_tree_render_matches_canonical_source` was
hardcoded `false` and never written into a coverage record.

Running the round trip the contract actually promises —

```text
source -> projection -> canonical elided tokens
       -> decode_complete_tree_elided -> render_go_source_from_raw_tree
       -> gofmt -> byte compare against gofmt(source)
```

— over a pinned 200-file Go stdlib sample gives **72/200 byte-identical**
(`tests/syntax_go_canonical_render_fidelity_test.rs`, fixture
`tests/fixtures/go_syntax_corpus/canonical-render-fidelity-sample-v1.json`).
The failing 128 files hold the large majority of the sampled bytes.

Consequence for any claim that divides a model's NLL by *canonical* bytes: the
byte-BPE text baseline must predict every layout decision in those bytes, while
the compiled stream never encodes them. The two arms are not modelling the same
information, so a bits-per-canonical-byte comparison between them is not
like-for-like until this gap is closed.

## What gofmt preserves, and what is therefore a decision

`go/printer` normalises almost all whitespace, but it reads the *source line
numbers* of a handful of constructs and reproduces them. Those are the only
places where two different byte sequences survive `gofmt` unchanged, so those
are exactly the decisions a lossless stream must carry. Everything else is
forced and must **not** be encoded — the renderer can emit anything gofmt
normalises, and paying tokens for it would be waste.

| Construct | `go/printer` rule | Decision or forced |
|---|---|---|
| Blank line between statements in a block | `linebreak(..., min=1, ...)` with `nlimit` capped at `maxNewlines = 2` | **decision** (already encoded: `BlockStmt/CaseClause/CommClause.BlankLineBefore`) |
| Blank line between struct fields / interface methods | `p.fieldList`, `linebreak(..., 1, ...)` | **decision** (already encoded: `FieldList.BlankLineBefore`) |
| Line break between composite-literal elements / call arguments | `exprList` reads source lines | **decision** (already encoded: `CompositeLit/CallExpr.LineBreakBefore`) |
| Blank line between top-level declarations | `p.declList`: `min = 2` when the declaration token changes, the declaration has a doc comment, or it is a multi-line `func` (`newSection`); otherwise `min = 1` and the source line delta decides | **decision, only in the `min = 1` case** — not encoded before this change |
| Blank line between specs inside a grouped `GenDecl` (import grouping, const/var sections) | `p.genDecl`: `linebreak(..., 1, ...)` for every spec | **decision** — not encoded |
| One-line function body | `p.funcBody` prints on one line iff `headerSize + bodySize <= 100`, and `p.bodySize` returns `maxSize+1` when `Lbrace` and `Rbrace` are on different source lines or the body has more than 5 statements | **decision** — not encoded |
| One-line `struct{...}` / `interface{...}` | `p.fieldList`: `srcIsOneLine := lineFor(lbrace) == lineFor(rbrace)`, plus `isOneLineFieldList` | **decision** — not encoded |
| Placement of a root-only comment group relative to top-level declarations | comments are re-emitted at their recorded positions | **decision** — not encoded |
| Placement of a comment group that follows the last statement of a block | as above | **decision** — not encoded |
| Indentation, alignment, spacing, operator spacing, semicolons, parenthesisation | fully normalised | forced — never encode |

Two corollaries matter for cost:

1. **Alignment is derived, not free.** gofmt aligns a run of struct fields,
   composite-literal keys or line comments within an *alignment section*, and a
   blank line ends a section. Several observed "column width" mismatches are
   therefore knock-ons of a missing blank-line decision, not separate
   information.
2. **Polarity is worth choosing per field.** For top-level declarations gofmt
   *forces* a blank line in the common case, so the source can only differ by
   *suppressing* one. Encoding the present blank lines would cost an index per
   declaration; encoding the suppressed ones costs an index only in the rare
   case. The suppressed-polarity encoding is exactly as lossless, because a
   suppressed separator can only appear where the choice was free (the source
   is gofmt-canonical, so a forced separator is always present and is never
   listed).

## Measured residue

Pinned 200-file sample, measured with an earlier research build of the encoder,
classified by first differing line:

| bucket | files |
|---|---|
| byte_identical | 72 |
| mismatch_comment_anchor | 48 |
| mismatch_decl_spacing | 38 |
| mismatch_other | 17 |
| render_error | 15 |
| mismatch_one_line_body | 9 |
| mismatch_one_line_struct_type | 1 |

A whole-file line diff (every hunk, not just the first) attributes the 113
failing pairs that rendered at all as follows — a file usually fails for more
than one reason:

| class | hunks | files |
|---|---|---|
| comment placement | 317 | 47 |
| blank line missing in render | 122 | 41 |
| blank line present in render but not in source | 121 | 25 |
| one-line function body flattened to multi-line | 55 | 24 |
| one-line struct/interface flattened | 8 | 6 |
| other (mostly displaced package clause and alignment knock-ons) | 193 | 65 |

14 files fail on *nothing but* an extra blank line between consecutive
same-kind top-level declarations (`internal/goarch/zgoarch_*.go`,
`internal/goos/zgoos_*.go`, `internal/goexperiment/exp_*.go`,
`os/signal/signal_unix.go`, `io/export_test.go`, ...). That single decision is
the cheapest conversion in the sample.

With the first tranche of fields applied (same sample, same round trip):

| bucket | files |
|---|---|
| byte_identical | **93** |
| mismatch_comment_anchor | 30 |
| mismatch_decl_spacing | 29 |
| mismatch_other | 19 |
| mismatch_one_line_body | 14 |
| render_error | 13-14 |
| mismatch_one_line_struct_type | 1 |

72 -> 93 of 200 (36% -> 46.5%), and render errors 15 -> 13-14 (repeat runs
shared the projection output directory, so the render-error residue moved by
one between runs; the class did not regress). The residue is
dominated by the fields listed as remaining below plus the pre-existing v2
comment-marker defects; the first-difference buckets shift as earlier causes
are removed, so the per-bucket numbers are triage aids rather than independent
counts.

Two findings from that run are worth carrying forward. First, the file-scope
anchors initially *raised* the render-error count from 15 to 17, because
`push_anchored_file_scope_comments` consumed the next root comment slot
unconditionally and sometimes met an attached-owner blank marker; guarding the
consumption (and releasing the reserved anchor) turns that into a placement
mismatch the gate can bucket and brings the count back below the baseline. Second, 14 of the 21
converted files were failing on nothing but the top-level declaration
separator, which is what the sparse `NoBlankLineBefore` polarity was chosen
for.

## Minimal additional decision fields

The Go grammar already carries layout decisions as *synthesized trivia fields*:
optional repeated `[]int` index lists with policy
`synthesized_comment_trivia_metadata` and a single `integer_payload`
alternative, minted in `goAstSchemaSyntheticTriviaFields`
(`scripts/analysis/go_ast_schema_inventory.go`), computed by
`syntheticMetadataField` in the projection script, mirrored in
`go_syntax_synthetic_trivia_field_names`
(`src/syntax_go_raw_token_stream.rs`), and read by the renderer through
`indexed_terminal_payloads`. Single-alternative integer payloads never mint
alternative `node_start` tokens, so adding one does **not** change the Go token
layout.

The design keeps that mechanism rather than introducing boolean decision
fields. A required singular boolean would cost one elided token per node and
would have to be spelled by every stream builder and declarative template; an
optional repeated index list costs one `repeated_empty` token when unused and
is already handled generically everywhere.

### First tranche — implemented

| Field | Owner | Semantics | Cost when unused |
|---|---|---|---|
| `CommentBefore` | `File` | index of the top-level declaration each root-only comment group precedes; index `len(Decls)` anchors a group after the last declaration; repeats allowed | 1 token |
| `NoBlankLineBefore` | `File` | declaration separators the source suppressed (sparse polarity, see above) | 1 token |
| `CommentBefore` (widened) | `BlockStmt` | index `len(List)` anchors a comment group that follows the last statement (the slot before the closing brace) | none (same field) |
| `BlankLineBefore` (widened) | `BlockStmt` | index `len(List)` records a blank line before that trailing comment group | none (same field) |

The `BlockStmt` widening needs no schema change: it only extends the legal
index range of two existing fields, so streams that do not use the new slot are
bit-identical. The two `File` fields do change the checked inventory fixture.

This tranche is implemented end to end (inventory generator, checked inventory
fixture, projection sidecar, raw-token trivia table, inventory-synthesis field
orders, raw-tree renderer, grammar-artifact pin). It is **not** part of this tree, because the schema change invalidates pinned
per-file expectations across the
Go syntax test surface: 143 checked projection reports (mechanically
regenerable, and regenerated on that branch), seven report-ingestion manifests
whose ~142 records each pin `expected_tct_token_count` and
`expected_tct_tokens_sha256`, and hardcoded raw/canonical token counts in
`tests/syntax_go_source_instance_projection_test.rs`. Landing it means
regenerating all of that in one tranche; leaving it half-regenerated would
leave the branch red, which is worse than leaving the stream lossy while the
gate honestly reports it.

Measured effect of the tranche on the pinned sample: see "Measured residue".

### Remaining, in dependency order

| Field | Owner | Semantics | Class it closes |
|---|---|---|---|
| `BlankLineBefore` | `GenDecl` | index into `Specs` where a blank line precedes; positive polarity (gofmt never forces one here, and blank lines inside grouped declarations are rare) | import grouping, const/var sections |
| `SingleLineBody` | `FuncDecl`, `FuncLit` | contains `0` iff the body was on one source line | one-line function bodies |
| `SingleLineFields` | `StructType`, `InterfaceType` | contains `0` iff the brace pair was on one source line | one-line struct/interface types |
| `BlankLineBefore` | `CompositeLit` | index into `Elts` where a blank line precedes | blank lines inside composite literals |

Independently of any field, three renderer defects must be fixed:

- `should_insert_blank_before_statement` (`syntax_go_raw_tree_renderer.rs`)
  inserts a blank line by sniffing the statement text (it matches on the
  identifier `n` and the operators `!=`, `>`, `&&`). `BlockStmt.BlankLineBefore`
  is authoritative; the heuristic can only add blank lines the source did not
  have, and must be deleted once the remaining blank-line fields land.
- The 15 render errors are all v2 comment-marker consumption
  (`next File.Comments slot ... is an attached-owner blank marker` ×8,
  `statement trailing comment anchor expected one line comment` ×4,
  `IfStmt.TrailingComment` ×2, one `BlockStmt` dispatch).
- `statementListCommentBeforeIndexLines` keeps at most one anchored group per
  index. Two root-only groups separated by a blank line between the same pair
  of statements still lose their order.

### Expected token cost

Per file, from the sample's structure (median Go stdlib file: ~20 top-level
declarations, ~10 grouped declarations, ~30 function declarations, ~5 struct
types):

- an unused trivia field costs exactly one elided token (`repeated_empty` is
  one free choice among the repetition alternatives);
- a used index costs one repetition-continue token plus one token per decimal
  digit of the index plus the payload terminator, i.e. ~3 tokens for a
  two-digit index.

| Field | Expected tokens/file |
|---|---|
| `File.CommentBefore` | 1 + ~3 per file-scope comment group (typically 0–3) |
| `File.NoBlankLineBefore` | 1 + ~3 per suppressed separator (typically 0, up to ~20 in generated `zgoarch_*.go`-style files) |
| `GenDecl.BlankLineBefore` | ~1 per grouped declaration + ~3 per import-group boundary |
| `SingleLineBody` (FuncDecl/FuncLit) | ~1 per function/literal |
| `SingleLineFields` (StructType/InterfaceType) | ~1 per type |

Measured for the first tranche, over the first 1,000 enumerated stdlib files,
by running the coverage gate once per branch and comparing
`total_canonical_token_count`: **+33,450 elided tokens, +0.152%** (+33.5 tokens
per file), for +6.7 percentage points of byte-identical round trip. Raw
(expanded) tokens move +0.102% and mask-forced tokens +0.012%. Extrapolating
the per-field shape, the complete field set should stay inside 1–2% of the
canonical stream.

Note that the campaign's "73.66M tokens over 5,215 files" baseline is a **BPE**
count (vocab 2,048), not the canonical elided count, so a canonical-stream
delta cannot be quoted against it. The BPE-token delta has to be measured after
retraining, which any landing here requires anyway.

## Layout-diff class and pin handling

This is a **stream-content change**, not a token-layout change.

- The Go token layout (`tests/fixtures/syntax_structural_model/go-token-layout-pin-v1.json`)
  is unchanged: vocab size, `PAD_TOKEN`, role offsets, token ranges and the 47
  pinned `alternative_node_start_tokens` all depend only on nonterminal
  alternatives and finite domains. New single-alternative `integer_payload`
  fields mint no node-start ids (`go_pinned_alternative_node_start_tokens`
  filters on `GrammarTerminalKind::Nonterminal`).
- The grammar artifact pin
  (`tests/fixtures/syntax_structural_model/go-grammar-artifact-pin-v1.json`)
  **does** change: the checked inventory fixture sha256 changes, so the lowered
  structural-model digest and the artifact `content_sha256` change with it.
- The layout-diff class is therefore `layout_only_unchanged` in
  `tct-token-diff` terms — unchanged ranges, changed structural meaning.
  Per `DEVELOPMENT.md`, any non-`exact_reuse` class must be treated like a
  `tct-token-diff` breaking change for downstream raw-token corpora and BPE
  artifacts: **encoded Go corpora and every BPE merge table trained on them
  must be regenerated**, and no embedding, encoded-sequence or model reuse may
  be claimed across the change. Unchanged token ranges are not a semantic
  compatibility guarantee.
- The C# token layout pin is untouched and must stay bit-identical; C# remains
  on the expanded stream contract and is the cross-target check that this
  change did not disturb shared structural-model infrastructure.
- The tranche was developed on a separate research branch of the private
  repository, apart from its main line.

## Effect of the first tranche at stdlib scale

Coverage gate, first 1,000 of the 5,215 enumerated parseable stdlib files,
4 jobs:

| metric | baseline encoder | first layout tranche |
|---|---|---|
| `raw_state_machine_complete_count` | 1000 / 1000 | 1000 / 1000 |
| `token_only_render_round_trip_equal_count` | 292 (29.2%) | **359 (35.9%)** |
| `total_canonical_token_count` | 21,972,442 | 22,005,892 (+0.152%) |

Not one file failed the Go-side source gofmt-idempotence check on either
branch: the entire gap between the old ~100% figure and these numbers is the
token round trip. The file rate is lower here than on the pinned 200-file
sample (46.5%) because this slice is the enumeration's first 1,000 paths
(`archive/`, `bufio/`, `bytes/`, `cmd/...`), which are larger and more
comment-dense than a uniform random sample, and large files fail more often.

## Gate correction

The stdlib coverage gate now runs the real round trip and fails the file when
it does not reproduce the canonical bytes:

- `go_token_only_canonical_render_fidelity`
  (`src/syntax_go_tokenizer_generator.rs`) decodes the canonical elided stream
  with the pinned unified Go grammar artifact, renders, gofmts, and compares
  against the checked source hash and byte count. It never panics: a decode,
  render or gofmt failure is reported as a non-match plus a diagnostic so
  corpus-scale callers can bucket the residue.
- `GoStdlibTokenizerCoverageRecord` gained
  `token_only_render_round_trip_equal` (the round trip),
  `token_only_render_diagnostic`, and `source_gofmt_reparse_ok` (the old
  Go-side quantity, honestly named).
- The misleading `render_reparse_equal` name is gone. The record field, the
  report counters (`token_only_render_round_trip_equal_count`,
  `..._percent`, `selected_..._percent`), the accepted bucket and the blocked
  bucket all name the round trip, and the coverage target percentage is now
  measured against it. A file that is gofmt-idempotent but does not round-trip
  is bucketed `token_only_render_round_trip_not_equal` with the hash pair in
  the record error.

## Validation

Run the focused syntax gates for changes on this route:

```bash
./scripts/testing/test-syntax-backend-gates.sh
cargo test --features experimental --test syntax_generated_codec_gate_test
cargo test --features experimental --test syntax_backend_boundary_test
go run scripts/analysis/go_syntax_corpus_check.go --self-test
python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture minimal-class --self-test
```

plus the fidelity gate itself:

```bash
cargo test --release --features experimental --test syntax_go_canonical_render_fidelity_test -- --ignored --nocapture
```

The full workspace Rust suite is not required: this change touches the Go
syntax frontend, the Go grammar inventory fixture, the Go raw-tree renderer and
the Go stdlib coverage binary. It does not touch the JSON Schema compiler, the
JSON tokenizer runtime, generated JSON Schema crates, shared token/vocabulary
invariants, or the C# structural model. Escalation would be required if a
change here altered the token layout pin or a shared structural-model contract;
this one does not.
