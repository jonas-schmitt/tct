# Non-JSON Frontend Development Notes

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The gate fixtures under
> `tests/fixtures/syntax_backend/` likewise pin the names of research-repository
> files (`TODO.md`, `scripts/testing/test-quality-audit.py`) that are not shipped
> here; the gates compare those name lists and never read the files. The stable
> JSON Schema core is documented in `README.md`, `ARCHITECTURE.md`, and
> `DEVELOPMENT.md`.

The crate layers experimental non-JSON frontends on top of the stable JSON
Schema compiler core: a syntax/grammar-level route (parser-backed metadata ->
`GrammarIR` -> `StructuralModel` -> token layout -> raw syntax state machine ->
generated codec) exercised with Go and C# syntax fixtures, a SQL
schema-linking frontend, tree-sitter subset codecs, and an exact-mask
contract. They are off by default and compiled only with the `experimental`
Cargo feature, which also enables their binaries, their integration tests,
and (together with `python`) the SQL schema-linking Python bindings:

```bash
cargo build --features experimental
cargo test --features experimental
cargo clippy --all-targets --features experimental
```

None of these surfaces carry a stability promise; token layouts, APIs,
binaries, and gates may change or be removed without notice. The JSON Schema
path documented in `DEVELOPMENT.md` is unaffected, and default builds do not
compile any frontend code.

The sections below are the maintained command reference for these frontends.
Contract documents live under `docs/`.

## Known Failing Go Gates

