# SQL Schema-Linking Runtime (stage-1 harness)

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Document path: `docs/sql-schema-linking-runtime.md`.

Status: **Implemented on `dispatch/sql-runtime-py`.** Rust-first A5 runtime with
Python (PyO3) bindings. A1/A2 text-prefix constraints are typed fail-closed in
this tranche. Claim boundary: `compiler_sql_schema_linking_only`. Not a paper
claim and not a production route.

## Purpose

Give the real stage-1 models harness a single compiled SQL frontend handle that:

1. Loads a Spider-style catalog JSON under a content SHA pin.
2. Caches the existing Rust parser, GrammarIR lower, StructuralModel compile,
   and menu binding once per catalog.
3. Exposes A5 encode / strict decode and incremental TCT decode (mask + apply).
4. Declares A1/A2 text-prefix constraints unavailable with a reusable blocker
   rather than faking surface-order masking on the from-first tree stream.

## Public surface

### Rust (`tct::sql_schema_linking`)

| Type / fn | Role |
| --- | --- |
| `SqlSchemaLinkingRuntime::from_catalog_json(json, sha256, db_id)` | Construct; fail closed on SHA mismatch or bad catalog |
| `SqlSchemaLinkingRuntime::fixture()` | In-repo join fixture (tests / local smoke) |
| `encode(sql) -> Vec<u32>` | Parse → expanded raw TCT stream |
| `decode(tokens) -> String` | Strict complete decode → rendered SQL + encode fixpoint |
| `begin_tct_decode() -> SqlTctDecodeState` | Incremental A5 mask/apply |
| `begin_text_constraint(pieces, dynamic_scope)` | Always `Err` (A1/A2 blocked) |
| `capability_report()` | A5 true; A1/A2 false + blocker |
| `TextTokenPieceMap` | Validated base-token-id → UTF-8 piece map bound to an exact 64-hex SHA-256 tokenizer hash |

`SqlTctDecodeState` methods: `allowed_token_ids()`, `apply(token)`,
`is_complete()`, `consumed_tokens()`, `applied_transitions()`, `render_sql()`,
`tokens()`, `active_site()`, `active_menu()`, `active_menu_bindings()`.
Incremental state owns a live `RawSyntaxLiveState`
cursor (O(one transition) per `apply`; not full-prefix replay). `render_sql`
uses the same strict complete-decode + encode-fixpoint contract as batch
`decode`.

### Python (`tct` extension, features `python` / `python-extension` plus `experimental`)

```python
from tct import SqlSchemaLinkingRuntime, SQL_TEXT_CONSTRAINT_BLOCKER_CODE

rt = SqlSchemaLinkingRuntime(catalog_json, catalog_sha256, db_id)
# or: rt = SqlSchemaLinkingRuntime.fixture()

tokens = rt.encode("SELECT name FROM customers")
sql = rt.decode(tokens)

st = rt.begin_tct_decode()
for t in tokens:
    assert t in st.allowed_token_ids()
    st.apply(t)
assert st.is_complete()
assert st.render_sql() == sql

# Elided free-choice walker (clone for speculative base-space probes).
elided_st = rt.begin_elided_tct_decode()
probe = elided_st.clone()

# BPE-space walker: merge_table_path is a filesystem path (not JSON text).
# Delegates to elided_bpe_mask; PAD is never admissible. Layout exact_reuse.
bpe_st = rt.begin_elided_bpe_tct_decode(merge_table_path, merge_count)
for bpe_id in bpe_token_ids:
    assert bpe_id in bpe_st.allowed_token_ids()
    base_expansion = bpe_st.apply(bpe_id)  # elided base tokens applied
assert bpe_st.is_complete()
assert bpe_st.render_sql() == sql

caps = rt.capability_report()
assert caps["a5_encode_decode"] and not caps["a1_text_structural_constraint"]
# A1 (dynamic_scope=False) and A2 (True) both raise:
rt.begin_text_constraint({1: "SELECT"}, "a" * 64, dynamic_scope=False)
```

Full command reference for the elided-BPE binding lives in `DEVELOPMENT.md`
("SQL elided-BPE PyO3 API").

### Opt-in qualified menu candidates

`active_menu_bindings()` is available on expanded, elided, and Python BPE
decode states. It returns the same fields and values as `active_menu()`, plus
a `bindings` list. The existing `active_menu()` dictionary and its
`referents: [{"ordinal", "name"}]` entries keep their historical shape.
Rust returns `runtime::SqlActiveMenuBindings { menu, bindings }`; serialization
flattens `menu` into the top-level object, matching Python.

Each binding has these fields:

| Field | Meaning |
| --- | --- |
| `token_id` | Admissible **base** token ID for this candidate at the current digit |
| `ordinal` | Full menu ordinal, with the same meaning as `referents[].ordinal` |
| `table_ordinal`, `table_name` | Zero-based catalog table index and its `table_names_original` spelling |
| `column_ordinal`, `column_name` | Zero-based index within that table's real columns and its `column_names_original` spelling; both `None` for table choices |

The indices belong to the runtime's pinned catalog bytes and `db_id`.
`column_ordinal` excludes Spider's `*` pseudo-column and is not the global
Spider column index. A `table` or factored `column_table` candidate identifies
only a table. A factored `column` candidate uses the compiler cursor's
already-consumed parent table digit, including a parent consumed implicitly
by forced elision. A `column_flat` candidate resolves through the cursor's
FROM/JOIN scope order. Duplicate column names therefore remain unambiguous;
names are never used to infer a parent table.

