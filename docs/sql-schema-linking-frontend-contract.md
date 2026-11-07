# SQL Schema-Linking Frontend Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Document path: `docs/sql-schema-linking-frontend-contract.md`.

Status: **Implemented (experimental; not a production route).** Promoted by a
Spider menu-index census in the downstream training repository on 2026-07-29.
This document is the durable design contract, not a paper claim.

**Tranche progress:** T1–**T12 complete** and present in this tree. Post-T12 **maintainer review repairs** and the **maintainer
acceptance verification tranche** (checked-in T11/T12 artifact parity tests,
strict malformed decode coverage, `git_head_sha` provenance cwd, strict-clippy
cleanup, report refresh, restored n+2/A2 claim-boundary wording) also landed.
Further claim-bearing work is **models-side** (claim corpus, real A1/A2/A3/A5
capability, authorization) — **not** CUDA availability or the old M7 replay.

## Why this frontend exists

Compiler elision and XGrammar jump-forward remove the same forced tokens. A
compiler-derived stream wins where it replaces an **open-vocabulary span with a
bounded menu index**. The models-side census measured:

| axis | result | bar |
|---|---|---|
| Schema-ref token share (Spider val, n=1034) | **34.61%** | ≥25% **PASS** |
| Global out-of-menu site rate (schema-prompted Qwen2.5-0.5B, n=200) | 6.50% | ≥10% fail |

Promote required **(a) OR (b)**; it rests on **(a)**, which passed. Bars were
not softened, and (b) must not be cited as a correctness win — it failed its
site bar. Menus are small (median ~4 tables, ~19 columns). “Column of the table
currently in scope” is context-sensitive and **not** CFG-expressible — the point
of a compiler front end over XGrammar. Fuller rationale, including why
Kubernetes was falsified and why Go was **not**, is in
the research strategy notes § “Why menu-index schema linking, and what it does
not falsify”.

Go remains the from-scratch pretraining north star. SQL is a **finetuning-track
schema-linking** frontend authorized by census, not a replacement for Go
syntax TCT.

## Claim boundary

```text
sql_schema_linking_frontend_contract_v0
compiler_sql_schema_linking_only
```

In scope:

- Parse/lower SQL (SQLite dialect first; Spider) under a **fixed DB schema**.
- Decision stream where table and column name sites are **menu indices**.
- **Scope-tracking** column menus (not global catalog-only).
- Encode/decode round-trips and SharedAlphabet / mixed-payload reuse.
- Handoff artifact consumable by the downstream training repository
  (the downstream SQL frontend handoff notes there).

Out of scope for v0:

- Full SQL dialect coverage (Postgres/MySQL extensions, procedural SQL).
- Training, paper numbers, or models-side eval harness ownership.
- Softening census bars.
- Bespoke codecs that ignore SharedAlphabet / generic codec layer.

## Required surfaces

1. **Schema pin** — Spider-style `tables.json` (or equivalent catalog) with
   stable sha; per-`db_id` table/column menus.
2. **Syntax/IR lower** — hand-built `GrammarIr` → `StructuralModel` →
   `CompiledSyntaxModel`, per **P1/P2** below. SQLite Spider subset is enough
   for v0: SELECT/FROM/JOIN/WHERE/GROUP/ORDER/subquery/aggregation, laddered
   M0–M6 in the tranche table.
3. **Menu sites**
   - Table site: menu = tables in the pinned schema.
   - Column site: menu = columns of **tables currently in scope** (FROM/JOIN
     aliases resolved). Global catalog union is diagnostic only.
4. **Non-menu surface** — keywords, operators, literals remain ordinary
   tokens or host/payload bands per existing mixed-payload policy.
5. **Aliases** — alias *definitions* are structural; alias *uses* as
   qualifiers are not open-vocab catalog names (same census convention).
6. **Star (`*`)** — not a menu-indexable column name; keep structural.
7. **SharedAlphabet** — default layout stays PerGrammar for pins; Shared mode
   must not require a one-off SQL codec. Concretely: menus map onto the
   existing `u8_integer_payload` band with a 256 ceiling and fail closed above
   it (**P9**).