The Go syntax track was carried over mid-flight from the private research
repository, whose last Go tranche ("Encode Go file-scope comment anchors and
declaration spacing") left several checked expectations out of sync with the
checked grammar artifact. The affected tests are marked
`#[ignore = "known failure: ..."]` so `cargo test --features experimental`
stays green; each ignore reason names the stale expectation. There are four
root causes:

- **Stale report-ingestion manifest.**
  `tests/fixtures/go_syntax_corpus/training-report-ingestion-current-plus-v1.json`
  records `package_metadata.grammar_artifact_content_sha256` `3f091674…`, but
  the live unified Go grammar artifact, like its pin fixture
  `tests/fixtures/syntax_structural_model/go-grammar-artifact-pin-v1.json`,
  hashes to `9a8f8f6c…`. The manifest's per-record `expected_tct_tokens_sha256`
  values are out of sync in the same way: for the checked httptest record it
  expects `efe6834b…` while the live canonical elided stream hashes to
  `f52f97a1…`.
- **Historical per-file projection report without its blocker.**
  `tests/fixtures/go_syntax_corpus/source-instance-projection-report-httptest-server-v1.json`
  keeps the per-file grammar id `go_httptest_server_source_file_syntax_v0`
  but does not record the `per_file_grammar_blocker` that the projection gate
  requires for per-file ids, so every test that loads all checked projection
  reports fails validation.
- **Stale Go inventory expectations.** `tests/go_ast_schema_inventory_test.rs`
  expects 13 synthesized trivia metadata fields and a `File` field order
  without `CommentBefore`/`NoBlankLineBefore`; the checked go1.24.3 inventory
  fixture carries 15 and includes both fields.
- **Stale token count.** `tests/syntax_go_source_instance_projection_test.rs`
  hardcodes 10553 raw tokens for the checked source-record emission; the live
  emission yields 10241.

Ignored tests per file:

| Test file | Ignored tests | Root cause |
| --- | --- | --- |
| `tests/syntax_go_source_instance_projection_test.rs` | 22 | 11 manifest, 10 per-file report, 1 token count |
| `tests/syntax_state_machine_replay_test.rs` | 8 | 6 per-file report, 2 manifest |
| `tests/go_ast_schema_inventory_test.rs` | 2 | inventory expectations |
| `tests/maintained_cli_binaries_test.rs` | 2 | manifest (Go tokenizer-generator CLI) |
| `tests/syntax_backend_boundary_test.rs` | 2 | manifest (state-API handoff) |
| `tests/syntax_generated_codec_gate_test.rs` | 1 | per-file report |
| `tests/syntax_go_unified_artifact_test.rs` | 1 | manifest (`expected_tct_tokens_sha256`) |

Run them, for example after regenerating the expectations with
`scripts/analysis/go_syntax_current_plus_batch_intake.py --refresh-existing
--apply`, with:

```bash
cargo test --features experimental --test syntax_go_source_instance_projection_test -- --ignored
```

and the same `-- --ignored` form for the other files in the table.
`--ignored` also runs every other ignored test in the named file (opt-in
breadth checks, and in `tests/syntax_backend_boundary_test.rs` the
`regenerate_*` fixture writers), so add a name filter where needed, for
example:

```bash
cargo test --features experimental --test syntax_backend_boundary_test generated_codec_state_api -- --ignored
```

Remove an ignore attribute in the same change that fixes its expectation.

The Go integration tests resolve the pinned Go toolchain themselves (see
[Pinned Go Toolchain Resolution](#pinned-go-toolchain-resolution)), so a plain
`cargo test --features experimental` works whenever the pinned release is
installed in one of the discovered locations; it does not have to be first on
`PATH`. When no matching toolchain is found, the Go projection tests fail with
the Go parser's version mismatch.

## Pinned Go Toolchain Resolution

The checked Go fixtures were produced by one Go release, the pin (currently
`go1.24.3`, defined by `GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION` in
`src/go_ast_schema_inventory.rs`). The Rust tests (`tct::go_toolchain`) and the
shell helpers (`scripts/testing/resolve-pinned-go.sh`) look for it in the same
order:

1. `TCT_GO_BIN`, when set; its version must equal the pin or resolution fails.
2. `go` on `PATH`, when `go version` equals the pin.
3. `$HOME/sdk/go1.24.3/bin/go`, then `$HOME/go/bin/go1.24.3`.
4. Otherwise resolution fails.

`gofmt` comes from `TCT_GOFMT_BIN` when set, else from the resolved
toolchain's `GOROOT`. To install the pinned release next to your normal Go:

```bash
go install golang.org/dl/go1.24.3@latest && go1.24.3 download
```

Shell commands and `go run` scripts that you start directly (not through a Rust
test) need the pin in their environment:

```bash
eval "$(scripts/testing/resolve-pinned-go.sh --export --repo-root "$PWD")"
```

`scripts/testing/test-syntax-backend-gates.sh` does this itself.

## Syntax Frontend Analysis

Source-language work is currently syntax/grammar-level. The Go north-star route
is parser-backed metadata -> `GrammarIR` -> `StructuralModel` -> token layout
-> raw syntax state machine -> generated codec/tokenizer. C# is the
syntax-level cross-target check. GoIR, ArenaIR, route-handoff provenance, and
exact-BPE source-prefix materialization are legacy/control surfaces, not current
validity-by-construction evidence.
Until the Go source-record raw-token generator core is reusable, do not use new
scratch witnesses, C# breadth, BPE/model artifacts, GoIR/ArenaIR, report
ladders, or source-file batches as the default next implementation unit unless
a precise Go generator-core blocker requires that evidence.

Use the syntax backend gates and parser-backed corpus/gap helpers for current
Go/C# work:

```bash
./scripts/testing/test-syntax-backend-gates.sh
python3 scripts/testing/run-syntax-validation-matrix.py --output target/tct-syntax-validation-matrix/execution-report-v1.json
python3 scripts/testing/run-syntax-validation-matrix.py --validate-report target/tct-syntax-validation-matrix/execution-report-v1.json
cargo test --features experimental --test syntax_generated_codec_gate_test
cargo test --features experimental --test syntax_backend_boundary_test
cargo test --features experimental --test syntax_structural_model_cross_target_equivalence_test
python scripts/analysis/route_source_manifest.py --self-test
python scripts/analysis/route_source_manifest.py \
  --target go-syntax \
  --source-root tests/fixtures/go_syntax_corpus/src \
  --output tests/fixtures/go_syntax_corpus/sources.txt \
  --json-output target/tct-analysis/go-syntax-corpus-route-manifest.json \
  --min-files 5 \
  --corpus-id go-stdlib-real-source-syntax-corpus-v1 \
  --corpus-source https://cs.opensource.google/go/go/+/refs/tags/go1.24.3:src \
  --corpus-license BSD-3-Clause \
  --selection-note 'checked real-source Go syntax sidecar corpus'
go run scripts/analysis/go_syntax_metadata_smoke.go --fixture minimal-function --self-test
go run scripts/analysis/go_syntax_corpus_check.go --self-test
go run scripts/analysis/go_syntax_corpus_check.go \
  --manifest tests/fixtures/go_syntax_corpus/manifest-v1.json \
  --json-output target/tct-analysis/go-syntax-corpus-check/report.json
python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture minimal-class --self-test
python3 scripts/analysis/csharp_real_source_metadata_gaps.py --self-test
./scripts/testing/test-csharp-real-source-metadata-gaps.sh
```

Checked Go source-instance tests require a Go toolchain matching the fixture
provenance, currently `go1.24.3`; the Rust tests resolve it as described in
[Pinned Go Toolchain Resolution](#pinned-go-toolchain-resolution). For commands
you start directly, use `TCT_GO_BIN` where the command supports it, or export
the pin first. The maintained syntax backend gate and local CI wrapper
export `TCT_REQUIRE_GO_SIDECARS=1`, so missing `go`/`gofmt` sidecars or a
non-`go1.24.3` `go version` fail instead of silently skipping render/reparse
checks. For ad hoc non-strict runs, unset or set `TCT_REQUIRE_GO_SIDECARS=0`;
the affected Rust tests emit `TCT_GO_SIDECAR_SKIP` with the exact reason.

`go_syntax_corpus_select.go` records both `go_version` and `go_root` in its
JSON report and rejects `--source-revision go*` values that do not match the
runtime Go version. Use `--allow-go-version-mismatch` only for provisional
evidence; the report records this as `go_version_check: mismatch_overridden`.

### Go Token Layout Pin And Layout-Diff Gate

The Go syntax token layouts (`go_syntax_token_layout`,
`go_syntax_source_file_token_layout`: structural 0..128, utf8 byte payload
128..384, integer payload 384..640, pad 640, vocab 641) and the pinned
alternative node_start token table in the checked inventory fixture
(`tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json`,
`structural_token_assignment` block) are pinned by the checked layout-pin
fixture `tests/fixtures/syntax_structural_model/go-token-layout-pin-v1.json`.
The fixture records the current layout descriptors, the superseded 577-era
descriptors, the serialized `SyntaxTokenLayoutDiff` transition (class
`breaking_change` for the 577 -> 641 widening), and the stream-contract
transition legs for the deliberate elision break (`previous_contract` plus
`stream_contract_transition`: 641-expanded -> 641-elided, `breaking_change`
with zero moved ranges). Every Go sidecar (the full inventory grammar and
each per-record `go_*_syntax_v0` grammar) resolves multi-alternative
node_start tokens through the pinned table; the alphabetical fallback applies
only to toy and C# grammars. New multi-alternative nonterminal node kinds
append at max assigned token id + 1 in the `structural_token_assignment`
block; existing ids never renumber.

### C# Token Layout Pin (Not A Tree-Sitter Grammar Pin)

The C# product syntax token layout (`csharp_syntax_token_layout`: structural
0..64, utf8 byte payload 64..320, u8 integer payload 320..576, pad 576,
vocab 577) is pinned by
`tests/fixtures/syntax_structural_model/csharp-token-layout-pin-v1.json`.
Absolute renumber protection for the syntax-level cross-target check uses the
same layout-diff machinery as Go (`src/syntax_token_layout_diff.rs`,
`CSHARP_TOKEN_LAYOUT_PIN_FIXTURE_PATH`). The fixture is generated by running
`csharp_syntax_token_layout()` (never hand-written absolute IDs) and records
`stream_contract: raw-syntax-stream-expanded-v1` so it cannot be confused with
Go's live elided contract. C# multi-alternative node_start IDs stay
alphabetical per grammar sidecar (empty `alternative_node_start_tokens` on
the product layout pin; no global inventory table like Go).

**Disambiguation:** this is a **token-layout pin**, not a tree-sitter grammar
pin. The project adds no new tree-sitter language or grammar pins; that does
**not** forbid locking the C# structural token layout against silent renumber,
which is what this fixture does.

Enforcement:

```bash
cargo test --features experimental --test syntax_backend_boundary_test csharp_token_layout_pin_fixture_matches_live_layout -- --exact
```

Deliberate layout break regenerator (treat like a `tct-token-diff`
`breaking_change` for downstream raw-token corpora and BPE artifacts; do not
update merely to make a refactor pass):

```bash
cargo test --features experimental --test syntax_backend_boundary_test regenerate_csharp_token_layout_pin_fixture -- --ignored
```

### Unified Pinned Go Grammar Artifact

All current-plus records validate and emit against one pinned
inventory-derived artifact (`tct-go-grammar-artifact-v1`,
`src/syntax_go_unified_artifact.rs`): the checked go1.24.3 inventory lowered
once into the unified grammar `go_ast_schema_inventory_go1_24_3`, the
source-file token layout, the pinned alternative node_start table, the
structural-model digest, the live stream contract, and the generated-codec
fixture sha bound by `content_sha256`. The checked pin fixture is
`tests/fixtures/syntax_structural_model/go-grammar-artifact-pin-v1.json`,
enforced by
`cargo test --features experimental --test syntax_backend_boundary_test go_grammar_artifact_pin_fixture_matches_live_artifact -- --exact`
and regenerated only via the ignored
`regenerate_go_grammar_artifact_pin_fixture` test for a deliberate identity
break. Checked projection reports carry the unified grammar id as
`candidate_grammar_id` and the inventory fixture as their metadata input
(inventory-as-metadata: `scripts/analysis/go_syntax_source_instance_project.go`
consumes the inventory's `grammar_ir_rules` directly and classifies
finite-domain terminals, presence positions, and channel directions onto the
inventory alternatives). The current-plus report-ingestion manifest pins
`grammar_artifact_id`, `grammar_artifact_content_sha256`, and
`stream_contract` in `package_metadata`; the manifest depth gate requires
them and revalidates every record against the live artifact. Per-file
metadata fixtures under `tests/fixtures/go_syntax_metadata/` remain checked
in as historical parser-provenance evidence only. The inventory generator
(`scripts/analysis/go_ast_schema_inventory.go`) preserves the pinned
structural token assignment from the checked fixture and appends the
synthesized comment/layout trivia metadata fields (CommentBefore,
TrailingComment, BlankLineBefore, LineBreakBefore) to the grammar rules so
comments stay representable in the unified grammar. That set does **not** yet
cover every gofmt-preserved layout decision, so the elided stream is not
byte-lossless to gofmt-canonical source; the gofmt decision inventory, the
missing fields, their token cost and the layout-diff class are in
`docs/go-elided-stream-lossless-layout.md`, and the measured residue is the
ignored `syntax_go_canonical_render_fidelity_test`.

### Elided Canonical Stream Contract

The live canonical raw-syntax stream contract is
`raw-syntax-stream-elided-v1` (`LIVE_RAW_SYNTAX_STREAM_CONTRACT`,
`src/syntax_token_layout_diff.rs`): a canonical encoded Go stream contains no
token whose admissible mask at its position is a singleton.

SQL adds a policy-tagged variant: compiling the SQL runtime with the
structural join-ON policy (`SqlJoinOnPolicy::Structural`;
`join_on_policy="structural"` on the Python constructor,
`--join-on-policy structural` on `tct-sql-elided-encode-corpus`) narrows
join-key column admissibility (equality ON pairs must combine the
just-joined table with an earlier in-scope table), so mask-forced positions
— and therefore canonical elided streams — differ from the `off` policy
while the token layout is unchanged. Such streams carry
`raw-syntax-stream-elided-v1+joinon-structural`
(`RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL`); a policy flip
with identical ranges classifies as `breaking_change` in the layout diff,
and strict elided decode rejects cross-policy streams
(`forced_token_in_elided_stream` / `join_on_structural_violation`), so
corpora and BPE artifacts from different policies cannot silently mix.

SQL adds a second, independent policy: **factored column addressing**
(`SqlColumnAddressing::Factored`; `column_addressing="factored"` on the
Python constructor, `--column-addressing factored` on
`tct-sql-elided-encode-corpus`). A column reference is emitted as two
positional menu digits — the catalog table index, masked to the tables
currently in scope, then the column index inside that table — instead of one
radix-coded ordinal over the flat scope-relative concatenation. The second
digit's cardinality depends on the first digit's value, which the radix engine
cannot express, so the engine carries a dedicated `MenuSiteKind::FactoredScopeColumns`
and a sibling digit codec (`src/syntax_state_machine/factored_menu_codec.rs`);
mask, apply, and encode all route through `factored_table_slice` so
mask == accept == encode holds by construction. The decoded value stays the
flat scope-relative ordinal, so the AST, render, and join-ON frame memory are
unchanged; only the token stream differs. Cost is one extra token per column
reference **only where the table digit is a free choice** — single-table
scopes force it, so it is elided and those queries pay nothing.

The two SQL policies compose, so the contract tag names the combination:
`raw-syntax-stream-elided-v1+factored-columns`
(`RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS`) and
`raw-syntax-stream-elided-v1+joinon-structural+factored-columns`
(`RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS`).
Every flip between the four policy tags classifies as `breaking_change` in
the layout diff even though token ranges are identical.

SQL rank-1 adds a third independent policy: **visit-all remaining fields**
(`visit_all_fields=True` on the Python constructor,
`--visit-all-fields` on `tct-sql-elided-encode-corpus`). Encode already
walks every field; historical decode also admits `node_end` as a second
spelling of "skip remaining trailing optionals", which is a free H=0
decision (`table_ref._`, `from_clause._`) immediately before the real
field decision. Visit-all makes that skip illegal, so `field_start` is
mask-forced and elided, while alias / join-list alternatives stay
representable. The same flag also folds `select_list` ×
`select_items.distinct` into a 3-way alternative (`star` |
`select_items` | `select_distinct_items`). Nested selects inherit the
fold; historical subquery lowering must not emit `select_items.distinct`
under this policy. Visit-all also moves `having` under `group_by_clause`
(HAVING requires GROUP BY), so queries without GROUP BY do not emit a
`select_statement.having` skip, and folds LIMIT presence into a 6-way
`select_list` (sequential P6 fields; LIMIT value stays a child of
`limited_*`), and folds ORDER BY presence the same way (`ordered_*` /
`ordered_limited_*`, 12-way). Render quotes subset-keyword identifiers
(`"count"`) so decode→reparse is not an aggregate. The locked-cell successor tag is
`raw-syntax-stream-elided-v1+joinon-structural+factored-columns+visit-all-fields`.
Do not evaluate an L6-v1 checkpoint under this contract. Factored binding
fails closed at bind time when a catalog has more tables, or a table more
columns, than the menu band width. Encode elides
forced positions (`CompiledSyntaxModel::elide_forced_raw_tokens`); decode
re-derives them deterministically from the live mask
(`expand_elided_raw_tokens`, `strict_decode_complete_elided`,
`decode_complete_tree_elided`, incremental `begin_elided_stream`). Strict
elided decode rejects a mask-forced token even when it equals the forced
value, which makes the canonical encoding unique. The expanded stream remains
the state machine's native semantics and stays the evidence layer for
segments, decoded-event offsets, replay properties, witness corpora, and the
rendered generated-codec fixtures; the canonical (elided) stream is what
`expected_tct_*` manifest expectations, downstream raw-token corpora, and
BPE artifacts consume. Generated-codec reports carry both:
`tct_tokens`/`tct_token_count`/`tct_tokens_sha256` are canonical, with
`stream_contract`, `expanded_token_count`, and `forced_token_count` recording
the elision evidence; the tokenizer-generator report mirrors this with
`canonical_token_count`/`canonical_tokens_sha256`/`canonical_max_token_id`
next to the expanded `raw_token_*` fields. On the source-record path those
canonical fields are owned by `GoSourceRecordRawTokenEmission` /
`GoSourceRecordGeneratorCore`
(`src/syntax_go_source_record_generator_core.rs`). That module also owns the
Go source-record empty-prefix, expanded selected-completion, incomplete-prefix,
and token-only evidence builders; elided selected-completion evidence walks the
canonical stream through the shared
`RawScratchGeneratorCore::elided_selected_completion` surface
(`CompiledSyntaxModel::begin_elided_stream` / expand / elided decode). Inventory
scratch-seed token-only / expanded selected-completion walks go through
`GoSourceRecordInventoryScratchSeedWalk` in the same module (shared
`RawScratchGeneratorCore::selected_completion` + sample steps). That walk also
elides each seed's expanded stream and stamps nested
`GoInventoryScratchSeedElidedSelectedCompletionEvidence` via the same shared
elided selected-completion surface, so seed evidence is not expanded-only.
Multi-token inventory seeds additionally probe incomplete-prefix rejection
through shared `RawScratchGeneratorCore::incomplete_prefix` and stamp nested
`GoInventoryScratchSeedIncompletePrefixRejectionEvidence` (partial prefix,
strict `premature_eof` rejection, continuation mask, selected next token,
final complete stream). The inventory scratch-seed **batch** also stamps
inventory-model empty-prefix evidence
(`GoInventoryScratchSeedEmptyPrefixEvidence` via shared
`RawScratchGeneratorCore::empty_prefix` against the inventory compiled model /
`GoSourceFile` root, including pad/malformed rejection) so inventory seeds and
empty-prefix start share one generator-core surface rather than only
source-record empty-prefix; the scratch-generation gate mirrors it as
`go_ast_inventory_empty_prefix` and validates seed first-tokens under that
mask. Inventory scratch-seed evidence summaries
(`GoTokenizerGeneratorManifestDepthScratchSeedSummary::from_inventory_seed`,
including frontier coverage, render/gofmt/reparse stamping, nested elided
selected-completion, and nested incomplete-prefix rejection) and the full
inventory scratch-seed **batch**
(`GoSourceRecordInventoryScratchSeedBatch::from_inventory` — inventory lower
→ structural model → compiled model → empty-prefix + seed list → per-seed
summaries + aggregates + package-var representative) are also
generator-core-owned. The inventory **seed+empty-prefix sub-gate**
(`GoSourceRecordInventoryScratchSubGate::from_inventory`) packages that
batch with generator-core / raw-token-emission API stamps and fixed claim
flags; the tokenizer-generator megamodule re-exports those types rather
than lowering inventory, assembling seed reports, or stamping
generator-core API/claim identity itself.
**Manifest-depth per-record readiness aggregation** is also
generator-core-owned:
`GoSourceRecordManifestDepthRecordReadiness::from_record_views` (via
`GoSourceRecordManifestDepthRecordReadinessView` /
`GoTokenizerGeneratorManifestDepthRecord::readiness_view`) counts
empty-prefix / expanded selected-completion /
**elided selected-completion** / incomplete-prefix / token-only /
render-reparse readiness over the record slice and
`with_inventory_seed_readiness` applies the inventory-seed conjunction.
Elided readiness requires status
`go_source_record_elided_selected_completion_ready`, live stream
contract, expand parity, every selected elided token admitted at a
non-forced position, EOF-valid final state, and reusable elided decode.
**Full scratch-generation gate assembly** is generator-core-owned:
`GoTokenizerGeneratorManifestDepthScratchGenerationGate::from_oracle_and_record_views`
merges oracle cutover attachment
(`GoSourceRecordOracleCutoverAttachmentView`), representative paths
(`GoSourceRecordManifestDepthRepresentativeView`), inventory sub-gate,
and record readiness under status/scope/claim constants, including
`elided_selected_completion_ready` parallel to
`selected_completion_smoke_ready`; gate validation re-runs through the
same surface. The megamodule only maps oracle evidence + records into
those views. **Manifest-depth record source-record evidence field
stamping** is generator-core-owned:
`GoSourceRecordManifestDepthRecordSourceFields::from_generator_evidence`
stamps API/stream identity plus empty-prefix / selected-completion /
**elided selected-completion** / incomplete-prefix / token-only
decode-render from retained `GoSourceRecordGeneratorEvidence` (artifacts
keep `source_record_generation`); report construction and record
construction both consume that stamp, with record construction requiring
report absorbed-field parity via `require_absorbed_report_parity`
(including elided selected-completion).
`GoSourceRecordElidedSelectedCompletionEvidence::validate_against_stream_identity`
re-validates the stamped elided path against report/record stream
identity without requiring the full emission object.
**Manifest-depth report-level readiness flags** are generator-core-owned:
`GoSourceRecordManifestDepthReportReadiness::from_scratch_generation_gate`
mirrors expanded selected-completion / **elided selected-completion** /
incomplete-prefix / token-only readiness from the assembled
`scratch_generation_gate` onto the tokenizer-generator report top-level
fields (including report field `elided_selected_completion_ready`);
`validate_against_gate` rejects any report/gate divergence so claims
cannot hardcode independently of gate readiness.
**Manifest-depth report-level vocabulary / raw-SM completeness flags**
are generator-core-owned:
`GoSourceRecordManifestDepthVocabSmCompleteness::from_record_views`
aggregates per-record `raw_tokens_below_vocab_size`, `pad_token_absent`,
and `raw_state_machine_complete` (plus aggregate max-token vs VOCAB_SIZE)
onto the report top-level fields;
`validate_against_record_views` rejects report/record divergence so those
claims cannot hardcode independently of record token accounting.
**Manifest-depth report-level canonical-render / go-reparse readiness**
is generator-core-owned:
`GoSourceRecordManifestDepthCanonicalRenderReadiness::from_record_views`
aggregates per-record `canonical_render_ready` plus gofmt/go-parser
sidecar pass bits onto report top-level `canonical_render_ready` and
pipeline `canonical_render_ready` / `go_reparse_ready`;
`validate_against_record_views` rejects report/pipeline divergence so
render claims cannot hardcode independently of record sidecar authority.
**Pipeline encode/decode stage readiness** is generator-core-owned:
`GoSourceRecordManifestDepthEncodeDecodeReadiness::from_vocab_sm_completeness`
derives `pipeline.raw_token_encode_ready` (token-emission integrity:
`raw_tokens_below_vocab_size` ∧ `pad_token_absent`) and
`pipeline.raw_state_machine_decode_ready` (`raw_state_machine_complete`)
from the vocab/SM completeness aggregate; `validate_against_record_views`
rejects pipeline/aggregate divergence. The single-record report path uses
`from_token_emission_and_sm_complete` for the same pure conjunctions.
**Process-only pipeline stage / exclusion claims** are generator-core-owned
constant bundles (not invented record evidence):
`GoSourceRecordPipelineProcessStageClaims::SUCCESS` stamps
`parser_backed_metadata_loaded` / `grammar_ir_lowered` /
`structural_model_lowered` / `token_layout_compiled` /
`raw_state_machine_compiled` as documented process claims for a successful
inventory-path run; `GoSourceRecordPipelineProcessExclusionClaims::SUCCESS`
stamps ArenaIR / route-handoff / BPE-authority must-stay-false;
`validate_against_success_bundle` rejects pipeline divergence from those
bundles. **Full pipeline field stamp assembly** is generator-core-owned:
`GoSourceRecordPipelineFieldStamp::from_encode_decode_and_canonical_render`
(manifest-depth) / `from_encode_decode_and_render_flags` (single-record)
folds process stages + exclusions + encode/decode + canonical-render
readiness into one view; the megamodule maps
`GoTokenizerGeneratorPipeline::from_field_stamp` rather than composing the
four surfaces at each construction site. **Manifest-depth report claim
field stamp assembly** is generator-core-owned:
`GoSourceRecordManifestDepthReportClaimStamp::from_gate_and_record_views`
folds vocab/SM completeness + gate-derived readiness + canonical-render
+ the full pipeline stamp into one view; construction maps report claim
fields and pipeline from that single stamp, and report validate rebuilds
it via `validate_against_gate_and_record_views` so dual-site hardcoding
cannot drift from gate/record aggregates. **Single-record canonical-render
pure-conjunction flags** are generator-core-owned:
`GoSourceRecordCanonicalRenderReadinessFlags::from_checked_report` /
`from_canonical_render` derive `canonical_render_ready` (gofmt passed ∧
go_parser passed ∧ matches_source ∧ canonical_sha256 == source.sha256)
and `go_reparse_ready` (go_parser passed) so the single-record report
path, selected-completion smoke, and token-only decode/render stamp the
same formula (matching emission sidecar readiness). The megamodule only
wires generated-codec parity and raw-tree render attempts when building
records. A stream-contract change is a `breaking_change`
through the layout-diff gate even with identical token ranges: raw-token
corpora and BPE artifacts built from expanded-v1 streams must be regenerated
or explicitly migrated before they are used with elided-v1 streams. C#
streams are unaffected: descriptors without an explicit `stream_contract`
field parse as expanded-v1.

The enforcement gate is:

```bash
cargo test --features experimental --test syntax_backend_boundary_test go_token_layout_pin_fixture_matches_live_layouts_and_breaking_transition -- --exact
```

It rebuilds the live descriptors from the layouts, the embedded pinned table,
and the raw role offsets, recomputes the previous->current diff, and fails any
layout-affecting change that does not go through a deliberate pin-fixture
update. For a deliberate layout break, update the regenerator's
previous-layout builders to the just-superseded layout, then regenerate the
pin fixture:

```bash
cargo test --features experimental --test syntax_backend_boundary_test regenerate_go_token_layout_pin_fixture -- --ignored
```

After a deliberate Go layout change, regenerate the current-plus corpus
fixtures (the report-ingestion manifest plus every checked projection report)
with the refresh mode:

```bash
python3 scripts/analysis/go_syntax_current_plus_batch_intake.py --refresh-existing --apply
```

The refresh re-runs the full per-record pipeline; checked syntax metadata that
predates the current metadata generator is regenerated in place before
re-validation. A layout break invalidates previously encoded raw-token
corpora and BPE artifacts; the layout-diff report is migration input only and
does not prove embedding, BPE, or encoded-sequence reuse.

Generated source-corpus, source-program, and generated-codec reports stay
under `target/` unless a later curation tranche intentionally promotes them. route-input preflights for retained non-JSON targets stay on C# (and Go
syntax source manifests). Java and TypeScript deferred frontend investigation
helpers were retired; reintroduce them only when the research strategy notes
promotes those targets.

For C# parity, use the Roslyn-backed syntax metadata helpers and route-owned
syntax gates. The checked C# real-source metadata-gap helper consumes the
selected corpus and reports unsupported syntax-metadata classes before
generated-codec expansion. The generated-codec corpus and completion handoff
then preserve supported whole-source records for the current C# files while
source-program validity, type correctness, arena IR, and stable backend claims
remain blocked.
The GoIR and target-specific arena IR utilities were retired on 2026-06-10;
only adapter rejection tests and their fixtures remain. Do not add new
maintained workflow examples for arena IR surfaces.

Keep the validation layers separate:

- Schema validity means a decoded JSON value satisfies the JSON Schema accepted
  by the generated tokenizer.
- Tokenizer round-trip validity means a supported instance can travel through
  `encode`/`decode` without semantic data loss and with consumed-token
  accounting matching the chosen strict or recovery path.
- Functional validation means an external domain tool accepts or rejects the
  decoded artifact for a real task, such as Kubernetes admission checks, ESLint
  config loading, or TypeScript compiler configuration. It is downstream
  evidence, not a stronger form of schema validity. Keep domain tools outside
  generated tokenizer hot paths.

---


### SQL elided-BPE PyO3 API

Binding-only surface for constrained decode in BPE space over the **elided**
SQL schema-linking stream. Semantics live in
`src/sql_schema_linking/elided_bpe_mask.rs` (exact admissible mask; PAD never
admissible). The SQL token layout is unchanged (`exact_reuse`).

`merge_table_path` is a **filesystem path** to a JSON BPE merge table (same
loader as `tct-sql-elided-bpe-validate --bpe-merges` /
`BpeMergeTable::from_file`), not JSON text.

```python
from tct import SqlSchemaLinkingRuntime

rt = SqlSchemaLinkingRuntime(catalog_json, catalog_sha256, db_id)
# or: rt = SqlSchemaLinkingRuntime.fixture()
# Join-key masking (structural join-ON policy; default "off"):
#   rt = SqlSchemaLinkingRuntime(catalog_json, catalog_sha256, db_id,
#                                join_on_policy="structural")
# Under "structural", equality join keys must pair one column of the
# just-joined table with one column of an earlier in-scope table; canonical
# elided streams CHANGE (singleton menus become forced and are elided), so
# rt.stream_contract is policy-tagged
# ("raw-syntax-stream-elided-v1+joinon-structural") and corpora/BPE artifacts
# from different policies must never mix. rt.join_on_policy and the
# capability report's join_on_policy record the compiled policy.
# Column addressing (factored (table, column) pairs; default "flat"):
#   rt = SqlSchemaLinkingRuntime(catalog_json, catalog_sha256, db_id,
#                                column_addressing="factored")
# Under "factored", a column reference is emitted as TWO menu digits — the
# catalog table index (masked to the tables currently in scope), then the
# column index inside that table — instead of one radix-coded ordinal over
# the flat scope-relative concatenation. Both digits are exactly the numbers
# a per-table schema prompt already shows, and a column's address no longer
# moves with FROM order. The AST value is unchanged (still the flat ordinal),
# so decode/render are identical; only the token stream differs. The two SQL
# policies compose and rt.stream_contract names the combination.
# rt.column_addressing and the capability report's column_addressing record
# the compiled policy.
# Rank-1 visit-all remaining fields (default false / historical):
#   rt = SqlSchemaLinkingRuntime(..., visit_all_fields=True)
# Forces visitation of trailing optional fields so skip-vs-visit (`rule._`)
# is mask-forced and elided. Alias / join-list alternatives stay
# representable. Stream-contract break; do not evaluate L6-v1 checkpoints
# under this policy. Tagged `+visit-all-fields` on `stream_contract`.

# Base elided walker: clone for speculative free-choice probes.
elided_st = rt.begin_elided_tct_decode()
probe = elided_st.clone()  # independent SqlElidedTctDecodeState

# Live-site introspection (expanded, elided, and BPE walkers alike):
site = elided_st.active_site()   # {"active_rule", "active_field", "status",
                                 #  "stack_depth", "eof_valid"} — role must
                                 # come from here, never from a token band.
menu = elided_st.active_menu()   # {"kind": none|table|column_table|column|
                                 #  column_flat, "referents": [{"ordinal",
                                 #  "name"}], "digits_seen", "width",
                                 #  "resolved", "addressing"}
# "referents" carries catalog *_original names only when "resolved" is true:
# the final radix digit or either factored digit. On a leading digit of a
# wide (width > 1) radix menu each admissible value spans a range of
# ordinals, and after digits_seen == width the ordinal is already chosen;
# both report resolved == false with empty referents. On the BPE walker both
# calls describe the site of the *next base decision* (meaningful between
# BPE tokens).

# BPE-space walker: exact mask via elided_bpe_mask::{bpe_token_is_admissible,
# try_apply_bpe_token}; merge_count truncates the table.
bpe_st = rt.begin_elided_bpe_tct_decode(merge_table_path, merge_count)
# Layout / pin fields (getters):
#   bpe_st.base_vocab_size, bpe_st.bpe_start, bpe_st.total_vocab_size,
#   bpe_st.pad_token, bpe_st.bpe_vocab_size
allowed = bpe_st.allowed_token_ids()          # list[int], PAD never included
base_expansion = bpe_st.apply(bpe_id)         # list[int] elided base tokens
# apply raises ValueError naming rejected_bpe_id when not admissible
done = bpe_st.is_complete()
n = bpe_st.consumed_bpe_tokens()
expanded = bpe_st.expanded_tokens()           # expanded-layer cursor tokens
sql = bpe_st.render_sql()                     # complete streams only
```

Focused checks:

```bash
cargo test --features python,experimental --lib python
# Gold-stream replay (uses host artifacts when present; else fixture fallback):
python3 -m pytest tests/python/test_sql_elided_bpe_mask_binding.py -q
```

Build the root extension wheel with maturin (`pyproject.toml` selects
`python-extension`); add `-F experimental` so the wheel includes these SQL
bindings. Prefer a valid PEP 427 platform tag in the filename
(e.g. `tct-0.1.0-cp312-cp312-linux_aarch64.whl`); do not invent a mangled tag
segment such as `STEP4-...`.

Generated Cargo/toolchain reproducibility checks are also explicit because they
build generated artifacts:

```bash
./scripts/testing/test-generated-reproducibility.sh
```

Run ignored tests locally only when intentionally exercising every ignored,
report-only, external-tooling, and slow generated-crate check. For normal slow
validation, prefer the narrower commands in `DEVELOPMENT.md`.

```bash
# Run every ignored test deliberately
cargo test -- --ignored --nocapture

# Run specific test file's ignored tests
cargo test --test random_token_github_workflow_test -- --ignored --nocapture

# Generate schema compatibility report
SCHEMA_NAME=package cargo test --test schema_compatibility_report test_individual -- --ignored --nocapture
```