The snapshot describes candidates for the **next** base choice. Leading wide
radix digits remain unresolved with empty `referents` and `bindings`; the
final digit returns full ordinal identities. For example, in a base-256 menu,
prefix `1` followed by digit `43` selects ordinal `299`: `token_id` encodes
digit `43`, while `ordinal` is `299`. Completed menu payloads and non-menu
positions have no bindings. Factored column addressing remains two positional
digits, each bounded by the existing menu band; this API does not extend its
capacity.

```python
snapshot = state.active_menu_bindings()  # before choosing the next base token
chosen_token = choose_from_mask(state.allowed_token_ids())
selected = next(
    (b for b in snapshot["bindings"] if b["token_id"] == chosen_token), None
)
state.apply(chosen_token)
# A contextual input binder may carry `selected` into subsequent positions.
```

The selected identity becomes available only after that choice. It must not
be inserted into features predicting its own target, including during
teacher-forced training. Calling the API after `apply` describes the new
site, not the previous choice. On a BPE walker, this snapshot covers only the
next base decision between BPE tokens: `token_id` is not a BPE ID, and a
merge may cross later sites the snapshot does not describe. The API performs
no state mutation and changes no vocabulary, layout, mask, or stream contract.
It provides compiler metadata only; it makes no model-quality claim.

## Correctness contract (A5)

- `capability_report().sql_text_contract` is
  `sqlite_quoted_identifiers_no_nul_v1`. Catalog identifiers always use SQL
  double quotes with embedded quotes doubled. Original catalog spellings take
  precedence over display spellings. String literals preserve their text with
  doubled single quotes; NUL literals fail closed in source parsing, AST
  encoding, strict decoding, rendering, and incremental mask/apply admission.
  NUL catalog identifiers also fail during rendering. This is a SQL text
  restriction, not a restriction on generic UTF-8 or integer/menu payload zero.
- Reuses `SqlSchemaLinkingCodec` (parser, tree, generic encode/decode, render).
  No second SQL grammar and no Python tokenizer clone.
- Fail closed on: wrong catalog SHA, malformed SQL, tokens ≥ `VOCAB_SIZE` or
  `PAD`, incomplete strict decode, extra tokens after completion, invalid
  token-piece maps (tokenizer hash not exact 64-hex SHA-256, non-UTF-8 pieces,
  duplicate ids).
- Encode/decode preserves tree identity and render fixpoint (same as codec
  `round_trip_source`). Batch `decode` and incremental `render_sql` share the
  strict complete-decode + encode-fixpoint contract.
- Incremental A5 state advances a live owned raw-syntax cursor against the
  immutable compiled model (one transition per apply). Regression coverage
  asserts `applied_transitions == N` after N applies (not triangular
  full-prefix replay cost).
- Incremental prefix masks equal apply acceptance over focused fixtures,
  including multi-digit menus on the wide capacity catalog.
- A5 column masks are already live-scope relative (JOIN excludes out-of-scope
  products columns). That is TCT menu scope, not A2 text masking.

## A1/A2 blocker (reusable)

Code: `sql_tree_stream_order_diverges_from_surface_text`

The SQL GrammarIR uses **from-first** field order (**P6** in
`docs/sql-schema-linking-frontend-contract.md`): the canonical TCT stream is a
tree serialization (`from` before `select_list`), while surface SQL places the
SELECT list before FROM. A1/A2 operate on conventional-tokenizer prefixes of
surface text. Driving those prefixes from the raw syntax state machine would
either admit identifiers before scope exists (unsound A2) or require a second
surface-order constraint grammar (forbidden clone).

Until a dedicated surface-aligned constraint engine is designed as a separate
tranche, `begin_text_constraint` fails closed for both:

- `dynamic_scope=false` — A1 structural text constraint
- `dynamic_scope=true` — A2 live table/column scope text constraint

A2 must not degrade to a global catalog allow-list; this surface refuses rather
than degrade.

## What this does not change

- SQL token layout pin, `VOCAB_SIZE`, menu band width
- Go / C# layout pins and shared alphabet IDs
- JSON Schema compiler behavior
- Models / paper claim documents

The SQL text contract changes canonical rendered strings and removes NUL from
literal masks. Token IDs and the spelling of supported NUL-free streams do not
change, but that does not establish experiment or artifact compatibility.
Provenance must record the SQL text contract and exact compiler/wheel revision;
existing pinned wheels, corpora, rendered gold text, predictions, and scores
must not be silently replaced. A wheel rebuild and a separate impact audit are
required before a campaign adopts this contract. Compare catalog identifiers,
NUL-containing literals, accepted row membership, token streams, and external
SQLite results on frozen outputs; separate changed SQL spelling from changed
denotation or execution. This bounded repair does not prove every possible
admitted SQL stream has valid SQLite syntax or the intended query semantics.

## Validation

```bash
cargo test --features experimental --lib sql_schema_linking
cargo test --features experimental --lib sql_schema_linking::runtime
cargo test --features experimental --test sql_renderer_sqlite_test
# Qualified identities: duplicate names, permutations, elision, wide menus:
cargo test --features experimental --lib sql_schema_linking::runtime::menu_binding_tests
# optional: cargo fmt --all -- --check && cargo clippy --all-targets
./scripts/testing/test-syntax-backend-gates.sh
# Python extension smoke (requires the python and experimental features):
cargo test --features python,experimental --lib python
# Use an isolated extension built from the source under test; an absent
# extension skips this module and does not validate the binding:
python3 -m pytest tests/python/test_sql_active_menu_bindings.py -q
# or build/import the cdylib with --features python-extension,experimental per DEVELOPMENT.md
```

## Related docs

- Frontend contract: `docs/sql-schema-linking-frontend-contract.md`
- Non-JSON runtime boundary: `docs/non-json-runtime-boundary.md`
- Experiment arms: the SQL fine-tuning experiment contract in the downstream
  training repository (not part of this repository)