8. **Decode** — tokens → canonical SQL (or IR→SQL) that Spider execution
   evaluation can run.

## Pre-authorized decisions (do not re-litigate)

These are **maintainer decisions already taken**. They are not open design
questions, and hitting one is **not** an architecture conflict — hard blocker
(6) in the retired SQL frontend handoff notes must not fire on anything settled
here. Implement them as written; if evidence contradicts one, record the precise
conflict on the branch and stop rather than silently substituting a design.

### P1. Parser: hand-written recursive descent, in Rust

There is no tree-sitter crate dependency in this repository, and `DEVELOPMENT.md`
forbids new tree-sitter language/grammar pins. The models-side census used
Python `sqlglot`, which is not in the container image and is not admissible in a
core path (`DEVELOPMENT.md`: core functionality is Rust-first).

Write a hand-written recursive-descent SQLite-subset parser in Rust, structured
like the `Parser` skeleton in `src/toy_grammar_frontend.rs` (byte cursor, typed
positional error carrying rule and offset), plus a hand-built `GrammarIr`.

### P2. Codec: generic layer, not a bespoke encoder

`src/toy_grammar_frontend.rs` is the model for the **parser only**. Its
`encode_program` / `decode_tokens` are a bespoke codec that never reaches
`CompiledSyntaxModel`, which is exactly what this contract forbids.

The structural template for the whole module is
`src/tree_sitter_json_subset_codec.rs`: hand-built `GrammarIr` → own token
layout → `lower_grammar_ir_to_structural_model` → `CompiledSyntaxModel` →
`encode_complete_tree` / `decode_complete_tree`.
`docs/generic-codec-layer-contract.md` already draws the boundary: encode and
decode are generic; `parse` and `render` stay language-specific. SQL contributes
parser, grammar, catalog, and renderer — nothing else.

### P3. Menu-ness is a compiled-model binding, not a terminal kind

**Do not add a `GrammarTerminalKind` variant.** `src/grammar_ir.rs` is untouched
by this campaign. A new variant breaks 9 exhaustive matches across
`grammar_ir_structural_model.rs`, `syntax_state_machine/model_facts.rs`,
`syntax_state_machine_replay.rs`, and `syntax_go_source_instance_projection.rs`,
silently mis-handles ~7 wildcard and `is_payload()` sites, and — because the
enum is serde `snake_case` — changes the JSON wire form of every checked sidecar
fixture, including `go-grammar-artifact-pin-v1.json`. That would drag the lane
into editing golden pins, which is hard blocker (2).

Instead, menu sites are declared as ordinary `IntegerPayload` leaves in the
`GrammarIr`, and menu-ness is a binding keyed by `(owner_rule, owner_field)` —
the same keying `ModelFacts::mixed_payload_fields` already uses.
`PayloadState` already carries both fields, so the key is available exactly
where it is needed.

### P4. The index is a scope-relative ordinal

Not a catalog-global column id:

- **Table site:** ordinal into `catalog.tables` (catalog order).
- **Column site:** ordinal into the concatenation, over tables in scope in
  FROM/JOIN order, of that table's columns in catalog order.

This keeps `VOCAB_SIZE` **catalog-independent**: one layout pin serves every
`db_id`, and adding a Spider database later is data rather than a token
renumber. A catalog-global scheme would need a per-database vocabulary, which
defeats the single-tokenizer handoff. This is the same field-relative identity
principle already settled for I5's `CHOICE(n, k)`, where one absolute token
legitimately means different things on different fields.

Carry the menu band as a dedicated `menu_index_payload` role resolved through
the **existing** `layout_range_by_role_any` helper into a new
`SyntaxTokenRoles::menu_range: Option<..>`. Menu values are `u32` in a dedicated
`PayloadState` field and **never** flow through `payload.bytes`.

### P5. Scope lives on the existing node stack

`FrameState` gains `scope_tables` on frames whose rule is a scope root. No
parallel stack, no side state. `state_from_prefix` replays from scratch, so
anything derived during replay is prefix-recoverable by construction — which is
the property that makes the mask honest.

### P6. From-first field order in the GrammarIr

`SELECT name FROM customers` puts the column site before the FROM in surface
text. Do **not** conclude from this that prefix-determined scope is impossible;
that reading is the single most likely way to deadlock this campaign.

The canonical stream is a **tree serialization, not surface text**. Declare
`select_statement` fields in this order:

```text
from (req), where (opt), group_by (opt), having (opt),
order_by (opt), limit (opt), select_list (req)
```

The renderer emits SQL keyword order. Because
`collect_open_field_admissible_tokens` advances `field_index` strictly in
declared order, no stream can present a column site before its FROM:
scope-before-use becomes a structural invariant of the compiled model rather
than a convention. Required surface 8 already licenses this ("tokens → canonical
SQL **or IR→SQL**").

### P7. Aliases and `*`

Because the column ordinal is scope-wide it uniquely identifies
`(table, column)`. Therefore qualifier *uses* (`customers.name`, `c.name`) need
**no token at all** — the qualifier is derived at render time. Alias
*definitions* are a structural marker; v0 renders canonical `t0, t1, …` and
does not preserve the original alias spelling. State that in the handoff.

`star` is a field-less rule under a multi-alternative `select_list`: one
structural choice token, zero menu tokens. Same treatment for `COUNT(*)`.

**Keyword-spelled column names.** A catalog may spell a column exactly as a
subset keyword (Spider has `yelp.checkin.count` and `railway.train.From`). Such
a column is an ordinary menu ordinal and must stay reachable:

- Parse disambiguates by grammar shape, never by a name list: an aggregate is
  `FUNC ( … )`, so a bare `COUNT` / `SUM` / `AVG` / `MIN` / `MAX` with no
  following `(` is a column reference.
- Render quotes any chosen identifier spelling in `SQL_SUBSET_KEYWORDS`
  (`src/sql_schema_linking/render.rs`) so the fixpoint reparse cannot re-lex it
  as a keyword. That constant is a **superset** of the parser's keyword
  literals; over-inclusion only adds quoting and is always safe.
- Two residuals are accepted, both quoted-only and both grammatically genuine
  (real SQL needs quoting there too). Neither exists in any Spider catalog:
  - a column named `DISTINCT`, because bare `SELECT DISTINCT FROM t` is the
    DISTINCT modifier with an empty select list;
  - a column named `SELECT` inside a parenthesized atom — `WHERE x = (select)`
    and `x IN (select)` open a subquery. The select-list, WHERE, GROUP BY, and
    ORDER BY positions all reach it bare.

The blast radius is bounded by `census_keyword_named_columns`
(`src/sql_schema_linking/spider_coverage.rs`), which probes every keyword-named
column in a `tables.json` through the real frontend path under both `Flat` and
`Factored` column addressing. Its always-on coverage is the synthetic
all-keyword catalog; the host Spider sweep runs only where a `tables.json` is
mounted (`TCT_SPIDER_TABLES_JSON`). Under-inclusion in `SQL_SUBSET_KEYWORDS` is
caught mechanically by the reserved-word sweep in `render.rs`
(`subset_keyword_list_covers_every_word_that_breaks_a_bare_column_reference`).

### P8. Round-trip claim: tree identity plus render fixpoint

The claim is `decode(encode(parse(sql)))` equals `parse(sql)` (tree identity)
**and** `encode(parse(render(tree)))` equals `encode(tree)` (render fixpoint).
Byte-identity with arbitrary input SQL is **not** claimed and must not be
chased.

Canonical form, fixed so it is not rediscovered every tranche:

- Qualify a column iff the scope holds **two or more** tables.
- An unqualified column name must match **exactly one** in-scope column; if it
  matches more than one in-scope table, parse **fails closed**. Qualified names
  resolve to the named table exactly.
- Render emits precedence- and associativity-aware parentheses so
  GrammarIR-decodable trees reparse with tree identity: OR < AND < comparison
  < atom; AND/OR are left-associative (right children of equal precedence are
  parenthesized); nested comparison operands are parenthesized (non-associative
  in this subset).

### P9. Explicit do-not list

Each entry maps to a concrete stall or damage mode observed while designing
this campaign:

- **No production route.** Do not edit `SYNTAX_CODEC_REGISTRY` in
  `src/syntax_production_route.rs` or
  `tests/fixtures/syntax_backend/production-route-manifest-v1.json`. Registering
  SQL forces lockstep count-parity edits in two large gate tests for no v0
  benefit.
- **No shared-alphabet revision.** Do not change any `SHARED_ALPHABET_*`
  constant, and do not raise `SHARED_ALPHABET_MAX_BRANCH_ARITY` (40, below
  realistic join menus) to fit menus into `CHOICE(n, k)`. Under SharedAlphabet,
  menus map onto the existing `u8_integer_payload` band (width **256**). Do
  **not** widen that band. Cardinalities above 256 use **multi-digit radix**
  over the same band (`max_menu_digits`, SQL = 2 → capacity 65_536) and fail
  closed only when `limit > width^max_menu_digits`.
- **SQL shared payload pool is intentional.** SQL's own layout
  (`sql-schema-linking-token-layout-v1`) co-locates utf8, integer, and menu
  roles on one 256-wide pool (vocab 321). That is an **SQL-only** opt-out of
  cross-language bands — do **not** edit Go/C# pins to match. See
  `docs/sql-shared-payload-pool-v1.md`.
- **Do not touch the shared Go/C# u8/UTF-8 payload path.** Helpers such as
  `payload_token_range_for_kind`, `is_u8_integer_range`,
  `payload_token_byte_for_candidates`, `emit_u8_integer_payload_bytes`, and
  `emit_utf8_payload_bytes` are shared with Go and C#. Widening them to carry
  values above 255 silently changes those targets. SQL's pool collapse remaps
  **SQL layout ranges only**.
- **No generated-codec source emission for SQL.** `src/syntax_generated_codec.rs`
  is very large and Go-centric; v0 stops at `CompiledSyntaxModel`.
- **Fixtures before external data.** Extend
  `tests/fixtures/sql_schema_linking/` with additional synthetic `db_id`s and
  gold queries — updating `fixture_pin.json` shas in the same commit — rather
  than requiring a host Spider corpus. Bounded gold stats over in-repo fixtures
  satisfy the checklist. Blocker (7) applies only after in-repo fixtures are
  genuinely exhausted.
- **Fix the menu band width once.** Keep `SQL_MENU_BAND_WIDTH = 256` pinned.
  Catalog capacity is `width^max_menu_digits` (SQL: 65_536), not a wider band.
  Catalogs or scopes exceeding that capacity fail closed at
  binding-construction time with a typed error and are recorded as an explicit
  statistic. Widening it after the pin lands is a `tct-token-diff` breaking
  change needing maintainer sign-off.
- **Subqueries are uncorrelated-only in v0.** Correlated column references are
  an explicit non-goal; record them as parse failures in the gold stats rather
  than extending scope resolution to reach outward.

### P10. Non-regression duty on shared surfaces

Every tranche that touches `src/syntax_state_machine/raw_replay.rs` or
`src/syntax_state_machine/encode.rs` must carry a test proving that models
compiled **without** a menu binding produce bit-identical roles and masks, and
must escalate to `cargo test` plus `./scripts/testing/test-syntax-backend-gates.sh`
rather than the focused syntax gate. Menu code paths must be unreachable when
no binding is supplied — guard on an `Option` and an empty binding, mirroring
the existing empty-`mixed_payload_fields` fast-out.

A new SQL token layout is naturally additive: the Go and C# pin tests are
hard-keyed to their own layout constructors and fixture paths, and there is no
registry-wide sweep requiring every layout to be pinned.

## Acceptance checklist (CPU)

Checklist boxes are **progress markers**, not early-exit gates. The live lane
continues until the full checklist is done or a hard blocker in
the retired SQL frontend handoff notes applies.

- [x] Fixture DB schemas (≥2 tables, FK join) with gold SELECT/JOIN queries.
      In-repo seed: `tests/fixtures/sql_schema_linking/`. **T1:** pin-verified
      catalog loader in `src/sql_schema_linking/catalog.rs` (3 tables, FKs,
      `*` dropped). **Host catalogs:** `SqlCatalog::load_unpinned_tables_json`
      / `list_db_ids` load multi-db Spider-style `tables.json` without pin
      digests for dry-run smoke and coverage (not claim-bearing official pins).
      every public pinned load revalidates `FixturePin`; PK indices fail closed
      (OOB, `*`, duplicates); both digests are 64-digit hex (case-agnostic);
      pin paths reject absolute/`..`; `load_fixture_*` requires fixture
      `db_id`; `load_fixture_bundle` verifies gold_queries digest. Encode/decode
      is later.
- [x] Complete v0 `GrammarIr` shape (**T3**): `src/sql_schema_linking/grammar.rs`
      from-first `select_statement` (P6), menu sites as IntegerPayload on
      `table_ref`/`column_ref` (P3), field-less `star`/`count_star` (P7), full
      M0–M6 ladder rules; lowers with SQL layout and compiles to
      `CompiledSyntaxModel`. Parser/encode still later.
- [x] Encode/decode inverse on single-table fixtures (**T5**): M0 parser,
      `tree.rs`/`render.rs`/`codec.rs` via generic `encode_complete_tree` /
      `decode_complete_tree`; gold `select_one_table` round-trips (tokens < PAD,
      one menu token per column site, decode→render fixpoint); `star` emits no
      column menu token.
- [x] Scope-tracking column menus + JOIN (**T6**): `scope_tables` on scope-root
      frames; table sites push; column menu from live scope; gold
      `join_two_tables` **all four** column-site masks cardinality **7 not 11**
      (no products; SELECT + ON sites). Pre-M6: **repeated table ordinals**
      fail closed in mask, transition, encode, and parse (no occurrence aliases).
      Fail-closed out-of-menu is T7.
- [x] Fail-closed on unknown menu index; full mask-equals-accept probe (**T7**):
      out-of-menu rejected by mask + transition; encode rejects out-of-scope
      column ordinals; binding rejects band overflow; gold prefix
      mask-equals-accept over `0..vocab_size` with **non-vacuous** proper-prefix
      errors and **full-vocab** completed-stream rejection (not a 64-token sample).
- [x] WHERE / GROUP / HAVING / ORDER / LIMIT golds (**T8**): M2 comparisons,
      AND/OR, integer/string literals; M3 GROUP BY / HAVING / ORDER BY / LIMIT
      (u8 band); golds `where_eq_string`, `where_cmp_and_or`, `join_with_where`,
      `group_order_limit`, `where_group_having_order_limit` round-trip; pin sha
      updated. Scope menu cardinality was already covered by T6 (7 not 11).
      **P8 parentheses:** render emits precedence- and associativity-aware
      parens so `AND (OR)`, `(OR) AND`, right-nested AND/OR, and nested
      comparison operands reparse to the same structure (render fixpoint).
- [x] Aggregation / DISTINCT / uncorrelated subquery (**T9**): `COUNT(*)` emits
      no column menu token; SUM/DISTINCT golds round-trip; scalar subquery gold
      opens inner scope (mask cardinality 4 = orders only); outer-only columns
      rejected at parse and mask. Golds: `count_star`, `sum_aggregate`,
      `select_distinct`, `scalar_subquery`. Encode scopes are **root-local**
      (nested subquery does not inherit/leak outer `scope_tables`). GrammarIR
      `aggregate_call.arg` is column-only; star under non-COUNT fails closed
      (dedicated `count_star` path only).
- [x] SharedAlphabet / mixed-payload reuse (**T10**): opt-in
      `for_catalog_with_mode(SharedAlphabet)`; menus map onto existing
      `u8_integer_payload` band (256 ceiling fail-closed, **P9**); gold
      round-trips and mask-equals-accept under shared mode (same non-vacuous
      full-vocab proof as T7); no one-off codec and no `SHARED_ALPHABET_*`
      constant changes. PerGrammar remains pin default.
      `count_menu_tokens` is **PerGrammar-only** (returns `None` when menu
      aliases the integer band); SharedAlphabet / mixed streams use
      `count_menu_tokens_by_role_replay`.
- [x] Bounded gold stats + menu-vs-baseline win (**T11**):
      `src/sql_schema_linking/report.rs`; in-repo golds 13/13 parse+round-trip;
      56 menu schema-ref tokens vs 502 baseline (≈8.96×); parse failures
      recorded explicitly; summary
      `reports/sql-schema-linking-gold-stats-v0.{json,md}` (`no_training_claims`).
- [x] Scope menu unit tests: after `FROM t1 JOIN t2`, column site menu ⊆
      cols(t1)∪cols(t2), not the full catalog when other tables exist.
      (T6 join mask cardinality 7 not 11; T9 subquery inner-only card 4.)
- [x] Handoff JSON (**T12**):
      `reports/sql-schema-linking-frontend-handoff-v0.{json,md}` — vocab/PAD,
      layout + schema pin shas, example stream hashes, decode→SQL path,
      SharedAlphabet note, `no_training_claims: true`, not a production route.
- [x] Models gate script path documented in the downstream training repository
      (the downstream SQL frontend handoff notes points at the checked-in handoff).

### Tranche ladder (T1–T12)

One commit-sized tranche at a time, TDD where practical: write the named test,
watch it fail for the expected reason, then implement. Reorder only when a later
item genuinely unblocks an earlier one. **T5 is the first end-to-end win and
T11 is the campaign milestone** — do not stop at either unless a hard blocker
applies.

Module layout, a directory from the start because the module-size report is
real: `src/sql_schema_linking/` with `catalog.rs`, `ast.rs`,
`parser/{mod,expr}.rs`, `grammar.rs`, `tree.rs`, `render.rs`, `codec.rs`,
`report.rs`.

| # | Adds | Test that must fail first | Gate |
| --- | --- | --- | --- |
| T1 | `catalog.rs`: load pinned Spider-style `tables.json`, verify sha256 against `fixture_pin.json`, drop the `[-1, "*"]` pseudo-column, resolve FKs, typed errors | catalog loads 3 tables, `orders` has 4 real columns, FK pairs resolve, mutated bytes fail the pin | focused |
| T2 | `sql_syntax_token_layout()` + `SQL_MENU_BAND_WIDTH`/vocab/PAD constants; own pin fixture under `tests/fixtures/syntax_structural_model/` | layout constructor matches its pin fixture; ranges validate; menu band does not overlap the u8 integer band | focused + `syntax_backend_boundary_test` to prove Go/C# pins untouched |
| T3 | `grammar.rs`: the **complete v0 `GrammarIr` shape**, from-first field order (**P6**) | lowering succeeds and `select_statement` orders `from` strictly before `select_list` | focused |
| T4 | Menu binding plumbing: `menu_range`, binding keyed by `(owner_rule, owner_field)`, menu branch in the admissible-mask and payload-apply paths — **table sites only**, no scope needed yet | table-site mask is exactly the catalog table menu and `transition` accepts exactly those; **plus** the P10 non-regression test | `cargo test` + syntax backend gates |
| T5 | **First end-to-end win.** M0 parser, `tree.rs`, `render.rs`, `codec.rs` on the generic encode/decode; column menu from a single-table FROM | gold `select_one_table` round-trips: every token below PAD, **exactly one token per column site**, decode→render byte-identical, reparse yields identical tokens; `star` emits no menu token | focused |
| T6 | Scope tracking: `scope_tables` on scope-root frames, push on table-site completion, column menu from scope, JOIN parse/render and ON predicates | on gold `join_two_tables`, every column-site mask has cardinality **7, not 11**, and no `products` column is admissible | `cargo test` + syntax backend gates |
| T7 | Fail-closed: typed out-of-menu and arity-overflow errors; binding rejects catalogs wider than the band | out-of-menu index rejected by **both** mask and transition; full `0..vocab_size` mask-equals-accept probe over every gold prefix | `cargo test` + syntax backend gates |
| T8 | M2+M3: WHERE with comparisons and AND/OR, literals, GROUP BY / HAVING / ORDER BY / LIMIT | new gold queries round-trip (append to `gold_queries.json`, update `fixture_pin.json` shas in the same commit) | focused |
| T9 | M4+M5: aggregation, `COUNT(*)`, DISTINCT, uncorrelated subquery scope frame | subquery opens an inner scope and outer-only columns are not admissible; `COUNT(*)` emits no menu token | focused |
| T10 | SharedAlphabet parity per **P9** | model round-trips under SharedAlphabet without a one-off codec; mask/accept holds in shared mode; above 256 fails closed | `cargo test` + syntax backend gates |
| T11 | **Milestone.** `report.rs` bounded gold stats (see below) | report records explicit parse failures and the menu-vs-baseline token comparison | focused + `./scripts/testing/test-syntax-backend-gates.sh` |
| T12 | Models handoff JSON; tick the checklist; update the research roadmap | handoff declares `no_training_claims: true` and pins layout + schema shas | `cargo fmt --all -- --check`, `cargo clippy --all-targets`, `cargo test`, `python3 -m pytest tests/python` |

Parser subset ladder, consumed by T5/T8/T9: **M0** single-table
`SELECT <column_ref+ | *> FROM <table>` → **M1** JOIN with ON → **M2** WHERE and
literals → **M3** GROUP/HAVING/ORDER/LIMIT → **M4** aggregation → **M5**
uncorrelated subquery → **M6** aliases. The **`GrammarIr` shape is designed once
at T3** even though the parser lands incrementally: adding fields later reshapes
the token stream, which is cheap now and a breaking change after the handoff.

### T11 milestone — the measurement that defines success

The campaign milestone is a bounded gold-stats artifact quantifying the
schema-linking win: **tokens per schema reference under menu-index encoding
versus the open-vocabulary UTF-8 baseline** on the same queries. The baseline is
what the existing payload path costs today — an n-byte identifier is n+2 tokens
(terminal, n byte tokens, payload end) — against exactly one token for a menu
site. This comparison is only a compiler-internal diagnostic and must not be
used as a paper-facing baseline or paper number; a paper-facing
residual-compression comparison must use the strongest steelmanned canonical-SQL
baseline from the menu-indexability criterion and the SQL fine-tuning experiment
contract kept in the downstream training repository (A2: parser/CFG,
jump-forward, and dynamic prefix-time scope masking).

Reuse rather than reinvent:

- the Go BPE compression-parity script (not included) for the report skeleton:
  `ARTIFACT_KIND`, an explicit `CLAIM_BOUNDARY` string, decision bands,
  `--self-test` alongside `--write`, `repo_commit`, `output_files`, and a
  `.json` + `.md` pair.
- `scripts/analysis/structure_only_bpe_measurement.py` for its per-position role
  bitmap, which maps almost one-to-one onto "menu site versus open-vocab payload
  span".

Report parse failures explicitly rather than silently excluding them; a
round-trip rate over a filtered corpus is not a result. Bulky streams stay under
`target/`; check in only the summary pair, per `reports/README.md`. The artifact
is packaging and measurement evidence only — it does not train, evaluate, or
support a paper claim.

## Non-goals / do not

- Do not start models fine-tuning before handoff.
- Do not merge to `main` without maintainer review (default branch policy).
- Do not cite axis (b) 6.5% as “correctness win” — it failed the site bar;
  promote rests on axis (a).
- Do not implement overnight breadth across all SQL dialects.
- Do not re-litigate or soften census bars; the promote decision is fixed.

## Cross-repo links

- SQL menu census and frontend handoff expectations: kept in the downstream
  training repository (not part of this repository)
- Stage-1 Rust/Python runtime (A5 + fail-closed A1/A2):
  `docs/sql-schema-linking-runtime.md`
- Queue entry: the research roadmap § SQL primary claim track (compiler-side done on
  `main`; claim-bearing Stage 1 is models-side)
- Shared alphabet: `docs/i5-shared-decision-token-alphabet-contract.md`
- Generic codec: `docs/generic-codec-layer-contract.md`
